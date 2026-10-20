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
| D18d «el BDP local no tiene perfil de documentos (403900)» | **Cierto a medias** | `Config_Export` tiene 1 fila (`Codigo=1`, «test») con los flags a 0: esa es la causa del 403900. Pero `Documentos_Cabecera` (`CIF_Cliente`, `Nombre_Cliente`, `Facturado`) **no lo lee nadie** en la app, así que D18d no se puede cerrar sin escribir esa lectura |
| D26b/f, D26e, D28 «se validan contra el BDP real» | **Falso** | Mismo TPV local y mismas tablas; el perfil de exportación se crea en `Utilidades → Importación/Exportación → Parámetros de Exportación` |
| Licencia 84 «solo se comprueba en el TPV real» | **Falso (corregido)** | D01 **está implementado**: `diagnosticar_bdp` (`src/handlers/configuracion.rs:726`) llama `POST /Service/GetApplicationVersion` con `application:84` (`:773-780`) y registra `posicion_salud` (`:751`) en `bdp_sync_cursor`. El TPV local es DEMO, así que lo que falta es **ejecutarlo**, no escribirlo |

## 1. Cerrado y verificado en código

| Ficha | Evidencia |
|---|---|
| **D20** — aviso y documento enlazados | Migración `migrations/20261027120000_ventas_bdp_invoice_number.up.sql` (columna + índice único parcial `(user_id, bdp_invoice_number)`); cruce en `src/services/bdp_avisos/worker.rs:196` (`cruzar_documento`), con el `NOT EXISTS` que hace de índice único sin romper la transacción (`:222-233`); tests del cruce en `tests/bdp_avisos_worker.rs` (helper `venta_con_factura:482`) |
| **D01** — licencia (Application 84) | `src/handlers/configuracion.rs:51,726,751,773-780,874` |
| **D32** — responsable y procedimiento | Global en `configuracion_restaurante.bdp_responsable_no_api` (`src/models/configuracion.rs:139`), por operación en `bdp_operaciones_no_api` (`migrations/20261026120000_*.up.sql`), handler GET/PUT `/bdp/operaciones-no-api` (`src/handlers/bdp_article_map/operaciones.rs:20,42`) y front `frontend/src/componentes/config-bdp/OperacionesNoApiCard.tsx`. Cumple la decisión de la usuaria: **configurable en el front** |
| **D26f** — inventario absoluto | `registrar_inventario` (`src/handlers/bdp_article_map/stock.rs:221`) → `OPERACION_INVENTARIO` → `update_massive_inventory` (`flush.rs:278-281`) |
| **D16** — cobro | Acciones `cobro` y `cobrar` ya en `tpv.mjs`; falta el ensayo con prueba positiva y negativa |

## 2. Lo que queda, y qué falta de verdad en cada uno

- [ ] **D16** — ensayo del cobro con `lote.mjs`: comanda → `Total` → cobro → forma de pago → confirmar, y el caso negativo (cobro sin líneas). Las acciones existen; falta la corrida y su evidencia.
- [ ] **D24a–d** — alta, modificación, desactivación y borrado **en caja del TPV local**, leyendo después desde la web sin botón. Necesita acciones nuevas de maestros en `tpv.mjs` (hoy hay `familia` y `articulo` de comanda, no de caja). D24d admite el respaldo de D32 si la API no expone el borrado, y D32 ya está hecho.
- [ ] **D26b / D26e** — `GetListStock` y `UpdateStock` están tipados (`consultas.rs:87-123`, `cliente_operaciones.rs:171`) pero **sin llamador de producción**: es trabajo de código, no de ensayo.
- [ ] **D28 / D28-auto** — la lectura ya existe (`POST /API/ExportProfiles/Documents` desde `src/services/bdp_documentos_venta.rs`, a tabla `bdp_documentos_venta`, bucle en `src/main.rs:133-159`). Falta **crear el perfil de exportación** en el BDP local con los flags de factura a 1 y probar `OnlyLastMovements` y ventanas solapadas con cursor.
- [ ] **D18d** — leer `Documentos_Cabecera` (`CIF_Cliente`, `Nombre_Cliente`, `Facturado`): no aparece en `src/` ni en `migrations/` (`git grep -i`), hay que escribir la lectura.
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

## Checklist de cierre

- [x] Las afirmaciones «no se puede en local» comprobadas una a una contra la evidencia.
- [x] D20, D01, D32 y D26f verificados en código con `archivo:línea`.
- [x] Los 7 defectos de herramienta que hacían fallar el gate, corregidos.
- [ ] `Arranque gate "RESTAURANTE" --cierre` a 0 nuevos.
- [ ] D16, D24a–d, D26b, D26e, D28, D18d, D29 y D02 cerrados con su ensayo.
- [ ] L0–L10 con evidencia en el checklist del índice.
