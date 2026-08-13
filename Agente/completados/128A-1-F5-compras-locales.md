# Tareas completadas — F5 (bloque 128A-1) — Compras locales (albaranes)

## F5 — Compras locales: CRUD albaranes + conciliación local (M18), flags solo bdp (M12)

- **Qué:** albaranes de compra locales sobre `bdp_purchase_notes` (`origen='local'`) con ciclo de vida
  completo sin BDP y conviviendo con los importados (`origen='bdp'`).
  - Backend: migración `20260816000000_bdp_purchase_notes_local` (columna `origen` con
    `CHECK (origen IN ('local','bdp'))` y default `'bdp'` — no altera importados existentes — más
    índice `idx_bdp_purchase_notes_user_origen`).
  - Modelo: `BdpPurchaseNote.origen: String`; nuevos `BdpPurchaseNoteLineaLocal` (IVA por línea A10),
    `CrearBdpPurchaseNoteRequest` (serie opcional, `L` si se omite — M18), `ActualizarBdpPurchaseNoteRequest`.
  - Repositorio: `crear_local` (serie `L`, número secuencial por usuario vía `COUNT(*)` con
    `origen='local'`, estado `pendiente`, total/líneas calculados con desglose base+IVA), `actualizar_local`
    (COALESCE por campo, recalcula `datos_bdp` si llegan líneas), `eliminar_local` (solo
    `pendiente`/`borrador`; conciliados no se borran D5), `find_by_id` con `origen` explícito.
  - Handlers: `POST/GET /api/bdp/purchase-notes` y `PUT/DELETE /api/bdp/purchase-notes/{id}`;
    validaciones (proveedor nombre o código; total o líneas); 404/400 por origen no local;
    **M12**: los gates de flags `ff_bdp_purchase_notes_*` solo aplican en modo efectivo `bdp`
    (`modo_efectivo_desde_config`), en `standalone` el CRUD local funciona sin flags; sync con BDP
    rechazado en `standalone`. **A10**: conciliación local usa el desglose por línea
    (`desglose_desde_datos`) en vez de IVA=0.
  - Frontend: `api/bdp.ts` con `origen` en `BdpPurchaseNote`, request/linea types y funciones+hooks
    `crearBdpPurchaseNote`/`actualizarBdpPurchaseNote`/`eliminarBdpPurchaseNote` (invalida
    `['bdp-purchase-notes']`, extrae `.data` de `customInstance`); `BdpComprasLocalModal` (crear/editar:
    serie, número, fecha, proveedor, total, líneas con IVA por línea); `BdpCompras` con badge de
    origen (`local`/`bdp`), botón «Nuevo albarán», editar/eliminar solo origen local y
    `purchaseFeatureEnabled` según modo efectivo (standalone → cargar sin flag; bdp → requiere
    `ff_bdp_purchase_notes_read`). Mocks con `origen: 'bdp'`.
- **Archivos:** `migrations/20260816000000_bdp_purchase_notes_local.{up,down}.sql`,
  `src/models/{bdp_purchase_note,mod}.rs`, `src/repositories/bdp_purchase_note.rs`,
  `src/handlers/{bdp_purchase_note,mod}.rs`, `tests/bdp_purchase_notes_lifecycle.rs`,
  `frontend/src/api/bdp.ts`, `frontend/src/componentes/bdp/{BdpCompras,BdpComprasLocalModal,bdp-mocks}.tsx/ts`.
- **Comandos y resultados:**
  - `node scripts/run-with-db.mjs test --test bdp_purchase_notes_lifecycle` → **18/18 PASS**.
  - `npm run task:check -- 128A-1 --profile rust --allow-heavy ...` → **PASS** (sentinel, fmt, check,
    clippy, rust-test).
  - `npm run task:check -- 128A-1 --full --allow-heavy --heavy-reason "F5: compras locales completo"`
    → **PASS** (sentinel, varsense, rust, frontend type-check, docs) — reporte
    `.quality-reports/branches/glory-rs-rest--f100af0a041e6e8a/128A-1/latest.md`.
- **Gotchas:** bug real corregido en `actualizar_local`: los placeholders SQL estaban desfasados
  (`$4`-`$9` en el UPDATE vs binds `$3`-`$8`), mezclando `fecha` (date) con `numero` (text) en el
  primer COALESCE (error `42804`); se alinearon los placeholders a los binds. Consultas F5 dinámicas
  (`sqlx::query`/`query_as` sin macro) porque el cache offline `.sqlx/` no tiene las columnas F5.
  `customInstance` devuelve `{ data, status }` → los fetchers extraen `.data`.
- **Sentinel:** el gate corrió la etapa sentinel (PASS, 0 errores; warnings preexistentes no bloquean).
- **GLORY:** no aplica; cambios del bloque 128A-1 en rama `glory-rs-rest`.
