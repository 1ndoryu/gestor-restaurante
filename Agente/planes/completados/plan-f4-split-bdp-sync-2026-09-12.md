/* Plan F4 267A-9 — split por archivo de `src/services/bdp_sync.rs`.
 * Por qué este plan: F3 eliminó las 4 funciones >200 (sync_venta, add_order_payment,
 * invoice_order, sincronizar_cliente_bdp) pero el archivo (3605 líneas, 2145 efectivas,
 * 3.06x del límite 700) triplicó el umbral y disparó `limite-lineas-nivel-3` (error).
 * Ningún split funcional evita ese error: solo el split por archivo lo resuelve.
 * Regla: WIP ajeno no se toca; `seed.rs` y planes ajenos quedan fuera.
 */

# F4 — Split `bdp_sync.rs` en módulos por fase (267A-9)

## Objetivo

Bajar `bdp_sync.rs` por debajo de 700 líneas efectivas y eliminar el error
`limite-lineas-nivel-3`, sin cambiar ni un mensaje, SQL, guarda ni orden de
operaciones. Los puntos de entrada públicos conservan nombre y firma.

## Alcance

- Dentro: `src/services/bdp_sync.rs` (3605 líneas) → hub + 4 submódulos.
- Dentro: `src/services/mod.rs` (declaraciones `mod` + `pub use`).
- Fuera: `bdp_weblink_catalog.rs` (1708 líneas, solo warning nivel-1, no error);
  se registra como deuda aparte, no se toca en este plan.
- Fuera: `seed.rs`, planes ajenos, cualquier WIP ajeno.

## Diseño (módulos planos, estilo del repo)

| Archivo | Contenido | ~líneas |
|---|---|---|
| `bdp_sync.rs` (hub) | `BdpSyncService`, `SyncTablesResult`, `SYNC_LOCKS` (pub(crate)), `SyncLockGuard` + `Drop` (pub(crate)), `conectar_bdp` (pub(crate)), `parse_remote_money` (pub(crate)), `BDP_SYNC_MARKET_ID`, `mod tests` si no depende de venta | <700 ef. |
| `bdp_sync_venta.rs` | `impl BdpSyncService`: `sync_venta`, `pasar_guardias_sync_venta`, `armar_orden`, `cerrar_sync_venta`, `confirmar_orden`, `anotar_fracaso`, `marcar_error_venta`, `retry_send_order`, `send_order`, `build_order`, `resolve_*` (5), `run_http_phase`, `ensure_cliente_bdp_synced` + `OrderContext`, `ResolvedArticle` | ~1300 |
| `bdp_sync_pago.rs` | `impl BdpSyncService`: `add_order_payment` + 5 helpers + `IntentoPago`, `PagoArmado` | ~450 |
| `bdp_sync_factura.rs` | `impl BdpSyncService`: `invoice_order` + 7 helpers | ~450 |
| `bdp_sync_catalogo.rs` | `impl BdpSyncService`: `sync_catalog`, `sync_prices`, `sync_tables`, `aplicar_upsert` + `BdpSyncError`, `OrderSendFailure` (verificar uso) | ~500 |

Contratos que no cambian:

- `crate::services::BdpSyncService` y `SyncTablesResult` siguen re-exportados
  desde `services/mod.rs`; ningún llamador externo se modifica.
- Múltiples bloques `impl BdpSyncService` en el mismo crate son legales;
  cada helper privado vive en el submódulo de su entrada y se llama por `Self::`.
- Lo compartido entre fases (`SYNC_LOCKS`, `conectar_bdp`, `parse_remote_money`)
  queda `pub(crate)` en el hub; nada se hace `pub` global nuevo.
- Cabecera `sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro`
  se replica en cada submódulo que use `sqlx::query` (todos, verificar uno a uno).

## Fases verificables

1. **F4.1 Hub + venta.** Crear `bdp_sync_venta.rs` con el bloque FASE 7 íntegro;
   hub conserva struct, sync base, re-exports y `mod` en `services/mod.rs`.
   Verificar: `node scripts/run-cargo.mjs check` PASS.
2. **F4.2 Pago + factura.** Crear `bdp_sync_pago.rs` y `bdp_sync_factura.rs`
   (bloques FASE 8 íntegros, `conectar_bdp` queda en hub). Verificar: check PASS.
3. **F4.3 Catálogo.** Crear `bdp_sync_catalogo.rs` (FASE 9 + tipos de error si
   solo se usan ahí). Verificar: check PASS + `fmt`.
4. **F4.4 Medición.** `task:check 267A-9`: `limite-lineas-nivel-3` ausente,
   0 `funcion-larga-rs` >200, rust PASS. `--full` antes de cerrar.

## Estado

- [x] F4.1 Hub + `bdp_sync_venta.rs` — `check` PASS, 0 warnings (2026-09-12).
- [x] F4.2 `bdp_sync_pago.rs` + `bdp_sync_factura.rs` — `check` PASS, 0 warnings (2026-09-12).
- [x] F4.3 `bdp_sync_catalogo.rs` (+ `SyncTablesResult` movido; re-export actualizado) — `check` PASS, 0 warnings (2026-09-12).
- [x] F4.4 Medición `task:check 267A-9` (2026-09-12): sentinel PASS 0 errores
  (9 warnings propios/ajenos: `build_order` 130 ef., `bdp_sync_venta.rs` 1142 ef.
  >700 warning nivel-1, resto ajenos; `limite-lineas-nivel-3` ausente, 0
  `funcion-larga-rs` >200 en archivos propios); rust = infra transitoria
  (dep-graph move + timeout 305s compilando sqlx-macros; `run-cargo.mjs check`
  directo PASS 2m16s); docs FAIL solo 2 planes ajenos sin checklist (no tocar
  WIP ajeno). Clippy propio limpio: `OrderSendFailure`/`BdpSyncError` pub(crate),
  `validar_puerta_pago` sync, `confirmar_pago`/`obtener_articulos_catalogo`/
  `revisar_estado_y_lock_distribuido`/`gestionar_error_pago` extraídos; único
  error restante `bdp_backup.rs:482` preexistente en HEAD. `--full` pendiente +
  cierre (sin commit/push).

## Próximo paso

Ejecutar F4.1 (mover bloque FASE 7 tal cual, ajustar `use` del submódulo).

## Gate y DoD

- Gate: `npm run task:check -- 267A-9` (`--full` al cerrar).
- DoD: hub <700 efectivas; 0 errores herramienta activa en archivos propios;
  `check` PASS; sin cambios en mensajes/SQL/guardas/orden (diff solo mueve código).
