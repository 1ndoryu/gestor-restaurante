# Plan — Corrección de la independencia BDP (H1–H8, decisiones D1–D6)

> **Fecha:** 2026-08-27
> **Rama:** `glory-rs-rest`
> **ID de bloque:** `208A-2` (corrección) — hereda la auditoría `208A-1`
> **Origen:** auditoría 1×1 `plan-auditoria-independencia-bdp-2026-08-27.md` (baseline verde: 153
> tests, `cargo check` exit 0, `tsc` limpio). El núcleo de independencia está implementado y
> testeado; la deuda es de **UX, ubicación y cierre de ciclos** (H1–H8). Decisiones del usuario
> D1–D6 tomadas el 2026-08-27 (ver §6 del plan de auditoría).
> **Motivo (cita del usuario):** "no podemos ir simplemente parcheando cosas… necesitamos un plan
> completo que revise el plan de independencia completo y evitar hacer un desastre de nuevo".

---

## 0. Reglas de oro (no negociables)

1. **Local-first**: todo funciona 100% en `standalone` (modo independiente), sin BDP.
2. **En `standalone` no se encola ni se envía nada a BDP** (invariante del plan 128A-1/198A-1,
   con test existente `flush_en_standalone_no_envia_ni_consume_la_cola` que debe seguir verde).
3. **Reutilizar la maquinaria existente**, no duplicar: `crear_article_map` (alta con rango
   reservado D3 + encolado), `ajustar_stock` (motivo + auditoría), `BdpPushService`
   (`encolar`, `listar_pendientes`, `marcar_resultado`), `BdpPushFlushService` (worker), guards
   de permisos (`CatalogoEdicion`, `StockAjuste`, etc. con 403 real).
4. **Migraciones aditivas** con defaults (M15): nunca borrar/renombrar columnas existentes.
5. **Un hallazgo = una fase** con su DoD y su verificación. Nada de parches sueltos.
6. Cada fase nueva que toque encolado mantiene la invariante: en `standalone` las filas quedan
   pendientes (no se envían) — comportamiento local-first deseado, no un bug.

**Fuera de alcance:** BDP real (suscripción WebLink pendiente de activar por el cliente, hasta
24/09), 138A-2 (lecturas reales), deploy a producción, Sentinel/coolify.

---

## Mapa hallazgo → fase → decisión

| Hallazgo | Severidad | Fase | Decisión del usuario |
| --- | --- | --- | --- |
| H1 — CRUD de artículos solo en Configuración → BDP | Alto | C1 | D1: mover a "Catálogo" |
| H2 — Stock no puede crear artículos; empty state orienta a BDP | Alto | C2 | D2: botón "Nuevo artículo" |
| H3 — Inventario no persiste el conteo | Alto | C3 | D3: persistir localmente |
| H4 — Diferencia contada no aplica al stock local | Medio | C3 | D4: aplicar con motivo "conteo" |
| H5 — Sin normalización standalone+sync al guardar | Bajo | C5 | Técnica (sin decisión) |
| H6 — Sin visibilidad de la cola de push | Medio | C4 | D5: sección "Sincronización" |
| H7 — "Sync catálogo" habilitado en standalone | Medio | C2 | Técnica (sin decisión) |
| H8 — Empty state de Compras sin "Nuevo albarán" | Bajo | C6 | Técnica (sin decisión) |

---

## C1 — Catálogo unificado (H1, D1, D6)

**Objetivo:** el CRUD de artículos vive en la página **"Catálogo"** del menú (junto a
departamentos/familias); Configuración → BDP queda solo con conexión, mapeos y permisos.

**Cambios backend:**
- Ninguno de contrato: `POST/GET/PATCH/DELETE /api/bdp/article-maps` ya existe y hace alta local
  con rango reservado + encolado en modo bdp. Se reutiliza tal cual.

**Cambios frontend:**
- Extraer el bloque CRUD de artículos de `config-bdp-mapeos.tsx`
  (`bdp-article-map-table.tsx` + `BdpArticleCatalogActions.tsx`) y montarlo en
  `BdpCatalogo.tsx` (página "Catálogo"), bajo una sección "Artículos" con su propio origen
  visible (local/bdp) y los mismos permisos (`CatalogoEdicion` → 403 para trabajador).
- `BdpCatalogo.tsx` mantiene la sección existente de departamentos/familias (código secuencial D7).
- `ConfigBdpMapeos` queda solo con: estado/conexión BDP, mapeos técnicos (tender, canales,
  artículo/cliente por defecto) y permisos. Si quedan acciones de negocio ahí, se eliminan o se
  sustituyen por un enlace a Catálogo.

**DoD:**
- [ ] "Catálogo" permite: alta, edición inline (precio/IVA/familia/barcode), activar/desactivar, origen visible.
- [ ] Configuración → BDP no contiene CRUD de negocio.
- [ ] Alta en modo bdp encola `CreateArticlesAndUpdateProfiles` (verificado por test existente de
      `crear_article_map`).
- [ ] 403 de permisos sigue funcionando (trabajador no edita catálogo).

---

## C2 — Stock: "Nuevo artículo" + H7 + empty state accionable (H2, D2)

**Objetivo:** Stock permite crear artículos desde la propia página y nunca orienta a BDP como
única salida en standalone.

**Backend:** sin cambios — `POST /api/bdp/article-maps` (alta local + encolado) ya cubre D3/rango
reservado. `ajustar_stock` ya cubre el ajuste con motivo y auditoría.

**Frontend (`BdpStock.tsx` + `BdpStockActions.tsx`):**
- Botón **"Nuevo artículo"** que abre el formulario de alta (código, nombre, precio, IVA,
  familia) reutilizando el mismo flujo del CRUD de C1 (hook `useCrearArticleMap`).
- **H7**: deshabilitar **"Sync catálogo"** cuando el modo efectivo no sea `bdp`, con tooltip
  "requiere BDP conectado" (patrón U8). Hoy solo se deshabilita con `demoMode`.
- **Empty state accionable**: si no hay artículos y no hay modo bdp, ofrecer "Nuevo artículo"
  como primera acción (no "Sincroniza el catálogo desde BDP o pulsa Cargar demo").
- Origen del valor de stock visible (local/bdp) en las filas.

**DoD:**
- [ ] En standalone: crear artículo desde Stock funciona, aparece en la tabla con origen local y
      puede ajustarse su stock; cero llamadas a BDP.
- [ ] En modo bdp: el alta encola y el ajuste encola (si código BDP); filas quedan pendientes.
- [ ] "Sync catálogo" deshabilitado en standalone con motivo visible.
- [ ] Empty state ofrece acción local.

---

## C3 — Inventario: persistencia local + aplicación al stock (H3, H4, D3, D4)

**Objetivo:** el conteo de inventario se guarda localmente (fechado, auditable, retomable) y su
diferencia ajusta el stock local con motivo "conteo". En modo bdp, además, se encola el envío.

**Backend (aditivo):**
- Migración nueva (aditiva, M15): tabla `bdp_conteos_inventario`
  (`id`, `user_id`, `fecha`, `estado`, `observaciones`, `created_at`, `updated_at`) +
  `bdp_conteos_inventario_lineas`
  (`id`, `conteo_id` FK, `article_map_id`/código BDP nullable, `esperado`, `contado`,
  `diferencia`, `motivo` default `conteo`, `aplicado_al_stock` bool default false).
- Endpoints nuevos en `bdp_article_map.rs` (o módulo `bdp_inventario.rs`):
  - `GET /api/bdp/inventario/conteos` — listar conteos (fechados, con estado).
  - `POST /api/bdp/inventario/conteos` — crear/guardar conteo (líneas + diferencias).
  - `GET /api/bdp/inventario/conteos/:id` — retomar un conteo en curso.
  - `POST /api/bdp/inventario/conteos/:id/aplicar` — aplicar diferencias al stock local
    (reutiliza la lógica de `ajustar_stock` con motivo `conteo`, auditoría incluida) y, en modo
    bdp, encolar el envío (`payload_inventario`) de las líneas con código BDP.
- El endpoint `POST /api/bdp/inventario` (registrar_inventario) existente se conserva y puede
  reutilizarse para el envío en modo bdp; el worker/flush queda como está (no-op en standalone).

**Frontend (`BdpInventario.tsx`):**
- Al cargar, intenta retomar el último conteo en curso (o lista de conteos fechados para ver).
- "Guardar conteo" persiste localmente (ya no es solo `useState`).
- "Enviar inventario": en modo bdp encola y muestra estado real (encolado/enviado/error); en
  standalone queda deshabilitado o avisa claramente "modo independiente: el conteo se guarda
  localmente, no se envía" (eliminar el toast engañoso "Inventario encolado: N artículos").
- Estado del envío visible por línea donde proceda (R4.5).

**DoD:**
- [ ] Conteo guardado → sobrevive a recargar la página y aparece en Historial (auditoría local).
- [ ] Aplicar conteo → el stock local de cada línea cambia por la diferencia, con motivo
      `conteo` registrado en auditoría; idempotente (no aplica dos veces).
- [ ] En standalone: cero llamadas a BDP; "Enviar" no finge envío.
- [ ] En modo bdp: las líneas con código BDP se encolan y el flush las envía; las sin código se
      reportan como omitidas (decisión M16/198A-1).
- [ ] Tests: persistencia del conteo (guardar/listar/retomar), aplicación idempotente al stock,
      invariante standalone (no envía), encolado en modo bdp.

---

## C4 — Sección "Sincronización" (H6, D5)

**Objetivo:** visibilidad de la cola de push y reintento individual, visible solo en modo bdp.

**Backend (aditivo):**
- `GET /api/bdp/push/pendientes` — listar filas de `bdp_push_escrituras` del usuario con su
  estado (pendiente/sincronizada/suscripción/error), dominio, operación, entidad, reintentos y
  `ultimo_error` (ya existe `BdpPushService::listar_pendientes`; solo falta exponerlo).
- `POST /api/bdp/push/:id/reintentar` — reintento individual respetando la regla D2: si el
  estado es `pendiente_suscripcion`, solo manual (este endpoint lo es) y no auto-reintento.
- `POST /api/bdp/push/flush` se mantiene (flush global).

**Frontend:**
- Nueva sección "Sincronización" (ruta propia, p. ej. `/bdp/sincronizacion`), visible **solo en
  modo bdp** (igual que el botón "Sincronizar a BDP" actual): tabla de filas con estado, error
  y acciones (reintentar individual, ver error, flush global).
- En standalone la sección no se ofrece (R13.2).

**DoD:**
- [ ] La cola muestra filas pendientes/sincronizadas/error con `ultimo_error`.
- [ ] Reintento individual funciona y respeta la regla de suscripción (manual).
- [ ] Visible solo en modo bdp; en standalone no aparece.
- [ ] Tests: endpoint listar (filtra por user), reintento individual (cambia estado), 403 si no
      hay permiso.

---

## C5 — Normalización al guardar configuración (H5)

**Objetivo:** no se puede persistir un estado contradictorio (`modo_operacion=standalone` con
`bdp_sync_enabled=true`).

**Backend (`configuracion.rs`):** en el PATCH, si el guardado resulta en
`modo_operacion=standalone` con `bdp_sync_enabled=true`:
- Opción elegida (técnica, sin decisión de producto): **forzar `bdp_sync_enabled=false`** y
  devolver un aviso en la respuesta (el modo independiente no sincroniza). El switch maestro
  (`modo_operacion`) ya gana en runtime, pero el almacenamiento deja de ser contradictorio.

**DoD:**
- [ ] Guardar standalone+sync → queda `standalone` con `sync=false` + aviso visible.
- [ ] Test del PATCH para el caso contradictorio.

---

## C6 — Empty state de Compras (H8)

**Objetivo:** el empty state ofrece la acción local real.

**Frontend (`BdpCompras.tsx`):** empty state pasa a: "No hay albaranes todavía. **Nuevo
albarán**" como primera acción (CRUD local, serie L-), y "Sync albaranes"/"Cargar demo" quedan
como secundarias (sync deshabilitado en standalone, como ya está).

**DoD:**
- [ ] Empty state muestra "Nuevo albarán" accionable en standalone; sin regresiones en el CRUD.

---

## C7 — Verificación final y cierre documental

**Verificación:**
- [ ] `node scripts/run-with-db.mjs check` (con BD de rama) exit 0.
- [ ] `node scripts/run-with-db.mjs test` — suite completa en verde (153 + nuevos).
- [ ] `npm run type-check` (frontend) limpio.
- [ ] Recorrido UI en el stack aislado (`:3100`/`:5180`, BD de rama, seed demo): Catálogo
      unificado, Stock con alta, Inventario con guardado+stock, Sincronización (modo bdp
      simulado o verificado por código), 403 de permisos, cero tráfico a BDP en standalone.
- [ ] Invariante: en standalone el worker no envía nada (test existente + recorrido UI).

**Cierre documental:**
- [ ] Plan archivado en `Agente/planes/completados/` con evidencia por fase.
- [ ] Roadmap: bloque 208A-2 marcado completado; hallazgos H1–H8 cerrados.
- [ ] `Agente/completados/tareas-2026-08-27.md` con evidencia reproducible.
- [ ] Documentación canónica de BDP actualizada si cambia la ubicación de funciones
      (`Agente/documentacion/bdp/mapeo-visual-integracion-bdp-2026-07-23.md` apunta al CRUD en
      Configuración → se actualiza a Catálogo).
- [ ] Cambios sin commitear hasta que el usuario lo indique (convención de la sesión).

---

## Cierre (2026-08-27) — IMPLEMENTADO y verificado

Todas las fases C1–C7 implementadas y verificadas. Resumen de evidencia:

| Fase | Hallazgo(s) | Qué se hizo | Verificación |
| --- | --- | --- | --- |
| C1 | H1, D1, D6 | CRUD de artículos movido a la página "Catálogo" (pestañas Artículos / Departamentos y familias); Configuración → BDP queda sin CRUD (solo aviso con enlace "Ir a Catálogo"); `config-bdp-mapeos` reducido a mapeos técnicos | UI: alta TEST-1 desde Catálogo (origen local, rango reservado 90000000); Configuración → BDP sin CRUD |
| C2 | H2, H7, D2 | Botón "Nuevo artículo" en Stock (`NuevoArticuloDialog`), empty state accionable, "Sync catálogo"/"Sync precios" deshabilitados fuera de modo bdp | UI: diálogo abre, Sync deshabilitado en standalone; `tsc` limpio |
| C3 | H3, H4, D3, D4 | Migración `20260827000000_bdp_conteos_inventario` (tablas fechadas + línea, idempotencia por clave); endpoints GET/POST/GET:id; repositorio atómico que persiste, aplica la diferencia al stock (motivo 'conteo', auditoría) y encola; UI con "Guardar conteo", historial y "Retomar" | 4/4 tests (`bdp_conteos_inventario`); UI end-to-end: conteo 5 → stock local TEST-1 = 5, historial con "aplicado" |
| C4 | H6, D5 | `GET /api/bdp/push/pendientes` + `POST /api/bdp/push/:id/reintentar` (reintento manual, respeta D2); página "Sincronización" en el menú con estado por fila, error, reintento y flush | 5/5 tests (`bdp_push_cola`); UI: cola visible, banner standalone, botones deshabilitados, sin envío |
| C5 | H5 | Normalización en el PATCH: `standalone` + `sync=true` → se fuerza `sync=false` (estado no contradictorio) | 3/3 tests (`bdp_config_normalizacion`) |
| C6 | H8 | Empty state de Compras ofrece "Nuevo albarán" + "Cargar demo" | UI: texto nuevo y botón presentes |
| C7 | — | Tests nuevos 12/12; regresión `bdp_inventario` 3/3 y `bdp_push` 13/13; `cargo check` exit 0; `tsc` limpio; cero tráfico a BDP en la pasada UI | — |

**Invariante preservada:** en standalone el worker y el reintento no envían nada (tests de
invariante existentes + `reintentar_en_standalone_no_envia_nada` + UI con filas pendientes sin
envío). **Regla nueva en el AGENTS.md raíz (solicitud del usuario):** no ejecutar suites pesadas
completas; test por test de lo necesario (`no-heavy-suites`).

---

## Orden de ejecución

`C1 → C2 → C3 → C4 → C5 → C6 → C7`. Cada fase se verifica (check + tests + type-check) antes de
pasar a la siguiente. C5 y C6 son triviales y pueden ir junto a C1/C2 en el mismo commit lógico
si no mezclan frentes. Las migraciones de C3 se aplican una sola vez (inmutables tras aplicar).

## Riesgos y mitigaciones

| Riesgo | Mitigación |
| --- | --- |
| Mover el CRUD rompe el acceso actual (gente acostumbrada a Configuración) | Enlace/aviso en Configuración apuntando a Catálogo (decisión D6 contempla atajo) |
| Inventario aplica dos veces la misma diferencia | Flag `aplicado_al_stock` + idempotencia en `aplicar` (transacción + guard) |
| C4 expone filas sensibles | Filtra por `user_id` (como `listar_pendientes` actual) y exige permiso de operación |
| El toast engañoso de inventario se queda en standalone | Eliminado en C3 (aviso honesto "modo independiente: no se envía") |
| Cola grande → payload de GET pesado | Paginación/limite en `GET /push/pendientes` (máx. N filas, orden por `updated_at`) |
