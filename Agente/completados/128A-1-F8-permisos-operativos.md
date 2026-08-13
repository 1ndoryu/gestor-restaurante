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
    default `'admin'`), `ConfigBdp.tsx` con sección «Permisos operativos» (4 selects, tras
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
    `tests/bdp_f8_permisos.rs` **13/13** (403 por permiso con trabajador y default `admin`,
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
