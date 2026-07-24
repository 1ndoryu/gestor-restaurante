# Plan detallado: Pendientes integración BDP

> **Fecha:** 2026-07-23
> **Alcance:** Solo planificación. Sin código, sin comandos al BDP real.
> **Contexto:** Tras completar las mejoras de visibilidad (237A-3) e implementación de stock (237A-4), quedan pendientes decisiones y funcionalidades que requieren análisis profundo antes de implementar.

---

## Resumen ejecutivo

| # | Item | Tipo | Estado actual | ¿Listo para implementar? | Esfuerzo estimado |
|---|------|------|---------------|--------------------------|-------------------|
| C1 | Auto-arming (escritura automática) | Mejora de flujo | Pendiente decisión usuario | **Sí** — diseñado, seguro | ~10-12h |
| C2 | Toggle rápido en navbar | Mejora UX | Pendiente decisión usuario | **Sí** — trivial, pero C1 lo hace innecesario | ~3h |
| D1 | Verificación stock real | Verificación | Implementado básico (237A-4) | **Parcial** — falta verificar con BDP real | ~2h verificación + ~8h pantalla dedicada |
| D2 | Compras | Funcionalidad nueva | No implementado | **No** — requiere diseño completo | ~20-30h |
| D3 | Sincronización bidireccional automática | Funcionalidad nueva | Bloqueado explícitamente en código | **No recomendado** — riesgo crítico | ~40h+ |
| D4 | Pagos parciales | Funcionalidad nueva | Bloqueado explícitamente en código | **No** — requiere ledger independiente | ~15-20h |
| D5 | CancelOrder | Funcionalidad nueva | Bloqueado por BDP ("Subscripción no activada") | **No** — depende de BDP | ~4h (si BDP activa módulo) |

---

## C1 — Auto-arming (escritura automática al operar)

### Problema actual

Hoy, para enviar una venta a BDP, registrar un pago o facturar, el usuario debe:

1. Ir a Configuración → BDP → "Seguridad, respaldos e historial BDP"
2. Pulsar "Activar escritura temporal"
3. Confirmar texto de seguridad
4. Volver a Ventas
5. Realizar la operación (enviar a BDP / pagar / facturar)
6. El sistema desarma automáticamente después de la operación

**Esto es 5 pasos para una operación que debería ser 1.** En un restaurante con ritmo, esto es inaceptable.

### Diseño propuesto

**Flujo nuevo (1 paso):**

1. El usuario pulsa "Enviar a BDP" / "Pagar en BDP" / "Facturar en BDP" directamente desde la fila de la venta
2. Se abre un modal de confirmación con:
   - Descripción de la operación ("Registrar pago completo de €45.00 en BDP")
   - Campo de confirmación textual: el usuario debe escribir exactamente "CONFIRMAR" (o la palabra elegida)
   - Botón "Cancelar" / "Confirmar y ejecutar"
3. Al confirmar:
   - El backend crea un arming efímero (`bdp_write_arming` con TTL de 60 segundos)
   - Ejecuta la operación
   - Desarma automáticamente
   - Registra en auditoría como `AUTO_ARMING` (distinguir del arming manual)

### Arquitectura backend

```
Handler (venta-row-actions)
  → POST /api/ventas/:id/bdp-sync (ya existe)
    → Nuevo parámetro: { auto_arm: true, confirmation_text: "CONFIRMAR" }

BdpSyncService::sync_venta()
  → Si auto_arm && confirmation_text == "CONFIRMAR":
    → BdpWriteGuard::authorize_inline(user_id, "auto_arm", ttl=60s)
    → Ejecutar operación
    → BdpWriteGuard::disarm(user_id)
    → Audit log: tipo="auto_arm", operacion="create_order"
```

**Cambios necesarios:**

| Capa | Archivo | Cambio |
|------|---------|--------|
| Backend | `src/services/bdp_write_guard.rs` | Nuevo método `authorize_inline()` que crea arming efímero sin pasar por el endpoint manual |
| Backend | `src/handlers/ventas.rs` | Aceptar campo `auto_arm: bool` + `confirmation_text: String` en requests de sync/pago/factura |
| Backend | `src/handlers/bdp_customer_sync.rs` | Mismo patrón para CreateCustomer |
| Backend | `src/handlers/configuracion.rs` | Mantener endpoint manual actual como fallback |
| Frontend | `venta-row-actions.tsx` | Modal de confirmación inline en lugar de navegar a Configuración |
| Frontend | `bdp-sync-badge.tsx` | Indicador de que auto-arming está disponible |

### Seguridad

| Riesgo | Mitigación |
|--------|------------|
| Alguien pulsa "Enviar" sin intención | Confirmación textual obligatoria ("CONFIRMAR") |
| Reutilización de arming | TTL de 60 segundos, un solo uso |
| Bypass del flujo manual | Mantener endpoint manual como alternativa; auto-arming es opt-in en configuración |
| Auditoría insuficiente | Log diferenciado: `AUTO_ARMING` vs `MANUAL_ARMING` con IP, timestamp, operación |
| Rate limiting | Máximo 10 auto-armings por minuto por usuario |

### Configuración necesaria

Nuevo campo en `configuracion_restaurante`:
```sql
bdp_auto_arm BOOLEAN NOT NULL DEFAULT FALSE
```

- `FALSE` (default): comportamiento actual (arming manual obligatorio)
- `TRUE`: permite auto-arming con confirmación textual

**El usuario decide si activar esto.** Si no lo activa, el flujo actual sigue funcionando.

### Esfuerzo: ~10-12h

| Tarea | Tiempo |
|-------|--------|
| Backend: `authorize_inline()` + validación | 3h |
| Backend: modificar handlers sync/pago/factura | 2h |
| Frontend: modal de confirmación | 3h |
| Frontend: integración con venta-row-actions | 2h |
| Tests + auditoría | 2h |

### Recomendación: **IMPLEMENTAR**

Es la mejora de usabilidad más impactante. Sin esto, la integración de escritura es usable pero incómoda.

---

## C2 — Toggle rápido en navbar

### Problema

El indicador BDP en la navbar (`BdpStatusIndicator`) muestra el estado actual pero no permite cambiarlo. Para cambiar de "Solo lectura" a "Escritura temporal" hay que ir a Configuración → BDP → Panel de seguridad.

### Diseño propuesto

Convertir el badge `BdpStatusIndicator` en un componente interactivo:

```
[🟢 BDP Conectado] → click → Dropdown:
  ┌─────────────────────────────────┐
  │ Estado: Solo lectura             │
  │ ─────────────────────────────── │
  │ 🔒 Activar escritura temporal   │
  │ 📊 Ver historial BDP            │
  │ ⚙️ Configuración BDP            │
  └─────────────────────────────────┘
```

### Cambios necesarios

| Capa | Archivo | Cambio |
|------|---------|--------|
| Frontend | `site-header.tsx` | Convertir `BdpStatusIndicator` de badge estático a dropdown interactivo |
| Frontend | Nuevo hook | `useBdpSyncMode.ts` — mutation para cambiar modo |
| Backend | Ninguno | Reutilizar `PUT /api/configuracion/bdp/sync-mode` existente |

### Seguridad

- Solo visible para usuarios con rol admin/manager
- Cambiar a escritura requiere el flujo de arming existente (o auto-arming si C1 está activo)

### Esfuerzo: ~3h

### Recomendación: **IMPLEMENTAR SOLO SI C1 NO SE HACE**

Si C1 (auto-arming) se implementa, el toggle rápido pierde utilidad porque el usuario nunca necesita cambiar manualmente el modo. Si C1 se rechaza, C2 es la alternativa mínima.

---

## D1 — Verificación de stock real

### Estado actual

Implementado en 237A-4:
- `ExportArticles` → `current_stock: Option<Decimal>` con aliases `CurrentStock` + `Stock`
- Mapeo a `stock_actual` en `bdp_article_map`
- Columna Stock en tabla de mapeos
- Info log si ningún artículo trae stock

### Lo que falta verificar

1. **¿`ExportArticles` devuelve `CurrentStock` en la respuesta real?**
   - El campo está documentado en `PricesTableDataType` (línea 3685 del manual WebLink)
   - Podría estar anidado dentro de un sub-array `Prices` en vez de ser top-level
   - Si está anidado, el parser actual no lo captura → columna mostrará "—" para todos

2. **¿El módulo de almacén está activo en la instalación del restaurante?**
   - Si no está activo, `CurrentStock` vendrá como `None` siempre
   - El info log diagnosticará esto automáticamente

### Plan de verificación

**Paso 1 — Verificar sin BDP real (ahora):**
- Revisar la estructura exacta de la respuesta `ExportArticles` en el manual WebLink
- Confirmar si `CurrentStock` es campo top-level del artículo o está dentro de `Prices`

**Paso 2 — Verificar con BDP real (cuando el cliente haga pruebas):**
- Ejecutar "Sync catálogo" desde la app
- Revisar logs del servidor: buscar `[237A-4] Ningún artículo de ExportArticles trajo CurrentStock`
- Si aparece → el módulo de almacén no está activo o el campo está anidado

**Paso 3 — Si stock no aparece, decidir camino:**
- **Opción A** (rápida, ~2h): Ajustar el parser para extraer `CurrentStock` del sub-array `Prices` si está anidado
- **Opción B** (completa, ~8h): Implementar endpoints dedicados `GetStock`/`GetListStock` + pantalla de stock dedicada

### Endpoints BDP disponibles para stock

| Endpoint | Método | Descripción | Estado en Glory |
|----------|--------|-------------|-----------------|
| `ExportArticles` (campo `CurrentStock`) | Ya implementado | Stock viene como campo del artículo | ✅ Implementado, pendiente verificación real |
| `/API/Warehouse/GetStock` | POST | Stock de un artículo en un almacén específico | ❌ No implementado |
| `/API/Warehouse/GetListStock` | POST | Stock de múltiples artículos de una vez | ❌ No implementado |
| `/API/Warehouse/UpdateStock` | POST | Actualizar stock (escritura) | ❌ No implementado, fuera de alcance |
| `/API/Warehouse/Regularizations` | POST | Regularizaciones de inventario | ❌ No implementado, fuera de alcance |
| `/API/Warehouse/Transfers` | POST | Transferencias entre almacenes | ❌ No implementado, fuera de alcance |

### Si se implementa pantalla dedicada de stock (Opción B)

**Nuevos componentes:**

| Capa | Archivo nuevo | Descripción |
|------|---------------|-------------|
| Backend | `src/handlers/bdp_stock.rs` | Handler para `GET /api/bdp/stock` — consulta stock por artículo o todos |
| Backend | `src/services/bdp_weblink.rs` | Método `get_stock()` y `get_list_stock()` |
| Backend | `src/services/bdp_weblink_catalog.rs` | Structs `BdpGetStockRequest`, `BdpGetStockResponse` |
| Frontend | `bdp-stock-panel.tsx` | Tabla de stock con filtros por artículo/familia/almacén |
| Frontend | Hook | `useBdpStock.ts` — query para obtener stock |

**Flujo:**
1. Usuario hace click en "Ver stock" desde tabla de mapeos o sección dedicada
2. Frontend llama `GET /api/bdp/stock?almacen=1`
3. Backend llama a `GetListStock` en BDP
4. Muestra tabla con: Artículo | Código BDP | Stock actual | Almacén | Última actualización

### Esfuerzo

| Tarea | Tiempo |
|-------|--------|
| Verificar estructura respuesta ExportArticles | 2h |
| Ajustar parser si CurrentStock está anidado | 2h |
| Implementar endpoints GetStock/GetListStock | 3h |
| Pantalla dedicada de stock | 5h |
| **Total Opción B** | **~12h** |

### Recomendación: **VERIFICAR PRIMERO, luego decidir**

1. Verificar si `ExportArticles` ya trae stock (paso 1-2)
2. Si sí → no hacer nada más, la columna ya funciona
3. Si no → consultar al cliente si necesita pantalla dedicada de stock

---

## D2 — Compras (integración de proveedores)

### ¿Qué son las "compras" en BDP?

BDP tiene un módulo completo de gestión de compras que incluye:

| Concepto | Descripción |
|----------|-------------|
| **Proveedores** | Catálogo de proveedores con datos fiscales |
| **Albaranes de compra** | Documentos que registran la recepción de mercancía |
| **Facturas de compra** | Facturas emitidas por proveedores |
| **Recepciones** | Confirmación física de mercancía recibida |
| **Órdenes de compra** | Pedidos a proveedores |

### Endpoints BDP disponibles

| Endpoint | Método | Descripción |
|----------|--------|-------------|
| `ExportPurchaseNotes` | POST | Exportar albaranes de compra por perfil y rango de fechas/proveedores |
| `ExportManagmentDocumentsByExportProfile` | POST | Exportar albaranes y facturas de gestión (cabeceras + líneas + vencimientos) |

**Documentación encontrada en `# WEBLINK RESTAPI.md`:**

- **Línea 9614:** `ExportPurchaseNotes` — exporta albaranes de compra con estructura definida por perfil de exportación
- **Parámetros:** `InitialSupplier`, `FinalSupplier` (rango de proveedores), perfil de exportación
- **Respuesta incluye:** `Serie_Albaran`, `Num_Albaran`, `Fecha_Albaran`, `Cod_Proveedor`, `Nom_Proveedor`, líneas con artículos, cantidades, precios

### ¿Por qué no se incluyó originalmente?

1. **Complejidad:** Es un dominio completo propio (proveedores + albaranes + recepciones + conciliaciones con inventario). No es un endpoint simple como "crear comanda".
2. **Tiempo:** La integración original priorizó el flujo core del restaurante (crear comandas, pagar, facturar). Compras es un flujo secundario.
3. **Riesgo:** Integrar compras mal podría crear descorrelaciones contables entre Glory y BDP.

### Diseño propuesto (si se aprueba)

**Fase 1 — Solo lectura (~8h):**

| Capa | Archivo nuevo | Descripción |
|------|---------------|-------------|
| Backend | `src/services/bdp_weblink.rs` | Método `export_purchase_notes()` |
| Backend | `src/services/bdp_weblink_catalog.rs` | Structs `BdpPurchaseNoteRequest/Response` |
| Backend | `src/handlers/bdp_purchases.rs` | `GET /api/bdp/purchases` — listar albaranes de compra |
| Frontend | `bdp-purchases-panel.tsx` | Tabla de albaranes con filtros por fecha/proveedor |
| Migration | `bdp_purchase_notes` | Tabla local para cachear albaranes importados |

**Fase 2 — Escritura (~20h adicionales):**

| Capa | Archivo nuevo | Descripción |
|------|---------------|-------------|
| Backend | `src/services/bdp_sync.rs` | `create_purchase_note()` — crear albarán en BDP |
| Frontend | `FormularioCompraBdp.tsx` | Formulario para crear albarán con proveedor, artículos, cantidades |
| Migration | `bdp_suppliers` | Tabla local de proveedores importados de BDP |

### Flujo de datos (Fase 1 — lectura)

```
BDP WebLink API
  → ExportPurchaseNotes (POST, con rango fechas/proveedores)
    → Response: lista de albaranes con cabeceras + líneas
      → Backend: parse, upsert en bdp_purchase_notes
        → Frontend: tabla filtrable
```

### Riesgos

| Riesgo | Nivel | Mitigación |
|--------|-------|------------|
| Módulo no activo en BDP | Alto | Verificar con "Subscripción no activada" antes de prometer |
| Complejidad del dominio | Alto | Empezar solo lectura, validar con cliente antes de escritura |
| Descuadrages contables | Medio | Solo lectura elimina este riesgo; escritura requiere conciliación |
| Endpoints no documentados completamente | Medio | El manual tiene la estructura de respuesta pero no todos los campos están documentados |

### Esfuerzo total: ~20-30h

| Fase | Tiempo |
|------|--------|
| Fase 1 (solo lectura) | ~8h |
| Fase 2 (escritura) | ~20h |
| Tests + integración | ~4h |

### Recomendación: **PENDIENTE DE CONSULTA AL CLIENTE**

Preguntar:
1. ¿El módulo de compras está activo en su BDP?
2. ¿Necesitan ver albaranes de compra desde Glory?
3. ¿Necesitan crear albaranes desde Glory o solo consultar?

Si solo necesitan consultar → Fase 1 (8h).
Si necesitan crear → Fase 1 + Fase 2 (28h total).

---

## D3 — Sincronización bidireccional automática

### Estado actual

En `src/handlers/configuracion.rs:296`:
```rust
"Modo BDP inválido; use read_only o unidirectional. 
bidirectional está bloqueado hasta que exista un contrato implementado y auditado."
```

**El código rechaza explícitamente el modo `bidirectional`.**

### ¿Qué implica la bidireccionalidad?

| Dirección | Actualmente | Con bidireccionalidad |
|-----------|-------------|----------------------|
| BDP → Glory | Manual (botones "Sync") | Automático (polling/webhooks) |
| Glory → BDP | Manual (auto-arming propuesto en C1) | Automático (detección de cambios) |

### ¿Por qué está bloqueado?

1. **Bucles de sincronización:** Si Glory cambia un artículo → BDP recibe el cambio → BDP notifica a Glory → Glory procesa → BDP recibe... bucle infinito.
2. **Resolución de conflictos:** ¿Qué pasa si el mismo artículo se modifica en Glory y BDP simultáneamente? ¿Quién gana?
3. **Pérdida de datos:** Un cambio automático mal procesado podría sobreescribir datos maestros de BDP.
4. **Event sourcing:** Necesitaríamos un sistema completo de eventos con timestamps, deduplicación y rollback.

### ¿Podría implementarse técnicamente?

**Sí, pero con restricciones severas:**

| Componente | Necesario | Complejidad |
|------------|-----------|-------------|
| Webhooks BDP → Glory | BDP no tiene webhooks nativos | Polling frecuente (cada 30s) |
| Detección de cambios Glory → BDP | Triggers en BD o event log | Media |
| Resolución de conflictos | Timestamps + regla "último gana" o "BDP gana siempre" | Alta |
| Deduplicación | IDempotency keys por operación | Media |
| Rollback automático | Snapshot antes de cada sync + restore si falla | Alta |
| Monitoreo | Dashboard de sync status, errores, conflictos | Media |

### Esfuerzo: ~40h+

| Componente | Tiempo |
|------------|--------|
| Diseño del contrato de sincronización | 8h |
| Implementación polling bidireccional | 12h |
| Resolución de conflictos | 10h |
| Tests de estrés (bucles, colisiones) | 6h |
| Monitoreo y alertas | 4h |

### Recomendación: **RECHAZAR**

La bidireccionalidad automática es frágil, compleja y el riesgo de pérdida de datos es alto. La sincronización manual con auto-arming (C1) es suficiente para el caso de uso real del restaurante.

**Si el cliente insiste:** Proponer una versión limitada:
- Solo catálogo (artículos, precios) bidireccional, no comandas
- Conflictos: BDP siempre gana (fuente de verdad)
- Polling cada 5 minutos, no en tiempo real
- Esto reduce el esfuerzo a ~20h y el riesgo significativamente

---

## D4 — Pagos parciales

### Estado actual

En `src/services/bdp_sync.rs:1109`:
```rust
if (requested - pending).abs() > 0.005 {
    return Err(BdpSyncError::Rejected(
        "esta integración admite un único pago completo".into()
    ));
}
```

**Los pagos parciales están explícitamente bloqueados.**

### ¿Por qué están bloqueados?

1. **BDP no soporta pagos parciales nativos:** La API `AddOrderPayment` acepta un monto, pero no hay mecanismo para "fraccionar" una comanda en múltiples pagos.
2. **Riesgo de descuadres:** Si Glory registra un pago parcial pero BDP no lo entiende así, la caja cuadrará en Glory pero no en BDP.
3. **Idempotencia:** Sin un ledger independiente, no sabríamos cuánto se ha pagado realmente vs. cuánto Glory cree que se ha pagado.

### ¿Podría implementarse técnicamente?

**Sí, con un ledger independiente:**

| Componente | Descripción |
|------------|-------------|
| Tabla `bdp_partial_payments` | Registra intenciones de pago parcial: venta_id, monto, estado, timestamp |
| Servicio `PartialPaymentService` | Gestiona el ciclo de vida de pagos parciales |
| Validación contra BDP | Antes de cada pago parcial, consulta `GetOrder` para obtener saldo real |
| Reconciliación | Si BDP reporta un monto diferente al esperado, alertar y bloquear |

### Flujo propuesto

```
Usuario quiere pagar €30 de una comanda de €100
  → Frontend: muestra "Saldo pendiente: €100. ¿Cuánto pagar?"
  → Usuario ingresa €30
  → Backend:
    1. Consulta GetOrder → total=€100, paid=€0, pending=€100
    2. Valida: €30 ≤ €100 (OK)
    3. Registra en bdp_partial_payments: {venta_id, monto=30, estado="pendiente"}
    4. Llama AddOrderPayment(amount=30)
    5. Si éxito → actualiza estado a "completado"
    6. Si error → marca estado a "error", no reintenta
  → Frontend: muestra "Pago parcial de €30 registrado. Saldo pendiente: €70"
```

### Riesgos

| Riesgo | Nivel | Mitigación |
|--------|-------|------------|
| Descuadre Glory vs BDP | Alto | Reconciliación automática tras cada pago |
| BDP rechaza pago parcial | Medio | Validar contra GetOrder antes de enviar |
| Reintento automático duplica pago | Alto | Idempotency key por pago, no reintento automático |
| Usuario confundido por estado parcial | Medio | UI clara: "Pagado: €30 / Pendiente: €70" |

### Esfuerzo: ~15-20h

| Tarea | Tiempo |
|-------|--------|
| Migration + modelo `bdp_partial_payments` | 2h |
| Servicio de pagos parciales | 5h |
| Validación contra GetOrder | 2h |
| Frontend: selector de monto | 4h |
| Reconciliación automática | 3h |
| Tests | 2h |

### Recomendación: **PENDIENTE DE CONSULTA AL CLIENTE**

Preguntar:
1. ¿Necesitan pagar comandas en partes?
2. ¿Con qué frecuencia?
3. ¿Están dispuestos a asumir el riesgo de descuadres?

Si la respuesta es sí → implementar con ledger independiente.
Si es no → mantener bloqueo actual.

---

## D5 — CancelOrder

### Estado actual

En `src/services/bdp_weblink.rs:253-258`:
```rust
pub async fn cancel_order(&self, request: &BdpCancelOrderRequest) -> Result<BdpResponse, BdpWeblinkError> {
    self.post_authenticated_json(BDP_PATH_CANCEL_ORDER, request).await
}
```

**El método existe pero no está expuesto vía REST.** Al probarlo, BDP devuelve: `"Subscripción no activada"`.

### Documentación BDP

En `# WEBLINK RESTAPI.md:7456`:
```
### CancelOrder
Elimina una comanda en la base de datos.
Parámetros: OrderId, MarketplaceOrderId, MarketId, RoomNumber, TableNumber
```

**El endpoint existe en BDP pero requiere activación de módulo.**

### Diseño propuesto (si BDP activa el módulo)

| Capa | Archivo | Cambio |
|------|---------|--------|
| Backend | `src/handlers/ventas.rs` | Nuevo endpoint `POST /api/ventas/:id/bdp-cancel` |
| Backend | `src/services/bdp_sync.rs` | Método `cancel_order()` con validaciones |
| Frontend | `venta-row-actions.tsx` | Botón 🗑️ "Cancelar en BDP" |

**Validaciones antes de cancelar:**

1. Consultar `GetOrder` para verificar que la comanda existe en BDP
2. Verificar que el estado no sea `cancelled` (status=2) ni `invoiced` (status=3)
3. Verificar que la venta local esté sincronizada (`bdp_synced = true`)
4. Registrar en auditoría: motivo de cancelación (opcional), usuario, timestamp

**Flujo:**

```
Usuario pulsa "Cancelar en BDP" en venta-row-actions
  → Modal: "¿Cancelar comanda #X en BDP? Esta acción no se puede deshacer."
  → [Campo opcional: motivo]
  → [Confirmar] / [Cancelar]
  → Backend:
    1. GetOrder → verificar estado
    2. CancelOrder(order_id)
    3. Actualizar bdp_order_status = "cancelled" en Glory
    4. Audit log
```

### Seguridad

| Riesgo | Mitigación |
|--------|------------|
| Cancelar comanda en preparación | Confirmación explícita + motivo obligatorio |
| Cancelar comanda ya cancelada | Validar estado antes de enviar |
| Cancelar comanda facturada | Bloquear si status = invoiced |
| Pérdida de materia prima | Documentar que cancelar no revierte preparación |

### Esfuerzo: ~4h

| Tarea | Tiempo |
|-------|--------|
| Backend: handler + servicio | 2h |
| Frontend: modal + botón | 1.5h |
| Tests | 0.5h |

### Recomendación: **PENDIENTE DE BDP**

1. **Preguntar al cliente:** ¿Pueden activar el módulo de cancelación en BDP?
2. **Si sí →** Implementar (4h, bajo riesgo)
3. **Si no →** Mantener como limitación documentada

---

## Priorización recomendada

### Si el cliente quiere la mejor experiencia posible:

| Orden | Item | Por qué |
|-------|------|---------|
| 1 | **C1: Auto-arming** | Impacto máximo en usabilidad diaria |
| 2 | **D1: Verificar stock** | 2h para confirmar si la columna funciona |
| 3 | **D5: CancelOrder** | 4h si BDP activa el módulo |
| 4 | **D4: Pagos parciales** | Si el cliente lo necesita |
| 5 | **D2: Compras (Fase 1)** | Solo lectura, 8h |
| 6 | **C2: Toggle navbar** | Solo si C1 no se hace |
| 7 | **D2: Compras (Fase 2)** | Escritura, solo si Fase 1 funciona |
| 8 | **D3: Bidireccional** | No recomendado |

### Si el cliente quiere lo mínimo viable:

| Orden | Item | Por qué |
|-------|------|---------|
| 1 | **D1: Verificar stock** | 2h, ya implementado |
| 2 | **C1: Auto-arming** | Mejora crítica de UX |
| 3 | **D5: CancelOrder** | Si BDP lo permite |

---

## Preguntas para el cliente

1. **Auto-arming:** ¿Prefieren activar escritura automática con confirmación textual, o mantener el flujo manual actual?
2. **Stock:** ¿Necesitan ver stock detallado por almacén o con la columna actual es suficiente?
3. **Compras:** ¿Necesitan consultar albaranes de compra desde Glory? ¿Crearlos?
4. **CancelOrder:** ¿Pueden activar el módulo de cancelación en su BDP?
5. **Pagos parciales:** ¿Necesitan pagar comandas en partes?
6. **Bidireccional:** ¿Necesitan sincronización automática o con botones manuales es suficiente?

---

## Documentación de referencia

| Documento | Contenido |
|-----------|-----------|
| `# WEBLINK RESTAPI.md` | Manual completo de la API WebLink (11000+ líneas) |
| `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md` | Mapeo visual de dónde está cada funcionalidad |
| `Agente/usuario/auditoria-cruzada-bdp-endpoints-frontend-2026-07-23.md` | Tabla cruzada endpoints ↔ frontend |
| `Agente/planes/plan-visibilidad-bdp-frontend-2026-07-23.md` | Plan original de visibilidad |
| `Agente/documentacion/api/bdp-integration-status-2026-06-07.md` | Estado de integración completo |
| `Agente/usuario/auditoria-adversarial-bdp-2026-07-22.md` | Auditoría de seguridad |
