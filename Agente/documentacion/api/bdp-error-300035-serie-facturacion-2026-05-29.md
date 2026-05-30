# BDP Error 300035 — Serie de Facturación no definida para WebLink

> **Fecha:** 2026-05-29
> **Tarea:** 065A-4
> **Estado:** Bloqueante — requiere configuracion en BDP-Net (PC del restaurante)

---

## 1. El error

```
[300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA
```

BDP-Net devuelve este error al llamar a `/API/Orders/Create` **incluso en modo `OnlyCheck`** (`OrderOperationType=1`), sin `Payments` y con `Invoice=false`. Esto significa que BDP valida la existencia de una serie de facturacion asignada al terminal **antes** de cualquier escritura real — es una validacion de configuracion interna del TPV, no un bug de nuestra app.

---

## 2. Donde NO esta la configuracion

La documentacion de BDP-Net que el usuario encontro describe **`Utilidades → Configuracion Servicios Web`**. Esa pantalla contiene pestañas para:

| Pestaña | Que configura |
|---------|---------------|
| Weblink | IP, puerto, credenciales, comandos permitidos |
| BackOfficeWeb | IP, puerto, credenciales Google |
| Sinqro | IP, puerto, credenciales |
| Wordpress | IP, puerto, login obligatorio, encriptacion |
| WeblinkRestApi | IP, puerto, login, comandos, encriptacion |
| Customer & Food | IP, puerto, login, encriptacion |

**Ninguna de estas pestañas tiene un campo "Serie de Facturacion" o "Serie destino para pedidos WebLink".** Esta pantalla solo configura la capa de red y autenticacion del servicio Web. La serie de facturacion es parte de la configuracion **interna del TPV**, no del servicio Web.

---

## 3. Donde SI esta la configuracion (hipotesis + plan de busqueda)

### 3.1 Diagnostico: `/API/POSSeries/GetList`

Nuestro backend ya descubrio que el endpoint `/API/POSSeries/GetList` de WebLink **si devuelve series** configuradas en BDP. Esto confirma que BDP tiene el concepto de "series" y las expone por API. El problema es que `CreateOrder` **no tiene un campo para especificar la serie** — BDP la resuelve internamente segun la configuracion del terminal.

**Para diagnosticar**, ejecutar este comando desde la app o desde Swagger:
```
GET /api/configuracion/bdp/diagnostico
```
Y tambien revisar el `payload_preview` del dry-run para ver que series estan disponibles.

### 3.2 Ubicaciones probables en BDP-Net

Basado en la arquitectura de BDP-Net (sistema TPV español), las series de facturacion suelen configurarse en estos lugares. **Hay que revisarlos en orden:**

#### A. `Facturación → Series de Facturación` (o `Maestros → Series`)
- Ruta probable: Menu principal → **Facturacion** → **Series** o **Series de Facturacion**
- Alternativa: **Maestros** → **Series de Facturacion**
- Aqui se definen las series disponibles: ej. `A-2026` (factura completa), `B-2026` (simplificada), `T-2026` (ticket)
- **Lo que hay que verificar:** Que exista al menos una serie activa para el año en curso

#### B. `Configuración TPV → Parámetros del Terminal → pestaña Facturación`
- Ruta probable: **Utilidades** → **Configuracion TPV** → seleccionar terminal → pestaña **Parametros 2** o **Facturacion**
- Buscar campos como:
  - "Serie de facturacion por defecto"
  - "Serie facturacion simplificada"
  - "Tipo de documento por defecto"
  - "Serie para pedidos externos"
  - "Serie WebLink"
- **Lo que hay que verificar:** Que el terminal usado por WebLink tenga una serie asignada

#### C. `Utilidades → Configuración General → Facturación`
- Ruta probable: **Utilidades** → **Configuracion General** → pestaña **Facturacion** o **Documentos**
- Buscar campos relacionados con series por defecto, tipos de factura, o configuracion de numeracion

#### D. Sub-rejilla expandida del servicio Weblink
- En `Configuracion Servicios Web` → pestaña **Weblink**, cada fila tiene un boton `+` que expande:
  - Nivel 1: credenciales (usuario/contraseña)
  - Nivel 2: comandos permitidos
- **Revisar si hay un tercer nivel o campos ocultos** relacionados con serie/destino. Es poco probable pero vale la pena verificarlo.

#### E. `Configuración Adicional Wordpress` (boton en Servicios Web)
- La doc menciona un boton "Configuracion Adicional Wordpress" que lleva a otra pantalla
- Aunque es para Wordpress, podria tener parametros compartidos con Weblink
- Revisar si existe un equivalente para Weblink (la doc no lo menciona, pero el boton podria estar en otra ubicacion)

### 3.3 El Manual-WebLink (documento oficial)

La documentacion de BDP referencia un manual externo:
> `C:\BDP-NET\NetXXX\DatosGen\Manual-WebLink`

Este manual esta en el PC del restaurante. **Abrirlo y buscar:**
- "serie"
- "facturacion"
- "CreateOrder"
- "OrderOperationType"
- "300035"

Posiblemente el manual describa que `CreateOrder` requiere que el terminal tenga configurada una serie de facturacion simplificada, y explique donde se asigna.

---

## 4. Evidencia a recolectar

Antes de cambiar cualquier configuracion en BDP-Net, **documentar el estado actual:**

1. **Captura de `Configuracion Servicios Web` → Weblink** (todas las filas, expandidas)
2. **Captura de `Facturacion → Series`** mostrando todas las series definidas
3. **Captura de `Configuracion TPV → Parametros del Terminal`** (pestañas relevantes)
4. **Resultado de `/API/POSSeries/GetList`** desde el dry-run o diagnostico
5. **Contenido del Manual-WebLink** (PDF o CHM en `C:\BDP-NET\NetXXX\DatosGen\`)

---

## 5. Plan de accion

### Fase 1: Investigacion (en el PC del restaurante, fuera de horario)
1. Conectarse por RDP a `100.83.196.35`
2. Abrir BDP-Net como administrador
3. Ir a `Utilidades → Configuracion Servicios Web` → pestaña Weblink
4. **Hacer captura de la configuracion actual** (todas las filas, expandidas con `+`)
5. Navegar a `Facturacion → Series` (o ruta equivalente) — **captura**
6. Navegar a `Configuracion TPV → Parametros del Terminal` — revisar TODAS las pestañas buscando "serie", "facturacion", "documento" — **captura**
7. Abrir `C:\BDP-NET\NetXXX\DatosGen\Manual-WebLink` y buscar "serie" y "CreateOrder" — **captura o notas**
8. Ejecutar `Probar sincronizacion segura` desde la app para ver el error actual

### Fase 2: Correccion (si se encuentra la configuracion)
1. Asignar una serie de facturacion simplificada valida al terminal/configuracion correspondiente
2. Guardar cambios
3. Volver a ejecutar `Probar sincronizacion segura`
4. Verificar que `CreateOrder OnlyCheck` pasa a `ok: true`
5. Confirmar `listo_para_sincronizar = true` con `escritura_real = false`

### Fase 3: Si no se encuentra la configuracion
1. Contactar al soporte de BDP-Net preguntando especificamente:
   > "Al usar WebLink `/API/Orders/Create` con `OrderOperationType=1` (OnlyCheck), recibimos error `[300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA`. ¿Donde se configura la serie de facturacion que debe usar WebLink para crear pedidos? ¿Es por terminal, por servicio WebLink, o por tipo de documento?"
2. Adjuntar el payload de ejemplo y la respuesta de error

---

## 6. Notas tecnicas para referencia

### Payload enviado (sin Payments, OnlyCheck)
```json
{
  "EmployeeId": "<configurado>",
  "ItemsProfileId": "<configurado>",
  "OrderEndType": 0,
  "OrderOperationType": 1,
  "Invoice": false,
  "Order": {
    "MarketplaceOrderId": "GDRY...",
    "MarketId": 9901,
    "PosId": "<configurado>",
    "Type": 2,
    "Items": [{ "Id": <artículo real>, "Units": 1.0, ... }],
    "AlreadyInvoiced": false,
    "Comments": "GLORY DRY RUN - NO CREAR"
  }
}
```

### Endpoint llamado
```
POST /API/Orders/Create
Authorization: Bearer <token>
```

### Por que BDP valida serie incluso en OnlyCheck
`OrderOperationType=1` le dice a BDP "valida este pedido pero no lo crees". Sin embargo, BDP internamente necesita resolver que serie de facturacion usaria **si** el pedido se creara realmente. Como el terminal no tiene una serie asignada (o la serie no es valida para el tipo de documento), BDP rechaza la validacion con 300035.

Esto **no es un bug** — es BDP siendo estricto con su configuracion fiscal interna. La solucion es puramente de configuracion en el PC del restaurante.

---

## 7. Referencias

- `Agente/documentacion/api/bdp-weblink-2026-05-06.md` — Documentacion de la integracion
- `src/services/bdp_sync_preflight.rs` — Logica del dry-run
- `src/services/bdp_weblink.rs` — Cliente WebLink (`check_order()` fuerza `OrderOperationType=1`)
- `src/services/bdp_weblink_catalog.rs` — Struct `BdpCreateOrderRequest`
- `# WEBLINK RESTAPI.md` (raiz) — Documentacion completa del API extraida del manual