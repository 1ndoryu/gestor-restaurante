# Tareas completadas — F7 (bloque 128A-1) — Menús/packs locales

## F7 — Menús/packs locales (D2, §4.10, A12/M12) sobre catálogo local

- **Qué:** CRUD completo de menús/packs **locales** (standalone, sin depender del Explorador BDP)
  con líneas sobre el catálogo local (`bdp_article_map`), conviviendo con el Explorador BDP.
  M12: sin gates de flags — la sección local está siempre disponible en modo efectivo standalone.
  El Explorador BDP se conserva; se añade sección «Menús y packs locales» con badge de origen `Local`.
  - Backend — migración `20260818000000_bdp_menu_local`: `bdp_menus_locales` (id, user_id, tipo
    CHECK `menu|pack`, nombre, descripcion, precio NUMERIC(12,2), activo, created_at, updated_at,
    UNIQUE `(user_id, tipo, nombre)`, índice `idx_bdp_menus_locales_user_tipo`) +
    `bdp_menu_local_lineas` (id, menu_id FK CASCADE, articulo_codigo, descripcion, cantidad
    NUMERIC(12,3), precio_unitario, **orden** INT para orden determinista, created_at, índice
    `idx_bdp_menu_local_lineas_menu`).
  - Modelo `src/models/bdp_menu_local.rs`: enum `BdpMenuLocalTipo` (sqlx::Type VARCHAR, `as_str`,
    `From<String>`), `BdpMenuLocal` (FromRow), `BdpMenuLocalLinea` (incluye `orden: i32`),
    `BdpMenuLocalConLineas` (campos explícitos, sin `#[serde(flatten)]` para utoipa) y requests
    crear/actualizar/lista + `BdpMenuLocalLineaRequest`.
  - Repositorio `src/repositories/bdp_menu_local.rs`: `listar` (query dinámica con `PgArguments` +
    líneas con `ANY($1)`), `find_by_id` (recibe `&PgPool`), `crear` (tx), `actualizar` (tx, COALESCE;
    si llegan líneas las reemplaza y **recalcula precio** si no viene explícito), `eliminar` (CASCADE).
    Helper público `sumar_lineas`. Consultas dinámicas (sin macro: el cache offline `.sqlx/` no
    tiene las columnas F7).
  - Handlers `src/handlers/bdp_menu_local.rs`: `GET/POST /bdp/menus-locales` y
    `GET/PUT/DELETE /bdp/menus-locales/:id` con `#[utoipa::path]`, validaciones (tipo, nombre,
    ≥1 línea, cantidades/precios) y `map_error_unique` → `AppError::Conflict` (23505).
  - Frontend: tipos + fetchers + hooks (`useBdpMenusLocales`, `useBdpMenuLocal`,
    `useCrearBdpMenuLocal`, `useActualizarBdpMenuLocal`, `useEliminarBdpMenuLocal`) en
    `frontend/src/api/bdp.ts` (invalida `['bdp-menus-locales']`, extrae `.data`);
    `BdpMenuLocalModal.tsx` (crear/editar con tipo, nombre, descripción, precio, activo y líneas con
    `Select` de artículos de catálogo vía `useBdpArticleMaps`, clave `articulo_glory_codigo`,
    descripción autocompletada con `articulo_bdp_nombre`); `BdpExplorador.tsx` con sección local
    (tabla Nombre/Tipo/Precio/Artículos/Estado/Origen Local, botón «Nuevo menú/pack», editar/eliminar
    con confirmación y badges). No rompe la consulta BDP/demo existente.
- **Archivos:** `migrations/20260818000000_bdp_menu_local.{up,down}.sql`,
  `src/models/bdp_menu_local.rs`, `src/models/mod.rs`, `src/repositories/bdp_menu_local.rs`,
  `src/repositories/mod.rs`, `src/handlers/bdp_menu_local.rs`, `src/handlers/mod.rs`,
  `tests/bdp_f7_menus_locales.rs`, `frontend/src/api/bdp.ts`,
  `frontend/src/componentes/bdp/{BdpMenuLocalModal,BdpExplorador}.tsx`.
- **Comandos y resultados:**
  - `node scripts/run-with-db.mjs test --test bdp_f7_menus_locales` → **15/15 PASS** (crear con
    líneas y precio calculado, filtros tipo/activo/búsqueda, actualizar COALESCE + reemplazo de
    líneas + recálculo, eliminar CASCADE, aislamiento por usuario, UNIQUE 23505, handlers
    standalone sin flags, nombre vacío, sin líneas, tipo inválido, 404, conflicto duplicado, listar).
  - `node scripts/run-with-db.mjs check` → PASS; `node scripts/run-with-db.mjs clippy` → PASS
    (corregidos `explicit_auto_deref` en llamadas a `insertar_lineas` — el parámetro es
    `&mut PgConnection`, así que `&mut tx` es válido — y `explicit_counter_loop` del contador de
    orden vía `(0_i32..).zip(lineas.iter())`); `run-cargo.mjs fmt` aplicado.
  - `npm run task:check -- 128A-1 --full --allow-heavy --heavy-reason "F7 menus/packs locales"` →
    **PASS** (sentinel, varsense, rust incl. fmt, check, clippy `-D warnings` y tests, frontend
    type-check, docs). Primer intento: etapa rust ERROR por `rust-fmt` (diff) y timeout de la
    compilación en frío de los tests de integración; resuelto con `npm run fmt` y calentando
    artefactos (`check`/`clippy`/`test` incrementales antes del gate).
- **Gotchas:** consultas F7 dinámicas (`sqlx::query`/`query_as` sin macro) porque el cache offline
  `.sqlx/` no tiene las columnas F7; `Transaction` no implementa `Executor` por sí solo en sqlx 0.8 —
  en helpers que reciben `&mut Transaction` hay que pasar `&mut **tx`, y en métodos con parámetro
  concreto (`&mut PgConnection`) basta `&mut tx` (clippy `explicit_auto_deref`); orden determinista
  de líneas vía columna `orden` (evita `explicit_counter_loop` con `(0_i32..).zip(...)`).
  `customInstance` devuelve `{ data }` → los fetchers extraen `.data`. **Nota de calidad:** la
  pasada de `supervisor_reviewer` no pudo delegarse en este entorno (no hay tools de subagente);
  se documenta aquí como pendiente de cierre.
- **Sentinel:** el gate corrió la etapa sentinel (PASS, 0 errores).
- **GLORY:** no aplica; cambios del bloque 128A-1 en rama `glory-rs-rest`.

## Correcciones de la 2a revisión (F7-1..F7-4, commit `[128A-1] F7 correcciones`)

- **F7-1 (MEDIA)** `BdpMenuLocalTipo` deja `From<String>`/`From<&str>` (default silencioso a
  `Menu`) y pasa a `TryFrom<String>`/`TryFrom<&str>` con `Error = &'static str`. `crear`/
  `actualizar` convierten con `.try_into()` → `sqlx::Error::Protocol("tipo_invalido")` →
  `AppError::Validation("El tipo debe ser 'menu' o 'pack'")`. `listar_menus_locales` valida
  `params.tipo` con `validar_tipo` (400) antes de consultar; el filtro ya no deja pasar tipos
  arbitrarios ni usa un default silencioso. `map_error_unique` renombrado a `map_repo_error`
  (mapea `tipo_invalido` y `articulo_no_en_catalogo:<códigos>` → Validation con mensaje
  accionable). Tests: `handler_listar_filtro_tipo_invalido_rechaza` (handler) +
  `tipo_desconocido_falla_al_convertir` (unit) + `tipo_as_str_mapea_valores` con
  `.try_into().unwrap()`.
- **F7-2 (BAJA)** `validar_articulos_en_catalogo(pool, user_id, lineas)` en el repo: cada código
  no vacío debe existir en `bdp_article_map.articulo_glory_codigo` del usuario (`= ANY($1)` con
  `Vec<String>`); si falta → `Protocol("articulo_no_en_catalogo:<códigos>")` → Validation con
  mensaje accionable. Se llama en `crear` y en `actualizar` (solo cuando llega `lineas`); el
  contrato queda documentado en `BdpMenuLocalLineaRequest.articulo_codigo`. Test:
  `crear_menu_con_articulo_fuera_del_catalogo_rechazado`.
- **F7-3 (BAJA)** `auditar(conn, user_id, operacion, menu_id, payload)` inserta en `bdp_audit_log`
  (`direccion='internal'`, `resultado='exito'`, `origen_operacion='local'`,
  `target_entity_type='menu_local'`, `target_entity_id`, `authorization_reason`, sin
  `idempotency_key`) y se invoca dentro de la transacción en `menu_local_crear`,
  `menu_local_actualizar` (payload con `tipo_audit: Option<&str>` calculado antes del move de
  `tipo`) y `menu_local_eliminar` (firma cambiada a `eliminar(pool: &PgPool, id, user_id)` con tx
  interna). Test: `crud_menus_registra_auditoria_local` (crear/actualizar/eliminar → 3 filas de
  auditoría `local` con `target_entity_type='menu_local'`).
- **F7-4 (BAJA)** `listar_menus` escapa wildcards del término:
  `termino.trim().replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")` + `ESCAPE '\'` en
  ambos ILIKE: buscar `100%` o `Combo_` ya es literal. Test:
  `busqueda_escapa_wildcards_iliike`.
- **Tests:** helper nuevo `crear_articulo_catalogo(pool, user_id)` (inserta `bdp_article_map` con
  `articulo_glory_codigo='ART-001'`); añadido a los 6 tests de repo y 4 de handler que llegan al
  repo (los que fallan antes del repo — nombre vacío, sin líneas, tipo inválido — no lo necesitan).
  `tests/bdp_f7_menus_locales.rs` pasa de 15 a 19 tests.
- **Gate:** `check` OK; `fmt` OK; clippy `--all-targets -- -D warnings` OK (corregidos 3
  `explicit_auto_deref` → `&mut tx` en los call sites de `auditar`); `test --lib` 149/149;
  integración `bdp_f7_menus_locales` 19/19 + `bdp_f6_correcciones` 6/6.
- **Archivos:** `src/models/bdp_menu_local.rs`, `src/repositories/bdp_menu_local.rs`,
  `src/handlers/bdp_menu_local.rs`, `tests/bdp_f7_menus_locales.rs`, checklist F7 `[x]`.
