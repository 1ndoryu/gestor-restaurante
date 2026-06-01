# BDP-Net Error [300035] — Resumen completo

> **Fecha:** 2026-06-01
> **Tarea:** 065A-4
> **Estado:** Bloqueante — requiere respuesta de soporte BDP-Net o hallazgo adicional en el PC del restaurante
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

---

## 4. Lo que se descubrió (investigación)

### 4.1 Documentación BDP-Net revisada

| Documento | Ubicación | Hallazgo |
|-----------|-----------|----------|
| Ayuda BDP-Net (CHM) | AyudaHos.chm | Describe Configuración Servicios Web — **sin mención de series** |
| Manual-WebLink | `C:\BDP-NET\NetXXX\DatosGen\Manual-WebLink` | No encontrado ni revisado (pendiente) |
| Doc Series TPV | Ayuda BDP-Net | Confirma: "como mínimo una serie por terminal", tipos: Facturas Simplificadas, Rectificativas, Albaranes, Traspaso Hotel, Importe Cero, Sustitutivas |
| Doc Series (Compras) | Ayuda BDP-Net | Para documentos de compra — **no aplica a WebLink** |

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
| Falta serie en Parámetros del Terminal | ✅ Están asignadas en Facturas 1 (Barra/Mesas) y Facturas 2 |
| Error en credenciales/URL | ✅ Health + Login + GetVersion funcionan |
| Error en permisos de empleado | ✅ Empleado 31 aparece en POS/Employees/Get |
| Bug de nuestra app | ✅ El error viene de BDP, no de nuestro backend |

### 4.4 ¿Qué podría ser el problema?

| Hipótesis activa | Probabilidad | Cómo verificar |
|------------------|-------------|----------------|
| WebLink necesita una serie distinta a las del TPV manual | Alta | Soporte BDP |
| Existe un campo oculto en Config. Servicios Web → Weblink (expandido con `+`) | Media | Revisar más de cerca la sub-rejilla de Weblink |
| BDP requiere que la serie sea de tipo específico (ej. "TB" en vez de "TM") | Media | Probar cambiar la serie en Facturas 1 |
| El `Order.Type: 2` del payload mapea a un tipo de documento sin serie asignada | Media | Probar con `Type: 0` o `Type: 1` |
| WebLink REST API requiere un parámetro extra no documentado | Baja | Soporte BDP |

---

## 5. Mensaje para soporte BDP-Net (redactado)

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

Hemos verificado la configuración del terminal (nº 31) en los siguientes apartados:

| Ubicación                                       | Estado                |
|------------------------------------------------|-----------------------|
| Config TPV → Param. Terminal → Facturas 1      | Serie asignada (00031TM) |
| Config TPV → Param. Terminal → Facturas 2      | Serie asignada (00031AL) |
| Series TPV                                     | Series existen y activas|
| Utilidades → Config. Servicios Web → Weblink   | IP, puerto, credenciales OK |

Sin embargo, el error [300035] persiste incluso en modo solo validación
(sin escritura real).

Pregunta: ¿Dónde se configura la serie de facturación que debe utilizar el
servicio WebLink para los pedidos creados a través de /API/Orders/Create?
¿Se hereda de la serie asignada en Parámetros del Terminal, necesita una serie
específica para pedidos externos, o se configura en otro apartado?

Payload utilizado:
  EmployeeId: 31
  ItemsProfileId: 1
  PosId: 31
  OrderEndType: 0
  OrderOperationType: 1
  Invoice: false
  Order.Type: 2
  Order.AlreadyInvoiced: false

Agradeceríamos cualquier indicación para completar la integración.

Quedamos atentos.
Saludos
```

---

## 6. Próximos pasos

### Corto plazo (requiere RDP al PC del restaurante)

1. **Enviar mensaje de soporte** a BDP-Net con el texto anterior
2. **Revisar el Manual-WebLink** en `C:\BDP-NET\NetXXX\DatosGen\Manual-WebLink` — buscar "serie", "CreateOrder", "300035", "OrderOperationType"
3. **Probar cambiar el tipo de documento:** enviar `Order.Type` con valores 0, 1, 3 en vez de 2 para ver si BDP responde distinto
4. **Revisar la sub-rejilla expandida de Weblink** más de cerca — puede haber un campo de serie en un tercer nivel de expansión que no se vio

### Medio plazo (con respuesta de soporte)

5. **Implementar la corrección** según la respuesta de BDP
6. **Re-ejecutar `Probar sincronización segura`** hasta `listo_para_sincronizar = true`
7. **Activar escrituras reales** (`OrderOperationType` a modo normal)
8. **Documentar la resolución** y actualizar este archivo

### Pendiente de código (si se descubre campo de serie)

Si BDP indica que hay que enviar un campo de serie en el payload, o que se necesita un parámetro adicional:

- Actualizar `BdpCreateOrderRequest` en `src/services/bdp_weblink_catalog.rs`
- Actualizar `build_only_check_order()` en `src/services/bdp_sync_preflight.rs`
- Agregar campo `bdp_invoice_series` (o similar) en `ConfiguracionRestaurante`
- Actualizar la UI en `frontend/src/componentes/ConfigBdp.tsx`

---

## 7. Archivos relacionados

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

## 8. Restricciones

- **No crear/modificar** ventas, comandas, clientes, artículos ni pagos reales en el restaurante
- **No tocar** la configuración de series existentes sin evidencia de que es seguro
- **Criterio de cierre:** `listo_para_sincronizar = true` manteniendo `escritura_real = false`
