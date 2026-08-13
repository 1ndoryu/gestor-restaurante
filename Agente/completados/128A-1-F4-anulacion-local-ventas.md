# Tareas completadas — F4 (bloque 128A-1) — Anulación local de ventas

## F4 — Anulación local de ventas (modalidades D4, reglas M8–M11, delete D5)

- **Qué:** anulación local de ventas con modalidad configurable (`credito_completo` default | `estado_solo`).
  - Backend: migración `20260815000000_venta_anulacion` (columnas `anulada`, `anulada_at`,
    `anulacion_motivo`, `anulacion_usuario` en `ventas`; `anulacion_modalidad` en `configuracion`
    con default `credito_completo`).
  - Servicio `VentaService::anular`: motivo obligatorio en `credito_completo` (M10), bloqueo de ventas
    facturadas (M9), transición única con guard + idempotencia C1 vía `bdp_audit_log` (ON CONFLICT),
    sin llamada a `CancelOrder` (C3=b / M8): el estado «pendiente de anular en BDP» se deriva
    (`anulada=true AND bdp_synced=true AND status no final`) y el poller excluye esas ventas.
  - `total_periodo` excluye anuladas (reversión de IVA idempotente); `delete` desbloqueado solo para
    ventas no sincronizadas ni anuladas (D5) — las anuladas nunca se borran físicamente.
  - Guards BDP/Haddock saltan ventas anuladas (`bdp_sync.rs`, `haddock.rs`).
  - Frontend: botón «Anular» con confirmación `ANULAR {id}` + motivo en `venta-row-actions.tsx`,
    badge «Anulada» en `venta-table-body.tsx`, mutation con manejo 409/422 en `useListaVentas.ts`,
    selector «Modalidad de anulación» en `ConfigBdp.tsx` (config types/sync/guardar actualizados).
- **Archivos:** `migrations/20260815000000_venta_anulacion.{up,down}.sql`, `src/models/venta.rs`,
  `src/models/configuracion.rs`, `src/repositories/{venta,configuracion}.rs`,
  `src/services/{venta,configuracion,bdp_sync,haddock}.rs`, `src/handlers/ventas.rs`,
  `frontend/src/api/bdp.ts`, `frontend/src/api/generated/gestionRestauranteAPI.schemas.ts`,
  `frontend/src/components/{venta-row-actions,venta-table-body}.tsx`,
  `frontend/src/hooks/{useListaVentas,configuracion-types,useConfiguracionSync,useConfiguracion}.ts`,
  `frontend/src/componentes/{ListaVentas,ConfigBdp}.tsx`, tests constructores actualizados.
- **Comandos y resultados:**
  - `npm run task:check -- 128A-1` → **PASS** (sentinel 0 errores, varsense, rust fmt/check, frontend
    type-check, docs) — reporte `.quality-reports/branches/glory-rs-rest--f100af0a041e6e8a/128A-1/latest.md`.
- **Gotchas:** las consultas F4 se mantienen dinámicas (`sqlx::query`/`query_as` sin macro) porque el
  cache offline `.sqlx/` no tiene las columnas nuevas; `customInstance` devuelve `{ data, status }` y
  la anulación extrae `.data`. Ocupación de mesas se deriva de reservas (venta→reserva_id→mesa,
  fallback `num_mesa`), por lo que M11 no toca el plano en F4.
- **Sentinel:** el gate corrió la etapa sentinel (PASS, 0 errores; warnings preexistentes no bloquean).
- **GLORY:** no aplica; cambios del bloque 128A-1 en rama `glory-rs-rest`.
