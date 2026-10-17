# Plan — Fase D sobre BDP local (2026-10-10)

**ID:** 10AA-3 · **Estado:** en curso. **L0, L1, L2 completas** (L2.2 cerrada el 2026-10-17: los conteos de la API coinciden exactamente con el perfil del POS, 259 artículos y 24 departamentos). **L3.a y L3.c hechas (2026-10-17)**: `create_order` de `PRUEBA-1` aceptado por el BDP local (pedido 15, comanda en las tablas vivas) y `cancel_order` propagado (la comanda desapareció del BDP y del TPV); para la anulación hubo que corregir dos defectos del guard (§9.15 y §9.16), ya en `main`. **L4.1 hecha**: receptor mínimo verificado en `127.0.0.1:8099`. **L5.0 hecha (2026-10-17)**: base T2 creada (5 tablas, §8), y de paso los 15 alcances de escritura ya se pueden armar (§9.18). L3.b queda bloqueada por código (§9.9) y L3.a2 por falta de impresora de cocina. **F1 hecha (2026-10-17)**: receptor `/api/bdp/avisos/{accion}` + inbox, con el contrato real del distribuidor (§9.19); 17 pruebas verdes en la 89. · **Próximo paso:** L4.2, la usuaria configura la URL de avisos en TpvNet y provoca los 7 eventos; después L4.3–L4.4 y F2.
**Rev. 2026-10-17 (solo documento, sin tocar el TPV):** corregidas las referencias al anexo B8 y al alcance (los PDF de `~/Downloads` ya no existen: la fuente viva es el HTML del repo), el recuento de fichas (63 → 65), el estado de alta en el taller y el `git status` de §3, L0.4 cerrado por la recomendación de §9.2 y la atribución de commits de §7 al día.
**Rev. 2026-10-17 (tarde, tras cerrar L3.c):** L3.c pasa a HECHA con los dos defectos del guard documentados en §8 y §9.15–§9.16 (`92f2afb`, `46e49eb`, más `c92444a` en `scripts/run-cargo.mjs`, que resolvía una ruta de config muerta que impedía compilar en la 89). Añadido el defecto del filtro `?dominios=` a los gaps de código.

**Relacionado:** `plan-fase-d-avisos-bdp-2026-09-19.md` (diseño F0–F4), `plan-alcance-local-bdp-2026-10-01.md` (P0–P4 con TPV local), `Agente/usuario/alcance-fase-d-2026-10-02.md` (alcance rev. 10/10).

## Checklist de ejecución

Estado según §8 (2026-10-17): L3.a y L3.c hechas; L3.b bloqueada por código; L3.a2 sin impresora de cocina.

- [x] L0 — Preparación (L0.1–L0.4)
- [x] L1 — Conmutar la app a BDP local
- [x] L2 — Lectura contra BDP local (L2.1, L2.2 y L2.3 hechos)
- [ ] L3 — Escritura controlada `PRUEBA-*` (L3.a y L3.c hechas 2026-10-17; L3.b bloqueada por código, §9.9)
- [ ] L4 — F0 empírica de avisos en TPV local (L4.1 hecha; L4.2–L4.4 pendientes de la usuaria en el TPV)
- [ ] L5 — Fase D F1–F4 (código) y base T2 (L5.0 hecha 2026-10-17: las 5 tablas que faltaban, §8; **F1 hecha 2026-10-17**; F2–F4 pendientes)
- [ ] L6 — Pagos, propina, puntos y factura (D11a–D20)
- [ ] L7 — Stock (D26–D27)
- [ ] L8 — Maestros y documentos (D24–D25, D28)
- [ ] L9 — Avisos duplicados, caídas y reinicios (D04, D08, D10, D21–D23, D29, D31)
- [ ] L10 — Excepciones, accesos y piloto (D01, D02, D30, D32)

## 1. Objetivo

Dejar la aplicación apuntando al BDP local (TPV 36.2 licenciado en el portátil) en lugar del BDP real, y con ese TPV: (1) cerrar la F0 empírica de la fase D (Avisos de Acciones), que bloquea F2 y F4; (2) ejecutar en local las pruebas de aceptación D01–D32 que promete el alcance (`Agente/documentacion/bdp/alcance-fase-d-pdf-2026-10-10.html` y `Agente/usuario/alcance-fase-d-2026-10-02.md`; fichas en `Agente/planes/anexo-b8-aceptacion-2026-10-08.md`, rev. 10/10), cada una en su fase (§10). Lo que el PDF promete y este plan no puede probar en local queda marcado como diferido, con su motivo.

## 2. Alcance y no alcance

**Dentro**
- Conmutar la configuración BDP de real a local (en base de datos, no en `.env`).
- Revisar la cola de envíos pendientes antes de conmutar (L0.3).
- Pruebas de lectura y de escritura reversible contra el TPV local (comandas `PRUEBA-*`).
- F0 empírica de los 7 avisos en el TPV local.
- Tras la F0: F1–F4 de 199A-1, según el diseño del plan 2026-09-19, precedidas de la base T2 (L5.0, hecha).
- Pruebas de aceptación D01–D32 en local (L6–L10), con la evidencia que exige cada ficha del anexo B8.

**Fuera**
- BDP real (`IP_BDP:8068`, PC remoto por Tailscale) y cualquier host de producción: ni lectura ni escritura.
- `bdp_sync_mode = automatic`. Este plan opera en `read_only`, salvo escrituras por lote autorizado (L3).
- Pagos y facturas (`Payment/Add`, `InvoiceOrder`) hasta confirmar suscripción: no se prueban en L3; pasan a L6 con esa precondición.
- Producción y piloto (D30, F4): fuera, permiso aparte. Antes de F4, el despliegue por etapas del PDF (observación → maestros → lectura → escritura → pagos/factura → stock) y el rollback por escrito.
- Envío a Guillermo y cambios en PDF o alcance: permiso expreso. Commit e integración: solo en el cierre (§7).
- Preguntas al distribuidor: decisión 2026-09-20 (`plan-fase-d-avisos-bdp-2026-09-19.md:21`). Lo no documentado se resuelve por prueba en el TPV.

## 3. Hechos verificados y pendientes

**Verificado en código, configuración o documentación**

- La configuración BDP vive en la tabla `configuracion_restaurante` (`migrations/20260506000000_bdp_weblink_config.up.sql:7-14`).
- Las variables de `.env` solo arrancan la configuración. Tras `bdp_env_bootstrap_applied_at` se ignoran (`src/services/bdp_config_bootstrap.rs:165-173`; la marca se fija en l.210). Por eso el cambio de host se hace por API o por la pantalla de Configuración, no editando `.env`.
- `PATCH /api/configuracion` con `bdp_base_url`, login, password, código integrador o pos desarma la sincronización y fuerza `bdp_sync_mode = read_only` en el mismo UPDATE (`src/handlers/configuracion.rs:197-215`). Rutas: `GET/PATCH /api/configuracion` (l.111, l.130).
- Kill switch de escritura (`src/services/bdp_weblink.rs:795-831`): loopback (`localhost`, `127.0.0.1`, `::1`) siempre se permite. Cualquier otro host exige coincidencia exacta con `BDP_WRITE_ALLOWED_ORIGINS`, sin credenciales, query ni path distinto de `/`. Solo cubre escrituras; las lecturas no tienen este freno.
- Modo efectivo: `modo_efectivo_desde_config` (`src/services/modo_operacion.rs`). En `auto` depende de `bdp_sync_enabled`.
- Cola de envíos: existen las tablas `bdp_push_pendientes`, `bdp_write_arming` y `bdp_poll_schedule` (migraciones). Si hay filas pendientes, al cambiar la URL podrían salir hacia el TPV local. Por eso L0.3 va antes de L1.
- Puerto del BDP real: `IP_BDP:8068` (`Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md:6`). Puerto por defecto en código: `localhost:8068` (`src/services/bdp_sync.rs:115`). Eso no prueba la instancia local.
- `RESTAURANTE` ya figura en `workspace-manager/scripts/dev/registro.json` (L0.0, hecho el 2026-10-10). Sin entrada no hay mando `Arranque` (AGENTS §9.2). **Actualizado 2026-10-17:** el proyecto está dado de alta y bajo mando (`Arranque status "RESTAURANTE"` → `bajo-mando`, `dev89`, tmux 1/1, sonda verde).
- Frontend: `frontend/vite.config.ts:19` declara `5173`. El `:5182` de las verificaciones del 09-16 es un arranque manual, no configuración.
- Scripts existentes: `scripts/run-cargo.mjs`, `scripts/run-with-db.mjs`, `scripts/branch-db.mjs`. Gate canónico: `npm run gate:check` (`package.json:37`, `scripts/quality/gate-check.mjs`).
- **Base T2 (resuelto el 2026-10-17, L5.0):** el 2026-10-10 se verificó que no existía (búsqueda en `migrations/` y `src/`: ningún resultado para `bdp_sync_cursor`, `bdp_id_map`, `bdp_stock_ledger`, `bdp_avisos_inbox` ni outbox; testigo en el mismo barrido: `bdp_push_pendientes` y `bdp_write_arming` sí aparecen, 66 coincidencias en 17 archivos). Ya está creada: `migrations/20261017120000_bdp_base_t2.{up,down}.sql` con **5** tablas, no 6 — el *outbox* que el PDF nombra ya es `bdp_push_pendientes` y no se duplica (§8).
- Caja cerrada (2026-10-10, L3.a): `POST /api/ventas` crea la venta; su envío a BDP devuelve `301400` (caja no abierta, POS 1). No hay endpoint WebLink de apertura de caja (el manual local solo documenta comandos `ws*` sin caja); se abre en TpvNet («Apertura de Caja»).
- Gaps de código vistos en L2/L3: modificar una venta no tiene ruta BDP; `/bdp/article-stock` lee caché (no `GetStock` en vivo); comentarios desfasados («C3=b: nunca se llama CancelOrder») en `src/services/venta.rs:291` y `src/handlers/ventas.rs:225`, cuando `anular` ya encola `venta/cancelar` (`src/services/venta.rs:349-367`); `clippy -D warnings` falla por avisos preexistentes en `bdp_push.rs`, `bdp_weblink.rs` y `modo_operacion.rs`.
- Contradicción: `roadmap.md` 01AA-1 dice "P0 siguiente", pero `plan-alcance-local-bdp-2026-10-01.md:144` dice "P0 COMPLETO".
- Contradicción: `roadmap.md` 199A-1 menciona "12 preguntas a BDP", frente a la decisión de 2026-09-20 "sin preguntas a BDP".
- Git (2026-10-10): `main` sin ramas locales; `git status` mostraba 32 entradas ajenas. **Actualizado 2026-10-17:** árbol limpio salvo `quality-tools.json` modificado (subida de pin de Sentinel a 0.7.26 y de VarSense, sin commitear; no se toca aquí).

**No verificado (condiciona o bloquea)**

- Host y puerto del WebLink del TPV local. No consta en el repositorio. L0.1 lo averigua sin tocar la app.
- Licencia activa en vivo (solo consta en documentación; el TPV local figura como DEMO y D01 espera la licencia 84, que no es replicable en local).
- Serie `PRUEBA`: creada según bitácora, no confirmada por el distribuidor.
- Suscripción para `Payment/Add` e `InvoiceOrder`.
- Si el aviso de traspaso trae origen y destino.
- `CancelOrder` con efecto en cocina: `roadmap.md` l.118 dice "verificado 2026-09-18"; el alcance dice "pendiente de prueba local".
- `GET /api/configuracion/bdp/diagnostico` (l.534): no leído. Solo se usa si se confirma que es de solo lectura.

## 4. Reglas de ejecución

- Ninguna escritura al TPV local sin permiso expreso por lote.
- Cada escritura deja fila en `bdp_audit_log` y comprobación visual en el TPV.
- Secretos (login, password, código integrador, IP real): nunca en repo, plan, logs ni chat. El volcado de L1.1 contiene credenciales: va a `C:\tmp\fase-d-local\` (fuera del repo) y se borra al cerrar.
- Compilar solo con los wrappers de AGENTS §0.2 (`CARGO_TARGET_DIR` en `C:\tmp`). Medir `C:\tmp` antes de compilar; si roza 20 GB, limpiar.
- Stage explícito por archivo (AGENTS §5). En `roadmap.md`, por hunk: tiene ediciones ajenas. Nunca `git add .`. No se tocan ni se stashean los cambios ajenos.
- Subagentes: máximo 3 en paralelo; respuestas de 15 líneas como máximo, con `archivo:línea`. El principal comprueba el dato crítico antes de decidir con él y revisa el diff del `implementador` antes de cerrar.
- Diseño y decisiones técnicas reversibles: las decide el agente (AGENTS §0.3). La usuaria decide solo lo listado en §9.

## 5. Fases

### L0 — Preparación (solo lectura y registro; sin escrituras al TPV)

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L0.0 | Registrar RESTAURANTE en `workspace-manager/scripts/dev/registro.json` (cambio en otro repo; se integra en su repo al cierre). Anotar problemas previos: `GET http://localhost:8787/api/gate/analisis?clave=RESTAURANTE` | Principal | Entrada en registro; problemas previos anotados (AGENTS §6.1) |
| L0.1 | Host y puerto del WebLink local, sin tocar la app: proceso que escucha en el portátil (`netstat -ano`) y `/Auth/Login` de solo lectura contra ese candidato | Principal | URL base anotada sin IP real. Si hay varios candidatos, decisión 4. **Estado 2026-10-10 (10:18):** TPV reabierta por la usuaria como administrador (TpvNet PID 22312, SvrWCF PID 21252; token elevado comprobado). `Config_WebLink` 0 filas (sqlcmd); sin listener de WebLink (SvrWCF escucha en 9011 y 9021; no confirmado como WebLink). Computer-use no controla ventanas elevadas (UIPI); pc-control (cua-driver) tampoco: su SKILL.md declara UAC/High-IL fuera de alcance. La activación de Servicios Web la hace la usuaria en la TPV. Puerto candidato libre según netstat: 8070. Licencia (DEMO) por verificar. Bloqueado hasta que la usuaria active WebLink y diga el puerto |
| L0.2 | Login contra esa URL, sin escrituras | Principal | Token o error documentado |
| L0.3 | Estado actual en BD (SQL de solo lectura): `bdp_base_url`, `bdp_sync_enabled`, `bdp_sync_mode`, filas de `bdp_push_pendientes` y `bdp_write_arming` | Principal | Cola vacía, o pendientes resueltos con la usuaria antes de L1. **Resultado 2026-10-10:** `bdp_base_url` sigue en el BDP real (`IP_BDP:8068`); `bdp_sync_enabled=false`, `bdp_sync_mode=read_only`, `modo_operacion=standalone`, `push_modalidad=automatico`; `bdp_write_arming` y `bdp_poll_schedule` vacías. Cola: 0 `pendiente`, 9 `sincronizado`, 4 `error` (37b18564, 5d92004c, 67b8d9ae, dc84a08a: «ALMACÉN 1 NO EXISTE» ×3 y «CÓDIGO DE IVA 0 NO EXISTE» ×1). `error` es estado activo: el flush las reintenta. Decisión pendiente (ver L2.3) |
| L0.4 | Decidir `CancelOrder` | Usuaria (decisión 2) | **Cerrado 2026-10-17** con la recomendación de §9.2: se mantiene «pendiente de prueba local» y el roadmap (`roadmap.md:118`) ya está alineado. La prueba real exige la caja abierta (L3.c) |
| L0.5 | Corregir contradicciones de documentación: roadmap 01AA-1 ("P0 completo; siguiente P1", según `plan-alcance-local-bdp-2026-10-01.md:144`) y 199A-1 (quitar "12 preguntas a BDP") | Principal (decisión reversible del agente) | Roadmap coherente con el plan de alcance y con la decisión 2026-09-20 |
| L0.6 | Rama: no se crea todavía. Se crea desde `main` con el primer commit autorizado | Principal | Ninguna rama suelta en ningún momento (CLAUDE.md "Ramas") |

L0 cierra cuando L0.1–L0.4 tienen respuesta y L0.3 no deja pendientes sin resolver.

### L1 — Conmutar la app a BDP local

Precondición: L0.1–L0.3 OK y permiso expreso (decisión 1).

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L1.1 | Volcar la fila de `configuracion_restaurante` a `C:\tmp\fase-d-local\config-antes.sql` (incluye credenciales, por eso fuera del repo) | Principal | Archivo fuera del repo |
| L1.2 | Poner `bdp_sync_enabled = false`. Luego, en Configuración → BDP, la URL local (L0.1), login, password, código integrador y pos. La usuaria escribe los valores en la pantalla, no en el chat. Antes de guardar, comprobar que la URL no es `IP_BDP` | Usuaria escribe; principal verifica | `GET /api/configuracion` devuelve la URL local, `bdp_sync_enabled = false` y `bdp_sync_mode = read_only` |
| L1.3 | Solo si el host no es loopback: añadir la URL exacta a `BDP_WRITE_ALLOWED_ORIGINS` en `.env` y reiniciar con `Arranque up "<id>"` (requiere L0.0) | Principal, con permiso | Escritura permitida solo a esa URL |
| L1.4 | Buscar `IP_BDP` en el log del backend (`Arranque logs "<id>"`) tras L1.2 y tras L2 | Verificador (devuelve solo incidencias) | Cero apariciones |

**Rollback:** restaurar los campos de conexión desde `config-antes.sql`. `bdp_sync_enabled` y `bdp_sync_mode` quedan en `false` y `read_only`; no se reactiva ni la sincronización ni el armado de escrituras.

### L2 — Lectura contra BDP local (sin escribir)

Precondición: L1 OK.

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L2.1 | Script de lectura (solo `Get*` y `Export*`): login, `GetStock`, `ExportArticles`, `ExportDepartments`, pedidos abiertos | Implementador escribe en `C:\tmp`; principal lo ejecuta | Respuestas, tiempos y campos críticos registrados |
| L2.2 | Comparar conteos básicos con la pantalla del TPV | Usuaria + principal | Coincidencia o incidencia anotada |
| L2.3 | Activar `bdp_sync_enabled` solo para lectura, con `read_only`. Precondición: decidir las 4 filas `error` de la cola (L0.3). Al pasar a modo BDP, el flush automático (`push_modalidad=automatico`) las reintentaría contra el TPV local | Principal | Stock, Explorador e Historial leen del TPV local |

**Gate L2:** lectura OK o incidencias documentadas. Sin escrituras en este paso.

### L3 — Escritura controlada (`PRUEBA-*`, reversible)

Precondición: L2 OK y permiso por lote. Un lote cada vez:

- **L3.a HECHA (2026-10-17).** Crear comanda `PRUEBA-1` y verificar en TPV (ficha D03). Precondición: caja del POS 1 abierta en TpvNet; sin ella el envío falla con `301400` (sin comanda en BDP, venta local intacta). Resultado y evidencia en §8. Queda una consecuencia abierta: `bdp_default_article_name` no coincide con el catálogo real del código 1001.
- **L3.a2** Crear con envío a cocina (D03a, `OrderEndType=0`). La app usa hoy `OrderEndType=1`; el PDF espera `0` (crea, aparca y envía a cocina). Requiere impresora de cocina en el TPV local.
- **L3.b** Modificar líneas de `PRUEBA-1` (D05–D07). **Bloqueada por código:** modificar una venta no tiene ruta BDP; hay que implementarla antes (tarea aparte, §9 punto 9).
- **L3.c HECHA (2026-10-17).** Cancelar `PRUEBA-1` (`CancelOrder`, según decisión 2; D09 y D09a). Empezó bloqueada por dos defectos del guard (§9.15 y §9.16), corregidos en el mismo día; después el push salió y la comanda desapareció del BDP y de las tablas vivas del TPV. Evidencia en §8. D09a (efecto en cocina) sigue sin testigo de impresora.
- **L3.d** Pagos y facturas: pasan a L6 (bloqueados hasta confirmar suscripción).

L3 es ensayo exploratorio previo a la base T2: ninguna ficha D se da por superada con su resultado (falta la evidencia en `bdp_id_map` y demás tablas de T2). Se repite en L6–L9 con T2.

Por lote, el verificador compara la fila de `bdp_audit_log`, la respuesta de BDP y el estado en el TPV, y devuelve solo discrepancias. El modo efectivo se comprueba antes de cada lote y se vuelve a `read_only` al terminar.

**Gate L3:** lote sin discrepancias → siguiente lote. Discrepancia → parar y analizar. No reintentar a ciegas.

### L4 — F0 empírica de avisos en TPV local

Precondición: L3.a–L3.c OK. La usuaria configura en el TPV local la URL de destino de los avisos (paso manual en el TPV) y el receptor es alcanzable desde el TPV.

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L4.1 | Receptor mínimo que registra avisos en log (sin inbox): escucha solo en `127.0.0.1`, log en `C:\tmp`, parada con PID verificado | Implementador (script según especificación) | Registra un aviso de prueba |
| L4.2 | Provocar los 7 eventos (crear, modificar, cambiar estado, borrar, finalizar, traspasar, traspasar línea) con `PRUEBA-*` | Usuaria en TPV; principal anota | 7 eventos con payload |
| L4.3 | Matriz: payload real, valores de `Estado` 0–3, orden, duplicados, timestamps, origen y destino | Investigador extrae del log | `Agente/documentacion/bdp/avisos-local-matriz-<fecha>.md` con evidencia por fila |
| L4.4 | Valor seguro por defecto para lo no documentado (p. ej. `Estado` desconocido: registrar y no actuar) | Principal | Decisión anotada en la matriz |

**Gate L4:** 7 eventos con evidencia. Lo que no se pueda probar queda como limitación.

### L5 — Fase D F1–F4 (código)

Solo después de L4. El diseño es el del plan 2026-09-19; aquí no se re-diseña.

- **L5.0 Base T2 (roadmap 09AA-2): HECHA el 2026-10-17** (detalle y evidencia en §8). El PDF pide 6 tablas antes de las pruebas de aceptación (inbox, outbox, mapeo de IDs, cursor, pendientes agotados, libro de stock); se crearon **5**, porque *outbox* ya es `bdp_push_pendientes` y no se duplica. Incluye el bloqueo por artículo y almacén y el libro S/V que D26g declara como trabajo a implementar. L5.0 iba antes de F1: F1 ya no tiene ese bloqueo.
- **F1 Receptor e inbox: HECHA el 2026-10-17.** `GET|POST /api/bdp/avisos/{accion}` (las siete acciones, con alias), inbox `bdp_avisos_inbox` con clave única `{acción}|{comanda}|{timestamp canónico}`; secreto compartido `BDP_AVISOS_SECRETO` (tiempo constante, 503 si no está), límite de 16 KiB y 120/min, payload censurado, buzón con sesión de Admin (`GET /api/bdp/avisos`, `POST /api/bdp/avisos/{id}/reintentar`). Contrato y configuración del TPV: `Agente/documentacion/bdp/avisos-receptor-199A-1-2026-10-17.md`. Evidencia: `tests/bdp_avisos.rs` (11), `tests/bdp_avisos_limite.rs` (1), 7 unitarios en el servicio, clippy limpio en lo nuevo (§9.19), suite completa **475 pasan / 0 fallan**, gate de cierre **0 nuevos** (base 226) y **sonda HTTP real** contra el servidor de la 89 con el secreto del `.env` (404 sin acción, 401 sin secreto, 202 el aviso bueno, 200 el repetido, 422 sin comanda; el `Secreto` quedó censurado en el buzón). Cerrada con `Arranque cierre "RESTAURANTE" --tarea 199A-1` en verde; commits `14dd2c1` (receptor), `3c53d42` (pruebas) y `78b52f3` (docs), en `main`.
- **F2** worker de los 7 eventos (`GetOrder` para completar datos). Principal decide; implementador para handlers repetitivos.
- **F3** reconciliación, perfiles y XML para distribuidor. Pendiente de respuesta del distribuidor.
- **F4** TPV real y piloto de 7 días. Fuera de este plan; requiere permiso aparte.

Tests de L5 según AGENTS §8.1: por archivo tocado en cada bloque; suite completa una vez, al cierre.

### L6 — Pagos, propina, puntos y factura (D11a–D20)

Precondición: L3 OK, L5.0 hecho (ID de pago estable e idempotencia en T2), caja abierta, serie `PRUEBA`, suscripción de `Payment/Add` e `InvoiceOrder` confirmada en el TPV local (licencia DEMO por verificar) y permiso por lote. Pérdida de respuesta y timeout (D15, D15t, D19, D19t, D12c) se simulan con un proxy local en `127.0.0.1` (en `C:\tmp`, se para al terminar) que corta la respuesta tras reenviar: no se toca el TPV.

| Lote | Fichas | Qué se comprueba |
|---|---|---|
| L6.a Propina | D11a, D11b, D11c | `AddTip=true` suma; `AddTip=false` sustituye; a cero sin residuo |
| L6.b Puntos | D12a, D12b, D12c | Saldo ±N con ID único; sin reintento ciego de `AddPoints`. Pendiente de soporte en el terminal: si no lo soporta, se declara |
| L6.c Pagos | D13, D13a, D14, D14s, D15, D15t, D16, D17 | Un solo cobro por ID; consulta antes de repetir; tope de tres pagos; pago de caja visible tras el repaso |
| L6.d Facturas | D18, D18a, D18b, D18c, D18d, D19, D19t, D20 | Número único; simplificada al cobrar; `InvoiceOrder` solo sobre no facturada; guarda `Status=3`; `BillingDetails`; enlace aviso-documento |

### L7 — Stock (D26–D27)

- **Lectura (ya posible):** D26a (`GetStock`, incluido almacén inexistente) y D26b (`GetListStock`: artículo inexistente solo falla su línea; almacén inexistente). `GetStock` está probado en L2.1 (stock 0).
- **Escritura (D26, D26c–D26g, D27):** precondición L5.0, ya cumplida (libro de movimientos y bloqueo por artículo y almacén), permiso por lote y los datos del distribuidor que el PDF deja pendientes (cuál de los 6 almacenes vistos en local es el real; motivos válidos). Las ventas de caja también mueven stock: D27 se prueba con una venta web convergida, y un valor ajeno no se trata como error sin comprobar si viene de una venta.
- D26b, D26f y D26g (`Units` null, almacén inexistente, fiabilidad de `GetStock` tras escribir) se ensayan en local pero no se dan por aprobadas hasta confirmarse en el BDP real.

### L8 — Maestros y documentos (D24–D25, D28)

- **Plano, formas de pago y empleados (D24-p, D24-m):** lectura con `Rooms/GetTables`, `Tenders/GetList`, `Employees/Get`; ya posible con la app; criterio: sin altas fantasma.
- **Alta, modificación, desactivación y borrado en caja (D24a–D24d):** la usuaria opera en TpvNet; la app lee sin botón (necesita lectura periódica con cursor de T2). Si la API no expone el borrado, se verifica el mensaje de D32.
- **Clientes (D25):** (a) cambio solo en BDP, (b) solo en la web (`CreateCustomer` con `Overwrite=true`: requiere aprobación expresa de la usuaria; sin ella el caso se declara no soportado), (c) cambios contradictorios con conflicto visible.
- **Documentos (D28, D28-auto, D28-auto-R, D28a):** perfil de exportación programado en el TPV local; invocar `ExportProfiles/Documents` desde código, `OnlyLastMovements` true/false, ventanas solapadas, reinicio a mitad y albaranes por `ExportPurchaseNotes`. Precondición: L5.0, ya cumplida (cursor).

### L9 — Avisos duplicados, caídas y reinicios (D04, D08, D10, D21–D23, D29, D31)

Precondición: L4 hecho y F1–F2 implementados (L5.0 ya está) (receptor, inbox y worker).

- **D21, D21a:** se ensayan ya en L4 (matriz de 7 eventos y traspaso de comanda completa con `GetOrder`); aquí se repiten con el receptor definitivo de F2 y su log por tipo.
- **D10:** anular en caja una venta vinculada; el aviso `Cancelar` deja la venta anulada en la web (estado en ambos lados).

- **D22, D22a:** cada uno de los 7 avisos normal, duplicado y fuera de orden, con los payloads reales de L4 (repetición desde el log). Traspaso de línea (D22a) con origen y destino consultados con `GetOrder`.
- **D23a, D23b:** trabajador caído con receptor operativo; receptor inaccesible. D23b solo recupera lo localizable y declara el límite.
- **D23-m:** comanda manual con aviso perdido. No hay lista de abiertas confirmada: se resuelve por listado si existe o por límite formal declarado. La pregunta a BDP la decide la usuaria (§9 punto 11).
- **D04, D08:** respuesta perdida al crear (consulta antes de repetir, comanda única en `bdp_id_map`); toque simultáneo caja y web (conflicto visible, sin pisar).
- **D29:** tres reinicios en pruebas separadas: proceso de sincronización, WebLinkRestAPI o su servicio, BDP.NET. El de BDP.NET local lo hace la usuaria (TpvNet elevado). En cada uno: alerta, cola, cursor, reanudación y conciliación.
- **D31:** llamar al camarero desde una mesa vinculada; avisa en caja sin dejar estado.

### L10 — Excepciones, accesos y piloto (D01, D02, D30, D32)

- **D01, D02:** salud, versión y login ya verificados en L2.1; el registro formal (`bdp_sync_cursor`) tiene ya su tabla (L5.0), falta escribirlo. La licencia del TPV local es DEMO: la comprobación de la licencia 84 se hace en el TPV real. D02: revisión de logs sin secretos legibles en L1.4 y al cierre.
- **D32:** mensaje con procedimiento y responsable ante cada operación que la API no expone (traspaso desde la web, borrar artículos, rectificativas, devoluciones, apertura y cierre, arqueo, efectivo, PDF, reimpresión en cocina, controles de cocina, series, cambios tras el pago). Es código de la web (F2/F3) más la lista de responsables, que asignan Guillermo y el propietario por escrito.
- **D30 piloto y despliegue por etapas:** fuera de este plan (F4, permiso aparte). Requisitos antes de abrir F4: todas las fichas superadas con su evidencia, responsables de las excepciones, umbral de rendimiento y rollback por escrito.

## 6. Delegación

| Tarea | Agente | Paralelo |
|---|---|---|
| L0.1 comprobación de proceso y puerto (comandos) | principal | 1 |
| L0.3 consulta de estado y cola | principal | 1 |
| L1.4 revisión de logs | verificador | 1 |
| L2.1 y L4.1 scripts | implementador | 1 |
| L2.1 ejecución, L3 comprobación por lote | principal + verificador | 1 |
| L4.3 matriz desde log | investigador | 1 |
| L5 tests y builds | verificador | 1 |
| L5.0 migraciones de T2 (según especificación del principal) | implementador | 1 | **hecho 2026-10-17** |
| L6–L9 scripts de ficha (un patrón por familia) | implementador | 1 |
| L6–L10 ejecución y lectura de resultados contra el TPV | principal + verificador | 1 |
| L9 matriz de avisos duplicados y fuera de orden desde log | investigador | 1 |

La configuración (L1.2), las decisiones y la lectura de resultados de escritura las hace el principal o la usuaria.

## 7. Verificación y Definition of Done

- **L1:** `GET /api/configuracion` muestra URL local, `bdp_sync_enabled = false` y `read_only`. Cero apariciones de `IP_BDP` en logs.
- **L2:** lectura con evidencia.
- **L3:** lotes con `bdp_audit_log` y estado en TPV verificados.
- **L4:** matriz de 7 eventos.
- **L5.0 (HECHA 2026-10-17):** las 5 tablas de T2 que faltaban creadas por migración, con tests de idempotencia (inbox, mapeo, cursor, agotados, libro de stock). *Outbox* no se crea: ya es `bdp_push_pendientes`.
- **L6–L10:** ninguna ficha D se da por superada sin la evidencia que su propia ficha exige (anexo B8): estado en TPV vía `GetOrder`/`GetStock`, filas en `bdp_audit_log`, `bdp_id_map` y la cola. Fichas que dependan del BDP real (licencia 84, almacén real, D26b/f/g) quedan «ensayadas en local, pendientes de real», nunca «superadas». El estado de cada una vive en §10.
- **Calidad (AGENTS §6):** `npm run gate:check` en RESTAURANTE y re-análisis `POST http://localhost:8787/api/gate/analizar` con `{"clave":"RESTAURANTE","forzar":true}`. Sin hallazgos nuevos atribuibles al cambio.
- **Documentación:** roadmap (10AA-3 cerrada; 01AA-1 y 199A-1 alineadas), `Agente/completados/tareas-<fecha>.md`, matriz de avisos. El plan se mueve a `Agente/planes/completados/` con sus enlaces actualizados (AGENTS §7.2).
- **Limpieza:** receptor parado (PID verificado), `C:\tmp\fase-d-local\` borrado, procesos propios parados.
- **Commit e integración (CLAUDE.md "Ramas"):** commits por bloque si la política del repositorio lo exige (AGENTS §5), stage explícito, cada commit con la línea `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Rama `fase-d-bdp-local` integrada en `main` con fast-forward o merge, sin force-push. El push a `main` es publicación a remoto compartido: se pide confirmación antes (AGENTS §0.3). Antes de borrar ramas local y remota: `git bundle create` y `git bundle verify`. Si la integración necesita una decisión de la usuaria (conflicto, rama no fast-forward), anotar en el roadmap con su motivo y preguntar.

## 8. Estado y próximo paso

- **Estado:** L1 hecho (2026-10-10): volcado en `C:\tmp\fase-d-local\config-antes.sql`; `bdp_base_url=http://127.0.0.1:8070`, login `wan`, código integrador de `.env`; `bdp_sync_enabled=false`, `bdp_sync_mode=read_only`. Login del WebLink verificado; sin `IP_BDP` en el log del backend; `bdp_write_arming` vacía. Arranque RESTAURANTE verde (3105, 5182) tras registrar el proyecto en el mando.
- **Cambios en BD (2026-10-10, reversibles; backups `*-pre-weblink-2026-10-10.bak` verificados):** `bdp_pos_id` 31→1 (decisión 5a, pendiente de confirmar por la usuaria); `push_modalidad` automatico→manual; `bdp_sync_enabled` false→true; `modo_operacion` standalone→auto. Sin cambio: `bdp_sync_mode=read_only`, `bdp_employee_id=1`, `bdp_items_profile_id=1`. Cola `bdp_push_pendientes`: 9 sincronizadas y 4 en `error`, sin tocar.
- **L2.3 hecho (2026-10-10, solo GET, `C:\tmp\fase-d-local\l23-lectura-app.mjs`):** login 200; `/bdp/explorar` 200: artículos 2, clientes 5, **departamentos 24** (tras corregir el explorador: usaba `ExportDepartments` con rango 1..999, que devuelve lista vacía; ahora usa `ExportFromProfile`, igual que el perfil), salones 2, empleados 3; `/bdp/departamentos/perfil` 200: 24. `/bdp/article-stock` 200: 557 filas, todas stock 0, `ultima_sync_at` máx. 2026-09-15 → **es caché local antigua, no lectura viva del TPV** (gap, ver punto 7). `/bdp/audit` 200: 57 filas; 0 de hoy contra la IP real.
- **Cambio de código (sin push):** `src/services/bdp_explorer.rs` (departamentos vía perfil); `scripts/run-cargo.mjs` (cap 4096→20480 MB, techo de AGENTS §0.2); `.gitignore` (`.env.prod`). `cargo check --all-targets` OK; `clippy -D warnings` falla por avisos preexistentes en `bdp_push.rs`, `bdp_weblink.rs`, `modo_operacion.rs` (sin relación con el cambio).
- **L2.1 hecho (2026-10-10, solo lectura):** health, versión y login 200; tenders 4; departamentos 24 (perfil 1); artículos 2 en Export y 20 en GetPOSList; stock del artículo 1001 en almacén 1 = 0. `Tenders/GetPOSList` con POS 31 → `403500 NO EXISTE NINGÚN TERMINAL`; con POS 1 → 3 medios de pago. Salidas en `C:\tmp\fase-d-local\l2-salida.txt` y `l2-salida-pos1.txt` (borrar al cerrar). Pedidos abiertos no consultados: sin lista confirmada en la doc.
- **Incidencia de L1/L2:** la primera ejecución de L2.1 tomó `BDP_BASE_URL` de `.env` (la IP real `IP_BDP:8068`). Fallo de conexión por timeout; no hubo petición HTTP. Se repitió con la config de BD (`127.0.0.1:8070`).
- **L3 bloqueada (2026-10-10):** la venta `PRUEBA-1` (`40025555-56a0-486c-b55d-6ce571eabd5e`) se creó en la app y el armado `unidirectional` (snapshot completo `44c8ca62-d0b4-4f5b-ad90-cb2875b41122`) se consumió; el BDP rechazó `create_order` con `301400` (caja no abierta en el POS 1, `OrderEndType=1`). No existe endpoint WebLink para abrir caja: se abre en TpvNet («Apertura de Caja»). Además, la sesión de Windows quedó bloqueada y no se puede operar el TPV. Antes de reintentar hay que rearmar (snapshot ≤ 24 h, una operación `create_order`, 10 min). Según otro agente (no verificado por mí), el TPV tiene abierto el diálogo `APERTURA Y SELECCIÓN DE TURNOS` (Terminal 1, fecha 01/10/2026, Turno 2) y falta pulsar **Aceptar**; el control de TpvNet es por píxeles con el pipe elevado `claude-pc-elev` (`Agente/documentacion/bdp/control-escritorio-2026-10-01.md` §§132-160). A las 2026-10-10 el daemon de ese pipe no responde («Cua Driver daemon is not running»): hay que relanzarlo elevado (UAC, la usuaria).
- **Reintento L3.a (2026-10-10 22:36–22:38, tras orden de completar el plan):** TpvNet abierto en MENÚ GENERAL (Windows ya desbloqueado, Terminal 1, v36.2, CONEXION LOCAL), pero la caja sigue cerrada: rearmado OK (snapshot `18488515-9756-4571-8d7d-ff31efeb0041`, armado `98137357-ca37-4629-96ba-7de043676329`) y `bdp-sync` devuelve el mismo `301400` (`bdp_synced=false`, sin comanda en BDP, venta local intacta). Modo vuelto a `read_only`, URL local confirmada. El armado se consumió: el próximo reintento exige rearmar de nuevo.
- **L4.1 HECHA (2026-10-17 14:27):** receptor mínimo `C:\tmp\fase-d-local\l41-receptor.mjs`, escucha **solo** en `127.0.0.1:8099`, log en `C:\tmp\fase-d-local\avisos-l4.log` (fuera del repo). Verificado: `/salud` → `{"ok":true}` y `POST /api/bdp/avisos/test` → HTTP 200 con la línea `{"ts":"2026-10-17T14:27:53.099Z","accion":"test",...}` en el log. PID en `l41-receptor.pid`; se para matando ese node (PID verificado). Queda en marcha para L4.2. Falta la parte del TPV: configurar la URL de avisos en TpvNet (**Utilidades → Configuración Servicios Web → Configuración Avisos de Acciones**) y provocar los 7 eventos reales (L4.2, usuaria en TPV).
- **L2.2 HECHA (2026-10-17):** los conteos de la API cuadran **exactamente** con las tablas de perfil de la base del TPV, que es la fuente de la pantalla. La discrepancia aparente del informe de L2.1 (que anotó 20 artículos y 24 departamentos frente a los 421 y 31 de las tablas maestras) no era un defecto: el script de L2.1 pedía `GetPOSList` con `ItemsPerPage=20` y **solo leía la página 1**.
  | Fuente | Artículos | Departamentos |
  |---|---|---|
  | Tablas maestras del TPV (`Articulos` / `Departamentos`) | 421 | 31 |
  | Perfil del POS (`Articulos_Perfil` / `Departamentos_Perfiles`, `Perfil=1`) | **259** | **24** (+4 sub) |
  | WebLink (`Articles/GetPOSList` / `Departments/ExportFromProfile`) | **259** (6 páginas) | **24** (+4 sub) |
  | App (`bdp_article_stock`, caché del 2026-09-15) | 557 filas | — |
  El POS solo expone el catálogo de su perfil, así que 259/24 es lo que la pantalla del TPV muestra y lo que la API devuelve: **coincidencia**. La base local tiene un único perfil (`Perfil=1`), que es el que la app tiene configurado. Queda por explicar la caché de stock de la app (557 filas > 259 artículos del perfil), que es el gap del punto 7. Script: `C:\tmp\fase-d-local\l22-conteos.mjs` (borrar al cerrar).
- **Gaps de código vistos en L2/L3 (tareas aparte, no se mezclan):** modificar una venta no tiene ruta BDP; `/bdp/article-stock` lee caché sin consulta en vivo; `?dominios=a&dominios=b` del flush no funciona (`POST /bdp/push/flush` responde 400: `src/handlers/bdp_push.rs:45` usa `Query<...Option<Vec<String>>>`, y serde_urlencoded no deserializa claves repetidas en una secuencia, así que el filtro documentado en el propio doc-comment es inalcanzable; el workaround es flush global, que el guard filtra por alcance); comentarios «C3=b: nunca se llama CancelOrder» desfasados en `src/services/venta.rs:291` y `src/handlers/ventas.rs:225` (anular ya encola `venta/cancelar`); `clippy -D warnings` con avisos preexistentes.
- **Alcance ampliado (2026-10-10):** el plan se completó frente al alcance `Agente/usuario/alcance-fase-d-2026-10-02.md` y su PDF (`Agente/documentacion/bdp/alcance-fase-d-pdf-2026-10-10.html`) con L5.0 (T2) y L6–L10. Reparto de las **65** fichas en §10.
- **Control PC TPV (2026-10-10, tras autorización de la usuaria "inténtalo, yo le doy aceptar"):** TpvNet corre en integridad High (UIPI): `background` (PostMessage) imposible; `invoke_menu` sin UIA falla (`menu_path_unavailable`); `hotkey Ctrl+A` foreground se envía pero sin efecto. Vía OK: `invoke_menu ["Tpv-Táctil"]` por el pipe elevado `claude-pc-elev` (cuelga 45-120 s, fire-and-forget documentado) → abrió **APERTURA Y SELECCIÓN DE TURNOS** (window 44893530): Terminal 1, Fecha 01/10/2026, Turno 2, botones Salir/Ayuda/Abrir Turno/Aceptar. OJO: el resaltado naranja de «Caja y Cierres» en el MENÚ GENERAL es previo (no lo abrí yo); la vía correcta es Tpv-Táctil, no Caja y Cierres. Lección en `control-escritorio-2026-10-01.md` §§132-160.
- **Diagnóstico del bloqueo (2026-10-17, lectura SQL de `Bdp-Net_NetHos_36.dbo.Caja_Arqueo`):** la instrucción anterior («pulsar **Aceptar**») era errónea. Con una sola fila, la caja del POS 1 estaba **cerrada**: `Fecha` = 01/10/2026 (fecha de trabajo), `Turno_Activo` = 0, `Fecha_Apertura_T1` = 01/10/2026 17:11:59, `Fecha_Cierre_T1` = 02/10/2026 20:07:36 (el **turno** sí se cerró) y `Fecha_Cierre_Caja` = **01/01/1900** (nunca se cerró la **caja**). Por eso BDP rechazaba `create_order` con `301400`: la fecha de trabajo no avanza hasta que hay cierre de caja, no basta con cerrar el turno. En el diálogo APERTURA Y SELECCIÓN DE TURNOS el botón vivo es **«Abrir Turno»**, no «Aceptar» (deshabilitado). Ruta en el TPV: **Caja y Cierres → Cierres** (elemento UIA [78] de `MENÚ GENERAL`). **Resuelto por la usuaria el 2026-10-17 09:16–09:22:** `Fecha_Cierre_Caja` = 17/10/2026 09:16:55, turno nuevo abierto 09:22:47, `Turno_Activo` = 1, fecha de trabajo 17/10/2026, Terminal 1 / Turno 1 / Camarero 01.
- **L3.a HECHA (2026-10-17 13:40–13:41Z):** tras el cierre de caja de la usuaria, el mismo lote que falló el 2026-10-10 pasa entero. Rearmado: snapshot completo `1562dfa3-64e3-460e-aefd-d0f54dffb2f0` + `sync-mode` `unidirectional` con alcance `create_order`, 1 operación, objetivo la venta `PRUEBA-1` (`C:\tmp\fase-d-local\l3-armar.mjs`). `POST /ventas/:id/bdp-sync` → **HTTP 200**, `bdp_synced=true`, `bdp_order_id=15`, `bdp_sync_error=null` (desaparece el `301400`); `bdp-status` → `pending` (Status 0). Evidencia por los tres lados: (a) app → `bdp_audit_log` `create_order | glory_to_bdp | exito | 13:40:53Z`; (b) **BDP** → `POST /API/Orders/Get` del WebLink local devuelve `OrderId 15`, `MarketplaceOrderId G4002555556a048`, `MarketName Glory`, `PosId 1`, camarero 01, 1× `COCA-COLA` (PLU 1001) a 10,00 IVA 10 %, `Total 10`, `CreationDate 13:40:52Z`, `Status 0`; (c) **TPV** → las tablas vivas `Comandas_Cabecera` (Tipo 10, Terminal 1, Turno 1, `Hora_Apertura` 09:40:52 local, `Total_Lineas` 10) y `Comandas_Lineas` (PLU 1001 `COCA-COLA`, 1 ud, 10,00, IVA 10, empleado 1). El modo volvió solo a `read_only` al consumirse el armado. Lectura del pedido: `C:\tmp\fase-d-local\bdp-order.mjs` (borrar al cerrar).
- **Dato de configuración (L3.a):** la comanda llega al BDP con el nombre **`COCA-COLA`** para el código 1001, mientras la app tiene `bdp_default_article_name = "CAFE BOMBON"` para ese mismo código. El nombre lo resuelve el BDP desde su catálogo; el `bdp_default_article_name` de la app solo se usa como respaldo. Conviene alinear el valor de la app con el catálogo real antes de L6 (afecta a D11–D20).
- **L3.c HECHA (2026-10-17 14:08Z):** anulación de `PRUEBA-1` propagada al BDP local. Hubo que corregir antes **dos** defectos del guard de autorización (ninguno en la ruta de escritura):
  1. **El armado rechazaba `cancel_order`** (§9.15): el emparejamiento alcance↔objetivo solo admitía `create_order | add_payment | invoice` para objetivo `venta`, así que `PUT /configuracion/bdp/sync-mode` respondía **422** y no había forma de autorizar una anulación por API. Corregido en `src/handlers/configuracion.rs` (constante `BDP_WRITE_SCOPES_DE_VENTA` + `alcance_casa_con_objetivo`), con 5 tests unitarios. Commit `92f2afb`.
  2. **El armado manual vigente bloqueaba el push que autorizaba** (§9.16): `BdpWriteGuard::armar_push` caía siempre en `auto_arm_inner`, que se niega a pisar un armado activo; con el armado de `cancel_order` ya concedido, el flush devolvía `errores:5` y la fila quedaba muerta en la cola. Se probó la causa vaciando el armado a mano (`remaining_operations = 0` → re-flush → `sincronizados: 3`). Corregido en `src/services/bdp_write_guard.rs` con `armado_vigente_para`: si ya hay un armado vigente que cubre **esa misma** operación (mismo alcance, entidad, huella de conexión, snapshot y `bdp_sync_mode`), se reutiliza en vez de intentar crear otro; cualquier otra entidad sigue cayendo al auto-armado, que se niega. Test propio + suite completa (452 pasan / 0 fallan / 48 ignorados). Commit `46e49eb`.
  Evidencia de los tres lados: (a) app → `bdp_audit_log` `cancel_order | glory_to_bdp | exito | 2026-10-17T14:08:51.727331Z`, motivo `auto_arm:cancel_order:venta:634f62d1-…`; (b) **BDP** → `/API/Orders/Get` del WebLink devuelve `[301000]-NO EXISTE NINGUNA COMANDA CON ESTE ID`; (c) **TPV** → `SELECT … FROM Comandas_Cabecera WHERE Tipo=10` devuelve **0 filas**. La anulación salió por la vía de auto-armado, no por la manual: es exactamente el defecto 2 el que dejaba la manual muerta.
- **Entorno de ejecución para la fase D (2026-10-17, provisional y fuera de `Arranque`):** la app corre en la 89, pero el WebLink solo escucha en el loopback del portátil (127.0.0.1:8070), así que el taller no lo alcanza (`curl` desde la 89 devuelve `000`; `workspace-manager/src` no tiene soporte de túnel inverso). Se levanta a mano un túnel inverso **solo loopback**: `ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -o ServerAliveCountMax=3 -R 127.0.0.1:8070:127.0.0.1:8070 dev89`. Hay que **pararlo al cerrar la fase D** (`ssh dev89 "pkill -f 'ssh.*8070'"` desde el portátil, o matar el `ssh` local) y decidir si el taller debe soportarlo de serie. Además, el `.env` que `Arranque up` reescribió apuntaba a `glory_backend`, que estaba **vacía** (0 usuarios); se restauró ahí una copia de la base real del portátil, `glory_backend_kamples` (dirección sancionada: el portátil es el origen). Quedan `users=58`, `ventas=46`, `bdp_audit_log` con las 59 filas previas y `bdp_base_url=http://127.0.0.1:8070`, `bdp_sync_enabled=true`, `bdp_sync_mode=read_only`, `bdp_pos_id=1`.
- **L5.0 HECHA (2026-10-17): base T2.** `migrations/20261017120000_bdp_base_t2.up.sql` (+ `.down.sql`) crea **cinco** tablas, todas con `user_id` como el resto del esquema y todas nacidas vacías (no migran datos): `bdp_avisos_inbox` (buzón, idempotente por `(user_id, clave_idempotencia)`), `bdp_id_map` (mapeo Glory↔BDP, con índice único parcial sobre `bdp_id` para que dos entidades no reclamen el mismo registro), `bdp_sync_cursor` (único por `(user_id, dominio, perfil)`; `perfil` es `NOT NULL` a propósito porque en PostgreSQL los NULL no chocan entre sí), `bdp_pendientes_agotados` (cola de revisión manual, con `resuelto_por` y `nota_resolucion`) y `bdp_stock_ledger` (libro S/V con `uq_bdp_stock_ledger_bloqueo (user_id, articulo_codigo, almacen) WHERE estado = 'pendiente'`, que es el bloqueo por artículo y almacén de D26g).
  **El PDF nombra seis tablas; se crean cinco a propósito.** La sexta, *outbox*, ya existe: `bdp_push_pendientes` es la cola de salida transaccional (se escribe en la misma transacción que el cambio local, con estados, reintentos e índice único parcial de una sola fila activa por entidad y operación). Duplicarla habría dejado dos colas que pueden discrepar. La correspondencia queda documentada en la cabecera de la migración. El flujo de avisos hace lecturas, no escrituras, así que tampoco necesita outbox propia.
  Evidencia: `tests/bdp_base_t2.rs`, 5 tests (`#[sqlx::test(migrations = "./migrations")]`), **5 pasan / 0 fallan**; suite completa **456 pasan / 0 fallan / 48 ignorados**; `up`/`down` verificados en la 89 sobre la base desechable `t2_check` (`up` crea exactamente esas 5 tablas, `down` deja 0, base borrada después); `Arranque gate --cierre` **0 nuevos** (base 226). De paso se cerró el defecto de la misma clase que el punto 15 en los 15 alcances de escritura (punto 18).
- **Próximo paso:** **L4.2** (la usuaria configura la URL de avisos en TpvNet y provoca los 7 eventos; el receptor ya está en marcha y el destino vacío está localizado, punto 17), luego L4.3–L4.4 y **F2** (worker de los 7 eventos, `GetOrder`). F1 ya está hecha (§9.19). L3.b sigue bloqueada por código (punto 9) y L3.a2 exige impresora de cocina. Sigue pendiente decidir las 4 filas `error` de la cola (recomendado: no tocarlas; nunca flush sin `?dominios=venta`) y el gap de stock (punto 7).
- **Control del PC (2026-10-17):** la causa de los clics que «caían lejos» quedó cerrada y no era la herramienta: mezclaba espacios de coordenadas (físico 2256×1504 del driver frente a lógico 1805×1203) y no pasaba `capture_scope`. Receta y verificación por recaptura en la skill `pc-control` (§«Coordenadas» y §«Verificar cada clic»). Dato nuevo: los hijos de un menú colapsado **no traen `frame`** en el árbol UIA, así que clicar el submenú por píxel es adivinar (un clic a 27 px abrió «Liquidación Empleados» [82] en vez de «Cierres» [78]); hay que expandir el padre y re-snapshottear.

## 9. Decisiones pendientes (por recomendación)

1. **Permiso para L1**: hecho (2026-10-10).
2. **`CancelOrder`**: mantener "pendiente de prueba local" y corregir `roadmap.md` l.118. **Resuelto 2026-10-17:** el roadmap ya estaba alineado (`roadmap.md:118` dice exactamente eso) y **L3.c se completó el mismo día** tras corregir los puntos 15 y 16. El roadmap ya refleja que la anulación llega al BDP.
3. **Permiso por lote** antes de cada escritura en el TPV local, y antes de enviar nada a Guillermo.
4. **Push a `main` al cierre** (publicación a remoto compartido, AGENTS §0.3). Recomendado: sí, con fast-forward y sin force-push, una vez validado el cierre.
5. **`bdp_pos_id` → aplicado: 1** (decisión del agente, reversible; la usuaria debe confirmar que es su terminal). El 31 no existe en el BDP local; el 1 es el único terminal de `Kdi-Net_Tpv-Net_36.Terminales` (Codigo 1, "TERMINAL 1") y `Tenders/GetPOSList` responde con 3 medios de pago. Alternativa descartada: crear un terminal 31 en el TPV (escribe en el TPV; requiere permiso por lote).
6. **`.env`**: `BDP_BASE_URL` apunta a `127.0.0.1:8070` y `BDP_LOGIN`/`BDP_PASSWORD` son los del WebLink local; las claves de producción quedan en `.env.prod` (ignorado, no tocar). La BD manda: el bootstrap ya está aplicado (`bdp_env_bootstrap_applied_at`).
7. **Stock (gap de L2.3)**: `/bdp/article-stock` lee la caché local (557 filas, sync 2026-09-15), no el TPV en vivo. Decidir si cuenta como fallo de L2.3 o se verifica con lectura directa (`Warehouse/GetStock`, ya probada en L2.1 con stock 0).
8. **Cola `error` (4 filas)**: recomendado no tocarlas. Un flush manual procesaría también `error`; si se hace, filtrar con `?dominios=venta`.
9. **Modificar venta sin ruta BDP** (bloquea L3.b y D05–D07). Recomendado: tarea aparte en el roadmap y diseñar la ruta dentro de F2/F3 (L5), con prueba en L6–L9. Alternativa: ampliar ahora el alcance del plan, a costa de retrasar L3.
10. **Caja del POS 1** (`301400`; bloqueaba L3 y todo L6–L10). **Resuelto 2026-10-17:** la usuaria cerró la caja y abrió turno nuevo (ver §8). Lo que faltaba era el **cierre de caja**, no el de turno. Sobre `OrderEndType`: D03a usa 0 y la app usa 1; recomendado probar primero con 1 (valor actual) y, si el TPV lo rechaza, ensayar 0 antes de tocar código.
11. **D23-m (comanda manual con aviso perdido)** contradice «sin preguntas a BDP»: no hay lista de abiertas confirmada. Recomendado: no preguntar a BDP ahora; resolver por listado si existe y, si no, declarar el límite formal. Alternativa: preguntar a BDP (decide la usuaria; sale por Guillermo).
12. **Suscripción y licencia** (Payment/Add, InvoiceOrder, licencia 84 de D01): el TPV local figura como DEMO. Recomendado: ensayar en local lo que el DEMO permita; lo demás queda «pendiente de real». Alternativa: pedir licencia de pruebas (la usuaria).
13. **Almacén real y motivos de stock** (L7): hay 6 almacenes en local y el PDF no dice cuál es el real ni qué motivos son válidos. Recomendado: preguntar a Guillermo en la próxima ronda y ensayar mientras en el almacén 1.
14. **`CreateCustomer` con `Overwrite=true`** (D25b): sobrescribe datos del cliente en caja. Recomendado: no soportarlo salvo aprobación expresa de la usuaria; sin ella D25b se declara «no soportado».
15. **`cancel_order` no se podía armar** (bloqueaba L3.c y D09/D09a; hallado y **corregido el 2026-10-17**). `VALID_BDP_WRITE_SCOPES` incluía `cancel_order`, pero el emparejamiento alcance↔objetivo solo admitía `create_order | add_payment | invoice` para objetivo `venta`, así que el armado por lote devolvía 422 y la única vía de escritura para cancelar quedaba cerrada en `read_only`. Corregido en `src/handlers/configuracion.rs` (constante `BDP_WRITE_SCOPES_DE_VENTA` + función `alcance_casa_con_objetivo`) con 5 tests unitarios, gate limpio. Commit `92f2afb`. **Decidido: arreglarlo** (opción recomendada, con el visto bueno de la usuaria al preguntárselo por tocar el guard de autorización). La misma clase de defecto en los otros alcances se cerró el mismo día: punto 18.
16. **El armado manual vigente bloqueaba el push que autorizaba** (segundo defecto, hallado al reintentar L3.c; **corregido el 2026-10-17**). `BdpWriteGuard::armar_push` caía siempre en `auto_arm_inner`, que se niega a pisar un armado activo; el armado manual concedido por lote nombraba destino y entidad, y esa misma fila era la que quedaba muerta en la cola (`push/flush` → `sincronizados:0, errores:5`). Corregido con `armado_vigente_para`: si hay un armado vigente que cubre esa operación exacta (mismo alcance, entidad, huella, snapshot y `bdp_sync_mode`) se reutiliza; otra entidad sigue cayendo al auto-armado, que se niega, así que el fail-closed no se relaja. Test propio + suite completa (452 pasan / 0 fallan / 48 ignorados). Commit `46e49eb`. **Decidido: arreglarlo**, con el mismo visto bueno.
17. **Destino de los avisos sin configurar** (localizado el 2026-10-17; bloquea L4.2 y con ello L5.0). En la base del TPV, `Config_WebService` tiene **1 fila** con `Terminal=1`, `Activo=True` e **`IP_ServidorMensajes` vacío**: por eso hoy no sale ningún aviso hacia la web. La configuración de entrada sí está puesta (`Config_WebService_Servidores`: `ListeningIP=127.0.0.1`, `ListeningPort=8070`, `ConexionSegura=False`, `Audiencia=restaurante-local`, que es el WebLink que usa L2/L3). `Config_WebLink` también tiene una columna `IP_ServidorMensajes` y sigue con **0 filas**. Ruta en la UI del TPV: **Utilidades → Configuración Servicios Web → Configuración Avisos de Acciones**. Lo escribe la usuaria (es una escritura en el TPV). Falta confirmar en la UI si el campo admite `host:puerto` o una URL con ruta, porque el receptor de L4.1 atiende en `/api/bdp/avisos/<accion>`.
18. **Los quince alcances de escritura ya se pueden armar** (tarea «Revisar armado de `create_article` y `add_points`»; **resuelto el 2026-10-17**, roadmap 10AA-3-B). Era la misma clase del punto 15, en dos capas. (a) **Vocabulario**: la API emparejaba el alcance contra `"cliente"` o `"venta"`, pero el worker de push pasa a `target_entity_type` el **dominio literal** (`"articulo"`, `"stock"`, `"cliente_puntos"`, `"propina"`, `"departamento"`, `"familia"`), así que 10 de los 15 alcances devolvían 422 siempre. (b) **Identificador**: el armado exigía `target_entity_id = entidad_uuid(dominio, entidad_id)`, un SHA-256 que la API no podía calcular desde fuera. Corregido con `tipo_destino_de_alcance`, `alcance_usa_uuid_propio` y `entidad_uuid` compartidos en `src/services/bdp_write_guard.rs`; `target_entity_id` pasa de `Uuid` a texto (compatible hacia atrás: un UUID sigue siendo texto válido) y los alcances de venta y cliente siguen exigiendo UUID propio, sin derivarlo. 4 tests unitarios en `src/handlers/configuracion.rs`. Para la fase D el que hacía falta era `add_points` (D12a–D12c); `create_article` **no** lo era, porque D24a dice que el alta se hace en la caja y la web solo lee. Se arreglaron los 15 igualmente: el mapa de tipos de destino cuesta lo mismo y cierra el bloque. Commit `a95aa58`.

19. **Contrato real de los siete avisos, sin adivinar** (2026-10-17, F1). El ZIP del distribuidor `AvisosDeAcciones_Documentacion` seguía sin abrir desde septiembre; se extrajo a `C:\tmp\avisos-bdp\` (fuera del repo) y de él salen los nombres exactos de los campos (`DataContracts/*.cs`), las dos formas de petición (GET por query con `[FromQuery]`, POST con cuerpo JSON `[FromBody]`) y la clave del asunto: **los nombres de los parámetros los fija la configuración del TPV** (`Clave`/`Valor` por acción), así que el receptor los lee por alias y sin distinguir mayúsculas, acentos ni separadores. El PDF no documenta reintentos, tiempos de espera ni respuesta esperada, y el TPV no devuelve identificador de operación: la idempotencia la sostenemos nosotros. El demo `WebhooksServer_Test.exe` del paquete es un receptor, no un emisor, así que no sirve para provocar los eventos de L4.2 (los provoca la usuaria en el TPV). Decisión de admisión (L4.4): solo se rechaza con 4xx lo que no se puede ni identificar —falta `IdentificadorComanda`—; todo lo demás que falte o venga raro se guarda con `requiere_revision` y su nota, sin actuar. Un aviso nunca se pierde por un campo inesperado.

**Decididas por el agente (reversibles, AGENTS §0.3), sin preguntar:** rama al primer commit (L0.6); correcciones documentales (L0.5); commits por archivo con atribución; integración local en `main` al cierre según CLAUDE.md "Ramas". Si la integración choca con algo, se anota en el roadmap y se pregunta.

## 10. Cobertura del PDF (anexo B8, fichas D01–D32)

Cada ficha tiene fase. Estado a 2026-10-10: **ninguna superada**; «parcial» = ensayada en lectura. Fuente: `Agente/documentacion/bdp/alcance-fase-d-pdf-2026-10-10.html` y `Agente/planes/anexo-b8-aceptacion-2026-10-08.md` (rev. 10/10; 65 fichas, todas con fase asignada — comprobado el 2026-10-17).

| Fichas | Qué comprueba | Fase | Estado / dependencia |
|---|---|---|---|
| D01, D02 | Salud, versión, login, licencia 84; logs sin secretos | L2.1, L10 | Parcial (L2.1 OK; licencia 84 solo en el TPV real; `bdp_sync_cursor` ya existe, falta escribirlo desde el código) |
| D03, D03a | Crear comanda sin y con envío a cocina | L3.a, L3.a2 → L6–L9 | **D03 superada en L3.a (2026-10-17)**: comanda 15 en el BDP y en `Comandas_Cabecera`/`Comandas_Lineas` del TPV. D03a (envío a cocina, `OrderEndType=0`) pendiente de impresora |
| D04, D08 | Respuesta perdida al crear; toque simultáneo caja y web | L9 | F2 (base T2 lista) |
| D05–D07 | Modificar venta (añadir, quitar, cambiar líneas) | L3.b | Bloqueada por código (punto 9) |
| D09, D09a | Anular desde la web, con efecto en cocina | L3.c | **D09 superada en L3.c (2026-10-17)**: la anulación llegó al BDP (`301000` en `Orders/Get`) y la comanda desapareció de `Comandas_Cabecera`. D09a (efecto en cocina) pendiente de testigo de impresora auxiliar |
| D10 | Anular en caja y reflejo en la web | L9 | Tras F2 |
| D11a–c | Propina: suma, sustitución, cero | L6.a | Tras L3, T2 y permiso |
| D12a–c | Puntos: saldo ±N, sin reintento ciego | L6.b | Pendiente de soporte del terminal |
| D13, D13a, D14, D14s, D15, D15t, D16, D17 | Pagos: un cobro por ID, consulta antes de repetir, tope de tres, pago de caja | L6.c | Precondición de suscripción (punto 12) |
| D18, D18a–d, D19, D19t, D20 | Facturas: número único, simplificada, `InvoiceOrder`, `Status=3`, `BillingDetails` | L6.d | Idem |
| D21, D21a | Siete avisos; traspaso de comanda completa | L4, L9 | El receptor de los siete avisos ya existe (F1, 2026-10-17); L4.2 (eventos reales) sigue pendiente de la usuaria en el TPV |
| D22, D22a | Aviso duplicado y fuera de orden; traspaso de línea | L9 | Tras L4 y F2 |
| D23a, D23b | Trabajador caído; receptor inaccesible | L9 | Tras F2 |
| D23-m | Comanda manual con aviso perdido | L9 | Decisión (punto 11) |
| D29 | Tres reinicios (sincronización, WebLinkRestAPI, BDP.NET) | L9 | El de BDP.NET lo hace la usuaria |
| D31 | Llamar al camarero | L9 | Tras F2 |
| D24a–d | Alta, modificación, desactivación y borrado en caja | L8 | Operación en TpvNet por la usuaria; lectura con cursor (tabla lista) |
| D24-p, D24-m | Plano, formas de pago y empleados | L8 | Posible ya (lectura) |
| D25 | Clientes: cambio en BDP, en la web (`Overwrite=true`), contradictorios | L8 | Decisión (punto 14) |
| D26a, D26b | `GetStock` y `GetListStock` | L7 | Parcial (`GetStock` OK en L2.1, stock 0) |
| D26, D26c–D26g, D27 | Escritura de stock, libro y ventas de caja | L7 | Permiso y datos del distribuidor (punto 13); tabla del libro lista |
| D28, D28-auto, D28-auto-R, D28a | Exportar documentos, perfil programado, reinicio, albaranes | L8 | Cursor ya disponible (tabla lista) |
| D32 | Mensaje de procedimiento ante cada operación que la API no expone | L10 | Código F2/F3 y lista de responsables por escrito |
| D30 | Piloto y despliegue por etapas | Fuera (F4) | Permiso aparte; requisitos en L10 |

**Base T2:** L5.0 (roadmap 09AA-2), hecha el 2026-10-17 (5 tablas; *outbox* ya era `bdp_push_pendientes`). **Despliegue por etapas del PDF** (observación → maestros → lectura → escritura → pagos/factura → stock): se ejecuta en F4, no aquí; aquí solo se deja probado el rollback en local. **Paradas del PDF** (duplicado económico, descuadre fiscal, pérdida de comanda, degradación del TPV): criterio de aborto de cualquier lote de L6–L10.
