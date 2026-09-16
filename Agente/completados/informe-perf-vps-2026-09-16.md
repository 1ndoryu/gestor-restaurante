# Informe PERF-VPS — Rendimiento staging en VPS (2026-09-16)

Bloque 169A-4. Plan: `Agente/planes/plan-perf-vps-2026-09-16.md`.
Datos crudos: `scripts/perf/resultados/` (9× `.json` + 9× `.stats.jsonl`).
Resumen también en `Agente/completados/tareas-2026-09-16.md` (§169A-4 PERF-VPS).

## 1. Objetivo y setup

Dimensionar CPU/RAM del servicio replicando operaciones mínimas reales, sin
tráfico ni escrituras en el BDP real (regla oro 149A-2).

- Staging `restaurante-perf` (`p0sk8swg8o8kcokko0w4ocwg`) en `https://perf.wandori.us`
  (VPS 4 vCPU, 7 GB RAM, 3 GB swap). Build del `main` desplegado vía
  `coolify-manager-rs` 1.0.0; seed demo; `modo_operacion=standalone`,
  `bdp_sync_enabled=false`, sin `BDP_BASE_URL`.
- Harness `scripts/perf/harness.mjs` (cero deps): escenarios
  `base {1u,300s}`, `pico {10u,600s}`, `sostenido {3u,1800s}`; login demo + 3
  trabajadores; OPS B1–B6 con peso (dashboard×3, ventas_listar×3, article_maps×2,
  article_stock×2, clientes/reservas/gastos_listar×1, venta_crear×1,
  gasto_crear×1 con `descripcion/proveedor=perf-harness`).
- Muestreador `scripts/perf/muestrear.mjs`: `docker stats` app+postgres vía
  `host-exec --command` cada 5 s (solo lectura).
- Tiers aplicados en vivo con `docker update --cpus/--memory/--memory-swap`
  (sin reinicio): Tmax = sin límites; T2 = 2 CPU/2 GB; T1 = 1 CPU/1 GB.
  Cooldown 2 min + `health` OK entre runs. CPU% = % de 1 core.

## 2. Matriz 9/9 (~251k peticiones)

| Run | Peticiones | req/s | Errores | p50 | p95 | p99 | App cpu avg/max + RSS | PG cpu avg/max + RSS |
|---|---|---|---|---|---|---|---|---|
| Tmax-base | 1.073 | 3,58 | 0 | 119 | 1091 | 1746 | 2,9/15,6 ~147 MB | 4,2/26,3 ~38 MB |
| Tmax-pico | 41.132 | 68,5 | 0 | 120 | 234 | 504 | 27/127 203/207 MB | 42/116 60/63 MB |
| Tmax-sost | 34.139 | 19,0 | 0 | 115 | 324 | 1096 | 10,2/27,6 170 MB | 18,1/40,8 57/63 MB |
| T2-base | 2.337 | 7,79 | 0 | 119 | 187 | 295 | 4,3/10,2 169/170 MB | 8,2/34,3 51/52 MB |
| T2-pico | 45.607 | 76,0 | 0 | 120 | 196 | 305 | 26/107 226/263 MB | 65,9/104 61/66 MB |
| T2-sost | 42.509 | 23,6 | 0 | 118 | 175 | 282 | 11,8/55 226/228 MB | 29,5/109,6 57/63 MB |
| T1-base | 2.331 | 7,77 | 0 | 120 | 183 | 309 | 5,4/27,6 227 MB | 12/48,3 54 MB |
| T1-pico | 41.996 | 70,0 | 1 | 125 | 232 | 377 | 22,3/37,9 229/231 MB | 85,3/115 66/68 MB |
| T1-sost | 40.155 | 22,3 | 0 | 123 | 199 | 336 | 11,9/83,8 230/232 MB | 37,4/135,8 62/67 MB |

Notas:

- Tmax-base p95 alto = listados en frío (outlier; no se repite en ningún otro run).
- Tmax-sost: muestreo parcial (103 ok) por purga del binario — ver §4.
- Único error (1/41.996 en pico@T1, `dashboard_resumen`, 0,002 %): transitorio
  de lado cliente; cero errores en logs app (400 líneas), `restarts=0`,
  `oom=false` en ambos contenedores.
- Operación más pesada: `ventas_listar` (p50 ~125–160 ms, dataset crece con
  cada run porque el harness crea ventas/gastos `perf-harness`).

## 3. Veredicto

**T1 (1 vCPU / 1 GB) sobrado para este perfil** (base 1u, pico 10u, sostenido
3u/30m): los 3 tiers cumplen el criterio (sostenido p95 < 800 ms, errores
< 1 %, cero OOM). Recomendación = T1; si el pico crece, T2 da headroom.

- Cuello de botella: postgres CPU en pico (throttle a cuota en T1).
- App nunca pasa de ~1,1 cores ni 263 MB RSS.
- Sin leaks: RSS app plana 30 min (T2 225,8→225,8 MiB; T1 230,4→230,4 MiB),
  postgres estable ~60 MB.

## 4. Incidentes y gotchas

1. `GloryTmpSweep` purgó `C:\tmp\glory-target\coolify-manager` a mitad de matriz
   (`ENOENT` desde 20:49:51). Rebuild 6m31s (sccache) + copia a salvo en
   `C:\Users\Owner\AppData\Local\Temp\opencode\coolify-manager.exe`;
   `muestrear.mjs` usa esa ruta (override `COOLIFY_MANAGER_EXE`).
2. `docker update --memory` exige `--memory-swap` simultáneo.
3. `host-exec` exige `--command`; `container-stats` solo devuelve el contenedor app.
4. `GET /api/dashboard/resumen` exige `?year=&month=`.
5. T2 superó a Tmax en throughput pico (76 vs 68,5 req/s): con cuota, el
   scheduler fija los contenedores en menos cores (mejor localidad); sin cuota
   compiten con los otros 10 sitios del host.

## 5. Pendientes (usuario)

- Repo `1ndoryu/gestor-restaurante` → volver a privado (`gh` sin auth).
- Destino staging `restaurante-perf` + DNS `perf.wandori.us` (C.5: borrar/conservar).
- ~35k filas `perf-harness` (ventas+gastos) solo en staging.
- GitHub avisa 46 vulnerabilidades Dependabot (2 críticas) — tarea aparte.
