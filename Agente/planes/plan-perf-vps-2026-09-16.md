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

- [ ] A.1 `new --name restaurante-perf --template rust` (repo/ramas del proyecto,
      `--app-bin glory-backend`, health `/api/health`). Con `--skip-cache` si el
      template lo pide. **Autorización 1: crear el servicio.**
- [ ] A.2 `sync-env`: `MODO` standalone forzado (`modo_operacion=standalone`),
      `DATABASE_URL` con `postgres-{uuid}` (evita DNS collision 28P01), `PORT`,
      secretos de seed. Sin `BDP_*` reales (base_url vacía o dummy no enrutable).
- [ ] A.3 `deploy --name restaurante-perf` (build Rust ~8-12 min; 503 durante el build
      es normal). **Autorización 2: desplegar.**
- [ ] A.4 `health` + `seed` (vía `exec`: binario `seed` ya incluido en la imagen
      `Dockerfile:56`) + `PATCH /api/configuracion {modo_operacion:standalone}` y
      verificación `bdp_sync_enabled=false`.
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
fijan vía compose del servicio antes de cada run; el tier inicial de A.5 pasa a ser
**Tmax** (techo sin throttling) y luego se baja a T2 y T1 para medir degradación y OOMs.

- [ ] C.1 Durante cada run, muestrear cada 5 s: `container-stats` (app + postgres),
      más `GET /api/health` como testigo de saturación.
- [ ] C.2 Métricas a capturar: CPU % (media/pico), RSS app y postgres (media/pico),
      p50/p95 por endpoint, req/s sostenidos, tasa de error, tamaño BD, latencia p95
      del health bajo carga.
- [ ] C.3 Criterio de dimensionado **por tier**: un tier es viable si en `sostenido`
      p95 < 800 ms, errores < 1 % y cero OOM; recomendación = tier viable más pequeño
      × 1.8 de headroom. Tabla comparativa T1/T2/Tmax en el informe. Si ni Tmax pasa
      `pico` → hay cuello de botella: se abre tarea de mejora (no se optimiza
      a ciegas en este plan).
- [ ] C.4 Informe en `Agente/completados/tareas-YYYY-MM-DD.md` + tabla resumen en roadmap.
- [ ] C.5 Limpieza: `stop`/`delete` del staging (autorización 3) o conservarlo como
      entorno perf permanente (decisión del usuario).

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
