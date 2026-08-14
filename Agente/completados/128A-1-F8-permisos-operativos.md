# Tareas completadas — F8 (bloque 128A-1) — Permisos operativos configurables

## F8 — Permisos operativos por acción (D8, §4.11, M17) — enforcement backend

- **Qué:** 4 permisos **por acción** configurables en Configuración con enforcement en
  **backend** (M17: la UI solo refleja; el backend es la fuente de verdad):
  `permisos_catalogo_edicion`, `permisos_stock_ajuste`, `permisos_albaranes_gestion`,
  `permisos_anulacion_ventas`. Cada uno admite `admin` (default) | `admin_trabajador` | `todos`.
  - Backend — migración `20260819000000_bdp_permisos_operativos`: 4 columnas
    `VARCHAR(20) NOT NULL DEFAULT 'admin'` con CHECK `IN ('admin','admin_trabajador','todos')`
    en `configuracion_restaurante` (aditiva M15: filas existentes quedan en `admin` sin tocar
    datos previos; la API también valida en el handler).
  - Modelo `src/models/configuracion.rs`: 4 campos `permisos_*: String` en
    `ConfiguracionRestaurante` (tras `anulacion_modalidad`) + 4 `Option<String>` con
    `#[validate(length(max = 20))]` en `ActualizarConfiguracionRequest`.
  - Repositorio `src/repositories/configuracion.rs`: `UPDATE_CONFIG_SQL` extendido con
    `COALESCE($51..$54)` + 4 binds para el PATCH parcial.
  - Servicio nuevo `src/services/permisos.rs` (registrado en `mod.rs`):
    `AccionPermiso` (CatalogoEdicion/StockAjuste/AlbaranesGestion/AnulacionVentas con
    `columna()`/`valor()`), `NivelPermiso` (`VALORES: [&str; 3]`, `desde_valor` **fail-closed** a
    Admin ante valor desconocido, `permite(role)`), `permiso_habilitado(config, accion, user)`
    basado en `user.effective_role` (consistente con `AuthUser::require_role`) y
    `verificar_permiso(pool, accion, user) -> Result<(), AppError>` (403 Forbidden).
  - Validación `src/services/configuracion.rs`: valores `permisos_*` ∈ `NivelPermiso::VALORES`
    → si no, `AppError::Validation` (defensa en profundidad junto al CHECK de la BD).
  - Enforcement en handlers: `bdp_article_map.rs` (`ajustar_stock` → StockAjuste;
    `crear/actualizar/eliminar_article_map` → CatalogoEdicion), `bdp_purchase_note.rs`
    (5 handlers → AlbaranesGestion), `ventas.rs` `anular_venta` → AnulacionVentas.
  - Exports de tests `src/handlers/mod.rs`: `pub use` de `actualizar_article_map`,
    `ajustar_stock`, `crear_article_map`, `eliminar_article_map` y `anular_venta` (los módulos
    son privados; patrón de `bdp_menu_local`).
  - Frontend: `configuracion-types.ts` (4 campos + defaults `'admin'`),
    `useConfiguracion.ts` (body PATCH), `useConfiguracionSync.ts` (sync servidor→local con
    default `'admin'`), `ConfigBdp.tsx` con sección «Permisos operativos» (6 selects, tras
    modalidad de anulación) y `gestionRestauranteAPI.schemas.ts` (campos en
    `ActualizarConfiguracionRequest` y `ConfiguracionRestaurante`, orden alfabético).
- **Alcance y decisión (auditoría F8, §4.11):** de los endpoints enumerados se protegen los
  sensibles **sin BDP** del bloque (article-maps CRUD, ajuste stock, purchase-notes, anulación).
  Se documenta la decisión de **no gatear** sync-prices, sync-tables, bdp-payment, bdp-invoice,
  customers/import, clientes/:id/bdp-sync ni bdp-poll: son acciones de sincronización/escritura
  **BDP** ya protegidas por guards existentes (`bdp_sync_enabled`, modo `bdp`, feature flags y
  `BdpWriteGuard`) y no son operaciones locales del staff; gatearlas añadiría superficie sin
  necesidad.
- **Archivos:** `migrations/20260819000000_bdp_permisos_operativos.{up,down}.sql`,
  `src/models/configuracion.rs`, `src/repositories/configuracion.rs`, `src/services/permisos.rs`,
  `src/services/mod.rs`, `src/services/configuracion.rs`, `src/handlers/bdp_article_map.rs`,
  `src/handlers/bdp_purchase_note.rs`, `src/handlers/ventas.rs`, `src/handlers/mod.rs`,
  `tests/bdp_f8_permisos.rs`, `frontend/src/hooks/configuracion-types.ts`,
  `frontend/src/hooks/useConfiguracion.ts`, `frontend/src/hooks/useConfiguracionSync.ts`,
  `frontend/src/componentes/ConfigBdp.tsx`,
  `frontend/src/api/generated/gestionRestauranteAPI.schemas.ts`. Literales `permisos_*`
  actualizados en `src/services/{modo_operacion,bdp_weblink,haddock,bdp_backup,
  bdp_sync_preflight}.rs` y `tests/{haddock_db,bdp_service_integration,bdp_simulator_integration,
  bdp_readonly}.rs`.
- **Comandos y resultados:**
  - `node scripts/run-with-db.mjs check` → PASS; `node scripts/run-with-db.mjs clippy` → PASS
    (`-D warnings`, con `#[must_use]` en helpers públicos); `npm run fmt` OK.
  - Suite completa `node scripts/run-with-db.mjs test` → **PASS (exit 0)**; en particular
    `tests/bdp_f8_permisos.rs` **24/24** (403 por permiso con trabajador y default `admin`,
    admin sin 403, ampliación a `todos`/`admin_trabajador` que habilita al trabajador, PATCH
    inválido → Validation, persistencia de permisos vía PATCH).
  - Frontend `npm run type-check` → PASS (tras ajustar `useConfiguracionSync.ts`).
  - `npm run task:check -- 128A-1 --full --allow-heavy --heavy-reason "F8 permisos operativos
    configurables"` → **PASS** (sentinel, varsense, rust, frontend type-check, docs).
    Primer intento bloqueado por cuota de targets (15 GiB, target en escritura reciente):
    se liberó borrando 28 PDBs (3.94 GiB) del target `C:\tmp\glory-target\
    glory_backend_glory_rs_rest` y el gate pasó a la segunda ejecución.
- **Gotchas:** la cuota del gate (`maxTargetGb: 15` en `quality.config.json`) se evalúa en cada
  gate; un target en escritura reciente (< 30 min) se protege como activo, así que con un solo
  target grande el gate falla hasta reducir tamaño (borrar `.pdb` del `debug/` no afecta la
  recompilación incremental). `desde_valor` es **fail-closed** (desconocido → Admin). Los
  handlers protegidos usan `verificar_permiso(pool, accion, user)` al inicio; `effective_role`
  evita discrepancias con `require_role` del middleware.
- **Sentinel:** el gate corrió la etapa sentinel (PASS, 0 errores).
- **GLORY:** no aplica; cambios del bloque 128A-1 en rama `glory-rs-rest`.

## Correcciones de la 2a revisión (F8-1..F8-4)

- **F8-1 [MEDIA] pagos y facturación local sin permiso:** 2 variantes nuevas
  `AccionPermiso::PagosLocales`/`FacturacionLocal` (`src/services/permisos.rs` con
  `columna()`/`valor()`) + migración aditiva `20260820000000_bdp_permisos_operativos_locales`
  (columnas `permisos_pagos_locales`/`permisos_facturacion_local`, `VARCHAR(20) NOT NULL
  DEFAULT 'admin'` con CHECK, M15: no toca filas existentes). Guards en `src/handlers/ventas.rs`
  al inicio de `pago_parcial_local` y `factura_local` (antes de validaciones). Modelo y repo:
  2 campos en `ConfiguracionRestaurante`/`ActualizarConfiguracionRequest`
  (`#[validate(length(max = 20))]`), `UPDATE_CONFIG_SQL` `COALESCE($55..$56)` + binds, y
  validación en `src/services/configuracion.rs`. Frontend: 2 selects nuevos
  (`permisos-pagos-locales`, `permisos-facturacion-local`) en `ConfigBdp.tsx` + campos/defaults
  en `configuracion-types.ts`; como el esquema Orval generado aún no los trae, se extendió el
  tipo local `CuerpoConfiguracionLocal` (sin tocar `frontend/src/api/generated/*`).
- **F8-2 [BAJA] `eliminar_venta` sin permiso:** decisión de **reusar
  `AccionPermiso::AnulacionVentas`** (misma clase de escritura destructiva, default `admin`);
  documentado en comentario del handler. Evita un permiso nuevo con semántica idéntica.
- **F8-3 [BAJA] tests 403 por endpoint:** añadidos 403 para
  `actualizar/eliminar/marcar_borrador/conciliar_purchase_note` y `eliminar_venta`
  (albaranes ya tenían `crear`). `tests/bdp_f8_permisos.rs` pasa de 14 a **24 tests**, incluidos
  admin sin 403 (delete/anulación de venta inexistente) y ampliación a `todos` para pagos y
  factura local.
- **F8-4 [BAJA] `verificar_permiso` escribía en lectura:** nuevo
  `ConfiguracionRepository::obtener` (SELECT puro, devuelve `Option`) y `verificar_permiso` lo
  usa directamente; si no hay fila de configuración falla cerrado a `NivelPermiso::Admin` **sin
  crear fila**. Test: `verificar_permiso_sin_config_no_crea_fila_y_falla_cerrado`.
  `obtener_o_crear` sigue disponible para quienes sí necesitan crear (p. ej.
  `sincronizar_purchase_notes`), comportamiento intacto.
- **Gate y evidencia:** `test --lib` 149/149; no-regresión F6 (6) + F6 local pagos/factura (11)
  + F4 anulación/delete (5); `tests/bdp_f8_permisos.rs` **24/24**; `fmt`, `clippy --all-targets
  -- -D warnings` y `npm --prefix frontend run type-check` → PASS. Inicializadores de test
  actualizados en `src/services/{haddock,bdp_backup,bdp_sync_preflight,bdp_weblink,
  modo_operacion}.rs` y `tests/{haddock_db,bdp_service_integration,bdp_simulator_integration,
  bdp_readonly}.rs` con los 2 campos nuevos.
