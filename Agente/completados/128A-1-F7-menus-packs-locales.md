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
