# Plan: BDP Sync Service — Implementación completa

> **HISTÓRICO — NO EJECUTAR.** La sincronización automática y las pruebas con `OnlyCheck` aquí descritas fueron sustituidas por permisos de una sola operación, modo Solo lectura y simulación local.

> **Fecha:** 2026-06-07
> **Tarea:** 065A-4 (continuación)
> **Objetivo:** Implementar el flujo Glory → BDP: cuando se crea/actualiza una venta en Glory, crear la comanda correspondiente en el TPV (BDP-Net)
> **Principio:** Dry-run primero, escritura real solo cuando el cliente confirme

---

## Contexto: el problema del mapeo

Glory ventas son **monolíticas** (una descripción + un total). BDP comandas tienen **líneas de artículos** con códigos específicos. No existe tabla de mapeo Glory ↔ BDP.

**Solución propuesta:** Campo configurable `bdp_default_article_code` en ConfiguracionRestaurante. Por defecto, toda venta Glory se envía como un único artículo BDP con ese código. El cliente puede configurar un artículo genérico "Venta Glory" en BDP, o se puede extender a mapeo por-producto en el futuro.

---

## Fase 1: Configuración

### 1.1 — Nuevo campo en ConfiguracionRestaurante
- `bdp_default_article_code: String` (default: "GLORY")
- `bdp_default_article_name: String` (default: "Servicio Glory")
- Migration SQL para añadir columnas a tabla configuracion

### 1.2 — Migración SQL
```sql
ALTER TABLE configuracion
ADD COLUMN IF NOT EXISTS bdp_default_article_code TEXT NOT NULL DEFAULT 'GLORY',
ADD COLUMN IF NOT EXISTS bdp_default_article_name TEXT NOT NULL DEFAULT 'Servicio Glory';
```

---

## Fase 2: BdpSyncService

### 2.1 — Archivo: `src/services/bdp_sync.rs`

**Patrón:** Igual que HaddockService — servicio estático, sync en background, mutex por venta, retry con backoff.

**Métodos:**
- `sync_venta(pool, venta, config, is_update)` — orquesta el flujo completo
- `map_venta_to_order(venta, config) -> BdpCreateOrderRequest` — mapea Glory → BDP
- `send_order(client, order) -> Result<OrderId, Error>` — envía a BDP
- `update_sync_status(pool, venta_id, success, error)` — actualiza campos BDP en Venta

**Flujo:**
```
1. Verificar bdp_sync_enabled && bdp_configurado(config)
2. Lock por venta_id (prevenir duplicados)
3. Guard: si ya sincronizada y es create, saltar
4. Login a BDP
5. Construir Order: Type=0, OrderEndType=1 (pendiente), article=default
6. CreateOrder (OperationType=0, escritura real)
7. Si OK: update_venta_bdp_status(true, None)
8. Si Error: update_venta_bdp_status(false, error_msg)
9. Retry hasta 3 veces con backoff
```

### 2.2 — Mapeo Glory → BDP

| Campo Glory | Campo BDP | Transformación |
|---|---|---|
| descripcion | Items[0].Name | Directo (o default article name) |
| importe_base + importe_iva | Items[0].Price | Suma de ambos |
| iva_porcentaje | Items[0].VatPct | Directo |
| metodo_pago | TenderId | Mapeo: efectivo→1, tarjeta→2, etc. |
| comensales | (no hay campo directo) | Ignorar por ahora |
| fecha | ExecutionTime | Formato ISO |
| user_id → config.bdp_employee_id | EmployeeId | Configurado |
| config.bdp_pos_id | Order.PosId | Configurado |
| config.bdp_items_profile_id | ItemsProfileId | Configurado |

**Order.Type = 0** (Barra/Ticket aparcado) — único que pasa validación en POS 31.
**OrderEndType = 1** (pendiente de validación) — no se factura automáticamente, el TPV lo muestra como pendiente.

### 2.3 — Campos nuevos en Venta

```rust
pub bdp_synced: bool,
pub bdp_synced_at: Option<DateTime<Utc>>,
pub bdp_sync_error: Option<String>,
pub bdp_order_id: Option<i64>,  // OrderId devuelto por BDP
```

Migration:
```sql
ALTER TABLE ventas
ADD COLUMN IF NOT EXISTS bdp_synced BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS bdp_synced_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS bdp_sync_error TEXT,
ADD COLUMN IF NOT EXISTS bdp_order_id BIGINT;
```

---

## Fase 3: Hook en VentaService

### 3.1 — Modificar `VentaService::create()`
Añadir `Self::spawn_bdp_sync(pool.clone(), user_id, venta.clone(), false);`
(Igual que spawn_haddock_sync)

### 3.2 — Modificar `VentaService::update()`
Añadir `Self::spawn_bdp_sync(pool.clone(), user_id, venta.clone(), true);`

### 3.3 — `spawn_bdp_sync()` helper
```rust
fn spawn_bdp_sync(pool: PgPool, user_id: Uuid, venta: Venta, is_update: bool) {
    tokio::spawn(async move {
        let config = ConfiguracionRepository::obtener_o_crear(&pool, user_id).await...;
        BdpSyncService::sync_venta(&pool, &venta, &config, is_update).await;
    });
}
```

---

## Fase 4: API Endpoints

### 4.1 — Retry endpoint
`POST /api/ventas/:id/bdp-sync` — reintenta sync de una venta individual
(Igual que `/api/ventas/:id/haddock-sync`)

### 4.2 — Actualizar VentaRepository
- `update_bdp_status(pool, venta_id, synced, error, order_id)` — igual que `update_haddock_status`

---

## Fase 5: Tests

### 5.1 — Unit tests con wiremock
- `map_venta_to_order_uses_default_article` — mapeo correcto
- `map_venta_to_order_uses_venta_total` — importe correcto
- `sync_venta_skips_when_disabled` — no sync si bdp_sync_enabled=false
- `sync_venta_skips_duplicate_when_already_synced` — guard de duplicados

### 5.2 — Test de integración (dry-run)
- Crear servicio con OperationType=1 (OnlyCheck) para validar sin efectos reales
- Verificar que el payload construido pasa validación BDP

---

## Checklist de verificación

| # | Test | Criterio de éxito |
|---|------|-------------------|
| 1 | Migration SQL | Columnas creadas sin error |
| 2 | cargo check | Sin errores de compilación |
| 3 | cargo clippy | Sin warnings |
| 4 | cargo test | Todos los tests pasan |
| 5 | Dry-run BDP | Payload aceptado por BDP (OperationType=1) |
| 6 | Deploy + health | Servicio arranca correctamente |
| 7 | Crear venta → sync | Venta creada, sync en background, status actualizado |

---

## Nota para el cliente

> "La integración Glory → BDP está implementada. Cada venta que se registra en Glory crea automáticamente una comanda pendiente en el TPV de BDP. El restaurante ve la comanda en la consola de autocomanda y puede procesarla normalmente (cobrar, facturar). La sincronización usa un artículo configurable; se puede extender a mapeo por-producto cuando el cliente defina su catálogo."
