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
- [ ] **D24a–d** — alta, modificación, desactivación y borrado **en caja del TPV local**, leyendo después desde la web sin botón. Necesita acciones nuevas de maestros en `tpv.mjs` (hoy hay `familia` y `articulo` de comanda, no de caja). D24d admite el respaldo de D32 si la API no expone el borrado, y D32 ya está hecho.
- [ ] **D26b / D26e** — `GetListStock` y `UpdateStock` están tipados (`consultas.rs:87-123`, `cliente_operaciones.rs:171`) pero **sin llamador de producción**: es trabajo de código, no de ensayo.
- [x] **D28 / D28-auto** — cerrado. El perfil de exportación ya existía; lo que faltaba era `TipoExport=2` y un parser que entendiera la respuesta real. Ver §4.
- [x] **D18d** — cerrado sin escribir nada: la exportación de documentos ya trae `CIF_Cliente` y `Nombre_Cliente` por documento. Ver §4.
- [ ] **D29** — los tres reinicios (sincronización, WebLinkRestAPI, BDP.NET). La reanudación ya está: `procesar_lote` reclama con `FOR UPDATE SKIP LOCKED` y retoma los `procesando` caducados (`worker.rs:27,70,84`). Falta la corrida y la alerta de reinicio.
- [ ] **D02** — rotación de la contraseña del terminal local: no hay código de rotación ni registro de intentos; el único secreto gestionado es `BDP_AVISOS_SECRETO`.
- [ ] **L0–L10** — llevar la evidencia de cada fase al checklist del índice.

## 3. Defectos de herramienta encontrados y corregidos por el camino

Los siete hacían fallar el gate por infraestructura y parecían fallos del proyecto:

1. **`cm-heavy` no era reentrante** (el peor): `comandoEnVps` envuelve *todo* el ssh en `cm-heavy`, y la etapa de Rust vuelve a llamar a `cm-heavy` por dentro. Se bloqueaba contra sí misma 600 s, salía 75 y el error decía «otro build ocupa el lock». Corregido en `workspace-manager/scripts/remoto/provision-89.sh` con la marca `CM_HEAVY_HELD`.
2. La **sesión de Mutagen** no aprende que `sync.ignorar` creció (esta versión no tiene `sync configure`): se recrea sola desde `asegurarSesion`, invocada por el gate.
3. `.quality-reports` no estaba en `sync.ignorar`: Mutagen borraba en la 89 el `stages.json` recién escrito.
4. El **archivo de git** se generaba con CRLF: el hash del árbol no coincidía con el lock. Fijado `core.autocrlf=false` y `core.eol=lf`.
5. `run-cargo.mjs` hacía `ssh dev89` **desde la propia 89**: añadido `comandoLocal`.
6. El **plan de fase D partido en 11 fragmentos** dejaba 10 sin checklist. Las partes van ahora en `Agente/planes/fase-d-bdp-local-2026-10-10/`.
7. La **cabeza del repo padre en la 89** iba por detrás del pin del gitlink (alineada a mano; falta envolverla).
8. `pc.mjs` **subía la ventana equivocada del TPV**: el proceso tiene dos ventanas de nivel superior en el MISMO rectángulo (`MENÚ GENERAL`, con su barra de menús, y su hija `Tpv Táctil`) y el primer plano del driver sube «la del proceso». Tres clics a ciegas abrieron varios menús del BDP y hubo que cerrarlo a mano. Ahora `pulsar-local` **se niega a pulsar** si el punto lo tapa otra aplicación (`--tapada-ok` para forzarlo).
9. **`ventana-al-frente.ps1` no podía funcionar**: el TPV corre elevado y desde un proceso normal `SetWindowPos` devuelve `False` sin más (UIPI). Se retiró y se añadió `traer-al-frente` a `pc.mjs`, que va por el driver elevado (`bring_to_front`).
10. **Falso negativo al pulsar dentro de un diálogo empotrado**: `pulsarLocal` da por fallido el clic que sí entró, porque ni la captura de la ventana ni el testigo de píxel ven el diálogo, y con el modal abierto UIA está bloqueado (8,5 s sin visitar un nodo). Queda pendiente un `--sin-testigo` para el caso en que el efecto se comprueba después a mano.

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
Prueba: se puso el flag a 0 en `00001TB00000030` → la primera lectura `true`
devolvió ese documento y ninguna otra → la segunda lectura `true` ya no lo vio, y
el flag estaba otra vez a 1.

**Consecuencia para la app:** el TPV ya deja el flag a 1 al cerrar el documento, así
que `true` no sirve como mecanismo incremental aquí. La incrementalidad la da el
cursor de `bdp_sync_cursor` con un día de solape, que es lo que ya hace
`ventana()` en `src/services/bdp_documentos_venta.rs:47`.

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

## Checklist de cierre

- [x] Las afirmaciones «no se puede en local» comprobadas una a una contra la evidencia.
- [x] D20, D01, D32 y D26f verificados en código con `archivo:línea`.
- [x] Los 7 defectos de herramienta que hacían fallar el gate, corregidos.
- [ ] `Arranque gate "RESTAURANTE" --cierre` a 0 nuevos.
- [ ] D16, D24a–d, D26b, D26e, D28, D18d, D29 y D02 cerrados con su ensayo.
- [ ] L0–L10 con evidencia en el checklist del índice.
