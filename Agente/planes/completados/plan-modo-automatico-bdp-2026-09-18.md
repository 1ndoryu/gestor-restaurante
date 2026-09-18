# Plan 267A-7 — Tercer modo BDP: automático (2026-09-18)

## Objetivo

Añadir el modo `automatic` a `bdp_sync_mode`: escrituras Glory → BDP sin
confirmación por operación, activable desde Configuración y desde el badge del
header. Las escrituras siguen auditadas y sujetas a preflight; solo se salta el
armado temporal.

## Por qué no existía (decisión de seguridad)

Cada escritura al TPV es irreversible en la práctica (factura = documento
legal; pago sin "deshacer"; anulación deja rastro). El diseño exigía freno
humano por operación. El modo automático lo elimina a petición del
restaurante: activación consciente, reversible en un clic.

## Alcance

- Backend: `automatic` en `valid_modes` (hecho), rama en
  `cambiar_bdp_sync_mode` con confirmación + destino + backup + snapshot
  vigente, sin armado por operación (hecho); rama en `authorize` del
  write-guard (auditoría con motivo `modo_automatico`, sin consumir armado
  ni volver a `read_only`); helper `modo_permite_escritura` usado en los 5
  gates de push (`bdp_customer_sync`, `venta`, `bdp_sync_venta`,
  `bdp_sync_pago`, `bdp_sync_factura`).
- Frontend: `SyncMode` + tercera tarjeta en `ConfigBdp` (activar/desactivar
  con confirmación explícita); badge con 3 estados (`lectura` / `escritura` /
  `automático`) + entrada de menú; tercera opción en el selector "Permiso de
  operación" de `PanelBdpBackup` (solo confirmación de destino, sin
  alcance/objetivo).
- No alcance: `bidirectional` sigue bloqueado; auto-arming (`ff_bdp_auto_arm`)
  no cambia; PATCH de conexión sigue forzando `read_only` al tocar datos BDP
  (protege también el modo automático: si cambia la URL/clave, se cae a Solo
  lectura).

## Fases verificables

1. Backend compila (`check`), clippy limpio, tests `configuracion` + guard.
2. Frontend `type-check` + `build`.
3. Verificación funcional en navegador: tarjeta, selector, badge (captura).
4. Commit + push (deploy solo si el usuario lo pide: sin migraciones, pero
   toca escrituras reales → no auto-deploy).

## Estado

- [x] Inventario de implicaciones
- [x] Rama `authorize` en write-guard + helper + 5 gates
- [x] Frontend (tarjeta, badge 3 estados, selector PanelBdpBackup)
- [x] Diálogos propios: `dialogoConfirmacion.tsx` sustituye todos los
      `window.confirm/prompt` (ConfigBdp, PanelBdpBackup, BdpCompras,
      BdpExplorador); `toastConfirmacion.tsx` eliminado (sonner depende de
      rAF, congelado en el navegador empaquetado)
- [x] Tests + type-check (clippy limpio; `test --lib` 189/189; type-check 0
      errores en `src/`)
- [x] Verificación en navegador 2026-09-18: flujo completo con cuenta local
      (confirmar → pedir URL exacta → `PUT /api/configuracion/bdp/sync-mode`
      → 422 con mensaje real del backend en el toast; modo sigue
      `read_only`, badge `BDP: off`)
- [ ] Commit/push

## Hallazgo 267A-8 (causa raíz del toast genérico)

El toast mostraba "Request failed with status code 422" por dos causas
encadenadas: (1) el front enviaba `target_entity_id: ""` y el backend lo
declara `Option<Uuid>` → el extractor axum rechaza con 422 en texto plano
(sin campo `message`); (2) el `onError` solo leía `err.message` de axios.
Fix: `setSyncMode` envía `null` sin objetivo (`bdp-backup.ts:139-140`) y
`mensajeErrorBackend` extrae `response.data.message` o texto plano
(`ConfigBdp.tsx`). Sin cambios de backend.

## Definition of Done

`cargo clippy -D warnings` + suite verde, `npm run type-check` + build OK,
badge/tarjeta/selector mostrados en navegador, commit en `main`.
