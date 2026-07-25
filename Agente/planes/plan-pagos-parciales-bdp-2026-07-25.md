# Plan de pagos parciales BDP (AddOrderPayment)

> **Fecha:** 2026-07-25  
> **ID bloque:** 247A-9 / D4  
> **Rama:** `glory-rs-rest`  
> **Estado:** Implementado (backend + frontend + reconciliación de ambiguos). Tests de servicio con simulador BDP pendientes de ejecución real.  
> **Esfuerzo estimado:** ~18-22 h (backend: 8h, frontend: 5h, tests/QA: 5-7h, docs: 2h)

## 1. Contexto y objetivo

Hoy `AddOrderPayment` en `src/services/bdp_sync.rs` solo admite un **único pago completo** por comanda BDP. Si el importe enviado no coincide exactamente con el saldo pendiente (±0,005), se rechaza. Esto evita descuadres, pero impide pagar una comanda en varios plazos o métodos.

**Objetivo:** permitir pagos parciales, controlados por el feature flag `ff_bdp_partial_payments`, con un ledger local que garantice que nunca se sobrepase el total de la venta.

## 2. Alcance

### Dentro del alcance
- Ledger local de pagos parciales (`bdp_pagos`).
- Feature flag `ff_bdp_partial_payments` para activar/desactivar.
- Cálculo de saldo pendiente desde el ledger local (no confiar solo en BDP).
- Prevención de sobrepago.
- Idempotencia por pago individual.
- Bloqueo de facturación si queda saldo pendiente.
- UI para ver historial de pagos y añadir pagos parciales.
- Tests con simulador BDP local (sin llamadas reales).

### Fuera del alcance (por ahora)
- Pagos parciales sin comanda BDP previa.
- Pagos recurrentes o programados.
- Reembolsos parciales.
- Notas de compra (albaranes de proveedor).

## 3. Modelo de datos

### Nueva tabla: `bdp_pagos`

```sql
CREATE TABLE bdp_pagos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    venta_id UUID NOT NULL REFERENCES ventas(id) ON DELETE CASCADE,
    amount NUMERIC(10, 2) NOT NULL CHECK (amount > 0),
    tender_id INT NOT NULL,
    idempotency_key VARCHAR(255) NOT NULL UNIQUE,
    bdp_order_id BIGINT,
    bdp_payment_id VARCHAR(50), -- ID que devuelva BDP si lo hay
    resultado VARCHAR(50) NOT NULL DEFAULT 'exito' CHECK (resultado IN ('exito', 'ambiguo', 'error')),
    datos_respuesta JSONB,
    error_mensaje TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bdp_pagos_venta_id ON bdp_pagos(venta_id);
CREATE INDEX idx_bdp_pagos_venta_resultado ON bdp_pagos(venta_id, resultado);
```

### Razón del diseño
- `venta_id` enlaza con la venta local.
- `idempotency_key` evita doble cargo si se reintenta una petición.
- `bdp_order_id` permite reconciliar con BDP.
- `resultado` guarda el estado del pago; `ambiguo` queda para reconciliación manual/automática.
- No se almacena saldo directamente; se calcula como `total_venta - SUM(amount WHERE resultado='exito')`.

## 4. Backend — cambios por archivo

### 4.1 `migrations/20260725100000_bdp_pagos.{up,down}.sql`
- Crear tabla `bdp_pagos` con índices.
- Down: `DROP TABLE IF EXISTS bdp_pagos;`.

### 4.2 `src/models/bdp_pago.rs` (nuevo)

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BdpPago {
    pub id: Uuid,
    pub venta_id: Uuid,
    pub amount: Decimal,
    pub tender_id: i32,
    pub idempotency_key: String,
    pub bdp_order_id: Option<i64>,
    pub bdp_payment_id: Option<String>,
    pub resultado: String,
    pub datos_respuesta: Option<serde_json::Value>,
    pub error_mensaje: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct RegistrarBdpPagoRequest {
    pub amount: Decimal,
    pub tender_id: i32,
    pub idempotency_key: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BdpPagoBalance {
    pub total: Decimal,
    pub pagado: Decimal,
    pub pendiente: Decimal,
}
```

### 4.3 `src/repositories/bdp_pago.rs` (nuevo)

Métodos:
- `insertar(pool, venta_id, amount, tender_id, idempotency_key, bdp_order_id) -> Result<BdpPago, sqlx::Error>`
- `listar_por_venta(pool, venta_id) -> Result<Vec<BdpPago>, sqlx::Error>`
- `total_pagado(pool, venta_id) -> Result<Decimal, sqlx::Error>`
- `obtener_por_idempotency_key(pool, key) -> Result<Option<BdpPago>, sqlx::Error>`
- `actualizar_resultado(pool, id, resultado, datos_respuesta, error_mensaje) -> Result<(), sqlx::Error>`

### 4.4 `src/models/mod.rs`
Añadir `pub mod bdp_pago;` y re-exportar `BdpPago`, `RegistrarBdpPagoRequest`, `BdpPagoBalance`.

### 4.5 `src/repositories/mod.rs`
Añadir `pub mod bdp_pago;` y re-exportar `BdpPagoRepository`.

### 4.6 `src/services/bdp_sync.rs` — `add_order_payment`

1. **Feature flag gate:**
   ```rust
   let total_pagado = BdpPagoRepository::total_pagado(pool, venta.id).await?;
   let total = venta.importe_base + venta.importe_iva;
   let pendiente = total - total_pagado;
   let es_parcial = (amount - pendiente).abs() > Decimal::new(5, 3); // 0.005

   if es_parcial && !config.ff_bdp_partial_payments {
       return Err("Pagos parciales desactivados. Activa el feature flag en configuración.".into());
   }
   ```

2. **Validaciones de seguridad:**
   - `amount > 0` y `tender_id > 0`.
   - `amount <= pendiente + 0,005` (evita sobrepago).
   - Orden BDP no cancelada ni facturada (`status != 2,3`).

3. **Idempotencia:**
   - Si ya existe `idempotency_key` con resultado `exito`, devolver el pago existente sin llamar a BDP.
   - Reutilizar `BdpWriteGuard::check_idempotency` para compatibilidad con `bdp_audit_log`.

4. **Flujo atómico:**
   - Preparar snapshot de escritura.
   - Autorizar vía `BdpWriteGuard::authorize`.
   - Llamar a `client.add_order_payment`.
   - En transacción:
     - Insertar en `bdp_pagos`.
     - Cerrar auditoría con resultado.
     - Si el pago liquida el saldo y se solicita factura, invocar `InvoiceOrder` (opcional).
   - Manejar `ambiguo` para reconciliación.

5. **Reconciliación:**
   - Después del pago, si BDP devuelve estado ambiguo, actualizar `resultado='ambiguo'`.
   - Worker de reconciliación existente (`bdp_order_poller`) puede consultar `GetOrder` y comparar `Payments`.

### 4.7 `src/handlers/ventas.rs`

1. **Nuevo endpoint:** `GET /api/ventas/:id/bdp-payments`
   - Devuelve lista de pagos y balance.
   - Body: `{ pagos: Vec<BdpPago>, balance: BdpPagoBalance }`.

2. **Modificar `bdp_payment`:**
   - Aceptar `idempotency_key` generado por el cliente.
   - Validar saldo pendiente con `BdpPagoRepository::total_pagado`.
   - Llamar a `BdpSyncService::add_order_payment`.

3. **Modificar `bdp_invoice`:**
   - Antes de facturar, verificar que `pendiente <= 0,005`.
   - Si hay saldo pendiente, devolver 422 con mensaje claro.

### 4.8 `src/models/configuracion.rs`
- Feature flag `ff_bdp_partial_payments` ya existe. Asegurar que se expone en respuestas/actualizaciones.

## 5. Frontend

### 5.1 `frontend/src/components/venta-row-actions.tsx`
- Reemplazar diálogo de pago único por uno de pagos parciales.
- Mostrar:
  - Total de la venta.
  - Pagado.
  - Pendiente.
  - Historial de pagos (fecha, método, monto).
- Permitir añadir un nuevo pago con:
  - Importe editable (por defecto pendiente).
  - Tender ID.
  - Confirmación textual tipo "PAGAR {venta_id} {monto}".
- Botón "Facturar" deshabilitado si `pendiente > 0.01`.

### 5.2 `frontend/src/api/generated/ventas/ventas.ts`
- Regenerar con Orval tras añadir el endpoint `GET /api/ventas/:id/bdp-payments`.

## 6. Mitigaciones de riesgos

| Riesgo | Mitigación |
|--------|------------|
| Doble pago por doble click/reintentar | `idempotency_key` único por pago; clave UNIQUE en BD. |
| Sobrepago | Cálculo de saldo local antes de envío; rechazo si `amount > pendiente + 0.005`. |
| Estado inconsistente si BDP responde ambiguo | Marcar pago como `ambiguo`; worker de reconciliación consulta `GetOrder`. |
| Facturar con saldo pendiente | Endpoint `bdp_invoice` bloquea si `pendiente > 0.005`. |
| Concurrencia (dos pagos simultáneos) | Advisory lock por `venta_id` + transacción atómica. |
| Feature flag desactivado | Gate en backend y UI oculta/deshabilitada. |
| Perdida de snapshot/auditoría | Inserción en `bdp_pagos` y cierre de `bdp_audit_log` dentro de la misma transacción. |

## 7. Tests

### 7.1 Tests unitarios (Rust)
- Cálculo de saldo con varios pagos.
- Prevención de sobrepago.
- Feature flag gate.

### 7.2 Tests de integración con simulador BDP
- Pago parcial de 50% y luego 50% restante.
- Intento de pago por encima del saldo -> 422.
- Idempotencia: mismo `idempotency_key` -> solo un pago.
- Facturar tras pagar saldo completo -> 200.
- Facturar con saldo pendiente -> 422.
- Feature flag desactivado -> 422.

### 7.3 Tests frontend
- Renderizado del diálogo con historial.
- Validación de importe.
- Deshabilitación del botón de facturar.

## 8. Tareas y dependencias

| # | Tarea | Archivos | Estado | Esfuerzo |
|---|-------|----------|--------|----------|
| 1 | Migración `bdp_pagos` | `migrations/*bdp_pagos*` | ✅ Hecho | 30 min |
| 2 | Modelo `BdpPago` + request/response | `src/models/bdp_pago.rs` | ✅ Hecho | 30 min |
| 3 | Repository `BdpPagoRepository` | `src/repositories/bdp_pago.rs` | ✅ Hecho | 1 h |
| 4 | Integrar en `models/mod.rs` y `repositories/mod.rs` | `src/models/mod.rs`, `src/repositories/mod.rs` | ✅ Hecho | 15 min |
| 5 | Refactor `add_order_payment` para soportar parciales | `src/services/bdp_sync.rs` | ✅ Hecho | 3 h |
| 6 | Nuevo endpoint `GET /api/ventas/:id/bdp-payments` y modificar `bdp_payment`/`bdp_invoice` | `src/handlers/ventas.rs` | ✅ Hecho | 2 h |
| 7 | Tests de integración del ledger (`bdp_pagos`) | `tests/bdp_pagos.rs` | ✅ Hecho | 2 h |
| 8 | Tests de servicio/simulador para `add_order_payment` | `tests/bdp_partial_payments.rs` o similares | ⏳ Pendiente (sin BDP real) | 3 h |
| 9 | UI de pagos parciales | `frontend/src/components/venta-row-actions.tsx` | ✅ Hecho | 3 h |
| 10 | Regenerar API client + tests frontend | `frontend/src/api/generated/ventas/*` | ✅ Hecho (llamada directa con axios instance) | 1 h |
| 11 | Actualizar roadmap y documentación | `roadmap.md`, este plan | ✅ Hecho | 30 min |

## 9. Decisiones pendientes

- **D4.1:** ¿Se permite pagar antes de que la comanda esté en BDP? Hoy se requiere `bdp_order_id`. Respuesta propuesta: no, mantener requisito de comanda sincronizada primero.
- **D4.2:** ¿Se permite facturar automáticamente tras el último pago parcial? Respuesta propuesta: opcional, por defecto no; el usuario debe pulsar "Facturar" explícitamente.
- **D4.3:** ¿Se muestran pagos ambiguos en la UI como "pendiente de confirmar"? Respuesta propuesta: sí, en color amarillo, con botón de reintentar reconciliación.

## 10. Criterios de aceptación

- [x] Migración aplica sin errores.
- [ ] Tests de integración pasan con simulador BDP (pendiente de ejecución real; el ledger se testea sin BDP).
- [x] Pago parcial liquida el saldo correctamente (UI + backend).
- [x] Sobrepago es rechazado.
- [x] Facturación sin saldo completo es rechazada.
- [x] `ff_bdp_partial_payments=false` bloquea pagos parciales.
- [x] Frontend muestra historial y saldo pendiente.
- [x] `cargo test` y `cargo clippy` sin errores.
- [x] `npm run type-check` en frontend sin errores.

## 11. Referencias

- `src/services/bdp_sync.rs` — lógica actual de `add_order_payment`.
- `src/services/bdp_weblink_catalog.rs` — `BdpAddOrderPaymentRequest`.
- `src/handlers/ventas.rs` — endpoints de pagos/facturas.
- `src/models/configuracion.rs` — feature flag.
- `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` — mitigaciones de seguridad.
