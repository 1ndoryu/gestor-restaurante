# BDP WebLink REST API — Estado de Integración

> **Fecha:** 2026-06-07 (actualizado 2026-07-02)
> **Autor:** Agente (análisis post-implementación 065A-5)
> **Stack:** Glory Rust Backend (Axum 0.7 + SQLx) ↔ BDP-NET WebLink REST API
> **Endpoint BDP:** `http://100.83.196.35:8068` (vía Tailscale)
> **POS:** 31 — CENTRAL 2026 (Series `00031TI`, IVA incluido)
> **Estado:** ✅ Integración verificada y funcionando en producción (última verificación 2026-06-30)

---

## 1. Resumen ejecutivo

| Métrica                                                        | Valor                                       |
| -------------------------------------------------------------- | ------------------------------------------- |
| Endpoints documentados en API BDP                              | ~55                                         |
| Endpoints con constante en catálogo (`bdp_weblink_catalog.rs`) | 21                                          |
| Endpoints con método en cliente (`BdpWeblinkClient`)           | 23                                          |
| Endpoints invocados en sync productivo                         | **2** (`CreateOrder`, `GetPOSArticlesList`) |
| Endpoints invocados solo en preflight                          | 8                                           |
| Endpoints con cliente pero sin invocar                         | 11                                          |
| Endpoints no implementados en absoluto                         | ~32                                         |
| Direccionalidad actual                                         | **Unidireccional (Glory → BDP)**            |
| Campos Glory no enviados en `CreateOrder`                      | ~10                                         |
| **Completitud de la integración**                              | **~5% del potencial**                       |

---

## 2. Inventario completo de endpoints

### Leyenda

- ✅ Catalogado + Cliente + Invocado en producción
- 📋 Catalogado + Cliente implementado, pero **nunca llamado**
- 🔧 Catalogado + Cliente, usado solo en preflight/diagnóstico
- ❌ No implementado en ninguna capa

### 2.1 Servicios

| Endpoint                | Método HTTP | Estado | Uso actual                                  |
| ----------------------- | ----------- | ------ | ------------------------------------------- |
| `ServiceHealth`         | GET         | 🔧     | Preflight: verifica conectividad            |
| `GetVersion`            | GET         | 🔧     | Preflight: versión BDP-NET                  |
| `GetApplicationVersion` | GET         | ❌     | —                                           |
| `Login`                 | POST        | ✅     | Interno: cada llamada autenticada lo invoca |

### 2.2 Artículos

| Endpoint                          | Método HTTP | Estado | Uso actual                                              |
| --------------------------------- | ----------- | ------ | ------------------------------------------------------- |
| `GetArticle`                      | POST        | ❌     | —                                                       |
| `GetPricesArticles`               | POST        | ❌     | —                                                       |
| `ExportArticles`                  | POST        | 📋     | Cliente tiene método, nunca llamado                     |
| `GetPOSArticlesList`              | POST        | ✅     | Sync: resuelve artículo por código. Preflight: verifica |
| `GetFullArticlesList`             | POST        | ❌     | —                                                       |
| `CreateArticlesAndUpdateProfiles` | POST        | ❌     | —                                                       |
| `ModifyPricesArticles`            | POST        | ❌     | —                                                       |
| `ModifyArticleAndUpdateProfile`   | POST        | ❌     | —                                                       |

### 2.3 Clientes

| Endpoint          | Método HTTP | Estado | Uso actual                          |
| ----------------- | ----------- | ------ | ----------------------------------- |
| `ExportCustomers` | POST        | 📋     | Cliente tiene método, nunca llamado |
| `CreateCustomer`  | POST        | 📋     | Cliente tiene método, nunca llamado |

### 2.4 Comandas (el núcleo de la integración)

| Endpoint          | Método HTTP | Estado | Uso actual                                                            |
| ----------------- | ----------- | ------ | --------------------------------------------------------------------- |
| `CreateOrder`     | POST        | ✅     | Sync: crea comanda (Type=0 Barra, OrderEndType=1). Preflight: dry-run |
| `GetOrder`        | POST        | 📋     | Cliente tiene método, nunca llamado                                   |
| `CancelOrder`     | POST        | 📋     | ⚠️ Devuelve "Subscripción no activada" — endpoint NO disponible       |
| `AddOrderTip`     | POST        | ❌     | —                                                                     |
| `AddOrderPayment` | POST        | 📋     | Cliente tiene método, nunca llamado                                   |
| `InvoiceOrder`    | POST        | 📋     | Cliente tiene método, nunca llamado                                   |

### 2.5 Comentarios

| Endpoint            | Método HTTP | Estado | Uso actual |
| ------------------- | ----------- | ------ | ---------- |
| `GetCommetsProfile` | POST        | ❌     | —          |

### 2.6 Departamentos

| Endpoint                            | Método HTTP | Estado | Uso actual                                   |
| ----------------------------------- | ----------- | ------ | -------------------------------------------- |
| `ExportDepartment`                  | POST        | 📋     | Cliente tiene método, nunca llamado          |
| `DepartmentsExportFromProfile`      | POST        | 🔧     | Preflight: verifica departamentos del perfil |
| `CreateDepartment`                  | POST        | ❌     | —                                            |
| `CreateDepartmentAndUpdateProfiles` | POST        | ❌     | —                                            |

### 2.7 Menús, Fast-Foods, Packs

| Endpoint                | Método HTTP | Estado | Uso actual |
| ----------------------- | ----------- | ------ | ---------- |
| `GetMenuDefinition`     | POST        | ❌     | —          |
| `GetFastfoodDefinition` | POST        | ❌     | —          |
| `GetPackDefinition`     | POST        | ❌     | —          |

### 2.8 Fidelización

| Endpoint    | Método HTTP | Estado | Uso actual |
| ----------- | ----------- | ------ | ---------- |
| `GetPoints` | POST        | ❌     | —          |
| `AddPoints` | POST        | ❌     | —          |

### 2.9 Terminales

| Endpoint   | Método HTTP | Estado | Uso actual                                                |
| ---------- | ----------- | ------ | --------------------------------------------------------- |
| `GetPOS`   | POST        | ⚠️    | **Devuelve `[404401]` desde ~junio 2026** — cambio en API de BDP. No afecta CreateOrder |
| `GetPOSes` | POST        | ⚠️    | **Devuelve respuesta vacía** — limitación de API. No afecta integración               |
| `GetPOSSeriesList` | POST | 🔧     | Probado exitosamente: devuelve las 15 series del terminal |

### 2.10 Empleados

| Endpoint          | Método HTTP | Estado | Uso actual                                 |
| ----------------- | ----------- | ------ | ------------------------------------------ |
| `GetEmployee`     | POST        | 🔧     | Preflight: verifica empleado configurado   |
| `GetEmployees`    | POST        | 📋     | Cliente tiene método, nunca llamado        |
| `GetPOSEmployees` | POST        | 🔧     | Preflight: verifica empleados del terminal |

### 2.11 Formas de Pago

| Endpoint           | Método HTTP | Estado | Uso actual                                      |
| ------------------ | ----------- | ------ | ----------------------------------------------- |
| `GetTenderList`    | POST        | 📋     | Cliente tiene método, nunca llamado             |
| `GetPOSTenderList` | POST        | 🔧     | Preflight: verifica formas de pago del terminal |

### 2.12 No implementados en absoluto (~20 endpoints)

- **Perfiles:** `GetProfilesListCreateDepartmentList`, `GetProfilesListCreateArticleList`, `GetProfileListModifyArticleList`
- **Exportación:** `ExportDocumentsByExportProfile`, `ExportStockAndSalesSummaryByExportProfile`, `ExportManagmentDocumentsByExportProfile`, `ExportPurchaseNotes`
- **Stock:** `CreateFamily`, `CreateSubfamily`, `GetStock`, `GetListStock`, `GetItemCostPrices`, `GetItemsCostPrices`, `Regularizations`, `Transfers`, `UpdateMassiveStock`, `UpdateStock`, `UpdateMassiveInventory`
- **Suplementos:** `GetSupplementsProfiles`, `GetPOSSupplementsProfile`
- **Talla/Color:** `GetInfoSAC`, `GetItemSAC`
- **Salones:** `GetRoomTables`, `GetRoomsTables`
- **Series:** (ya catalogado arriba en 2.9 como 🔧)

---

## 3. Flujo actual (lo que funciona hoy)

```
Glory: Venta creada/actualizada
  → VentaService::spawn_bdp_sync()
    → BdpSyncService::sync_venta()
      → Login a BDP (admin/kamples2026, JWT ~59 min)
      → GetPOSArticlesList para resolver artículo por código
      → CreateOrder con 1 artículo, Type=0 (Barra), OrderEndType=1 (pendiente)
      → Guarda bdp_synced=true, bdp_order_id en BD
```

### Lo que se envía en `CreateOrder`

```json
{
    "Order": {
        "Type": 0,
        "OrderEndType": 1,
        "EmployeeId": 1,
        "ItemsProfileId": 1,
        "AlreadyInvoiced": false,
        "Invoice": false,
        "MarketplaceOrderId": "G<timestamp_15chars>",
        "ExecutionTime": "2026-06-07T12:00:00",
        "Comments": "Venta #<uuid>",
        "Items": [
            {
                "ArtCode": 1001,
                "Units": 1,
                "Price": 45.5,
                "VatPct": 10.0,
                "Description": "Venta #<uuid>",
                "Op": 0
            }
        ],
        "Payments": []
    }
}
```

### Problemas del flujo actual

1. **1 sola línea** — Si la venta tiene 3 productos, BDP recibe 1 línea con el total
2. **Artículo genérico** — Mapea todo a `bdp_default_article_code` (1001 = CAFE BOMBON)
3. **Sin cliente** — `Customer` no se envía
4. **Sin pagos** — `Payments` siempre vacío
5. **Type hardcodeado** — Siempre 0 (Barra), ignora canal real
6. **Sin tracking post-creación** — No se consulta `GetOrder` para saber el estado

---

## 4. Gap de datos: Venta Glory → Order BDP

| Campo Glory                    | Tipo      | ¿Se envía? | Campo BDP disponible        | Notas                                           |
| ------------------------------ | --------- | ---------- | --------------------------- | ----------------------------------------------- |
| `descripcion`                  | `String`  | ❌         | `Order.Comments`            | Trivial de añadir                               |
| `canal`                        | enum      | ❌         | `Order.Type`                | 0=Barra, 1=Mesa, 2=Comedor, etc. Requiere mapeo |
| `metodo_pago`                  | `String`  | ❌         | `Order.Payments[].TenderId` | Requiere mapeo Glory→BDP                        |
| `cliente_id` / datos cliente   | FK        | ❌         | `Order.Customer`            | Requiere lookup de Cliente                      |
| `comensales`                   | `i32`     | ❌         | —                           | No hay campo equivalente en BDP                 |
| `turno`                        | enum      | ❌         | —                           | No hay campo equivalente                        |
| `reserva_id`                   | FK        | ❌         | —                           | No se incluye                                   |
| `importe_base` + `importe_iva` | `Decimal` | Parcial    | `OrderItem.Price`           | Solo se usa `Total`, no desglose                |
| Múltiples líneas               | —         | ❌         | `Order.Items[]`             | **Siempre 1 línea**                             |
| `iva_porcentaje` por línea     | `Decimal` | ❌         | `OrderItem.VatPct`          | Hardcodeado a 10%                               |
| Descuentos por línea           | —         | ❌         | `OrderItem.Discount`        | No implementado                                 |

---

## 5. Datos que BDP ofrece y Aplicación no consume

| Endpoint BDP                       | Datos disponibles                                                         | Utilidad                      |
| ---------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------- |
| `ExportArticles`                   | Catálogo completo: código, descripción, 5 tarifas, dto, IVA, departamento | Sincronizar precios sin config manual       |
| `ExportCustomers`                  | Clientes: nombre, dirección, teléfono, email, NIF                         | Precargar CRM de Glory                      |
| `GetOrder`                         | Estado comanda (0=abierta, 1=enviada, 2=servida, 3=facturada...)          | Saber en Glory si ya se cobró en TPV        |
| `GetPOSTenderList`                 | Formas de pago (efectivo, tarjeta, etc.)                                  | Mapear métodos de pago automáticamente      |
| `ExportDepartment`                 | Departamentos/categorías                                                  | Organizar productos en misma taxonomía      |
| `GetMenuDefinition`                | Definiciones de menús                                                     | Entender agrupaciones de artículos          |
| `GetFastfoodDefinition`            | Definiciones fast-food                                                    | Agrupaciones de artículos                   |
| `GetPackDefinition`                | Definiciones de packs                                                     | Agrupaciones de artículos                   |
| `GetPOSes`                         | Terminales disponibles                                                    | Verificar/configurar POS automáticamente    |
| `GetEmployees`                     | Empleados dados de alta                                                   | Validar/configurar empleado automáticamente |
| `GetPoints` / `AddPoints`          | Puntos de fidelización                                                    | Integración con programa de fidelidad       |
| `GetRoomTables` / `GetRoomsTables` | Salones y mesas                                                           | Mapear plano de sala                        |
| `GetPOSSeriesList`                 | Series de facturación                                                     | Configurar series por terminal              |

---

## 6. Arquitectura actual (archivos involucrados)

| Archivo                                     | Líneas | Rol                                                               |
| ------------------------------------------- | ------ | ----------------------------------------------------------------- |
| `src/services/bdp_weblink.rs`               | ~750   | Cliente HTTP base: login+token, 23 métodos, error sanitization    |
| `src/services/bdp_weblink_catalog.rs`       | ~200   | 21 constantes de ruta, structs request/response, spec inventory   |
| `src/services/bdp_sync.rs`                  | ~480   | `BdpSyncService`: sync_venta, retry, build_order, resolve_article |
| `src/services/bdp_sync_preflight.rs`        | ~460   | `BdpSyncPreflightService`: 9 checks + dry-run CreateOrder         |
| `src/services/venta.rs`                     | ~280   | `VentaService`: hooks create/update/delete para spawn BDP sync    |
| `src/handlers/ventas.rs`                    | ~210   | Endpoint `POST /api/ventas/:id/bdp-sync` (retry manual)           |
| `src/models/venta.rs`                       | ~160   | Modelo Venta + campos bdp_synced, bdp_order_id, etc.              |
| `src/models/configuracion.rs`               | ~100   | Config: bdp_url, bdp_login, bdp_sync_enabled, etc.                |
| `src/repositories/venta.rs`                 | ~380   | `update_bdp_status()` con `sqlx::query()` (sin macro)             |
| `migrations/20260607000000_bdp_sync_fields` | ~20    | Columnas bdp en ventas + configuracion                            |

---

## 7. Configuración BDP actual en `configuracion`

| Campo                      | Ejemplo                     | Descripción                      |
| -------------------------- | --------------------------- | -------------------------------- |
| `bdp_sync_enabled`         | `true`                      | Activa/desactiva sync automática |
| `bdp_url`                  | `http://100.83.196.35:8068` | URL del WebLink API              |
| `bdp_login`                | `admin`                     | Usuario de autenticación         |
| `bdp_password`             | `kamples2026`               | Password (encriptado en BD)      |
| `bdp_integrator_code`      | `VBW2MBM5`                  | Código de integrador             |
| `bdp_pos_id`               | `31`                        | Terminal POS                     |
| `bdp_employee_id`          | `1`                         | Empleado por defecto             |
| `bdp_items_profile_id`     | `1`                         | Perfil de artículos              |
| `bdp_default_article_code` | `1001`                      | Artículo genérico (fallback)     |
| `bdp_default_article_name` | `CAFE BOMBON`               | Nombre del artículo genérico     |

### Campos que faltarían para integración completa

| Campo propuesto             | Tipo     | Para qué                                                                     |
| --------------------------- | -------- | ---------------------------------------------------------------------------- |
| `bdp_tender_id`             | `i32`    | Forma de pago por defecto para mapear `metodo_pago`                          |
| `bdp_order_type_map`        | `jsonb`  | Tabla de mapeo canal → Type (ej: `{"comedor": 1, "barra": 0, "terraza": 0}`) |
| `bdp_default_customer_code` | `String` | Cliente genérico cuando no hay `cliente_id`                                  |
| `bdp_invoice_on_create`     | `bool`   | Si debe facturar automáticamente al crear comanda                            |
| `bdp_poll_interval_secs`    | `i32`    | Intervalo de polling para `GetOrder`                                         |

---

## 8. Restricciones conocidas de la API BDP

| Restricción                                     | Detalle                                    | Impacto                                        |
| ----------------------------------------------- | ------------------------------------------ | ---------------------------------------------- |
| `MarketplaceOrderId` máx 15 chars               | Usamos `G<timestamp_14>`                   | Ninguno (resuelto)                             |
| `AlreadyInvoiced` y `Invoice` son REQUIRED      | Deben ir dentro del objeto `Order`         | Ninguno (resuelto)                             |
| `CancelOrder` NO disponible                     | Devuelve "Subscripción no activada"        | **Alto** — no se pueden cancelar comandas      |
| `Type=0` (Barra) es el único válido para POS 31 | Otros types dan error                      | **Medio** — limita mapeo de canal              |
| JWT expira en ~59 min                           | Re-login automático en cliente             | Ninguno (resuelto)                             |
| `CreateOrder` con `OnlyCheck=true`              | Solo valida, no crea                       | Útil para preflight                            |
| Solo POST y GET                                 | No hay PUT/PATCH/DELETE                    | Las actualizaciones usan endpoints específicos |
| Respuesta de `CreateOrder`                      | Devuelve `OrderId` numérico                | Se guarda como `bdp_order_id`                  |
| `POS/Get` devuelve `[404401]`                   | Desde ~junio 2026, cambio en API de BDP    | **Bajo** — solo afecta preflight/diagnóstico   |
| `POSes/Get` devuelve vacío                      | Limitación de API, no expone terminales    | **Bajo** — no se usa en flujo productivo       |
| Serie no asignable por API                      | WebLink no expode qué serie va en Mesas/Barra | **Medio** — solo configurable en TPV de escritorio |

---

## 9. Prioridades para integración completa

### Fase 1 — Comandas reales (🔴 Crítico)

| #   | Tarea                                                      | Endpoints                | Archivos afectados                                  | Esfuerzo |
| --- | ---------------------------------------------------------- | ------------------------ | --------------------------------------------------- | -------- |
| 1.1 | **Múltiples líneas** — iterar `VentaLinea` → `OrderItem[]` | `CreateOrder`            | `bdp_sync.rs`, modelo `VentaLinea`                  | Medio    |
| 1.2 | **Mapeo artículos** Glory ↔ BDP                            | `GetPOSArticlesList`     | Nuevo: `bdp_article_map` en config o tabla dedicada | Medio    |
| 1.3 | **Cliente en comanda**                                     | `CreateOrder.Customer`   | `bdp_sync.rs`, modelo `Cliente`                     | Bajo     |
| 1.4 | **Pagos en comanda**                                       | `CreateOrder.Payments[]` | `bdp_sync.rs`, config `bdp_tender_id`               | Bajo     |
| 1.5 | **Canal → Type** configurable                              | `CreateOrder.Type`       | `bdp_sync.rs`, config `bdp_order_type_map`          | Bajo     |

### Fase 2 — Lifecycle de comandas (🟡 Importante)

| #   | Tarea                                                       | Endpoints         | Archivos afectados                                      | Esfuerzo                        |
| --- | ----------------------------------------------------------- | ----------------- | ------------------------------------------------------- | ------------------------------- |
| 2.1 | **Polling de estado** — consultar `GetOrder` tras crear     | `GetOrder`        | Nuevo: `bdp_order_lifecycle.rs` o ampliar `bdp_sync.rs` | Medio                           |
| 2.2 | **Reflejar facturación** — si BDP factura, actualizar Glory | `GetOrder`        | Modelo `Venta`: nuevo campo `bdp_invoiced`              | Medio                           |
| 2.3 | **Cancelar desde Glory**                                    | `CancelOrder`     | `bdp_sync.rs`, hook en `VentaService::delete()`         | Bajo (si el endpoint se activa) |
| 2.4 | **Agregar pago desde Glory**                                | `AddOrderPayment` | Nuevo método en `bdp_sync.rs`                           | Bajo                            |
| 2.5 | **Facturar desde Glory**                                    | `InvoiceOrder`    | Nuevo método en `bdp_sync.rs`                           | Bajo                            |

### Fase 3 — Sync bidireccional (🟢 Útil)

| #   | Tarea                             | Endpoints           | Archivos afectados                 | Esfuerzo |
| --- | --------------------------------- | ------------------- | ---------------------------------- | -------- |
| 3.1 | **Exportar clientes BDP → Glory** | `ExportCustomers`   | Nuevo: `bdp_customer_sync.rs`      | Medio    |
| 3.2 | **Crear cliente Glory → BDP**     | `CreateCustomer`    | Hook en `ClienteService::create()` | Bajo     |
| 3.3 | **Exportar catálogo BDP → Glory** | `ExportArticles`    | Nuevo: `bdp_catalog_sync.rs`       | Medio    |
| 3.4 | **Sincronizar precios**           | `ExportArticles`    | Tabla de mapeo artículos           | Medio    |
| 3.5 | **Preflight como gatekeeper**     | Todos los preflight | `bdp_sync_preflight.rs`            | Bajo     |

### Fase 4 — Extensiones (⚪ Futuro)

| #   | Tarea                     | Endpoints                                | Notas                                  |
| --- | ------------------------- | ---------------------------------------- | -------------------------------------- |
| 4.1 | Fidelización (puntos)     | `GetPoints`, `AddPoints`                 | Requiere modelo de puntos en Glory     |
| 4.2 | Menús y packs             | `GetMenuDefinition`, `GetPackDefinition` | Solo lectura informativa               |
| 4.3 | Stock                     | `GetStock`, `UpdateStock`                | Requiere modelo de inventario en Glory |
| 4.4 | Salones y mesas           | `GetRoomTables`                          | Requiere integración con plano de sala |
| 4.5 | Exportación de documentos | `ExportDocumentsByExportProfile`         | Reportes/contabilidad                  |

---

## 10. Diagrama de flujo objetivo (Fase 1+2)

```mermaid
sequenceDiagram
    participant G as Glory (Rust)
    participant B as BDP WebLink API
    participant P as POS 31

    Note over G: Venta creada/actualizada

    G->>B: Login (JWT)
    B-->>G: Token (59 min)

    G->>B: GetPOSArticlesList
    B-->>G: Artículos del perfil

    Note over G: Mapear VentaLineas → OrderItems

    G->>B: CreateOrder (múltiples líneas, cliente, pagos, canal)
    B-->>G: OrderId
    Note over G: Guardar bdp_order_id

    B->>P: Comanda aparece en TPV

    loop Polling (cada 30s)
        G->>B: GetOrder (OrderId)
        B-->>G: Status (0=abierta, 1=enviada, 2=servida, 3=facturada)
        Note over G: Actualizar estado local
    end

    Note over P: Cajero factura/cobra en TPV

    G->>B: GetOrder
    B-->>G: Status=3 (facturada)
    Note over G: Marcar venta como cobrada en Glory
```

---

## 11. Lecciones aprendidas (de implementación 065A-5)

| Lección                                         | Detalle                                                  |
| ----------------------------------------------- | -------------------------------------------------------- |
| `AlreadyInvoiced` DEBE ir dentro de `Order`     | Si va fuera, BDP devuelve error 300005                   |
| `AlreadyInvoiced` e `Invoice` son REQUIRED      | Ambos deben estar presentes siempre                      |
| `Type=0` (Barra) es el único válido para POS 31 | Otros types devuelven error 300047                       |
| `MarketplaceOrderId` máx 15 chars               | Usar `G<timestamp_14>`                                   |
| `CancelOrder` no está disponible                | Devuelve "Subscripción no activada" — no depende de esto |
| `CreateOrder` con `OnlyCheck=true` es útil      | Permite validar sin crear (preflight dry-run)            |
| `sqlx::query()` sin macro para nuevas columnas  | Evita necesitar `cargo sqlx prepare` cada vez            |
| Mutex por `venta_id`                            | Evita race conditions en sync concurrente                |
| `#[serde(rename_all = "PascalCase")]`           | Fundamental para el contrato JSON de BDP                 |
| JWT expira en ~59 min                           | El cliente maneja re-login automáticamente               |
| `POS/Get` cambió a `[404401]` (junio 2026)      | API de BDP actualizada; no afecta CreateOrder            |
| `MarketplaceOrderId` validado estrictamente      | Máx 15 chars — error `[301011]` si excede                |
| La API no expone asignación Mesas/Barra          | Solo visible en TPV de escritorio, no por WebLink        |
| Serie `00031TI` sigue activa tras problemas      | Cliente resolvió los 4 problemas con su técnico          |

---

## 12. Archivos de referencia

| Archivo                                                    | Contenido                                                |
| ---------------------------------------------------------- | -------------------------------------------------------- |
| `WEBLINK RESTAPI.md`                                       | Documentación completa de la API BDP (raíz del proyecto) |
| `src/services/bdp_weblink_catalog.rs`                      | Constantes de ruta + structs de request/response         |
| `src/services/bdp_weblink.rs`                              | Cliente HTTP con auth, 23 métodos                        |
| `src/services/bdp_sync.rs`                                 | Servicio de sync Glory → BDP                             |
| `src/services/bdp_sync_preflight.rs`                       | Preflight/dry-run                                        |
| `Agente/planes/plan-bdp-sync-implementation-2026-06-07.md` | Plan de implementación original                          |
| `Agente/planes/plan-bdp-testing-2026-06-07.md`             | Plan de testing                                          |
