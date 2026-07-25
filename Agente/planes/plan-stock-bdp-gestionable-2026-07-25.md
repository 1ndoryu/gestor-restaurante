# Plan: Stock BDP gestionable

> **Fecha:** 2026-07-25
> **Tarea roadmap:** UI4 — Evaluación de stock gestionable
> **Contexto:** Actualmente el stock es solo lectura, extraído del campo `CurrentStock`/`Stock` de `ExportArticles` y mostrado en `/bdp/stock`.

---

## 1. Resumen ejecutivo

Se evalúan dos opciones para el stock en Glory:

| Opción | Descripción | Riesgo | Esfuerzo |
|--------|-------------|--------|----------|
| **A — Solo lectura por almacén** | Mostrar stock desglosado por almacén usando `GetStock`/`GetListStock` | Bajo | ~6-8h |
| **B — Gestión de stock** | Permitir actualizar stock en BDP vía `UpdateStock`/`Regularizations` | Alto | ~16-24h |

**Recomendación:** implementar primero la Opción A. La Opción B solo si el cliente acepta el riesgo operacional y BDP activa el módulo de almacén.

---

## 2. Opción A — Stock por almacén (solo lectura)

### 2.1 Endpoints BDP a usar

BDP tiene endpoints de almacén documentados en el manual WebLink:

| Endpoint | Ruta aproximada | Descripción |
|----------|-----------------|-------------|
| `GetStock` | `/API/Warehouse/GetStock` | Stock de un artículo en un almacén |
| `GetListStock` | `/API/Warehouse/GetListStock` | Stock de varios artículos en un almacén |
| `GetWarehouses` | `/API/Warehouses/GetList` o similar | Listar almacenes disponibles |

> Nota: las rutas exactas deben verificarse en el manual WebLink. No se harán llamadas reales a BDP hasta tener confirmación del cliente o entorno de pruebas.

### 2.2 Cambios backend (~3-4h)

1. **Modelos** (`src/services/bdp_weblink_catalog.rs`):
   - `BdpGetStockRequest { warehouse_id: i32, article_code: String }`
   - `BdpGetListStockRequest { warehouse_id: i32, article_codes: Vec<String> }`
   - `BdpStockResponse { warehouse_id, article_code, quantity, warehouse_name }`

2. **Cliente BDP** (`src/services/bdp_weblink.rs`):
   - `get_stock(request) -> Result<Value, BdpWeblinkError>`
   - `get_list_stock(request) -> Result<Value, BdpWeblinkError>`
   - No requieren `ensure_write_target_allowed` porque son lecturas.

3. **Handlers** (`src/handlers/bdp_stock.rs` nuevo):
   - `GET /api/bdp/stock?warehouse_id={id}` → llama `GetListStock` para todos los artículos mapeados.
   - `GET /api/bdp/warehouses` → lista almacenes (si el endpoint existe).

4. **Base de datos**:
   - No requiere migraciones. Los datos se consultan en tiempo real a BDP y se cachean en memoria por 1 minuto si se desea.

### 2.3 Cambios frontend (~2-3h)

1. **Página `/bdp/stock`**:
   - Añadir selector de almacén (dropdown con warehouses disponibles).
   - Mostrar tabla: Artículo | Código BDP | Stock almacén seleccionado | Stock total.
   - Botón "Refrescar" para volver a consultar BDP.

2. **Hooks**:
   - Nuevos hooks manuales en `frontend/src/api/bdp-stock.ts` para `getStock` y `getListStock`.

### 2.4 Seguridad

- Solo lectura. No hay riesgo de modificar inventario.
- Reutiliza autenticación JWT existente.
- No requiere arming.

---

## 3. Opción B — Gestión de stock (escritura en BDP)

### 3.1 Endpoints BDP a usar

| Endpoint | Ruta aproximada | Descripción |
|----------|-----------------|-------------|
| `UpdateStock` | `/API/Warehouse/UpdateStock` | Actualiza stock de un artículo en un almacén |
| `Regularizations` | `/API/Warehouse/Regularizations` | Regularizaciones de inventario |
| `Transfers` | `/API/Warehouse/Transfers` | Transferencias entre almacenes |

### 3.2 Requisitos de seguridad obligatorios

Antes de permitir escritura, debe cumplirse:

1. **Feature flag** `ff_bdp_stock_write` en `configuracion_restaurante`, desactivado por defecto.
2. **Arming temporal** del usuario (mismo patrón que comandas/pagos).
3. **Confirmación textual** con el monto exacto de ajuste y el motivo.
4. **Idempotencia** por `idempotency_key` para evitar doble aplicación.
5. **Audit log** en `bdp_audit_log` con datos enviados, respuesta y usuario.
6. **Permisos**: solo usuarios con rol admin/gerente.
7. **Throttling**: contar dentro del semáforo BDP existente.

### 3.3 Cambios backend (~10-14h)

1. **Modelos** (`src/services/bdp_weblink_catalog.rs`):
   - `BdpUpdateStockRequest { warehouse_id, article_code, quantity, reason }`
   - `BdpRegularizationRequest { warehouse_id, article_code, quantity_adjustment, reason }`

2. **Cliente BDP** (`src/services/bdp_weblink.rs`):
   - `update_stock(...)` con `ensure_write_target_allowed`.
   - `regularize_stock(...)` con `ensure_write_target_allowed`.

3. **Servicio** (`src/services/bdp_stock.rs` nuevo):
   - `adjust_stock(user_id, article_map_id, warehouse_id, new_quantity, reason)`:
     - Validar arming activo.
     - Validar que el artículo esté mapeado.
     - Calcular diferencia (`new_quantity - current_quantity`).
     - Llamar `UpdateStock` con cantidad final o regularización según BDP.
     - Registrar en `bdp_audit_log`.
     - Invalidar caché de `bdp_article_map`.

4. **Handlers**:
   - `POST /api/bdp/stock/adjust` → ajustar stock.
   - `POST /api/bdp/stock/transfer` → transferir entre almacenes (opcional).

5. **Base de datos**:
   - Nueva tabla `bdp_stock_adjustments(article_map_id, warehouse_id, previous_quantity, new_quantity, reason, user_id, created_at)`.

### 3.4 Cambios frontend (~6-10h)

1. **Página `/bdp/stock`**:
   - Selector de almacén.
   - Botón "Ajustar stock" por fila (solo si `ff_bdp_stock_write` activo).
   - Modal con campo de cantidad y motivo.
   - Confirmación textual del tipo "Ajustar {cantidad} unidades de {artículo}".

2. **Hooks**:
   - `useAdjustStock()` mutation.
   - `useBdpWarehouses()` query.

---

## 4. Riesgos y mitigaciones

| Riesgo | Mitigación |
|--------|------------|
| BDP no tiene activado el módulo de almacén | Verificar con `GetStock` antes de mostrar la UI. Si devuelve error de módulo no activado, mostrar mensaje informativo. |
| Sobrescribir stock real de BDP por error | Opción A no escribe. Opción B requiere confirmación textual + arming + idempotencia. |
| Descuadre entre Glory y BDP | Audit log y tabla `bdp_stock_adjustments` para trazabilidad. |
| Race condition entre usuarios | Lock por `article_map_id + warehouse_id` durante el ajuste. |
| Endpoint BDP no documentado o cambia | No implementar escritura hasta validar contrato con BDP real. |

---

## 5. Flujo de decisión recomendado

```
¿El cliente necesita gestionar stock desde Glory?
├── No  → Mantener Opción A (solo lectura)
└── Sí  → ¿BDP tiene activado el módulo de almacén?
            ├── No  → Opción A + documentar limitación
            └── Sí  → ¿Aceptan riesgo y complejidad?
                     ├── No  → Opción A
                     └── Sí  → Opción B con todas las salvaguardas
```

---

## 6. Tareas concretas si se aprueba Opción A

1. Añadir modelos `BdpGetStockRequest`/`BdpGetListStockRequest` en `bdp_weblink_catalog.rs`.
2. Añadir métodos en `BdpWeblinkClient`.
3. Crear handler `GET /api/bdp/stock` y `GET /api/bdp/warehouses`.
4. Crear hooks frontend en `frontend/src/api/bdp-stock.ts`.
5. Actualizar `BdpStock.tsx` con selector de almacén.
6. Validar con simulador BDP local.

---

## 7. Tareas concretas si se aprueba Opción B

1. Crear feature flag `ff_bdp_stock_write` en `configuracion_restaurante`.
2. Crear tabla `bdp_stock_adjustments`.
3. Implementar servicio `BdpStockService::adjust`.
4. Implementar handler `POST /api/bdp/stock/adjust`.
5. Integrar `BdpWriteGuard` para arming.
6. Actualizar frontend con modal de ajuste y permisos.
7. Tests de integración con simulador BDP.

---

## 8. Esfuerzo total estimado

| Escenario | Esfuerzo |
|-----------|----------|
| Opción A (lectura por almacén) | ~6-8h |
| Opción B (gestión completa) | ~16-24h |
| Análisis previo y verificación de endpoints | ~2-4h |

---

## 9. Próximos pasos inmediatos

1. Confirmar con el cliente si quiere solo lectura por almacén o gestión completa.
2. Verificar en el manual WebLink las rutas exactas de `GetStock`/`GetListStock`/`UpdateStock`.
3. Si es posible, probar contra el simulador BDP local para confirmar contratos de datos.
