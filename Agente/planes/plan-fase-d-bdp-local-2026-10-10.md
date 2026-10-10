# Plan — Fase D sobre BDP local (2026-10-10)

**ID:** 10AA-3 · **Estado:** planificado y revisado (2026-10-10); ejecución en curso: L0, L1 y L2.1 hechos · **Próximo paso:** decidir `bdp_pos_id` (§9, punto 5) y después L2.2 con la usuaria.

**Relacionado:** `plan-fase-d-avisos-bdp-2026-09-19.md` (diseño F0–F4), `plan-alcance-local-bdp-2026-10-01.md` (P0–P4 con TPV local), `Agente/usuario/alcance-fase-d-2026-10-02.md` (alcance rev. 10/10).

## 1. Objetivo

Dejar la aplicación apuntando al BDP local (TPV 36.2 licenciado en el portátil) en lugar del BDP real, y con ese TPV cerrar la F0 empírica de la fase D (Avisos de Acciones). La F0 bloquea F2 y F4.

## 2. Alcance y no alcance

**Dentro**
- Conmutar la configuración BDP de real a local (en base de datos, no en `.env`).
- Revisar la cola de envíos pendientes antes de conmutar (L0.3).
- Pruebas de lectura y de escritura reversible contra el TPV local (comandas `PRUEBA-*`).
- F0 empírica de los 7 avisos en el TPV local.
- Tras la F0: F1–F4 de 199A-1, según el diseño del plan 2026-09-19.

**Fuera**
- BDP real (`100.83.196.35:8068`, PC remoto por Tailscale) y cualquier host de producción: ni lectura ni escritura.
- `bdp_sync_mode = automatic`. Este plan opera en `read_only`, salvo escrituras por lote autorizado (L3).
- Pagos y facturas (`Payment/Add`, `InvoiceOrder`) hasta confirmar suscripción.
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
- Puerto del BDP real: `100.83.196.35:8068` (`Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md:6`). Puerto por defecto en código: `localhost:8068` (`src/services/bdp_sync.rs:115`). Eso no prueba la instancia local.
- `RESTAURANTE` no figura en `workspace-manager/scripts/dev/registro.json`. Sin entrada no hay mando `Arranque` (AGENTS §9.2).
- Frontend: `frontend/vite.config.ts:19` declara `5173`. El `:5182` de las verificaciones del 09-16 es un arranque manual, no configuración.
- Scripts existentes: `scripts/run-cargo.mjs`, `scripts/run-with-db.mjs`, `scripts/branch-db.mjs`. Gate canónico: `npm run gate:check` (`package.json:37`, `scripts/quality/gate-check.mjs`).
- Contradicción: `roadmap.md` 01AA-1 dice "P0 siguiente", pero `plan-alcance-local-bdp-2026-10-01.md:144` dice "P0 COMPLETO".
- Contradicción: `roadmap.md` 199A-1 menciona "12 preguntas a BDP", frente a la decisión de 2026-09-20 "sin preguntas a BDP".
- Git: `main` sin ramas locales. `git status` muestra 32 entradas ajenas (modificados: `roadmap.md`, `Agente/planes/plan-deploy-desde-cero-2026-09-18.md`, `frontend/tsconfig.tsbuildinfo`; sin trackear: tareas 10-01 a 10-10 y documentación BDP).

**No verificado (condiciona o bloquea)**

- Host y puerto del WebLink del TPV local. No consta en el repositorio. L0.1 lo averigua sin tocar la app.
- Licencia activa en vivo (solo consta en documentación).
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
| L0.3 | Estado actual en BD (SQL de solo lectura): `bdp_base_url`, `bdp_sync_enabled`, `bdp_sync_mode`, filas de `bdp_push_pendientes` y `bdp_write_arming` | Principal | Cola vacía, o pendientes resueltos con la usuaria antes de L1. **Resultado 2026-10-10:** `bdp_base_url` sigue en el BDP real (`100.83.196.35:8068`); `bdp_sync_enabled=false`, `bdp_sync_mode=read_only`, `modo_operacion=standalone`, `push_modalidad=automatico`; `bdp_write_arming` y `bdp_poll_schedule` vacías. Cola: 0 `pendiente`, 9 `sincronizado`, 4 `error` (37b18564, 5d92004c, 67b8d9ae, dc84a08a: «ALMACÉN 1 NO EXISTE» ×3 y «CÓDIGO DE IVA 0 NO EXISTE» ×1). `error` es estado activo: el flush las reintenta. Decisión pendiente (ver L2.3) |
| L0.4 | Decidir `CancelOrder` | Usuaria (decisión 2) | Alcance y roadmap alineados |
| L0.5 | Corregir contradicciones de documentación: roadmap 01AA-1 ("P0 completo; siguiente P1", según `plan-alcance-local-bdp-2026-10-01.md:144`) y 199A-1 (quitar "12 preguntas a BDP") | Principal (decisión reversible del agente) | Roadmap coherente con el plan de alcance y con la decisión 2026-09-20 |
| L0.6 | Rama: no se crea todavía. Se crea desde `main` con el primer commit autorizado | Principal | Ninguna rama suelta en ningún momento (CLAUDE.md "Ramas") |

L0 cierra cuando L0.1–L0.4 tienen respuesta y L0.3 no deja pendientes sin resolver.

### L1 — Conmutar la app a BDP local

Precondición: L0.1–L0.3 OK y permiso expreso (decisión 1).

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L1.1 | Volcar la fila de `configuracion_restaurante` a `C:\tmp\fase-d-local\config-antes.sql` (incluye credenciales, por eso fuera del repo) | Principal | Archivo fuera del repo |
| L1.2 | Poner `bdp_sync_enabled = false`. Luego, en Configuración → BDP, la URL local (L0.1), login, password, código integrador y pos. La usuaria escribe los valores en la pantalla, no en el chat. Antes de guardar, comprobar que la URL no es `100.83.196.35` | Usuaria escribe; principal verifica | `GET /api/configuracion` devuelve la URL local, `bdp_sync_enabled = false` y `bdp_sync_mode = read_only` |
| L1.3 | Solo si el host no es loopback: añadir la URL exacta a `BDP_WRITE_ALLOWED_ORIGINS` en `.env` y reiniciar con `Arranque up "<id>"` (requiere L0.0) | Principal, con permiso | Escritura permitida solo a esa URL |
| L1.4 | Buscar `100.83.196.35` en el log del backend (`Arranque logs "<id>"`) tras L1.2 y tras L2 | Verificador (devuelve solo incidencias) | Cero apariciones |

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

- **L3.a** Crear comanda `PRUEBA-1` y verificar en TPV.
- **L3.b** Modificar líneas de `PRUEBA-1`.
- **L3.c** Cancelar `PRUEBA-1` (`CancelOrder`, según decisión 2).
- **L3.d** Pagos y facturas: bloqueados hasta confirmar suscripción. No entran en este plan.

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

- **F1** receptor e inbox (`bdp_avisos_inbox`, `POST /api/bdp/avisos/:accion`, idempotencia por evento + pedido + timestamp; la ruta aún no existe). Principal diseña; implementador crea migración y estructura según especificación; verificador ejecuta tests.
- **F2** worker de los 7 eventos (`GetOrder` para completar datos). Principal decide; implementador para handlers repetitivos.
- **F3** reconciliación, perfiles y XML para distribuidor. Pendiente de respuesta del distribuidor.
- **F4** TPV real y piloto de 7 días. Fuera de este plan; requiere permiso aparte.

Tests de L5 según AGENTS §8.1: por archivo tocado en cada bloque; suite completa una vez, al cierre.

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

La configuración (L1.2), las decisiones y la lectura de resultados de escritura las hace el principal o la usuaria.

## 7. Verificación y Definition of Done

- **L1:** `GET /api/configuracion` muestra URL local, `bdp_sync_enabled = false` y `read_only`. Cero apariciones de `100.83.196.35` en logs.
- **L2:** lectura con evidencia.
- **L3:** lotes con `bdp_audit_log` y estado en TPV verificados.
- **L4:** matriz de 7 eventos.
- **Calidad (AGENTS §6):** `npm run gate:check` en RESTAURANTE y re-análisis `POST http://localhost:8787/api/gate/analizar` con `{"clave":"RESTAURANTE","forzar":true}`. Sin hallazgos nuevos atribuibles al cambio.
- **Documentación:** roadmap (10AA-3 cerrada; 01AA-1 y 199A-1 alineadas), `Agente/completados/tareas-<fecha>.md`, matriz de avisos. El plan se mueve a `Agente/planes/completados/` con sus enlaces actualizados (AGENTS §7.2).
- **Limpieza:** receptor parado (PID verificado), `C:\tmp\fase-d-local\` borrado, procesos propios parados.
- **Commit e integración (CLAUDE.md "Ramas"):** commits por bloque si la política del repositorio lo exige (AGENTS §5), stage explícito, cada commit con la línea `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`. Rama `fase-d-bdp-local` integrada en `main` con fast-forward o merge, sin force-push. El push a `main` es publicación a remoto compartido: se pide confirmación antes (AGENTS §0.3). Antes de borrar ramas local y remota: `git bundle create` y `git bundle verify`. Si la integración necesita una decisión de la usuaria (conflicto, rama no fast-forward), anotar en el roadmap con su motivo y preguntar.

## 8. Estado y próximo paso

- **Estado:** L1 hecho (2026-10-10): volcado en `C:\tmp\fase-d-local\config-antes.sql`; `bdp_base_url=http://127.0.0.1:8070`, login `wan`, código integrador de `.env`; `bdp_sync_enabled=false`, `bdp_sync_mode=read_only`. Login del WebLink verificado; sin `100.83.196.35` en el log del backend; `bdp_write_arming` vacía. Arranque RESTAURANTE verde (3105, 5182) tras registrar el proyecto en el mando.
- **Cambios en BD (2026-10-10, reversibles; backups `*-pre-weblink-2026-10-10.bak` verificados):** `bdp_pos_id` 31→1 (decisión 5a, pendiente de confirmar por la usuaria); `push_modalidad` automatico→manual; `bdp_sync_enabled` false→true; `modo_operacion` standalone→auto. Sin cambio: `bdp_sync_mode=read_only`, `bdp_employee_id=1`, `bdp_items_profile_id=1`. Cola `bdp_push_pendientes`: 9 sincronizadas y 4 en `error`, sin tocar.
- **L2.3 hecho (2026-10-10, solo GET, `C:\tmp\fase-d-local\l23-lectura-app.mjs`):** login 200; `/bdp/explorar` 200: artículos 2, clientes 5, **departamentos 24** (tras corregir el explorador: usaba `ExportDepartments` con rango 1..999, que devuelve lista vacía; ahora usa `ExportFromProfile`, igual que el perfil), salones 2, empleados 3; `/bdp/departamentos/perfil` 200: 24. `/bdp/article-stock` 200: 557 filas, todas stock 0, `ultima_sync_at` máx. 2026-09-15 → **es caché local antigua, no lectura viva del TPV** (gap, ver punto 7). `/bdp/audit` 200: 57 filas; 0 de hoy contra la IP real.
- **Cambio de código (sin push):** `src/services/bdp_explorer.rs` (departamentos vía perfil); `scripts/run-cargo.mjs` (cap 4096→20480 MB, techo de AGENTS §0.2); `.gitignore` (`.env.prod`). `cargo check --all-targets` OK; `clippy -D warnings` falla por avisos preexistentes en `bdp_push.rs`, `bdp_weblink.rs`, `modo_operacion.rs` (sin relación con el cambio).
- **L2.1 hecho (2026-10-10, solo lectura):** health, versión y login 200; tenders 4; departamentos 24 (perfil 1); artículos 2 en Export y 20 en GetPOSList; stock del artículo 1001 en almacén 1 = 0. `Tenders/GetPOSList` con POS 31 → `403500 NO EXISTE NINGÚN TERMINAL`; con POS 1 → 3 medios de pago. Salidas en `C:\tmp\fase-d-local\l2-salida.txt` y `l2-salida-pos1.txt` (borrar al cerrar). Pedidos abiertos no consultados: sin lista confirmada en la doc.
- **Incidencia de L1/L2:** la primera ejecución de L2.1 tomó `BDP_BASE_URL` de `.env` (la IP real `100.83.196.35:8068`). Fallo de conexión por timeout; no hubo petición HTTP. Se repitió con la config de BD (`127.0.0.1:8070`).
- **Próximo paso:** L2.2 (la usuaria compara conteos con el TPV) y decidir las 4 filas `error` (recomendado: no tocarlas; nunca flush sin `?dominios=venta`) y el gap de stock (punto 7). L3 solo con permiso por lote.

## 9. Decisiones pendientes (por recomendación)

1. **Permiso para L1**: hecho (2026-10-10).
2. **`CancelOrder`**: mantener "pendiente de prueba local" y corregir `roadmap.md` l.118. Bloquea L3.c.
3. **Permiso por lote** antes de cada escritura en el TPV local, y antes de enviar nada a Guillermo.
4. **Push a `main` al cierre** (publicación a remoto compartido, AGENTS §0.3). Recomendado: sí, con fast-forward y sin force-push, una vez validado el cierre.
5. **`bdp_pos_id` → aplicado: 1** (decisión del agente, reversible; la usuaria debe confirmar que es su terminal). El 31 no existe en el BDP local; el 1 es el único terminal de `Kdi-Net_Tpv-Net_36.Terminales` (Codigo 1, "TERMINAL 1") y `Tenders/GetPOSList` responde con 3 medios de pago. Alternativa descartada: crear un terminal 31 en el TPV (escribe en el TPV; requiere permiso por lote).
6. **`.env`**: `BDP_BASE_URL` apunta a `127.0.0.1:8070` y `BDP_LOGIN`/`BDP_PASSWORD` son los del WebLink local; las claves de producción quedan en `.env.prod` (ignorado, no tocar). La BD manda: el bootstrap ya está aplicado (`bdp_env_bootstrap_applied_at`).
7. **Stock (gap de L2.3)**: `/bdp/article-stock` lee la caché local (557 filas, sync 2026-09-15), no el TPV en vivo. Decidir si cuenta como fallo de L2.3 o se verifica con lectura directa (`Warehouse/GetStock`, ya probada en L2.1 con stock 0).
8. **Cola `error` (4 filas)**: recomendado no tocarlas. Un flush manual procesaría también `error`; si se hace, filtrar con `?dominios=venta`.

**Decididas por el agente (reversibles, AGENTS §0.3), sin preguntar:** rama al primer commit (L0.6); correcciones documentales (L0.5); commits por archivo con atribución; integración local en `main` al cierre según CLAUDE.md "Ramas". Si la integración choca con algo, se anota en el roadmap y se pregunta.
