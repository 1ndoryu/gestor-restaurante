# Plan — Auditoría profunda del modo BDP `automatic` (189A-3, 2026-09-18)

## Objetivo
Garantizar al 100% que el modo automático (nuevo en 267A-7) no genera
errores, defectos ni fallos: ni escrituras no autorizadas, ni escrituras sin
auditar/respaldar, ni bypass de guards, ni estados inconsistentes.

## Alcance
- Activación: `PUT /api/configuracion/bdp/sync-mode` rama `automatic`
  (`src/handlers/configuracion.rs:440-`).
- Guards: `authorize_automatic`, `modo_permite_escritura` en los 5 gates de
  `bdp_push.rs`, `BdpWriteArmingRepository::buscar_snapshot_vigente`.
- Escritura: backup pre-write, auditoría por operación, reversibilidad real.
- Efectos laterales: `PATCH /api/configuracion` (fuerza `read_only`),
  `standalone`+`sync_enabled`, `bidirectional` bloqueado.
- Frontend: `flujoModoBdp.activarAutomatico`, tarjeta, badge, selector.
- Concurrencia: dos admins, cambio de modo mid-write, expiración de snapshot.

## Fases verificables
1. Guards de activación + snapshot (lectura código + tests).
2. Path de escritura: gates, backup, auditoría (lectura + tests backend).
3. Frontend + PATCH + concurrencia (lectura + razonamiento).
4. Informe de hallazgos con severidad + registro en completadas.

## Definition of Done
- Informe con cada riesgo clasificado (defecto real / observación / falso
  positivo) y evidencia por ruta:símbolo.
- Defectos reales → tareas separadas en roadmap (no se mezclan aquí).
- Sin cambios de código en esta tarea (es auditoría).
