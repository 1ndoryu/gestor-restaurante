# Feature Flags BDP — Documentación

> **Fecha:** 2026-07-26
> **Propósito:** Referencia completa de los 6 feature flags BDP implementados en el proyecto.
> **Fuente:** Verificado contra `src/models/configuracion.rs`, `src/repositories/configuracion.rs`, handlers y servicios.

---

## Resumen

La integración BDP utiliza **6 feature flags** booleanos en la tabla `configuracion_restaurante`. Todos están **desactivados por defecto** (`false`) y deben activarse explícitamente por restaurante. **Solo aplican en modo `bdp`** (M12): con `modo_operacion = standalone` estos flags quedan inactivos y ocultos, ya que sus funciones dependen de la conexión BDP (ver secciones «Modo de operación» y «Permisos operativos»).

| # | Flag | Descripción | Default | Protege | Archivos clave |
|---|------|-------------|---------|---------|----------------|
| 1 | `ff_bdp_auto_arm` | Permite auto-arming (escritura temporal automática al operar) | `false` | Escrituras Glory→BDP | `bdp_write_guard.rs:56` |
| 2 | `ff_bdp_partial_payments` | Permite pagos parciales de comandas BDP | `false` | Pagos parciales | `bdp_sync.rs:1263` |
| 3 | `ff_bdp_cancel_order` | Permite cancelar comandas en BDP | `false` | Cancelación | (no expuesto — bloqueado por BDP) |
| 4 | `ff_bdp_purchase_notes_read` | Permite lectura de albaranes de compra BDP | `false` | Lectura albaranes | `bdp_purchase_note.rs:64,93` |
| 5 | `ff_bdp_purchase_notes_draft` | Permite crear borradores de compra locales | `false` | Borradores | `bdp_purchase_note.rs:187` |
| 6 | `ff_bdp_purchase_notes_receive` | Permite conciliar/recepcionar compras | `false` | Conciliación | `bdp_purchase_note.rs:235` |

---

## Detalle por flag

### 1. `ff_bdp_auto_arm`

**Qué hace:** Permite que las operaciones de escritura (crear comanda, pagar, facturar) activen automáticamente un arming temporal sin que el usuario navegue a Configuración. El arming se crea, consume y desarma en la misma operación.

**Dónde se verifica:** `src/services/bdp_write_guard.rs:56`
```rust
if !config.ff_bdp_auto_arm {
    return Ok(None); // no auto-arming, flujo manual
}
```

**Efecto cuando está `false`:** El usuario debe ir a Configuración → BDP → Seguridad → Activar escritura temporal antes de cada operación de escritura.

**Efecto cuando está `true`:** Al pulsar "Enviar a BDP" / "Pagar" / "Facturar", el sistema solicita confirmación dinámica, auto-arma, ejecuta y desarma.

**Relación con otros flags:** Independiente. No requiere otros flags.

---

### 2. `ff_bdp_partial_payments`

**Qué hace:** Permite pagar una comanda BDP en varios pagos parciales en lugar de un único pago completo.

**Dónde se verifica:** `src/services/bdp_sync.rs:1263`
```rust
if is_partial && !config.ff_bdp_partial_payments {
    return Err("Pagos parciales desactivados...");
}
```

**Efecto cuando está `false`:** Solo se permite un pago igual al saldo pendiente completo (±0.005).

**Efecto cuando está `true`:** Se puede pagar cualquier monto ≤ saldo pendiente. Cada pago se registra en la tabla `bdp_pagos` con idempotency_key.

**Requisitos previos:** La comanda debe estar sincronizada en BDP (`bdp_order_id` no nulo).

**Tabla asociada:** `bdp_pagos` — ledger local de pagos parciales.

---

### 3. `ff_bdp_cancel_order`

**Qué hace:** Permitiría cancelar comandas en BDP desde Glory.

**Estado actual:** **Bloqueado por BDP** — el endpoint `CancelOrder` devuelve "Subscripción no activada". El flag existe en la BD pero no tiene efecto práctico hasta que BDP active el módulo.

**Dónde se define:** `src/models/configuracion.rs:88`

**Efecto cuando BDP active el módulo:** Se expondría un endpoint `POST /api/ventas/:id/bdp-cancel` protegido por este flag.

---

### 4. `ff_bdp_purchase_notes_read`

**Qué hace:** Permite la lectura/sincronización de albaranes de compra desde BDP (`ExportPurchaseNotes`).

**Dónde se verifica:** `src/handlers/bdp_purchase_note.rs:64,93`
```rust
if !config.ff_bdp_purchase_notes_read {
    return Err(AppError::Validation("Lectura de albaranes BDP desactivada..."));
}
```

**Endpoints protegidos:**
- `GET /api/bdp/purchase-notes` — listar albaranes locales
- `POST /api/bdp/purchase-notes/sync` — sincronizar desde BDP

**Efecto cuando está `false`:** Los endpoints devuelven error 422.

---

### 5. `ff_bdp_purchase_notes_draft`

**Qué hace:** Permite marcar albaranes de compra como borradores locales (sin escribir en BDP).

**Dónde se verifica:** `src/handlers/bdp_purchase_note.rs:187`

**Endpoint protegido:** `POST /api/bdp/purchase-notes/:id/draft`

**Efecto cuando está `false`:** El endpoint devuelve error 422.

---

### 6. `ff_bdp_purchase_notes_receive`

**Qué hace:** Permite conciliar/recepcionar albaranes de compra (vincular con gastos existentes o crear nuevos).

**Dónde se verifica:** `src/handlers/bdp_purchase_note.rs:235`

**Endpoint protegido:** `POST /api/bdp/purchase-notes/:id/reconcile`

**Modelo asociado:** `BdpPurchaseNoteReconcileRequest` con campos `gasto_id` (opcional) y `crear_gasto` (boolean).

**Efecto cuando está `false`:** El endpoint devuelve error 422.

---

## Cómo activar los flags

### Vía API (recomendado)

```http
PUT /api/configuracion/bdp
Content-Type: application/json
Authorization: Bearer <token>

{
  "ff_bdp_auto_arm": true,
  "ff_bdp_partial_payments": true,
  "ff_bdp_purchase_notes_read": true
}
```

Solo los campos incluidos se actualizan (COALESCE en SQL). Los omitidos conservan su valor actual.

### Vía SQL directa (emergencia)

```sql
UPDATE configuracion_restaurante
SET ff_bdp_auto_arm = TRUE
WHERE user_id = '<uuid>';
```

---

## Flujo de rollout recomendado

1. Implementar feature en código, marcado como `false` por defecto.
2. Activar manualmente en el restaurante piloto desde Configuración.
3. Validar 1-2 semanas de uso real.
4. Si funciona, dejar como opt-in para otros restaurantes. No activar globalmente.

---

## Consideraciones de seguridad

- **Todos los flags son `false` por defecto.** El bootstrap (`BdpConfigBootstrapService`) no los modifica.
- **Activar un flag no habilita escrituras por sí solo.** También se requiere: `bdp_sync_enabled=true`, allowlist de destinos, y (para escrituras) arming o auto-arming.
- **Los flags no se exponen en logs ni en respuestas de error** al cliente. Solo son visibles en la respuesta de `GET /api/configuracion`.
- **Cambiar un flag no requiere redeploy.** Se actualiza en caliente vía API.

---

## Modo de operación (`modo_operacion`) — 128A-1/F1

Columna `modo_operacion` en `configuracion_restaurante` con valores `bdp` (default, integración
activa) | `standalone` (independencia total). En `standalone`:

- La app queda 100% operativa sin credenciales ni conexión BDP (catálogo local, stock local,
  compras locales, anulación local, historial, menús/packs locales).
- Los 6 feature flags booleanos y las funciones de sincronización quedan **inactivos y ocultos**
  (M12); el poller de pedidos ya guarda con `bdp_sync_enabled` y solo corre en modo `bdp`.
- El badge de la barra superior muestra «independiente» en lugar de «BDP: off/lectura/escritura».
- Cambiar a `bdp` con credenciales válidas restaura el flujo de sincronización completo
  (histéresis y degradación automática al caer BDP).

## Permisos operativos por acción — 128A-1/F8 (D8, M17)

Columnas `permisos_catalogo_edicion`, `permisos_stock_ajuste`, `permisos_albaranes_gestion`,
`permisos_anulacion_ventas` en `configuracion_restaurante`, con valores `admin` (default) |
`admin_trabajador` | `todos` y CHECK en BD. El enforcement es **backend** (M17): cada acción
sensible consulta el nivel configurado y el rol efectivo del usuario (`effective_role`), devolviendo
403 si no corresponde; la UI solo refleja el permiso.

- `admin`: solo el propietario (rol Admin).
- `admin_trabajador`: Admin y Trabajador (todo el staff autenticado).
- `todos`: cualquier usuario autenticado.

Acciones protegidas: CRUD de catálogo/mapeos (`CatalogoEdicion`), ajuste manual de stock
(`StockAjuste`), gestión de albaranes de compra (`AlbaranesGestion`) y anulación local de ventas
(`AnulacionVentas`). Se configuran desde Configuración → BDP → «Permisos operativos». Las
escrituras/sincronizaciones BDP (sync-prices, sync-tables, bdp-payment, bdp-invoice, etc.) no se
gatean con estos permisos porque siguen protegidas por los guards BDP existentes
(`bdp_sync_enabled`, modo `bdp`, feature flags y `BdpWriteGuard`).
