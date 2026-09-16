# Plan PERF-VPS — Despliegue seguro en VPS + informe de rendimiento sin tocar el BDP real

> **Fecha:** 2026-09-16 · **Rama operativa:** `main` · **Estado:** APROBADO 2026-09-16
> (decisión usuario: alcance C + VPS directo + push autorizado) · **ID tarea:** `169A-4`
> **Objetivo:** desplegar con seguridad en nuestra VPS y obtener un informe de
> rendimiento (CPU/RAM) que replique operaciones mínimas reales **sin generar
> ruido ni nada real en el BDP**, para dimensionar recursos y decidir si hay que
> mejorar el rendimiento. Las pruebas precisas se hacen **en la VPS**.

## 0. Principios (no negociables)

1. **Cero tráfico al BDP real.** Todo el perf se ejecuta en `modo_operacion=standalone`
   (`bdp_sync_enabled=false`; verificado P11.1: 0 conexiones a `100.83.196.35:8068`).
   El endpoint de diagnóstico NO se invoca durante el perf.
2. **Staging separado de producción.** Servicio Coolify nuevo (p. ej. `restaurante-perf`),
   con datos seed/demo. Nada de prod, nada del BDP real.
3. **Solo `coolify-manager-rs` para operaciones remotas.** SSH directo, Docker remoto
   y curl a la API prohibidos. Cada escritura remota requiere autorización explícita.
4. **Reproducible.** Harness versionado en el repo, informe con números (p50/p95,
   req/s, CPU %, RSS) en `Agente/completados/`.

## 1. Preflight (local, sin escrituras)

- [x] P0.1 Compilar manager (2026-09-16: compilado, `coolify-manager 1.0.0`;
  nota: un build huérfano quedó colgado tras un timeout del wrapper — se mató el
  `cargo` idle y el rebuild terminó en 1m17s).
- [x] P0.2 `container-stats --help`: existe con `--json` (muestreo por polling, sin
  `--watch`) → **sin adaptación del manager**.
- [x] P0.3 `list`: 10 sitios + 1 minecraft en target `default`; `glory-rest`
  (legacy) no se toca.
- [x] P0.4 `Dockerfile` + `Cargo.lock` + gitlink `glory-rs` presentes en el repo.
- [x] P0.5 Push `f0e3173` hecho (autorizado). Nota: GitHub reporta 37 vulns
  (1 crítica) en el repo — a tratar fuera de este plan, antes de prod.

## 2. Fase A — Staging en la VPS (requiere autorización explícita)

- [x] A.1 `new --name restaurante-perf --template rust` (2026-09-16: servicio
  creado, uuid `p0sk8swg8o8kcokko0w4ocwg`, dominio `https://perf.wandori.us`).
  **Bloqueo deploy:** el template clona `https://github.com/1ndoryu/gestor-restaurante.git`
  y el repo es privado → `fatal: could not read Username` (exit 128), evidencia en
  `C:\tmp\perf-deploy.log`. Decisión usuario: hacerlo público temporal y volver a
  privado tras el build.
- [x] A.0 Auditoría secretos pre-publicación (2026-09-16): VEREDICTO **publicable**.
  HEAD: sin `.env` trackeado (gitignored), solo `frontend/.env.example` con
  placeholders; cero tokens/keys (`ghp_/sk-/AKIA/xox`/private keys); asignaciones
  `BDP_*/JWT_SECRET/DATABASE_URL` solo placeholders o `${VAR}`. Historial (`--all`,
  valores enmascarados): sin valores reales (solo SQL `CASE`, `env::var`, `pass/TEST`
  en docs de tests, URLs `user:...@` ficticias). Rama remota `glory-rs-rest` también
  limpia. Refs `synara/checkpoints/*` solo locales, no están en `origin` (`main` +
  `glory-rs-rest`). Residuos no bloqueantes al hacerlo público: IP Tailscale BDP +
  `pos_id` en docs, credenciales demo del seed (`demo1234`/`trabajador123`, solo test),
  UUIDs Coolify en `temp-compose-glory-rest.yml`. `gh` sin auth → el flip lo hace el
  usuario en GitHub UI. Tras el build, volver a privado (los redeploys futuros
  necesitarían token o repo público).
- [x] A.2 `sync-env`: **sin push necesario.** Env actual verificado vía `exec`
  (2026-09-16): `DATABASE_URL` con `postgres-p0sk8swg8o8kcokko0w4ocwg` ✓ (gotcha
  28P01 OK), `JWT_SECRET` aleatorio 64 chars ✓, `HOST=0.0.0.0`, `RUST_LOG=info`,
  `BDP_BASE_URL`/real `BDP_*` ausentes, `PORT` default 3000. Nada real que sincronizar.
- [x] A.3 `deploy --name restaurante-perf` (2026-09-16: build OK con repo público,
  migraciones aplicadas, servidor en `:3000`; el health del manager falló solo por
  DNS inexistente → rollback E11 automático, pero el contenedor corriendo es el build
  nuevo). `setup-site-dns` ejecutado: `A perf.wandori.us → 66.94.100.241` [created]
  (previo `dnsConfig` cloudflare añadido a `settings.json`, backup en
  `settings.json.bak-20260916`). `health --all`: los otros 10 sitios + minecraft OK;
  solo `restaurante-perf` en rojo por cert pendiente (disparó 1 email de alerta a
  admin@wandori.us — ruido conocido).
- [x] A.4 seed + standalone (2026-09-16): `/app/seed` vía `exec` OK (demo
  `e78fa340…`, 84 reservas, 39 ventas, 3 trabajadores, etc.);
  `GET /api/configuracion/modo` → `auto/standalone`;
  `PATCH {modo:standalone}` → `standalone/standalone` (switch maestro forzado).
  Tráfico BDP imposible: standalone + base_url vacía + sin bootstrap + write-origins vacíos.
- [x] Cert + health (2026-09-16 ~20:01Z): causa raíz del 000 = Traefik intentó el
  cert ACME a las 19:47/19:50 con NXDOMAIN (DNS aún no existía) y no reintentó.
  `deploy-service --skip-build --skip-backup` (sin `--seed` para no borrar datos)
  refrescó el router → LE emitió → **deploy exitoso, `https://perf.wandori.us` 200**,
  autoheal instalado, resto de sitios verificados OK por el propio deploy
  (guillermo, padel, wandori, nakomi, cap, studio, kamples, glory-rest, agape, task).
  Post-swap verificado: login demo OK, `modo standalone/standalone`, seed intacto
  (8 clientes). **El repo ya puede volver a privado.** Fase A completa salvo A.5/A.6.
- [ ] A.5 Guardarraíl: límites CPU/RAM amplios al inicio (p. ej. 2 vCPU / 2 GB) para
      medir consumo real sin throttling; el informe dirá los límites finales.
- [ ] A.6 `backup` inicial del staging (opcional, barato; protege el seed calibrado).

## 3. Fase B — Harness de carga (versionado, sin BDP)

Perfil **"operaciones mínimas reales"** (todo local, todo standalone):

| # | Operación | Endpoint | Peso |
|---|-----------|----------|------|
| B1 | Login dueño + login trabajador | `POST /api/auth/login`, `login-trabajador` | 1×/usuario |
| B2 | Dashboard/resumen | `GET /api/ventas/resumen`, dashboard | alto |
| B3 | Crear venta + cobro local | `POST /api/ventas`, pagos/factura local | medio |
| B4 | Catálogo paginado + búsqueda | `GET /api/catalogo`, `/api/bdp/article-maps` | alto |
| B5 | Stock + ajuste | `GET /api/stock`, `PATCH` ajuste | medio |
| B6 | Gasto + reserva + cliente | `POST /api/gastos`, reservas, clientes | bajo |

- [ ] B.1 Escenarios: `base` (1 usuario, 5 min), `pico` (10 concurrentes, 10 min),
      `sostenido` (3 concurrentes, 30 min). Ramp-up progresivo, sin picos artificiales.
- [ ] B.2 Harness: script Node (`scripts/perf/harness.mjs`) con `autocannon`-style o
      k6 si está instalado; registra latencias p50/p95/p99 + req/s + errores.
- [ ] B.3 El harness corre **desde este PC** contra la URL del staging (carga por red
      real); las métricas servidoras se toman en la VPS (§4).

## 4. Fase C — Medición y informe (en la VPS)

Matriz de tiers de recursos (decisión usuario 2026-09-16: medir en los tres):

| Tier | CPU | RAM | Propósito |
|------|-----|-----|-----------|
| T1 mínimo | 1 vCPU | 1 GB | ¿aguanta el producto viable mínimo? |
| T2 medio | 2 vCPU | 2 GB | candidato a staging/prod pequeña |
| Tmax | 4 vCPU / 6 GB (VPS: 4 vCPU, 7 GB RAM, 3 GB swap, 166 GB libres — medido 2026-09-16 vía `host-exec` solo-lectura; se reservan ~1 GB al host) | techo real |

Cada escenario del harness (base/pico/sostenido, §3) se ejecuta **una vez por tier**
(9 runs). Entre runs, cooldown 2 min + `health` OK antes de empezar. Los límites se
fijan vía `docker update` en vivo (sin reinicio) antes de cada tier; el tier inicial
de A.5 pasa a ser **Tmax** (sin límites) y luego se baja a T2 y T1.
**Cierre 2026-09-16: matriz 9/9 ejecutada (~251k peticiones). Veredicto: T1 viable
para este perfil (pico 10u p95 232 ms, 1 error cliente transitorio en 42k);
cuello = postgres CPU en pico; sin leaks (RSS plana 30 min); sin OOM ni reinicios.
Informe en `Agente/completados/tareas-2026-09-16.md` (§169A-4 PERF-VPS).
Desvíos del plan: límites por `docker update --cpus/--memory/--memory-swap`
(en vez de compose; `--memory` exige `--memory-swap` a la vez); muestreo con
`docker stats` vía `host-exec --command` (container-stats solo devuelve app);
`host-exec` exige `--command`; dashboard exige `?year=&month=`.
Incidente: `GloryTmpSweep` purgó `C:\tmp\glory-target\coolify-manager` a mitad de
matriz → rebuild 6m31s + copia a salvo en
`C:\Users\Owner\AppData\Local\Temp\opencode\coolify-manager.exe`
(`muestrear.mjs` la usa; override `COOLIFY_MANAGER_EXE`).
Pendiente usuario: repo a privado; destino staging (C.5); push del cierre.**

- [x] C.1 Durante cada run, muestrear cada 5 s: `container-stats` (app + postgres),
      más `GET /api/health` como testigo de saturación.
      (Muestreo con `docker stats` app+pg; health OK entre runs; Tmax-sost parcial:
      103 muestras ok por purga del binario — ver incidente.)
- [x] C.2 Métricas a capturar: CPU % (media/pico), RSS app y postgres (media/pico),
      p50/p95 por endpoint, req/s sostenidos, tasa de error, tamaño BD, latencia p95
      del health bajo carga. (Todo en `scripts/perf/resultados/` + informe.)
- [x] C.3 Criterio de dimensionado **por tier**: un tier es viable si en `sostenido`
      p95 < 800 ms, errores < 1 % y cero OOM; recomendación = tier viable más pequeño
      × 1.8 de headroom. Tabla comparativa T1/T2/Tmax en el informe. Si ni Tmax pasa
      `pico` → hay cuello de botella: se abre tarea de mejora (no se optimiza
      a ciegas en este plan).
      (Los 3 tiers pasan: T1-sost p95 199 ms, 0 errores, 0 OOM. Recomendación T1;
      headroom ×1.8 sugiere 2 vCPU si el pico crece, pero con este perfil T1 basta.)
- [x] C.4 Informe en `Agente/completados/tareas-YYYY-MM-DD.md` + tabla resumen en roadmap.
- [ ] C.5 Limpieza: `stop`/`delete` del staging (autorización 3) o conservarlo como
      entorno perf permanente (decisión del usuario). **PENDIENTE.**

## 5. Adaptaciones de coolify-manager (solo si hacen falta)

1. Si `container-stats` no sirve para muestreo continuo → añadir `--watch/--interval`
   (cambio local en `../coolify-manager-rs`, PR propio, recompilar a `C:\tmp`).
2. Si `new --template rust` no resuelve este repo (`repo-url`/`app-bin`/`frontend-dir`
   ya existen como flags: ver skill §2a) → nada que adaptar.
3. Si falta `seed` remoto → ya existe en la imagen; solo `exec`. Sin adaptación.
4. Regla: primero usar lo que hay; adaptar solo ante bloqueo verificado.

## 6. Opciones (para decidir ahora)

- **Alcance A (recomendado):** solo standalone, cero BDP. Riesgo nulo, mide el 100 %
  del path local (que es donde está casi todo el producto).
- **Alcance B:** A + mock BDP local (stub HTTP en la VPS que imita latencia/respuestas
  Weblink con datos falsos). Mide el coste del path de integración (serialización,
  cola, poller) sin contactar el BDP real. +1 día.
- **Alcance C:** B + 2-3 lecturas reales puntuales autorizadas (solo para calibrar la
  latencia real BDP↔VPS, p. ej. `article-stock`). Requiere autorización explícita por
  operación; sigue sin escribir nada.
- **Dónde:** VPS-staging (preciso, recomendado) vs. local primero + VPS después
  (más lento, doble trabajo; útil si el push se retrasa).

## 7. Autorizaciones que pediré (nada se ejecuta sin ellas)

1. `push` de `main` (42 commits) o commit a desplegar — si no, staging con código viejo.
2. Crear servicio `restaurante-perf` en la VPS (`new`).
3. `deploy` + `sync-env` + `exec seed` en ese servicio.
4. (Solo alcance C) lecturas reales puntuales, una a una.
5. Destino final del staging (borrar o conservar).

## 8. DoD

Staging levantado por manager con evidencia; harness versionado; informe con
p50/p95, CPU/RAM media+pico y recomendación de recursos; decisión registrada
(mejorar rendimiento sí/no + qué); roadmap actualizado; staging limpio o
conservado por decisión explícita.
