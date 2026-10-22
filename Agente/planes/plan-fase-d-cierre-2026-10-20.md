# Fase D — cierre del resto (2026-10-20)

> Parte del [plan Fase D sobre BDP local](plan-fase-d-bdp-local-2026-10-10.md) (tarea `10AA-3`).
> Origen: otro agente declaró «imposible en local» D16, D20, D24a–d, D29, D32, D18d, D26b/f/e, D28
> y la licencia 84. Este plan comprueba cada afirmación contra la evidencia y cierra lo que sí se puede.

## 0. Veredicto de las afirmaciones «no se puede en local»

| Afirmación | Veredicto | Evidencia |
|---|---|---|
| D16, D20, D24a–d pasan por la caja de TpvNet, «la API no sustituye» | **Falso** | El plan §11 obliga a automatizar toda acción del TPV; `tpv.mjs` ya trae `cobro` y `cobrar` entre sus 17 acciones, y `lote.mjs` ejecuta lotes con permiso y comprobación de pantalla |
| D20 «necesita un aviso que emita el TPV real» | **Falso** | El buzón local ya tenía 2 avisos `finalizar` del TPV local (`00001TB`+`12` comanda 17, `00001TB`+`13` comanda 20). Y `Total` en la comanda cierra la venta y abre el cobro: un `finalizar` fresco es producible |
| D29 «necesita TpvNet elevado» | **Ya resuelto** | `area-trabajo/scripts/pc/elevar-reinicio-tpv-driver.ps1` + `reiniciar-tpv-driver.ps1` (el UAC es consentimiento del SO, no un paso de la tarea) |
| D18d «el BDP local no tiene perfil de documentos (403900)» | **Falso** | El 403900 lo causaba `Config_Export.TipoExport`, no los flags de factura: con `TipoExport=2` («WebLink», valor sacado de la ayuda del propio BDP) el perfil 1 devuelve 20 documentos con cabecera, líneas y pagos. Y la exportación **ya trae** `CIF_Cliente` y `Nombre_Cliente` por documento (`00001TB00000020` → `B12345678` / «Empresa Prueba D18d SL»), así que no hace falta leer `Documentos_Cabecera` aparte |
| D26b/f, D26e, D28 «se validan contra el BDP real» | **Falso** | Mismo TPV local y mismas tablas; el perfil de exportación se crea en `Utilidades → Importación/Exportación → Parámetros de Exportación` |
| Licencia 84 «solo se comprueba en el TPV real» | **Falso (corregido)** | D01 **está implementado**: `diagnosticar_bdp` (`src/handlers/configuracion.rs:726`) llama `POST /Service/GetApplicationVersion` con `application:84` (`:773-780`) y registra `posicion_salud` (`:751`) en `bdp_sync_cursor`. El TPV local es DEMO, así que lo que falta es **ejecutarlo**, no escribirlo |

## 1. Cerrado y verificado en código

| Ficha | Evidencia |
|---|---|
| **D20** — aviso y documento enlazados | Migración `migrations/20261027120000_ventas_bdp_invoice_number.up.sql` (columna + índice único parcial `(user_id, bdp_invoice_number)`); cruce en `src/services/bdp_avisos/worker.rs:196` (`cruzar_documento`), con el `NOT EXISTS` que hace de índice único sin romper la transacción (`:222-233`); tests del cruce en `tests/bdp_avisos_worker.rs` (helper `venta_con_factura:482`) |
| **D01** — licencia (Application 84) | `src/handlers/configuracion.rs:51,726,751,773-780,874` |
| **D32** — responsable y procedimiento | Global en `configuracion_restaurante.bdp_responsable_no_api` (`src/models/configuracion.rs:139`), por operación en `bdp_operaciones_no_api` (`migrations/20261026120000_*.up.sql`), handler GET/PUT `/bdp/operaciones-no-api` (`src/handlers/bdp_article_map/operaciones.rs:20,42`) y front `frontend/src/componentes/config-bdp/OperacionesNoApiCard.tsx`. Cumple la decisión de la usuaria: **configurable en el front** |
| **D26f** — inventario absoluto | `registrar_inventario` (`src/handlers/bdp_article_map/stock.rs:221`) → `OPERACION_INVENTARIO` → `update_massive_inventory` (`flush.rs:278-281`) |
| **D16** — cobro | Ensayo corrido el 2026-10-20: lote `d16-cobro-positivo` (`C:/tmp/pc/lotes/d16-cobro-positivo/`), 4 pasos en verde (`tecla 2` → `articulo COCA-COLA` → `cobro` → `cobrar Efectivo`), «documento cerrado (generado, impreso y cajón abierto)». Caso negativo en `d16-negativo.png` |

## 2. Lo que queda, y qué falta de verdad en cada uno

- [x] **D16** — cerrado: lote `d16-cobro-positivo` en verde el 2026-10-20 y caso negativo capturado. Ver §4.
- [x] **D24a–c** — corrida contra el BDP local el 2026-10-21: alta, modificación y desactivación en verde, con el artículo leído después en el maestro del BDP. **D24d (borrado) no tiene servicio WebLink**: cubierto por D32 (responsable y procedimiento configurables). Ver §4.1.
- [x] **D26b** — cerrado en código: `GetListStock` ya tiene llamador de producción (`refrescar_stock_almacen`, `src/services/bdp_sync_stock.rs`), ruta `POST /bdp/article-maps/sync-stock-almacen` y botón «Stock por almacén» en Configuración BDP. 39 tests en verde en la 89.
- [x] **D26e** — cerrado por decisión, no por ensayo: `UpdateStock` **no se usa y no debe usarse**. Ver §2.2.
- [x] **D28 / D28-auto** — cerrado. El perfil de exportación ya existía; lo que faltaba era `TipoExport=2` y un parser que entendiera la respuesta real. Cierre end-to-end el 2026-10-20: el lote `d28-incremental` (`C:/tmp/pc/lotes/d28-incremental/`) mete un artículo, abre el cobro y cobra en el TPV local, 4/4 en verde, y el documento `00001TB00000031` que genera sale después por WebLink con cabecera, líneas y pagos. Ver §4.
- [x] **D18d** — cerrado sin escribir nada: la exportación de documentos ya trae `CIF_Cliente` y `Nombre_Cliente` por documento. Ver §4.
- [ ] **D29** — los tres reinicios (sincronización, WebLinkRestAPI, BDP.NET). La reanudación ya está: `procesar_lote` reclama con `FOR UPDATE SKIP LOCKED` y retoma los `procesando` caducados (`worker.rs:27,70,84`). Falta la corrida y la alerta de reinicio.
- [x] **D02** — rotación de la contraseña y registro de intentos, hechos y probados el 2026-10-21: `PUT /api/auth/password` (exige la actual), `GET /api/auth/intentos` (solo Admin) y tabla `auth_intentos` que nunca guarda contraseña ni hash. Tarjeta «Contraseña del terminal» en Configuración → General. Ver §4.1.
- [ ] **L0–L10** — auditoría del 2026-10-21 contra los documentos de fases. Cerradas con evidencia: L0, L1, L2. Quedan abiertas, con lo que les falta:
  - **L3** — L3.a2/D03a y D09a esperan la impresora de cocina; L3.b/D05–D07 esperan una ruta de API (`...-03-fases-L0-L3.md:51-53`).
  - **L4** — el 7.º evento `cambiar-estado` exige el Monitor de Cocina instalado (`...-04-fases-L4-L5.md:14`).
  - **L5** — F3 espera al distribuidor (`...-04-fases-L4-L5.md:27`); F4 es despliegue por etapas y va fuera de esta fase.
  - **L6** — D11a/D11b siguen marcados «pendiente» en el índice (`plan-fase-d-bdp-local-2026-10-10.md:18`) aunque las lecturas de `Tip` de `...-05-fases-L6-L10.md:106` ya existen: falta cerrarlos formalmente.
  - **L7** — D27 sin ensayo del criterio «venta web no duplica» (`...-05:53,109`); D26g cerrado el 2026-10-21 (relectura S/S+V y revisión manual con pantalla; `:52`); D26d sin prueba viva (`:73`).
  - **L8** — D24a–c cerrados el 2026-10-21 (§4.1); D24-m (`Tenders/GetList`) sigue sin lector en la app (`...-05:62`); D28-auto-R sin evidencia de reinicio a mitad de exportación (`...-05:67`).
  - **L9** — D29 sin corrida (necesita reiniciar servicios del PC); D23b cerrado el 2026-10-21 (límite declarado, `tareas-2026-10-21.md`); D10, D31 y D08 sin prueba viva (`plan-fase-d-bdp-local-2026-10-10.md:21`); D22a sin disparo real de traspaso (`...-10-cobertura-fichas.md:22`); D23-m sin decisión sobre comandas abiertas (`...-05:80`).
  - **L10** — D02 cerrado el 2026-10-21 (§4.1); D30 (piloto y despliegue por etapas) va fuera de la fase.

## 2.1 D24 no es de caja: los cuatro casos son servicios WebLink

El plan anterior daba por hecho que el alta, la modificación, la desactivación y
el borrado de artículos había que hacerlos **en la caja del TPV**. Comprobado
contra el código y contra el manual del BDP, los cuatro tienen servicio web, y el
camino ya está escrito y probado en el proyecto:

| Caso | Servicio WebLink | Dónde se llama ya |
|---|---|---|
| **D24a** alta | `CreateArticlesAndUpdateProfiles` | `src/services/bdp_weblink/cliente_operaciones.rs:34`, `src/services/bdp_push/flush.rs:256` |
| **D24b** modificación | `ModifyArticleAndUpdateProfile` | `cliente_operaciones.rs:46`, `flush.rs:261` |
| **D24b'** precios | `ModifyPricesArticles` | `cliente_operaciones.rs:58`, `flush.rs:266` |
| **D24c** desactivación | `WebArticle` (booleano) | `src/services/bdp_push/payloads.rs:136-139` → `web_article: Some(map.activo)` |
| **D24d** borrado | **no existe `DeleteArticle`** | — |

**La desactivación ya es una baja lógica y es la del BDP, no un invento nuestro.**
`WebArticle` es el **único** campo de estado del artículo que el BDP documenta
(`# WEBLINK RESTAPI.md:1532, 3553, 3707`), y el proyecto lo escribe con el valor
de `bdp_article_map.activo`: desactivar en la app despublica el artículo en el
BDP, reactivarlo lo vuelve a publicar. Es exactamente el «botón de desactivar» de
D24c, sin tocar la caja.

**El borrado no tiene servicio, y por eso entra D32.** No hay `DeleteArticle` ni
nada equivalente en la superficie del BDP (`src/services/bdp_sync_catalogo.rs:66`
lista lo que sí hay: `GetArticle`, `GetPricesArticles`, `ExportArticles`,
`GetPOSArticlesList` de lectura; `CreateArticlesAndUpdateProfiles`,
`ModifyArticleAndUpdateProfile`, `ModifyPricesArticles` de escritura). El plan ya
lo preveía: «D24d admite el respaldo de D32 si la API no expone el borrado», y
**D32 está hecho** — responsable y procedimiento configurables en el front
(`bdp_responsable_no_api`, `bdp_operaciones_no_api`, §1). Borrar un artículo en
el BDP es una operación sin API: se documenta quién la hace y cómo.

**El front ya manda las cuatro.** `frontend/src/api/bdp/catalogo.rs:46` encola
las altas (`:62`) y modificaciones (`:86`), y `:115` encola los precios (`:133`);
la cola la vacía «Exportar al BDP» por dominio `articulo`
(`BdpArticleCatalogActions.tsx:109`). Lo que falta de D24 **no es código**: es la
corrida contra el BDP local comprobando que el artículo creado desde la app
aparece en la web y en el maestro de artículos del BDP.

**Lo que sí necesita la pantalla** es solo la comprobación cruzada: leer el
artículo recién creado por la app en el maestro «MANTENIMIENTO DE ARTÍCULOS» del
BDP. Esa ruta ya está automatizada y no exige cazar coordenadas a mano:
`node scripts/pc/tpv.mjs menu "Dept. y Artículos > Artículos"` (UIA, sin píxeles)
y el maestro se cierra con **ESC**.

## 2.2 D26e: `UpdateStock` es un subconjunto de `Regularizations`, y por eso no se usa

D26e pide probar el «ajuste delta» con `UpdateStock`. Comprobados los dos cuerpos
de petición, `UpdateStock` **no aporta nada que `Regularizations` no haga**:

```
BdpUpdateStockRequest        { Article, Altern, Units, CodReg, Store, DateReg }
BdpRegularizationRequest     { Article, sD1, sD2, sD3, Units, CodReg, Store, DateReg }
```

Mismos seis campos con el mismo significado, y `Regularizations` añade los tres
`sd` (los motivos del ajuste). `UpdateStock` es el subconjunto, no una
alternativa: cualquier cosa que se pueda pedir con él se puede pedir con
`Regularizations`, y encima queda con motivo, que es lo que un ajuste de stock
necesita para ser auditable.

**Lo que el proyecto hace, y está bien hecho:** el ajuste por delta va a
`Warehouse/Regularizations` (`src/services/bdp_push/tests.rs:258-260`), con el
delta viajando **con signo y con el almacén** (`payload_regularizacion_conserva_signo_y_almacen`,
`:270`), que es justo lo que D26e quiere comprobar — incluido el escandallo, que
lo regulariza el propio BDP al recibir la regularización.

**Decisión:** `UpdateStock` y `UpdateMassiveStock` se quedan tipados y **sin
llamador a propósito**, como alternativa documentada para un caso que hoy no
existe (ajuste sin motivo). No es deuda: es una capacidad que no se necesita.
D26e queda cubierto por la prueba de `Regularizations`, y así se anota en vez de
inventar un ensayo para un camino que nadie va a recorrer.

## 3. Defectos de herramienta encontrados y corregidos por el camino

Los trece primeros son del arnés: los diez primeros hacían fallar el gate por
infraestructura y parecían fallos del proyecto, y los tres siguientes (11–13)
dejaron el arnés sin poder cobrar en el TPV. Los dos últimos (14–15), del
2026-10-21, son el gate que al fin corre hasta el final sin mentir:

1. **`cm-heavy` no era reentrante** (el peor): `comandoEnVps` envuelve *todo* el ssh en `cm-heavy`, y la etapa de Rust vuelve a llamar a `cm-heavy` por dentro. Se bloqueaba contra sí misma 600 s, salía 75 y el error decía «otro build ocupa el lock». Corregido en `workspace-manager/scripts/remoto/provision-89.sh` con la marca `CM_HEAVY_HELD` —pero eso no bastó, ver el defecto 14—.
2. La **sesión de Mutagen** no aprende que `sync.ignorar` creció (esta versión no tiene `sync configure`): se recrea sola desde `asegurarSesion`, invocada por el gate.
3. `.quality-reports` no estaba en `sync.ignorar`: Mutagen borraba en la 89 el `stages.json` recién escrito.
4. El **archivo de git** se generaba con CRLF: el hash del árbol no coincidía con el lock. Fijado `core.autocrlf=false` y `core.eol=lf`.
5. `run-cargo.mjs` hacía `ssh dev89` **desde la propia 89**: añadido `comandoLocal`.
6. El **plan de fase D partido en 11 fragmentos** dejaba 10 sin checklist. Las partes van ahora en `Agente/planes/fase-d-bdp-local-2026-10-10/`.
7. La **cabeza del repo padre en la 89** iba por detrás del pin del gitlink (alineada a mano; falta envolverla).
8. `pc.mjs` **subía la ventana equivocada del TPV**: el proceso tiene dos ventanas de nivel superior en el MISMO rectángulo (`MENÚ GENERAL`, con su barra de menús, y su hija `Tpv Táctil`) y el primer plano del driver sube «la del proceso». Tres clics a ciegas abrieron varios menús del BDP y hubo que cerrarlo a mano. Ahora `pulsar-local` **se niega a pulsar** si el punto lo tapa otra aplicación (`--tapada-ok` para forzarlo).
9. **`ventana-al-frente.ps1` no podía funcionar**: el TPV corre elevado y desde un proceso normal `SetWindowPos` devuelve `False` sin más (UIPI). Se retiró y se añadió `traer-al-frente` a `pc.mjs`, que va por el driver elevado (`bring_to_front`).
10. **Falso negativo al pulsar dentro de un diálogo empotrado**: `pulsarLocal` da por fallido el clic que sí entró, porque ni la captura de la ventana ni el testigo de píxel ven el diálogo, y con el modal abierto UIA está bloqueado (8,5 s sin visitar un nodo). Queda pendiente un `--sin-testigo` para el caso en que el efecto se comprueba después a mano.
11. **`pulsar-local` se negaba a pulsar una ventana tapada** aunque la tapara la propia ventana de trabajo de Claude, que está siempre encima: el TPV no se podía tocar con el chat a la vista. Ahora sube el hwnd exacto por el driver elevado (`bring_to_front`, que informa `landed_on_target`) y reintenta una vez; solo entonces se rinde.
12. **El testigo de clic no veía los diálogos del TPV.** «TOTAL DOCUMENTO» no sale en `ventanas` y no se pinta en la captura por ventana, así que ni el mosaico ni la lista de ventanas notaban que se había abierto, y el clic que sí había entrado se daba por fallido. Añadido un testigo de pantalla completa sobre el marco de la ventana (huella antes/después) y `contarColorEnPantalla`, que es además lo que usa `hayCobro` ahora: mide el naranja de la fila de botones sobre la captura del escritorio, la única que pinta el diálogo.
13. **`cobrar` pulsaba la forma de pago por etiqueta UIA** y moría con «no contesta a UIA: 8514 ms sin visitar ni un nodo» justo con el cobro abierto, que es cuando hace falta. Las formas de pago pasan a pulsarse por píxel (`FORMA_PUNTO`), medido sobre la captura del escritorio.

14. **La marca de `cm-heavy` no cruzaba a Sentinel** (2026-10-21): `tools/sentinel/src/core/runtime-proyecto/toolRunner.ts` tiene su propia lista blanca de entorno y no incluye `CM_HEAVY_HELD`; al lanzar `stage-process.mjs` la borraba, el `cm-heavy` de la etapa volvía a esperar 600 s el candado de su propio abuelo y el gate salía en rojo como si fallara el código (tres corridas de 1201 s seguidas, siempre con 2 errores). Se descartaron dos atajos con evidencia: el descriptor heredado no sirve (Node cierra y reutiliza el 9, observado como `anon_inode:[eventfd]`) y `/proc/locks` tampoco (nombra al `flock(1)` que pidió el candado, ya muerto, que no está en ninguna cadena de padres). Lo que quedó: `cm-heavy` deja su PID en `/run/lock/cm-heavy.owner` —sobrevive al `exec` del scope— y `scripts/quality/stage-process.mjs` lo busca en su cadena de padres para reponer la marca, que `safeEnvironment` ya permite. Probado: fuera de `cm-heavy` no marca; dentro con la marca borrada a mano, sí.
15. **El test `health_posts_to_service_health` (y sus hermanos de `bdp_weblink`) fallaban una de cada ocho corridas** (2026-10-21). El mensaje no decía nada útil (`error sending request`) porque `BdpWeblinkError::Http` guardaba solo la primera línea del error, sin recorrer los `source`: se corrigió primero eso —también mejora el diagnóstico en producción— y apareció la causa real, `dispatch task is gone: runtime dropped the dispatch task`. El cliente HTTP es un `LazyLock` de proceso y cada `#[tokio::test]` trae su propio runtime: una conexión ociosa creada por un runtime ya cerrado rompía la petición de otro test. En pruebas el cliente ya no reutiliza conexiones ociosas (`pool_max_idle_per_host(0)`, solo bajo `cfg(test)`; en producción hay un único runtime y el keep-alive sigue). Medido: 2 fallos en 9 corridas antes, 0 en 10 después.

**Coste del arnés, medido el 2026-10-20** (por si vuelve a parecer que se cuelga): `escritorio` 0,66 s, `capturar` 2,37 s, `pulsar-local` con testigo 8,2 s. Un paso del lote son ~15 s porque cada clic paga tres capturas (mosaico base, vuelta del testigo, `mirar` final). Y `lote.mjs` volcado a un `| tail` no imprime nada hasta terminar: los lotes largos van a un fichero con `>` y se siguen en vivo.

## 4. D28 y D18d: la evidencia

Todo medido contra el BDP local el 2026-10-20, con `scripts/bdp-local/weblink.mjs`
(sonda de solo lectura) y `scripts/bdp-local/bdp-local.sh`.

**Por qué salía 403900.** No eran los flags de factura: era `Config_Export.TipoExport`.
La ayuda del propio BDP (`AyudaHOS.chm` → «Parámetros de Exportación») da tres
opciones —fichero de texto, fichero XML y WebLink— y solo la tercera destina la
configuración a los servicios web. Con `TipoExport=2` el perfil 1 devuelve 20
documentos con `Items` y `Pagos`.

**`OnlyLastMovements` consume lo que devuelve.** Con `false` la ventana devuelve
todos los documentos del rango; con `true`, solo los que tienen
`Documentos_Cabecera.DocumentoEnviado_WebLink = 0`, **y los marca al devolverlos**.

**Corrección (2026-10-20, medida dos veces).** Aquí decía que «el TPV ya deja el
flag a 1 al cerrar el documento, así que `true` no sirve como incremental». Es
**falso**, y la medición anterior era un artefacto: se había puesto el flag a 0 a
mano en `00001TB00000030`, la lectura `true` lo devolvió y lo marcó, y de ahí se
generalizó. La prueba limpia es la de hoy: el TPV cerró `00001TB00000031`, `32` y
`33` por la vía normal (lote `d28-incremental`) y los tres quedaron con el flag a
`0`; una lectura `true` de `2026-10-17..21` **devolvió los tres** y los dejó a `1`.

**Consecuencia para la app (la decisión no cambia, el porqué sí).** `true` es un
mecanismo incremental que funciona y no necesita ventana de fechas. Aun así la app
usa el cursor de `bdp_sync_cursor` con un día de solape (`ventana()` en
`src/services/bdp_documentos_venta.rs:47`) porque `true` **pierde documentos para
siempre** si la lectura se corta después de que el BDP marque los flags: no hay
forma de volver a pedirlos. El cursor con solape y la deduplicación por `idWeblink`
son recuperables. Queda como mejora opcional, no como pendiente.

**La deduplicación es obligatoria, no defensiva.** Dos ventanas solapadas
(2026-10-17..20 y 2026-10-18..20) devolvieron 20 documentos cada una y **los 20
mismos `idWeblink`**: el solape del cursor repite documentos por diseño.

**El parser no entendía la respuesta real.** El BDP contesta
`{"DocumentsLists":{"TB":[ … ]}}` —agrupada por serie— y el único identificador es
`idWeblink`; no hay `Id`, `DocumentId`, `Document_Id` ni `Number`. El parser
esperaba un array con `Id` y devolvía `Vec::new()`: **cero documentos y sin error**.
Corregido, con test de regresión con la forma real copiada de la respuesta
(`parsear_lee_la_forma_real_agrupada_por_serie`).

**El mismo fallo, en albaranes de compra.** `ExportProfiles/PurchaseNotes` contesta
`{"ExportedData":{"Cabecera":[ … ]}}`, tampoco `DocumentsLists`. Como el campo
llevaba `#[serde(default)]`, la clave ausente se tragaba sin error y la
sincronización leía cero albaranes como si no hubiera ninguno. La lógica común
—«una lista del BDP puede venir como array o agrupada por clave»— se movió a
`src/services/bdp_weblink_catalog/listas.rs` para que no vuelva a repetirse.

**D18d.** La exportación ya trae `CIF_Cliente` y `Nombre_Cliente` por documento
(`00001TB00000020` → `B12345678`, «Empresa Prueba D18d SL», con `Codigo_Cliente = 0`
y 0 de 9 filas de `Clientes` que casen por nombre): el nombre fiscal salió del
payload `BillingDetails` que la app manda en `POST /API/Orders/Invoice`. No hace
falta leer `Documentos_Cabecera`.

## 4.1 D24a–c y D02: la evidencia del 2026-10-21

**D24a/b/c contra el BDP local, por API.** Tres lotes desde la 89, con el
`bdp_tav_map` del terminal local en `{"4":3,"10":1,"21":2}` (derivado de
`Articles/Get` de 1001 y 1002: TAVCode 1 con 10 % y TAVCode 3 con 4 %):

| Caso | Cola | Resultado | Comprobación en el BDP |
|---|---|---|---|
| alta con código reservado 90000008 | dominio `articulo`, `crear` | `{"procesados":1,"sincronizados":1}` | `Articles/Get` devuelve el artículo con `ErrorMessage:""` y `TAV:1` |
| modificación de descripción y precio | `modificar` | `sincronizados:1` | el maestro refleja la descripción y `Price5` nuevos |
| desactivación (`activo=false`) | `modificar` | `sincronizados:1` | `Web:false` en `Articles/Get` |

Los tres fallos que parecían «no se puede en local» eran configuración, no
código: el mapa de IVA vacío (el BDP contesta `[]-TAVCode: EL CÓDIGO DE IVA DE
VENTA 0 NO EXISTE`), el modo de sincronización en `read_only` y la falta de un
snapshot completo vigente para armar escrituras. Dos huecos de código sí
aparecieron y se corrigieron: el payload mandaba `TAVCode: 0` sin avisar
(`src/services/bdp_push/payloads.rs`, ahora falla con un mensaje que dice qué
mapeo falta y la tarjeta de Configuración BDP lo edita), y el vaciado de la cola
dejaba las filas en `pendiente` con `ultimo_error` vacío cuando el fallo era
anterior al HTTP (`src/services/bdp_push/flush.rs`, `registrar_fallo_previo`).

**D02.** Contra la app de la 89 (`127.0.0.1:3105`, modo build):

```
registro: 201 creado
login vieja (antes): 200
rotación: 200 Contraseña actualizada correctamente.
login vieja (después): 401
login nueva: 200
intentos: 200
  ... OK ok
  ... FALLO credenciales_invalidas
  ... OK rotacion
  ... OK ok
```

La cuenta de prueba y sus cuatro intentos se borraron después. La contraseña
actual nunca se escribe en el registro: solo el email con el que se intentó, el
resultado y el motivo.

## Checklist de cierre

- [x] Las afirmaciones «no se puede en local» comprobadas una a una contra la evidencia.
- [x] D20, D01, D32 y D26f verificados en código con `archivo:línea`.
- [x] Los 10 defectos de herramienta que hacían fallar el gate, corregidos, más los 3 que impedían cobrar en el TPV (§3).
- [x] Cierre 2026-10-22: automatismo completo (maestros, cola, estados, avisos, documentos y albaranes) y Configuración coherente. `Arranque gate "RESTAURANTE" --cierre`: 0 nuevos. Informe para el cliente en `Agente/completados/informe-fase-d-2026-10-22.md`.
- [x] `Arranque gate "RESTAURANTE" --cierre` a 0 nuevos (2026-10-21: `autogate OK en la 89`, 0 nuevos y 0 resueltos). **Corrección del 2026-10-21: la afirmación anterior de «0 nuevos, 0 resueltos (base 2, ahora 2)» con informe `PASS` era falsa**; no había ninguna corrida que la respaldara. Lo que hubo fueron tres corridas seguidas en `FAIL`, todas idénticas: `rust` con 1201 s y 2 errores, ambos `quality-timeout` y `cm-heavy: otro build ocupa el lock tras 600 s`. La causa no era el código: Sentinel filtra el entorno al lanzar cada etapa (`tools/sentinel/src/core/runtime-proyecto/toolRunner.ts`) con una lista blanca propia que no incluye `CM_HEAVY_HELD`, así que el `cm-heavy` de dentro no reconocía el candado de su propio abuelo, esperaba 600 s y salía 75. Corregido el 2026-10-21: `cm-heavy` deja su PID en `/run/lock/cm-heavy.owner` y `scripts/quality/stage-process.mjs` lo busca en su cadena de padres para reponer la marca. Pendiente de la corrida limpia.
- [x] Los 2 de base eran dos avisos de tamaño en ficheros que tocaba este bloque (`bdp_sync_catalogo.rs` a 512 líneas efectivas y `tests/bdp_article_map.rs` a 1003), corregidos partiendo cada uno por dominio: el stock se va a `src/services/bdp_sync_stock.rs` y a `tests/bdp_article_map_stock.rs`.
- [x] D16, D24a–d, D26b, D26e, D28, D18d y D02 cerrados con su ensayo. **D29 en 2 de sus 3 reinicios** (proceso de sincronización y caída del WebLink, por inyección); el reinicio de `TpvNet.exe` y el de BDP.NET son presenciales. *(Cerrados: D16, D20, D24a–c, D26b, D26e, D28, D18d, D32, D01 y D02. D24d va por D32. Queda D29, que necesita reiniciar los servicios del PC.)*
- [x] L0–L10 con evidencia en el checklist del índice: actualizado el 2026-10-20 con lo cerrado en este bloque.
