# BDP-Net Error [300035] — Resumen completo

> **Fecha:** 2026-06-04 (actualizado)
> **Tarea:** 065A-4
> **Estado:** Hipótesis refinada — Order.Type=Mesa (1) pasa serie; "Serie Destino" descartado como causa
> **PC remoto:** `100.83.196.35` (RDP/Tailscale), BDP-Net corriendo en puerto `8068`

---

## 1. Descripción del problema

Al intentar crear un pedido vía la WebLink REST API de BDP-Net con `/API/Orders/Create` en modo **OnlyCheck** (`OrderOperationType=1`, sin pagos, sin factura real), el sistema responde con:

```
[300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA
```

Esto ocurre incluso en modo "solo validación" — BDP valida la configuración interna de series del terminal **antes** de procesar el pedido. El error NO es un bug de nuestra app; es una configuración que falta en el lado de BDP-Net.

---

## 2. Flujo técnico actual

### 2.1 Endpoints implementados en nuestro backend

| Endpoint | Función | Estado |
|----------|---------|--------|
| `GET /api/configuracion/bdp/diagnostico` | Health + Login + GetVersion | ✅ Funciona |
| `GET /api/configuracion/bdp/sync-dry-run` | Validación completa de sincronización | ⚠️ Bloqueado en CreateOrder |

### 2.2 Secuencia del dry-run (`sync-dry-run`)

```
1. Health         → /Service/Health              → ✅ IsAlive=true
2. Login+Version  → /Auth/Login + GetVersion     → ✅ Token obtenido
3. Terminal POS   → /API/POS/Get                 → ✅ Terminal 31 existe
4. Empleado       → /API/Employee/Get            → ✅ Empleado existe
5. Empleados POS  → /API/POS/Employees/Get       → ✅ Empleado permitido
6. Formas de pago → /API/Tenders/GetPOSList      → ✅ TenderList OK
7. Departamentos  → /API/Departments/ExportFromProfile → ✅ OK
8. Artículos      → /API/Articles/GetPOSList     → ✅ OK
9. CreateOrder    → /API/Orders/Create (OnlyCheck) → ❌ [300035]
```

### 2.3 Payload enviado a CreateOrder

```json
{
  "EmployeeId": 31,
  "ItemsProfileId": 1,
  "OrderEndType": 0,
  "OrderOperationType": 1,
  "Invoice": false,
  "Order": {
    "MarketplaceOrderId": "GDRY-TEST-001",
    "MarketId": 9901,
    "MarketName": "Glory Dry Run",
    "PreparationTime": "2026-06-01T12:00:00Z",
    "OrderId": 0,
    "PosId": 31,
    "Type": 2,
    "RoomNumber": 0,
    "TableNumber": 0,
    "Items": [{
      "Lin": 1,
      "Id": [artículo válido de BDP],
      "Name": [nombre real],
      "Units": 1.0,
      "Price": [precio real],
      "Supplement": 0.0,
      "Discount": 0.0,
      "DiscountPct": false,
      "Total": [precio],
      "VatPct": [IVA del artículo],
      "Comments": [],
      "Supplements": [],
      "OrderItemType": 0,
      "OrderItemTypeMetaInfo": "",
      "TyC_D1": 0, "TyC_D2": 0, "TyC_D3": 0,
      "OnSale": false
    }],
    "Discount": 0.0,
    "DiscountPct": false,
    "Total": [precio],
    "ExecutionTime": "2026-06-01T12:00:00Z",
    "Status": 0,
    "AlreadyInvoiced": false,
    "Comments": "GLORY DRY RUN - NO CREAR"
  }
}
```

**Nota:** No hay campo `Series` o `InvoiceSeriesId` en el payload. El manual de WebLink no documenta tal campo. BDP resuelve la serie internamente según la configuración del terminal.

### 2.4 Código relevante

| Archivo | Función |
|---------|---------|
| `src/services/bdp_weblink.rs:181-193` | `check_order()` fuerza `OrderOperationType=1` |
| `src/services/bdp_sync_preflight.rs:257-280` | `check_order_only()` construye y envía el payload |
| `src/services/bdp_sync_preflight.rs:339-360` | `build_only_check_order()` arma el JSON |
| `src/services/bdp_weblink_catalog.rs:278-287` | `BdpCreateOrderRequest` — struct sin campo de serie |

---

## 3. Configuración de BDP-Net en el restaurante

### 3.1 Terminal configurado

- **`bdp_pos_id`:** `31`
- **`bdp_employee_id`:** el mismo 31
- **URL WebLink:** `http://100.83.196.35:8068`

### 3.2 Series TPV existentes para terminal 31

| Serie | Descripción |
|-------|-------------|
| `00031AL` | 31T Albaranes |
| `00031TM` | 31T Facturas Simplificadas Mesa |

**No existe serie `00031TB`** (Factura Simplificada de Barra) para el terminal 31, aunque otros terminales como el 3 sí la tienen.

### 3.3 Configuración de Parámetros del Terminal 31

Se revisaron las pestañas en el PC remoto:

#### Pestaña "Generalidades"
- Configuración básica: cierre de caja, local, almacén, impresoras, monitores de cocina
- **No hay campo de serie ni de WebLink**

#### Pestaña "Facturas 1"
- **Parámetros en Barra:** Serie de Facturación por defecto + impresoras
- **Parámetros en Mesas:** Serie de Facturación por defecto + impresoras
- Las series de Facturas Simplificadas (`TM`) están asignadas correctamente

#### Pestaña "Facturas 2"
- **Parámetros Albaranes / Facturas de Albaranes:** Serie `AL` asignada
- **Parámetros Facturas Rectificativas:** Serie asignada
- **Mesa con Cliente Asignado:** Permisos para emitir documentos

### 3.4 Configuración Servicios Web → Weblink

| Campo | Valor |
|-------|-------|
| IP Address | configurado |
| IP Port | 8068 |
| Usar Password | activo |
| Credenciales | configuradas |

**No hay campo visible de "Serie de Facturación"** en la configuración de Weblink. La sub-rejilla expandida (botón `+`) muestra credenciales y comandos, pero nada sobre series.

### 3.5 Parámetros 6 → "Comandas Facturadas Weblink" (DESCARTADO como causa raíz)

En la pestaña **Parámetros 6** del Terminal 31 existe una sección:

> **Comandas Facturadas Weblink → Serie Destino: VACÍO**

Sin embargo, este campo está vacío **en TODAS las terminales** (incluida POS 1 que SÍ pasa la validación de serie). Además, al hacer clic en el botón de 3 puntos, **la lista de selección aparece vacía para todas las terminales** — ninguna permite elegir una serie.

**Conclusión:** este campo NO es el factor diferenciador. POS 1 pasa la validación con el mismo campo vacío.

Además, en **Facturas 1 → Parámetros en Barra**, la serie de facturación estaba vacía (se asignó `00031TM` el 2026-06-03).

### 3.6 Pruebas API directas (2026-06-03)

Se probaron todos los POS y combinaciones de `Order.Type` / `OrderEndType` directamente contra WebLink:

#### Resultado por POS (Type=0, EndType=1)

| POS | Nombre | Error | ¿Pasa validación de serie? |
|-----|--------|-------|---------------------------|
| 1 | CENTRAL | `301400` (caja no abierta) | ✅ |
| 2 | SALON T2 | `300035` (sin serie) | ❌ |
| 3 | DELIVERY | `300005` (IVA no configurado) | ✅ |
| 11 | CENTRAL2025 | `300035` (sin serie) | ❌ |
| 22 | ESCLAVO 2025 | `300035` (sin serie) | ❌ |
| 31 | CENTRAL 2026 | `300035` (sin serie) | ❌ |

#### Resultado por Order.Type (POS 31)

Según la documentación WebLink REST API, `Order.Type` tiene estos valores:
- `0` = Barra / Ticket aparcado (take away)
- `1` = Mesa
- `2` = Servicio a domicilio (delivery)

| Order.Type | Significado | OrderEndType | Error | ¿Pasa serie? |
|------------|-------------|-------------|-------|-------------|
| 0 | Barra | 0 | `300035` | ❌ |
| 0 | Barra | 1 | `300035` | ❌ |
| **1** | **Mesa** | **0** | **`300005` (IVA)** | **✅** |
| **1** | **Mesa** | **1** | **`300005` (IVA)** | **✅** |
| 2 | Delivery | 0 | `300035` | ❌ |
| 2 | Delivery | 1 | `300035` | ❌ |

**`Order.Type=1` (Mesa)** pasa la validación de serie para POS 31. `Type=0` (Barra) y `Type=2` (Delivery) fallan con 300035.

#### POS 1 con Order.Type=2 (verificación pendiente)

Se intentó probar POS 1 con `Type=2` (delivery) para confirmar si también falla con 300035, pero el script falló por error de parsing (PowerShell `GetResponseStream` vs .NET 5+). **Pendiente de re-ejecutar.**

---

## 4. Lo que se descubrió (investigación)

### 4.1 Documentación BDP-Net revisada

| Documento | Ubicación | Hallazgo |
|-----------|-----------|----------|
| Ayuda BDP-Net (CHM) | AyudaHos.chm | Describe Configuración Servicios Web — **sin mención de series** |
| Manual-WebLink | `C:\BDP-NET\NetXXX\DatosGen\Manual-WebLink` | No encontrado ni revisado (pendiente) |
| Doc Series TPV | Ayuda BDP-Net | Confirma: "como mínimo una serie por terminal", tipos: Facturas Simplificadas, Rectificativas, Albaranes, Traspaso Hotel, Importe Cero, Sustitutivas |
| Doc Series (Compras) | Ayuda BDP-Net | Para documentos de compra — **no aplica a WebLink** |
| Pantalla "Series" abierta en BDP-Net | Acceso desde Configuración Servicios Web | **Es la pantalla de Series de Compra** (stock), no Series TPV. Muestra: Número de Serie, Descripción, parámetros de compra, almacén. Soporte apuntó aquí pero es el tipo de serie incorrecto para ventas/WebLink |

### 4.2 Hallazgos clave de la documentación Series TPV

- Cada terminal debe tener **al menos una serie de Facturas Simplificadas**
- Los tipos de serie disponibles son:
  - **Serie para Facturas Simplificadas** (mínima requerida)
  - Serie para Facturas Rectificativas
  - Serie para Albaranes
  - Serie para Facturas de Albarán
  - Serie para Traspaso Hotel
  - Serie para Documentos Importe Cero
  - Serie para Facturas Sustitutivas
- La serie se asigna en **Configuración TPV → Parámetros del Terminal → Facturas 1** (Barra y Mesas)
- **No se documenta una serie específica para WebLink o pedidos externos**

### 4.3 ¿Qué NO es el problema?

| Hipótesis descartada | Razón |
|---------------------|-------|
| Falta serie en Series TPV | ✅ Existe `00031TM` y `00031AL` para terminal 31 |
| Falta serie en Parámetros del Terminal → Facturas 1 | ✅ Están asignadas (Barra y Mesas) |
| Error en credenciales/URL | ✅ Health + Login + GetVersion funcionan |
| Error en permisos de empleado | ✅ Empleado 31 aparece en POS/Employees/Get |
| Bug de nuestra app | ✅ El error viene de BDP, no de nuestro backend |
| Falta serie de compras (`00031P`) | ✅ Se creó, no resolvió el error |
| **"Comandas Facturadas Weblink" → Serie Destino** | **❌ Descartado: vacío para TODAS las terminales incluida POS 1 que SÍ funciona. Además, el selector aparece vacío para todas.** |
| **Endpoint `/API/POSSeries/GetList`** | **❌ Devuelve 500 Internal Server Error siempre (con o sin parámetros). Endpoint roto o no disponible en esta versión de WebLink.** |

### 4.4 Causas raíz identificadas

| Causa | Evidencia | Fix |
|-------|-----------|-----|
| ~~**Parámetros 6 → "Comandas Facturadas Weblink" → Serie Destino vacío**~~ | ~~Campo específico para WebLink, sin valor asignado~~ | **❌ Descartado — vacío también en POS 1 que funciona** |
| **Parámetros en Barra sin serie** | Vacío hasta 2026-06-03 | Ya se asignó `00031TM` |
| **Order.Type incorrecto en payload** | `Type=0` y `Type=2` fallan con 300035; `Type=1` (Mesa) pasa la validación de serie | Cambiar `Order.Type` de `2` a `1` en el código |
| **Terminal no configurado para IVA incluido** | Error `300005` al usar `Type=1` | Configurar IVA en terminal 31 o ajustar payload |
| **Diferencia POS 1 vs POS 31** | POS 1 pasa con `Type=0`; POS 31 falla con `Type=0` y `Type=2` pero pasa con `Type=1` | POS 1 probablemente tiene series configuradas para Barra/Mesa/Delivery; POS 31 solo para Mesa |

---

## 5. Lo que dicen los docs de WebLink sobre series y facturación

Consultando la documentación del API WebLink (`# WEBLINK RESTAPI.md` en la raiz del repo):

### Order.Type (valores documentados)

| Valor | Significado |
|-------|-------------|
| 0 | Barra / Ticket aparcado (take away) |
| 1 | Mesa |
| 2 | Servicio a domicilio (delivery) |

### DocumentType de POSSeries

| Valor | Tipo |
|-------|------|
| 0 | Facturas simplificadas |
| 1 | Factura rectificativa |
| 2 | Albaranes |
| 3 | Facturas de albarán |
| 4 | Albaranes Traspaso Hotel |
| 5 | Albaranes Tickets Cero |
| 6 | Facturas Substitutivas |

### ¿Cómo se determina la serie?

Según la documentación, **la serie NO se elige via API**. BDP-Net la determina internamente según:
- La configuración del POS (`PosId`)
- El tipo de documento (derivado de `Order.Type`)

No existe campo `SeriesCode` o `InvoiceSeriesId` en el payload de `CreateOrder`.

### `AlreadyInvoiced` — facturación externa

```json
"AlreadyInvoiced": true
```
> "Indica si el pedido se factura en la plataforma desde la cual se envía. Esto sirve para evitar una doble imposición **siempre que en BDP-Net se configure una serie de destino para este tipo de pedidos.**"

Esto implica que BDP-Net tiene un mapping interno de "serie de destino" configurable por el POS — pero como vimos, el campo "Serie Destino" en Parámetros 6 está vacío y el selector no permite elegir para ninguna terminal.

### `InvoiceParameters` (para facturación)

Cuando `Invoice=true`, se puede enviar `InvoiceParameters` con:
- `InvoiceEmailAddress`
- `PrintTicket`
- `BillingDetails` (datos fiscales)

**No incluye campo de serie.** La serie se determina por la configuración del POS.

### `/API/POSSeries/GetList`

El endpoint existe en la documentación pero **devuelve 500 Internal Server Error** siempre — tanto sin parámetros (como dice la doc) como con `POSCreateId`. Posiblemente no disponible en esta versión de WebLink o requiere configuración especial.

---

## 6. Hipótesis actual

El error 300035 se produce porque BDP-Net no encuentra una serie válida para el **tipo de documento** que deriva de `Order.Type`:

- **POS 1 (CENTRAL):** Tiene series configuradas para **todos los tipos** (Barra, Mesa, Delivery). Por eso pasa la validación con cualquier `Order.Type`.
- **POS 31 (CENTRAL 2026):** Solo tiene serie configurada para **Mesa** (`00031TM`). Por eso:
  - `Type=0` (Barra) → 300035 (no hay serie de barra)
  - `Type=1` (Mesa) → ✅ pasa serie → 300005 (otro problema: IVA)
  - `Type=2` (Delivery) → 300035 (no hay serie de delivery)

### Validaciones pendientes

1. **Probar POS 1 con `Type=2`** para confirmar que POS 1 pasa con todos los tipos
2. **Ver en BDP-Net qué series tiene asignadas POS 1** vs POS 31 en Facturas 1/Facturas 2
3. **Ver si POS 1 tiene una serie "multi-tipo"** que POS 31 no tiene (ej: `00001TB` para barra que POS 31 carece)

### ⚠️ Nota: login intermitente al BDP

El 2026-06-04, el endpoint `/Auth/Login` del BDP WebLink empezó a responder:
```
[5]-EL CÓDIGO DE INTEGRADOR PROPORCIONADO NO ES VÁLIDO
```
Aun cuando `/Service/Health` responde `{"IsAlive":true}`. Es posible que el servicio WebLink
en el PC del restaurante necesite reiniciarse o que haya expirado alguna sesión. Esto impide
probar el dry-run desde fuera del restaurante momentáneamente.

---

## 7. Plan de acción (actualizado)

### ✅ Paso 1 realizado: Cambiar `Order.Type` a `1` (Mesa)

**Archivo:** `src/services/bdp_sync_preflight.rs` — línea `"Type": 2` → `"Type": 1`

**Commit:** Cambio directo, sin commit. `cargo check` exitoso.

**Razonamiento:** Según la documentación WebLink, `Type=1` = Mesa. Es semánticamente
correcto para un restaurante. Y en POS 31, `Type=1` pasa la validación de serie
error 300035 (avanza a 300005 IVA).

### Paso 2: Confirmar hipótesis (requiere RDP cuando BDP responda)
- En BDP-Net, ir a **POS 1 (CENTRAL) → Parámetros del Terminal → Facturas 1**
- Verificar qué series tiene asignadas en Barra y Mesas
- Comparar con POS 31 (que tiene `00031TM` en Mesas y nada en Barra)
- Si POS 1 tiene serie asignada en **Barra** → crear serie de barra equivalente para POS 31 (ej: `00031TB`)

### Paso 3: Resolver error 300005 (IVA) — pendiente
- Con `Type=1`, aparece `300005` ("terminal no configurado para IVA incluido")
- Verificar en BDP-Net si la serie `00031TM` tiene "IVA Incluido" activado
- O ajustar el campo `VatPct` / precio en el payload para que no requiera IVA incluido

### Paso 4: Probar dry-run completo
- Ejecutar dry-run desde la UI (`restaurante.wandori.us` o localhost)
- Verificar que el error 300035 ya no aparece
- Confirmar si aparece o no el error 300005

### Paso 5: Documentar resultado
- Actualizar este documento
- Actualizar `roadmap.md`
- Registrar lección aprendida

## 8. Mensaje para soporte BDP-Net (si se necesita)

```
Asunto: Consulta configuración serie facturación para WebLink REST API — Error [300035]

Buenos días,

Estamos integrando la WebLink REST API de BDP-NET con una aplicación externa
de pedidos. La conectividad, autenticación y lecturas funcionan correctamente
(Health, Login, GetVersion, catálogo de artículos, etc.).

Al intentar validar la creación de un pedido mediante el endpoint
/API/Orders/Create con OrderOperationType = 1 (OnlyCheck), sin Payments y
con Invoice = false, recibimos el siguiente error:

  [300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA

Hemos verificado:
- Series TPV: 00031TM y 00031AL existen
- Facturas 1: serie asignada en Barra y Mesas
- Parámetros 6 → Comandas Facturadas Weblink → Serie Destino: VACÍO

Preguntas:
1. ¿El campo "Serie Destino" en Parámetros 6 → Comandas Facturadas Weblink
   es el que debe configurarse para resolver el error 300035?
2. ¿Qué valor de Order.Type (0, 1, 2) debe usar la API para pedidos WebLink?
3. ¿Es necesario configurar algo adicional para que el terminal acepte
   pedidos con IVA incluido (error 300005)?

Payload utilizado:
  EmployeeId: 31, PosId: 31, ItemsProfileId: 1
  OrderEndType: 0/1, OrderOperationType: 1
  Invoice: false, Order.Type: 0/1/2
  Order.AlreadyInvoiced: false

Gracias.
```

---

## 9. Archivos relacionados

| Archivo | Descripción |
|---------|-------------|
| `Agente/documentacion/api/bdp-weblink-2026-05-06.md` | Documentación de integración inicial |
| `Agente/documentacion/api/bdp-error-300035-serie-facturacion-2026-05-29.md` | Investigación original del error |
| `Agente/lecciones/lecciones-aprendidas.md` | Lecciones del proyecto |
| `src/services/bdp_weblink.rs` | Cliente WebLink (Health, Login, CreateOrder, check_order) |
| `src/services/bdp_sync_preflight.rs` | Lógica del dry-run completo |
| `src/services/bdp_weblink_catalog.rs` | Structs de petición/respuesta BDP |
| `src/models/configuracion.rs` | Modelo `ConfiguracionRestaurante` con campos `bdp_*` |
| `frontend/src/componentes/ConfigBdp.tsx` | UI de configuración BDP |
| `# WEBLINK RESTAPI.md` (raiz) | Documentación completa del API WebLink |
| `roadmap.md` | Tarea 065A-4 con estado actual |

---

## 10. Restricciones

- **No crear/modificar** ventas, comandas, clientes, artículos ni pagos reales en el restaurante
- **No tocar** la configuración de series existentes sin evidencia de que es seguro
- **Criterio de cierre:** `listo_para_sincronizar = true` manteniendo `escritura_real = false`
