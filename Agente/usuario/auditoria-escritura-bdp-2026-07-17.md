# Auditoría de Riesgos — Sección 3 ESCRITURA BDP

> **Fecha:** 2026-07-17
> **Objetivo:** Analizar cada endpoint de escritura BDP para identificar riesgos, mitigaciones y proponer pruebas seguras.
> **Principio:** Ninguna prueba debe dañar ni alterar el funcionamiento normal del BDP/TPV.

---

## 🔒 Mecanismos de seguridad globales

### 1. `bdp_sync_mode = "read_only"` (GUARD PRINCIPAL)

**Ubicación:** `src/services/bdp_sync.rs` — 4 puntos de control:
- L78: `sync_venta()` → bloquea CreateOrder
- L815: `ensure_cliente_bdp_synced()` → bloquea auto-sync de clientes
- L969: `add_order_payment()` → bloquea pagos
- L1059: `invoice_order()` → bloquea facturación

**Comportamiento:** Si `bdp_sync_mode` está vacío o es `"read_only"`, el servicio **no ejecuta** la escritura. `sync_venta()` hace `return` silencioso (log info). `add_order_payment()` e `invoice_order()` devuelven `Err("BDP en modo solo lectura...")`.

**⚠️ HALLAZGO:** El handler `sincronizar_cliente_bdp()` en `src/handlers/bdp_customer_sync.rs` (L219) **NO tiene guard `read_only`**. Llama directamente a `BdpWeblinkClient::create_customer()` sin verificar el modo de sync. Esto significa que `POST /api/clientes/:id/bdp-sync` **ignora el modo read_only** y ejecuta `CreateCustomer` siempre que BDP esté configurado.

| Endpoint | Guard read_only | Ubicación |
|---|---|---|
| `sync_venta()` (auto) | ✅ Sí | bdp_sync.rs:78 |
| `ensure_cliente_bdp_synced()` (auto) | ✅ Sí | bdp_sync.rs:815 |
| `sincronizar_cliente_bdp` (manual) | ❌ **NO** | bdp_customer_sync.rs:219 |
| `add_order_payment()` | ✅ Sí | bdp_sync.rs:969 |
| `invoice_order()` | ✅ Sí | bdp_sync.rs:1059 |
| `retry_bdp_sync` (handler) | ✅ Indirecto (vía sync_venta) | venta.rs:268 |

**Mitigación actual:** Para testing con `sync_mode = "read_only"`, el riesgo es **bajo** excepto para el endpoint manual de clientes. **Para pruebas seguras, NO usar `POST /api/clientes/:id/bdp-sync`.**

### 2. Pre-write audit (`registrar_escritura`)

**Ubicación:** `src/services/bdp_backup.rs:315`

**Comportamiento:**
- Si `bdp_auto_backup_before_write = true`: inserta entrada en `bdp_audit_log` ANTES de la escritura BDP.
- Para `add_payment` e `invoice`: genera snapshot del estado actual de la orden (1 llamada a `GetOrder`).
- Si el audit falla: **log warning pero continúa** con la escritura. El audit es "best effort", no bloqueante.

**Riesgo:** Si `bdp_auto_backup_before_write = false`, no se genera registro ni snapshot. Escritura ocurre sin trazabilidad.

**Mitigación para pruebas:** Verificar que `bdp_auto_backup_before_write = true` antes de cualquier prueba de escritura.

### 3. Mutex por venta (`SYNC_LOCKS`)

**Ubicación:** `src/services/bdp_sync.rs:62`

**Comportamiento:** `LazyLock<StdMutex<HashMap<Uuid, Arc<TokioMutex>>>>` — un lock Tokio por `venta.id`. Si ya hay un sync en progreso para esa venta, el segundo intento hace `try_lock()` y retorna silenciosamente.

**Riesgo:** Muy bajo. Previene duplicación de comandas por la misma venta.

### 4. Reintentos con backoff exponencial

**Comportamiento:** `retry_send_order` reintenta hasta 3 veces con `sleep(1 << attempt)` segundos (0s, 1s, 2s). Errores de auth (`401/403`) no se reintentan.

**Riesgo:** Si BDP está lento, puede generar 3 intentos de CreateOrder. BDP usa `MarketplaceOrderId` para deduplicar, así que el riesgo de comandas duplicadas es bajo.

---

## 📋 Análisis por endpoint de escritura

### A. CreateOrder — `POST /API/Orders/Create`

**Handler:** `reintentar_sync_bdp` (POST `/api/ventas/:id/bdp-sync`) y auto via `spawn_bdp_sync`

**Flujo:**
1. Login BDP
2. Resolver artículo, contexto (tender, order_type, customer)
3. Pre-write audit log
4. `retry_send_order()` → `send_order()` → `build_order()` → `POST /API/Orders/Create`
5. Si OK: `UPDATE ventas SET bdp_synced=true, bdp_order_id=?`
6. Si error: `UPDATE ventas SET bdp_sync_error=?`

**Escritura en BDP:** Crea una comanda real en el TPV. `OperationType=0` (CheckAndCreate). `OrderEndType=1` (pendiente, no facturada, no impresa).

**Escritura local:** Actualiza `bdp_synced`, `bdp_order_id`, `bdp_order_status`, `bdp_sync_error`.

| Riesgo | Nivel | Mitigación | Prueba segura propuesta |
|---|---|---|---|
| Crea comanda real en TPV | 🔴 Alto | Guard read_only ✅ | Probar solo con `sync_mode != "read_only"` y venta de prueba |
| Comanda duplicada | 🟡 Medio | MarketplaceOrderId único + mutex por venta | Verificar 1 sola comanda en TPV tras sync |
| Artículo inexistente en BDP | 🟡 Medio | BDP devuelve error 300xxx, se guarda en `bdp_sync_error` | Verificar que el artículo default (1001) existe |
| Monto incorrecto | 🟡 Medio | Usa `importe_base + importe_iva` de la venta local | Crear venta con importe conocido, comparar en TPV |
| Serie incorrecta (300035) | 🟢 Bajo | Ya documentado y configurado (00031TI) | Verificar en TPV que la serie es correcta |
| CancelOrder imposible | 🟡 Medio | BDP devuelve "Subscripción no activada" | Documentado — no se puede cancelar vía API |

**Veredicto para pruebas:** ✅ SEGURO con `sync_mode != "read_only"` + venta de prueba con importe pequeño (ej: 1€). La comanda aparece como pendiente en el TPV y puede cerrarse manualmente.

---

### B. CreateCustomer — `POST /API/Customers/Create`

**Handler manual:** `sincronizar_cliente_bdp` (POST `/api/clientes/:id/bdp-sync`)
**Handler auto:** `ensure_cliente_bdp_synced` (llamado desde `sync_venta` si `bdp_auto_sync_customers = true`)

**Flujo manual:**
1. Login BDP
2. Generar código BDP: si tiene `bdp_customer_code` → reutilizar; si no → `900_000 + (uuid % 99_999)`
3. `POST /API/Customers/Create` con `Overwrite=true/false`
4. Si OK: `UPDATE clientes SET bdp_customer_code=?, bdp_synced=true`
5. Si error: `UPDATE clientes SET bdp_sync_error=?`

**Flujo auto (desde sync_venta):**
1. Si cliente ya tiene `bdp_customer_code` → retorna directo
2. ExportCustomers → obtener max code → next_code = max + 1
3. CreateCustomer con code = next_code
4. Guardar `bdp_customer_code` en cliente

**Escritura en BDP:** Crea o actualiza un cliente en la base de datos del TPV.

**Escritura local:** Actualiza `bdp_customer_code`, `bdp_synced`, `bdp_sync_error`.

| Riesgo | Nivel | Mitigación | Prueba segura propuesta |
|---|---|---|---|
| Crea cliente real en BDP | 🔴 Alto | Guard read_only en auto-sync ✅, **NO en handler manual** ⚠️ | Usar cliente de prueba con datos ficticios |
| Código BDP colisiona | 🟡 Medio | Handler: hash UUID (900k+). Auto: max+1 | Verificar que el código no existe previamente |
| Sobrescribe cliente existente | 🟡 Medio | `Overwrite=false` para nuevos, `true` para existentes | Verificar `bdp_customer_code` antes de sync |
| ⚠️ Handler ignora read_only | 🔴 Alto | **NO MITIGADO** — bug detectado | **No usar endpoint manual en pruebas con read_only** |

**Veredicto para pruebas:** ⚠️ PRECAUCIÓN. El endpoint manual **NO respeta read_only**. Para pruebas seguras:
1. Usar cliente de prueba (no el real del restaurante).
2. Usar datos ficticios (nombre "TEST DELETE ME").
3. O mejor: probar solo el flujo automático (crear venta con cliente asignado) que SÍ tiene guard.

---

### C. AddOrderPayment — `POST /API/Orders/Payment/Add`

**Handler:** `bdp_invoice` (POST `/api/ventas/:id/bdp-invoice`) con `amount + tender_id`

**Flujo:**
1. Login BDP
2. Pre-write audit + snapshot del estado actual de la orden
3. `POST /API/Orders/Payment/Add` con order_id, amount, tender_id
4. Si BDP devuelve `InvoiceNumber`: `UPDATE ventas SET bdp_invoiced=true, bdp_order_status='invoiced'`

**Escritura en BDP:** Registra un pago parcial contra una orden existente. Puede facturar automáticamente.

**Escritura local:** Marca `bdp_invoiced=true` si BDP factura.

| Riesgo | Nivel | Mitigación | Prueba segura propuesta |
|---|---|---|---|
| Pago real en TPV | 🔴 Alto | Guard read_only ✅ | Solo probar con sync_mode != read_only |
| Facturación automática | 🔴 Alto | BDP puede facturar al recibir pago | Verificar en TPV antes y después |
| Monto incorrecto | 🟡 Medio | amount viene del request del usuario | Enviar monto de prueba conocido (ej: 1€) |
| Doble pago | 🟡 Medio | No hay dedup — cada llamada registra nuevo pago | Verificar 1 solo pago en TPV |

**Veredicto para pruebas:** ✅ SEGURO con `sync_mode != "read_only"` + venta ya sincronizada + monto de prueba. Verificar en TPV que el pago aparece.

---

### D. InvoiceOrder — `POST /API/Orders/Invoice`

**Handler:** `bdp_invoice` (POST `/api/ventas/:id/bdp-invoice`) sin amount

**Flujo:**
1. Login BDP
2. Pre-write audit + snapshot
3. `POST /API/Orders/Invoice` con order_id, pos_id, employee_id
4. `UPDATE ventas SET bdp_invoiced=true, bdp_order_status='invoiced'`

**Escritura en BDP:** Factura la orden (emite ticket/factura). **OPERACIÓN IRREVERSIBLE** — una vez facturada, la orden no se puede des-facturar.

**Escritura local:** Marca `bdp_invoiced=true`.

| Riesgo | Nivel | Mitigación | Prueba segura propuesta |
|---|---|---|---|
| Factura real en TPV | 🔴 **CRÍTICO** | Guard read_only ✅ | Solo probar con orden de prueba que se pueda cerrar |
| Irreversible | 🔴 **CRÍTICO** | No hay rollback vía API | Aceptar que la orden quedará facturada |
| Serie incorrecta | 🟡 Medio | Configurada en POS 31 | Verificar serie antes de facturar |

**Veredicto para pruebas:** ⚠️ OPERACIÓN IRREVERSIBLE. Solo facturar una orden de prueba creada específicamente para esto. **NUNCA facturar una orden real del restaurante.**

---

### E. Polling — `GET /API/Orders/GetOrder`

**Handler:** `bdp_poll` (POST `/api/ventas/bdp-poll`) y `obtener_bdp_status` (GET `/api/ventas/:id/bdp-status`)

**Flujo:**
1. Busca ventas con `bdp_synced=true` y status no final
2. Por cada venta: `POST /API/Orders/GetOrder`
3. Mapea status: 0=pending, 1=accepted, 2=cancelled, 3=invoiced
4. `UPDATE ventas SET bdp_order_status=?`

**Escritura en BDP:** ❌ NINGUNA. Solo lectura.
**Escritura local:** Actualiza `bdp_order_status`.

| Riesgo | Nivel | Mitigación | Prueba segura propuesta |
|---|---|---|---|
| Sin riesgo BDP | 🟢 Nulo | Solo lectura | Llamar sin restricción |
| Actualiza status local | 🟢 Bajo | Solo campo de estado | Verificar que refleja BDP |

**Veredicto para pruebas:** ✅ TOTALMENTE SEGURO. No modifica nada en BDP.

---

### F. Sync catálogo — `ExportArticles` → upsert local

**Handler:** `sync_catalog` (en servicio)

**Flujo:**
1. `POST /API/Articles/Export` → lee catálogo completo de BDP
2. Upsert en tabla local `bdp_article_map`

**Escritura en BDP:** ❌ NINGUNA. Solo lectura.
**Escritura local:** Upsert en `bdp_article_map` (tabla Glory).

**Veredicto:** ✅ TOTALMENTE SEGURO. Solo lectura de BDP, escritura solo en BD local.

---

## 🐛 Bugs detectados en la auditoría

### BUG-1: `sincronizar_cliente_bdp` no tiene guard `read_only`

**Archivo:** `src/handlers/bdp_customer_sync.rs:219`
**Severidad:** 🔴 Alto
**Descripción:** El handler `POST /api/clientes/:id/bdp-sync` llama directamente a `BdpWeblinkClient::create_customer()` sin verificar `config.bdp_sync_mode`. Ignora el modo `read_only`.
**Impacto:** Un usuario puede crear clientes en BDP aunque el sistema esté en modo solo lectura.
**Fix propuesto:** Agregar guard antes de la llamada:
```rust
if config.bdp_sync_mode.is_empty() || config.bdp_sync_mode == "read_only" {
    return Err(AppError::Validation(
        "BDP en modo solo lectura. Cambia el modo en configuración para sincronizar clientes.".into()
    ));
}
```

### BUG-2: `retry_bdp_sync` siempre pasa `is_update=false`

**Archivo:** `src/services/venta.rs:276`
**Severidad:** 🟢 Bajo (comportamiento intencional)
**Descripción:** `retry_bdp_sync` pasa `is_update=false` a `sync_venta()`, lo que activa el guard "ya sincronizada". Si la venta ya tiene `bdp_synced=true`, el reintento se salta silenciosamente.
**Impacto:** Si una venta falló y quedó con `bdp_synced=false`, el retry funciona. Si quedó con `bdp_synced=true` pero con error, el retry no reenvía.
**Mitigación actual:** El flujo de error marca `bdp_synced=false` cuando hay fallo, así que el retry debería funcionar en la mayoría de los casos.

---

## 📊 Resumen de riesgos por operación

| Operación | Escritura BDP | Guard read_only | Reversible | Riesgo |
|---|---|---|---|---|
| CreateOrder | ✅ Sí | ✅ Sí | ❌ No (CancelOrder no funciona) | 🔴 Alto |
| CreateCustomer (auto) | ✅ Sí | ✅ Sí | ⚠️ Parcial (Overwrite) | 🟡 Medio |
| CreateCustomer (manual) | ✅ Sí | ❌ **NO** | ⚠️ Parcial | 🔴 Alto |
| AddOrderPayment | ✅ Sí | ✅ Sí | ❌ No | 🔴 Alto |
| InvoiceOrder | ✅ Sí | ✅ Sí | ❌ No (irreversible) | 🔴 Crítico |
| GetOrder (polling) | ❌ No | N/A | N/A | 🟢 Nulo |
| ExportArticles | ❌ No | N/A | N/A | 🟢 Nulo |
| ExportCustomers | ❌ No | N/A | N/A | 🟢 Nulo |

---

## ✅ Pruebas seguras propuestas (sin riesgo para BDP)

### Tier 1: Sin riesgo (probar siempre)
1. **Verificar guard read_only:** Con `sync_mode="read_only"`, intentar sync → confirmar que devuelve error/ignora.
2. **Polling:** `POST /api/ventas/bdp-poll` → solo lectura, actualiza status local.
3. **ExportArticles:** Solo lectura del catálogo BDP.
4. **Pre-write audit log:** Verificar que se inserta en `bdp_audit_log` al intentar escritura (aunque falle por read_only).

### Tier 2: Riesgo controlado (probar con cuidado)
5. **CreateOrder con venta de prueba:** Crear venta Glory con importe mínimo (1€), sync → comanda pendiente en TPV. Cerrar manualmente en TPV después.
6. **CreateCustomer con datos ficticios:** Cliente "TEST DELETE ME" → sync → verificar en BDP. Eliminar manualmente en TPV después.
7. **AddOrderPayment con monto mínimo:** Venta sincronizada + pago de 0.01€ → verificar en TPV.

### Tier 3: Operación irreversible (solo si el usuario autoriza)
8. **InvoiceOrder:** Facturar orden de prueba → irreversible. Solo si el usuario acepta que la orden quedará facturada en el TPV.

---

## 🔧 Correcciones recomendadas antes de pruebas de escritura

1. **FIX BUG-1:** Agregar guard `read_only` a `sincronizar_cliente_bdp` handler.
2. **Verificar `bdp_auto_backup_before_write = true`** en la configuración antes de pruebas.
3. **Crear venta de prueba** con importe conocido (ej: "Test sync 1€") para no mezclar con ventas reales.
4. **Documentar en TPV** que se están haciendo pruebas — el camarero debe saber que aparecerán comandas de prueba.
