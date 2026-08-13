# Tareas completadas — F6 (bloque 128A-1) — Auditoría local + pagos parciales + factura local

## F6 — Historial/auditoría local (A11), pagos parciales locales (A8/M13) y factura local mínima (A7/D9)

- **Qué:** operaciones locales con auditoría visible en Historial sin BDP, pagos parciales locales sobre
  el ledger `bdp_pagos` y factura local mínima con numeración propia por usuario.
  - Backend — migración `20260817000000_bdp_audit_origen_local`: `bdp_audit_log.origen_operacion`
    (`VARCHAR(10) NOT NULL DEFAULT 'bdp'` con `CHECK (origen_operacion IN ('local','bdp'))` e índice
    `idx_bdp_audit_user_origen`), `ventas.facturada_local BOOL DEFAULT false`, `ventas.factura_numero`,
    `ventas.factura_fecha` y UNIQUE parcial `uq_ventas_user_factura_numero (user_id, factura_numero)
    WHERE NOT NULL`. El default `'bdp'` no altera las entradas audit existentes.
  - Modelo: `Venta`/`VentaConCliente` con `facturada_local`, `factura_numero`, `factura_fecha`;
    `BdpAuditEntry.origen_operacion: String` (SELECT de `listar_audit`).
  - Auditoría local (`origen_operacion='local'`) en: `venta.rs::anular`, `bdp_article_map.rs::ajustar_stock`,
    `pago_parcial_local` y `factura_local`. `BdpBackupService` re-exportado en `glory_backend::services`.
  - Repositorio `VentaRepository::facturar_local` (`src/repositories/venta.rs:665`): transacción con
    `SELECT ... FOR UPDATE`; guards (anulada → `venta_anulada_no_facturable`, ya facturada local/BDP →
    `venta_ya_facturada`, ledger con filas y `total-pagado > 0.001` → `venta_con_pagos_pendientes`);
    numeración `F-{año}-{n:04}` por `COUNT`+1 del usuario; audit `factura_local` con ON CONFLICT;
    UPDATE `facturada_local=true`. Retorna `(Venta, Uuid, Option<String>, bool)`.
  - Repositorio `BdpPagoRepository::insertar_local` (`src/repositories/bdp_pago.rs`): transacción con
    lock, guards anulada/facturada → Conflict y pendiente → Validation, INSERT
    `ON CONFLICT (idempotency_key) DO NOTHING`, duplicado → fila previa + `audit_id=None`, auditoría
    `pago_parcial_local` origen `local`.
  - **Bug corregido (claves vacías):** `None`/`""` como `idempotency_key` colisionaba (ledger exige
    `NOT NULL UNIQUE`; audit usa `(user_id, idempotency_key)`). `insertar_local` normaliza `""` →
    `local-{venta_id}-{uuid}` y `facturar_local` normaliza `None`/`""` → `factura-local-{id}-{uuid}`.
    Dos pagos sin clave ya no se colapsan; facturar dos ventas sin clave ya no devuelve éxito falso.
  - Servicios: `pago_parcial_local` (mismo venta+importe repetido → éxito idempotente; distinta
    venta/importe → Conflict), `facturar_local` (retry x3 ante 23505 por carrera de numeración),
    mapeo `Protocol` → Conflict/Validation y `RowNotFound` → NotFound (nunca 500).
  - Handlers: `POST /api/ventas/:id/pagos-locales` y `POST /api/ventas/:id/factura-local` con
    confirmaciones `PAGO LOCAL {id} {amount:.2}` y `FACTURA LOCAL {id}`; guard en `bdp_invoice` que
    rechaza ventas con `facturada_local`; OpenAPI registrado (incluye `anular_venta` + `AnularVentaResponse`
    que F4 había omitido).
  - Frontend: `BdpHistorial.tsx` con badge de origen (Local sky / BDP secondary), filtro
    todos/local/bdp, columna Origen y detalle con origen; `venta-row-actions.tsx` con acciones
    `pagoLocal`/`facturaLocal`, `puedePagoLocal`/`puedeFacturaLocal` (`!anulada && !facturada &&
    !puedePagar`), diálogo de pago local (balance del `GET /bdp-payments`, `PAGO LOCAL {id} {amount}`)
    y de factura local (bloqueada si `pagos.length>0 && pendiente>0.01`); handler `ejecutarLocal` con
    `crypto.randomUUID()`; badge violeta «Facturada {nº}» en `venta-table-body.tsx`.
- **Archivos:** `migrations/20260817000000_bdp_audit_origen_local.{up,down}.sql`,
  `src/models/venta.rs`, `src/services/{bdp_backup,venta,bdp_sync}.rs`,
  `src/repositories/{venta,bdp_pago,bdp_article_map}.rs`, `src/handlers/{ventas,mod}.rs`,
  `tests/bdp_f6_local_pagos_factura.rs`, `tests/bdp_service_integration.rs`,
  `frontend/src/api/{bdp-backup,bdp}.ts`, `frontend/src/componentes/bdp/{BdpHistorial,bdp-mocks}.tsx/ts`,
  `frontend/src/components/{venta-row-actions,venta-table-body}.tsx`.
- **Comandos y resultados:**
  - `node scripts/run-with-db.mjs test --test bdp_f6_local_pagos_factura` → **11/11 PASS**.
  - `node scripts/run-with-db.mjs test --test bdp_pagos --test bdp_backup --test bdp_service_integration
    --test bdp_venta_lineas` → 7+27+8(+3 ignored)+9 PASS.
  - `node scripts/run-with-db.mjs check` → PASS; `node scripts/run-with-db.mjs clippy` → PASS, 0 warnings
    (corregidos `map_unwrap_or`, `uninlined_format_args`, `needless_continue`, `too_many_lines` x2 vía
    `#[allow(clippy::too_many_lines)]` en `insertar_local` y `facturar_local`); `run-cargo.mjs fmt` aplicado.
  - `npm --prefix frontend run type-check` → PASS.
  - `npm run task:check -- 128A-1 --full --allow-heavy --heavy-reason "F6: auditoria local + pagos
    parciales + factura local"` → **PASS** (sentinel, varsense, rust incl. clippy `-D warnings` y tests,
    frontend type-check, docs).
- **Gotchas:** consultas F6 dinámicas (`sqlx::query`/`query_as` sin macro) porque el cache offline
  `.sqlx/` no tiene las columnas F6; no tocar `frontend/src/api/generated/*` (Orval). `customInstance`
  devuelve `{ data, status }` → los fetchers extraen `.data`. `bdp_pagos.idempotency_key` es
  `NOT NULL UNIQUE`; audit usa conflict target `(user_id, idempotency_key)`. `tender_id` es INT NOT NULL
  en `bdp_pagos`; la UI pide el número. `bdp_sync.rs:1526` gatea `ff_bdp_partial_payments` solo en flujo
  BDP (M12 OK). **Nota de calidad:** la pasada de `supervisor_reviewer` no pudo delegarse en este entorno
  (no hay tools de subagente); se documenta aquí como pendiente de cierre.
- **Sentinel:** el gate corrió la etapa sentinel (PASS, 0 errores).
- **GLORY:** no aplica; cambios del bloque 128A-1 en rama `glory-rs-rest`.
