# Plan 179A-1 — abaratar origen y llegar a 50k (2026-09-17)

Origen: informe `Agente/completados/informe-50k-2026-09-17.md` (169A-5 D5):
techo origen ~230 DYNAMIC/s, pg ~1 núcleo, pool 8/10 (no es cuello de pool),
app sobrada, borde 0 errores. Para 50k hacen falta 5.000 req/s borde con
95 % HIT → ~250 origen/s: marginal por encima del techo actual.

## Objetivo

Tramo verde `consulta50k` mix-90 **≥2.500 req/s borde** (hito 1 ≈ 25k personas)
y **≥5.000 req/s** (hito 2 = 50k), p95 < 800 ms, err < 0,1 %, con la misma
regla de honestidad (cifra = tramo verde redondeado abajo).

## Alcance / no alcance

- Sí: listados calientes (ventas, gastos, clientes, reservas), dashboard,
  índices, caché en app, pool, re-medición con `consulta50k`.
- No: escrituras/BDP/sync, subida de HIT% en borde compartido para datos
  privados (imposible por diseño: `Authorization`), réplica de lectura ni
  cambio de VPS (descartados abajo, no en silencio).

## Hallazgo que fija el orden (2026-09-17, solo lectura)

- `src/repositories/venta.rs:158-219`: cada `listar_ventas` = SELECT con
  `LEFT JOIN clientes` + 6 ILIKE + 2 CASE + ORDER BY dinámico + `LIMIT/OFFSET`
  **más** un COUNT con los mismos filtros. Doble coste por petición; mismo
  patrón probable en gastos/clientes/reservas (verificar en F2).
- Solo `dashboard_resumen` tiene caché en app (`src/services/dashboard.rs:69`,
  TTL 30 s). Los listados pegan a pg en cada petición.
- `ORDER BY` dinámico impide un único índice covering; el default
  (`v.fecha DESC, v.created_at DESC` + `user_id`) sí es indexable.

## Fases (cada una: código → fmt/clippy/test → deploy staging → EXPLAIN +
`consulta50k` corto → commit propio)

- [x] **F1. Caché en app para listados** (mayor impacto, menor riesgo).
  Implementado 2026-09-17, commit `524aaa0` + push (despliegue Coolify en
  curso): `ListadosCache` en `src/lib.rs` (clave `(user_id, endpoint)`,
  TTL 15 s, solo combo default), `respuesta_cacheable_bytes` +
  `leer/guardar/invalidar_listado` en `src/middleware/cache_control.rs`,
  aplicado en `ventas/gastos/clientes/reservas` + invalidación en escrituras
  (incl. `chatbot`). Verificado local: `fmt:check` limpio, `cargo check`
  limpio, `clippy` limpio salvo `dead_code` preexistente ajeno de 169A-2
  (`bdp_sync_preflight.rs:609`, no tocado), `cargo test --lib` 186/186.
  Suite de integración NO ejecutada (petición usuario 2026-09-17: congela
  la máquina). Pendiente: confirmar deploy, cabeceras en staging,
  `pg_stat_activity` + tramo `consulta50k` corto.
  Hallazgo verificación 2026-09-17 (sin suite local, 1 tramo cada vez):
  F1 en vivo (`Cache-Control: public, max-age=15` + ETag en staging) pero
  2× u100 (90 s: 404 req/s p95 540 ms; 120 s: 477 req/s p95 462 ms) por
  debajo del baseline pre-F1 (560-567 req/s, p95 ~331-349 ms). No es
  regresión del código (overhead F1 = un Mutex + clone por petición):
  (a) el harness usa UNA sola cuenta compartida y 5 VUs escritores invalidan
  su caché en bucle → hit rate bajo con esta mezcla; (b) las tablas crecieron
  con los escritores de todos los tramos (17k ventas/gastos demo hoy vs base
  saneada del baseline) → cada MISS (SELECT + COUNT exacto) es más caro.
  Conclusión: F2 (COUNT estimado + índice default) ataca la causa real; antes
  de re-medir, sanear la base como en 169A-5 o el baseline deriva.
  Verificación 2026-09-17 con manager reconstruido (`1.0.0` release,
  `C:\tmp\glory-target\coolify-manager` + copia en `%TEMP%\opencode`):
  F2a EXPLAIN en staging (base con bloat): ventas default 0,52 ms
  (Index Scan `idx_ventas_user_fecha_origen50k` + Nested Loop Memoize a PK:
  F2c bien descartado), clientes 0,36 ms, reservas 0,59 ms. DoD F2
  (ventas_listar <5 ms) cumplido a nivel query.
  Saneamiento 2026-09-17: DELETE solo filas `perf-harness` del harness
  (16.939 ventas + 16.936 gamas; quedan 40/33 manuales demo). Tramo u100/120s
  posterior sobre base limpia con build F1: **629 req/s, p50 134 ms,
  p95 259 ms, 0 err** (`50k-u100-f1-limpio.json`) — supera el baseline pre-F1
  (560-567 req/s, p95 ~340 ms). La "regresión" era 100 % bloat, no F1.
  Extender el patrón `resumen_cache` (`src/lib.rs` + `invalidar_*` en
  escrituras) a `listar_ventas/gastos/clientes/reservas` **solo combo
  default** (1ª página, sin filtros/búsqueda) con TTL 15-30 s; clave
  `(user_id, endpoint, params-hash)` para acotar memoria; invalidar en
  escrituras como `invalidar_resumen`. DoD: EXPLAIN sin cambios pero
  `pg_stat_activity` en tramo u100 muestra <50 % de queries de listado vs
  hoy; tramo mix-60 u100 p95 < 250 ms.
- [x] **F2. Recortar coste por query** (2026-09-17, commit pendiente push).
  F2a EXPLAIN en staging: BLOQUEADO (binario `coolify-manager-rs`
  desaparecido de `%TEMP%\opencode`; reconstruirlo es una compilación
  release pesada — aparcado para no cargar la máquina; el informe 169A-5 ya
  documentó ~14 ms por listado). Se sigue sin esa medición.
  F2b: ventas/gastos YA cubiertos por D2 (`idx_ventas_user_fecha_origen50k`,
  `idx_gastos_user_fecha_origen50k`); añadidos los que faltaban, migración
  reversible `20260917000001_origen50k_f2`: `idx_clientes_user_apellidos_f2`
  `(user_id, apellidos DESC, nombre ASC)` y `idx_reservas_user_fecha_f2`
  `(user_id, fecha DESC, hora ASC)` = defaults exactos de `cliente.rs:175-186`
  y `reserva.rs:137-155`.
  F2c (vía sin JOIN): DESCARTADO con motivo — el JOIN de ventas es a PK de
  clientes sobre ≤20 filas tras early-stop por índice, y `nombre_cliente`
  forma parte del contrato de respuesta; quitarlo cambiaría la API.
  F2d: ventas/gastos/reservas YA tenían COUNT barato sin filtros; solo
  clientes hacía siempre el COUNT con la cadena ILIKE → split aplicado en
  `cliente.rs` (COUNT exacto solo con búsqueda).
  DoD: `ventas_listar` default p50 < 5 ms en `EXPLAIN ANALYZE` staging.
  F2a: `EXPLAIN ANALYZE` de los 4 listados en staging (con y sin filtros);
  F2b: índice compuesto default `(user_id, fecha DESC, created_at DESC)` en
  ventas/gastos vía migración reversible (como `20260917000000_origen50k`);
  F2c: vía rápida sin `LEFT JOIN clientes` cuando no hay búsqueda (dos
  caminos en el repositorio, no un flag global); F2d: COUNT exacto solo con
  filtros, sin filtros usar estimado/`COUNT` cacheado con el mismo TTL.
  DoD: `ventas_listar` default p50 < 5 ms en `EXPLAIN ANALYZE` staging.
- [x] **F3. Dashboard en paralelo.** (2026-09-17) `resumen_mes`
  (`src/services/dashboard.rs:28`): `obtener_o_crear` + `total_gastos` en
  `tokio::join!`; `total_ventas` después (depende de la modalidad).
  Verificado local: `fmt` + `check` + `clippy` limpios (salvo `dead_code`
  ajeno 169A-2), `test --lib` 186/186. DoD: miss de caché p50 dividido ~×3
  (de ~30-45 ms a ~12-18 ms en staging).
  (`src/services/dashboard.rs:28`): config + total_ventas + total_gastos son
  independientes → `tokio::join`. DoD: miss de caché p50 dividido ~×3
  (de ~30-45 ms a ~12-18 ms en staging).
- [x] **F4. Pool y pg, con compuerta de medición.** Decisión inicial (2026-09-17,
  sin código): NO subir — evidencia `50k-u250.stats.jsonl` (169A-5): pg
  69-106 % CPU (1 núcleo saturado) con pool 8/10 sin agotar.
  **Revisado el mismo día: la compuerta se cumplió tras F1-F3.**
  Evidencia `50k-u500-f2f3.stats.jsonl` (u500 mix-90, 1490 req/s VERDE):
  pg p50 40 % / max 55 %, app p50 47 % / max 139 % — CPU libre en ambos;
  u1000 colapsa (529 req/s, auto-stop 50 s) con p95 uniforme incluso en
  CF-HIT = firma de cola de pool, no de pg. Cambio aplicado:
  `src/main.rs:22-23` `max_connections(10)` → **20** (+ comentario con
  criterio de reversión: si pg >80 % en re-medición, revertir).
  Verificado `npm run check:back` (solo warning `dead_code` preexistente
  169A-2). DoD: u1000 no colapsa por cola de pool.
- [ ] **F5. Re-medición y claim.** BLOQUEADO hasta reconstruir
  `coolify-manager-rs` (binario perdido): el saneamiento de base (vuelta a
  ~0 filas demo como en 169A-5) y el muestreo pg (`container-stats`,
  `run-sql`, `muestrear.mjs`) lo exigen; sin sanear, cada tramo añade miles
  de filas de escritores y el baseline deriva (~17k hoy). No lanzar más
  tramos hasta entonces. Al re-medir: `consulta50k` mix-90 u500 → u1000,
  informe nuevo, cifra redondeada abajo.
- [ ] **F5. Re-medición y claim.** `consulta50k` mix-90 u500 → u1000 (+T1/T2
  solo si el contenedor pasa a ser el cuello); informe individual nuevo;
  cifra redondeada abajo. Si el hito 2 no sale, el informe dice el nuevo
  techo y su causa (misma honestidad que 169A-5).

## Opciones descartadas (explícito)

- Cachear listados por usuario en CF: imposible, son privados (`Authorization`
  nunca en clave, D2/D3).
- Réplica de lectura / VPS mayor: coste e infraestructura fuera de este bloque;
  se re-evalúa si F1-F4 tocan techo de hardware (pg >90 % con queries <5 ms).
- Subir TTLs del borde: no toca el techo de origen.

## Riesgos

- Dato viejo en listados (15-30 s): aceptable, mismo precedente que dashboard
  D2; invalidación en escritura lo acota.
- Memoria app: la caché por usuario crece con usuarios distintos; acotar por
  combo default + TTL corto + vigilar RSS en `stats.jsonl` (precedente:
  1,2-1,57 GB bajo 250-500 VU, liberó parcial en idle).
- Regla prioritaria: cero tráfico BDP real; staging `restaurante-perf` como
  en 169A-5.

## Gate y cierre

Gate por bloque: `cargo fmt --check`, `clippy -D warnings`, tests afectados,
deploy vía `coolify-manager-rs`, tramo `consulta50k` corto como evidencia
funcional. Commits `179A-1 F<n>:` + push. Cierre: plan a
`Agente/planes/completados/`, informe en `Agente/completados/`, roadmap
actualizado, prevención si aparece fricción repetible.
