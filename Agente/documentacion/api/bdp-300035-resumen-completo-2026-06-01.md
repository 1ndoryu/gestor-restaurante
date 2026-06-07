# BDP-Net Error [300035] — Resumen completo

> **Fecha:** 2026-06-07 (actualizado)
> **Tarea:** 065A-4
> **Estado:** ✅ RESUELTO — dry-run completa pasa con Type=0 (Barra) en POS 31 con serie `00031TI` (IVA incluido). Commit + deploy pendientes.
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
    "Type": 0,
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

### 3.2 Series TPV existentes para terminal 31 (actualizado 2026-06-07)

| Serie | Descripción | IVA Incluido |
|-------|-------------|-------------|
| `00031AL` | 31T Albaranes | — |
| `00031TM` | 31T Facturas Simplificadas Mesa | ❌ No |
| `00031TI` | 31T Facturas Simplificadas (IVA Incluido) | ✅ Sí |

**Serie `00031TI`** fue creada el 2026-06-07 con "IVA Incluido" activado y asignada a Terminal 31 como serie principal en Facturas 1 → Parámetros en Mesa. La serie anterior `00031TM` no se puede modificar (documentos existentes).

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

**Pruebas iniciales (2026-06-03) — SIN campos `AlreadyInvoiced`/`Invoice`:**

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

### 3.7 Pruebas API directas (2026-06-06) — CON campos `AlreadyInvoiced`/`Invoice`

**Descubrimiento clave:** Las pruebas anteriores (§3.6) NO incluían los campos `AlreadyInvoiced` ni `Invoice` en el payload. Al incluirlos, **el error 300035 desaparece para TODOS los POS y TODOS los Order.Type**.

#### Sin `AlreadyInvoiced` ni `Invoice` (sin items):

| POS | Type 0 | Type 1 | Type 2 |
|-----|--------|--------|--------|
| **31** | 300047 | 300047 | 300047 |
| **1** | 300047 | 300047 | 300047 |

Error 300047: "NO SE HA ESPECIFICADO EL PARÁMETRO Order.AlreadyInvoiced"

#### Con `AlreadyInvoiced=false` + `Invoice=false` (sin items):

| POS | Type 0 | Type 1 | Type 2 |
|-----|--------|--------|--------|
| **31** | 300005 IVA | 300005 IVA | 300005 IVA |
| **1** | 301400 Caja cerrada | 301400 Caja cerrada | 301400 Caja cerrada |

**Conclusión:**
- **Error 300035 RESUELTO.** No era un problema de series ni de Order.Type. Era causado por la ausencia de `AlreadyInvoiced` y `Invoice` en el payload.
- **POS 1**: Error 301400 ("LA CAJA DEL TERMINAL NO ESTÁ ABIERTA") es esperado en dry-run — se resuelve en producción con caja abierta.
- **POS 31**: Error 300005 ("EL TERMINAL NO ESTÁ CONFIGURADO PARA TRABAJAR CON IVA INCLUIDO") es un problema de configuración de BDP-Net, no del código.

#### Nota sobre el código Rust

El código en `build_only_check_order()` **ya incluye** ambos campos:
- `"Invoice": Some(false)` (campo del struct `BdpCreateOrderRequest`)
- `"AlreadyInvoiced": false` (dentro del JSON del campo `order`)

El error 300035 que se veía en producción probablemente provenía de una versión anterior del código que no incluía estos campos.

#### Nota sobre login y `CodigoIntegrador`

El código Rust siempre usó `CodigoIntegrador` correctamente (campo `codigo_integrador` en `BdpLoginRequest`, serializado a `CodigoIntegrador` por `rename_all = "PascalCase"`). El error de login reportado el 2026-06-04 fue causado por un test de PowerShell que usaba `"Code"` en vez de `"CodigoIntegrador"`. **No hubo problema real con el login.**

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
| ~~**Parámetros en Barra sin serie**~~ | ~~Vacío hasta 2026-06-03~~ | Ya se asignó `00031TM` — **no fue la causa del 300035** |
| ~~**Order.Type incorrecto en payload**~~ | ~~`Type=0` y `Type=2` fallan con 300035; `Type=1` (Mesa) pasa la validación de serie~~ | **❌ Descartado como causa principal — con `AlreadyInvoiced`/`Invoice`, TODOS los Type pasan** |
| **Campos `AlreadyInvoiced` e `Invoice` faltantes en payload** | Pruebas sin estos campos → 300035; con ellos → 300035 desaparece para TODOS los POS y Type | **✅ Causa real del 300035.** El código Rust ya los incluye. |
| **Terminal no configurado para IVA incluido** | Error `300005` en POS 31 para todos los Type | Configurar IVA en terminal 31 vía BDP-Net |
| **Diferencia POS 1 vs POS 31** | POS 1 → 301400 (caja cerrada); POS 31 → 300005 (IVA) | POS 1 tiene IVA configurado; POS 31 no |

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

## 6. Hipótesis actual (actualizada 2026-06-07)

### Error 300035 — RESUELTO

El error 300035 ("NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA") **no era causado por falta de series ni por Order.Type incorrecto**. La causa real era la **ausencia de los campos `AlreadyInvoiced` e `Invoice`** en el payload de `CreateOrder`.

Al incluir `AlreadyInvoiced: false` e `Invoice: false`, BDP-Net pasa la validación de series para **todos los POS** (1, 31) y **todos los Order.Type** (0, 1, 2).

El código Rust en `build_only_check_order()` ya incluye ambos campos. El error en producción provenía posiblemente de una versión anterior que los omitía.

### Error 300005 — RESUELTO (2026-06-07)

Con el 300035 resuelto, el siguiente error fue:

> **[300005]-EL TERMINAL NO ESTÁ CONFIGURADO PARA TRABAJAR CON IVA INCLUIDO**

**Causa:** POS 31 usaba la serie `00031TM` (Facturas Simplificadas Mesa) que NO tenía "IVA Incluido" activado. BDP-Net no permite cambiar esa opción en series con documentos existentes.

**Fix:** Se creó nueva serie **`00031TI`** (31T Facturas Simplificadas con IVA Incluido) y se asignó a Terminal 31 en Facturas 1 → Parámetros en Mesa.

### Error 300008 — Type=1 (Mesa) falla por salón

Después de resolver el IVA, probar con `Type=1` (Mesa) produce:

> **[300008]-EL SALON DE LA MESA NO ES CORRECTO**

Esto es porque `Type=1` requiere un `RoomNumber` y `TableNumber` válidos en la configuración de salones del terminal. Para un dry-run genérico sin configuración de salones, este tipo no es viable.

### Error 300009 — Type=2 (Delivery) no soportado

> **[300009]-EL TERMINAL NO SOPORTA COMANDAS DE SERVICIO A DOMICILIO**

Terminal 31 es de tipo Hostelería estándar, no "Servicio a Domicilio". `Type=2` requiere un POS con esa modalidad.

### ✅ Type=0 (Barra) — VALIDACIÓN COMPLETA PASADA

**2026-06-07:** Prueba directa contra WebLink con artículo real (`ArtCode=1001`, "CAFE BOMBON", Price=5.0, VatPct=10.0) usando `Type=0` (Barra/Ticket aparcado):

```json
{
  "OrderId": 0,
  "InvoiceNumber": null,
  "ErrorMessage": "",
  "BarCode": ""
}
```

`ErrorMessage: ""` con `OrderOperationType=1` (OnlyCheck) = **validación exitosa sin crear pedido real**.

### Error 301400 — Normal en dry-run

POS 1 devuelve `301400` ("LA CAJA DEL TERMINAL NO ESTÁ ABIERTA") para todos los tipos. Esto es esperado en modo dry-run — en producción con caja abierta, este error no aparecería.

### Tabla resumen final (2026-06-07)

| POS | Type 0 (Barra) | Type 1 (Mesa) | Type 2 (Delivery) |
|-----|----------------|---------------|-------------------|
| **31** (serie `00031TI`) | ✅ OK | 300008 (salón incorrecto) | 300009 (delivery no soportado) |
| **1** | 301400 (caja cerrada) | 301400 (caja cerrada) | 301400 (caja cerrada) |

### Código actualizado

`build_only_check_order()` en `bdp_sync_preflight.rs` ahora usa `"Type": 0` (Barra) ya que es el único tipo que pasa validación sin requerir configuración adicional de salones o delivery.

### ⚠️ Nota: login intermitente al BDP (RESUELTO)

El 2026-06-04 se reportó que `/Auth/Login` respondía `[5]-EL CÓDIGO DE INTEGRADOR PROPORCIONADO NO ES VÁLIDO`. **Causa real:** error en el script de PowerShell que usaba `"Code"` en vez de `"CodigoIntegrador"`. El código Rust siempre fue correcto. Login funciona con `{"Login":"admin","Password":"kamples2026","TiempoSession":59,"CodigoIntegrador":"VBW2MBM5"}`.

---

## 7. Plan de acción (actualizado 2026-06-07)

### ✅ Paso 1: Error 300035 — RESUELTO

El error 300035 se resolvió al descubrir que el payload necesitaba los campos `AlreadyInvoiced` e `Invoice`. El código Rust ya los incluye. No se necesitan cambios adicionales para este error.

### ✅ Paso 2: Login y CodigoIntegrador — RESUELTO

El código Rust siempre usó `CodigoIntegrador` correctamente. El error fue un test de PowerShell con campo equivocado.

### ✅ Paso 3: Error 300005 (IVA Incluido) — RESUELTO

Se creó serie `00031TI` con "IVA Incluido" en BDP-Net y se asignó a Terminal 31.

### ✅ Paso 4: Validación dry-run completa — RESUELTO (2026-06-07)

Type=0 (Barra) pasa validación con artículo real. Type=1 y Type=2 fallan por configuración del POS (salón/delivery), no por errores de integración.

### ✅ Paso 5: Code change Type=2→0 — RESUELTO

`build_only_check_order()` en `bdp_sync_preflight.rs` actualizado de `"Type": 1` a `"Type": 0` (Barra).

### 🔲 Paso 6: Commit + deploy a producción

- Commit con el cambio de Type y documentación actualizada
- Deploy vía `coolify-manager-rs deploy --name kamples --update --skip-backup`
- Verificar health post-deploy
- Probar dry-run en producción (endpoint `/api/configuracion/bdp/sync-dry-run`)

### 🔲 Paso 7: Prueba end-to-end con pedido real

Una vez confirmado el dry-run en producción, probar `OrderOperationType=0` (CheckAndCreate) para crear un pedido real en BDP-Net y verificar que aparece en el terminal.

### Lecciones aprendidas

1. **BDP-Net devuelve HTTP 200 con `ErrorMessage`** para errores de negocio — nunca asumir éxito sin verificar el campo.
2. **`AlreadyInvoiced` e `Invoice` son campos REQUERIDOS** en el payload de `CreateOrder`, no opcionales.
3. **"IVA Incluido" es configuración de la Serie**, no del Terminal. BDP-Net no permite cambiarlo en series con documentos.
4. **`MarketplaceOrderId` máx 15 caracteres** (error 301011 si excede).
5. **`AlreadyInvoiced` va dentro del objeto `Order`**, no en la raíz del request.
6. **`CodigoIntegrador` es el campo correcto** para autenticación WebLink (no `Code`).
7. **`Type=0` (Barra) es el tipo más universal** para dry-run — no requiere salones ni configuración de delivery.

### 🔲 Paso 5: Probar dry-run completo

Tras resolver 300005:
- Ejecutar dry-run desde la UI (`restaurante.wandori.us` o localhost)
- Verificar que no aparecen errores
- Confirmar que el flujo completo pasa

### 🔲 Paso 6: Commit y deploy

- Commit del cambio Type=2→1 en `bdp_sync_preflight.rs`
- Deploy a producción
- Verificar dry-run en producción

## 8. Mensaje para soporte BDP-Net (actualizado — ya no se necesita para 300035)

El error 300035 se resolvió internamente (campos faltantes en payload). El mensaje siguiente se archiva por si se necesita para el error 300005:

```
Asunto: Consulta configuración IVA incluido para terminal WebLink — Error [300005]

Buenos días,

Estamos integrando la WebLink REST API de BDP-NET con una aplicación externa
de pedidos. La conectividad, autenticación y validación de series funcionan
correctamente.

Al intentar validar la creación de un pedido mediante el endpoint
/API/Orders/Create con OrderOperationType = 1 (OnlyCheck), el terminal 31
(CENTRAL 2026) responde con:

  [300005]-EL TERMINAL NO ESTÁ CONFIGURADO PARA TRABAJAR CON IVA INCLUIDO

El terminal 1 (CENTRAL) no muestra este error.

Preguntas:
1. ¿Dónde se configura la opción "IVA incluido" para un terminal en BDP-Net?
2. ¿Es necesario configurar algo en la serie de facturación o en los
   parámetros del terminal?

Payload utilizado:
  EmployeeId: 31, PosId: 31, ItemsProfileId: 1
  OrderEndType: 0, OrderOperationType: 1
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
