# Plan: Auditoría Completa del Proyecto — 2026-03-30

## Objetivo
Revisión exhaustiva de backend, frontend, deploy, seguridad y UX. Anotar todos los problemas y corregirlos.

## Metodología
1. Investigar cada capa (backend Rust, frontend React, BD, deploy, producción)
2. Anotar los problemas encontrados clasificados por severidad
3. Corregir en orden: CRÍTICO → ALTO → MEDIO → BAJO

---

## Resultados de la Investigación

### Compilación y Types
- [x] `cargo check` — PASA
- [x] `cargo clippy -- -D warnings` — PASA (0 warnings)
- [x] `npx tsc --noEmit` — PASA (0 errores)
- [x] SQL injection — SEGURO (todos los queries usan sqlx macros con `$1`, `$2`)

### Producción (restaurante.wandori.us)
- [x] Health check — 200 OK
- [x] Login demo — 200 OK (token válido)
- [x] Ventas, Gastos, Reservas, Dashboard, Plano, Clientes — todos 200
- [x] Configuración, Campañas, Recordatorios, Canales, Etiquetas — todos 200
- [x] Notificaciones, API Keys — 200
- [x] Frontend SPA — 200
- [x] Swagger UI — 200
- [x] Secrets en .gitignore — ✅ .env excluido

---

## Problemas Encontrados

### 🔴 CRÍTICOS (rompen seguridad/funcionalidad)

#### C1 — CORS `allow_origin(Any)` en producción
- **Archivo:** `src/handlers/mod.rs` L306-309
- **Problema:** CORS completamente abierto permite a cualquier sitio web hacer requests autenticados
- **Fix:** Restringir a orígenes permitidos (restaurante.wandori.us, localhost:5173 en dev)

#### C2 — Roadmap URLs obsoletas (timeout/404)
- **Archivo:** `roadmap.md` L3-4
- **Problema:** URLs `http://app-b8s0cks444o0sogo8kg8wcgw.IP_VPS.sslip.io` (404) y `http://IP_VPS:3001` (timeout) no funcionan. Solo `http://restaurante.wandori.us` funciona
- **Fix:** Actualizar roadmap con URL correcta

### 🟠 ALTOS (afectan confiabilidad)

#### A1 — Raw `fetch()` bypassing axios interceptors
- **Archivos:**
  - `frontend/src/componentes/ErrorBoundary.tsx` L27 — `/api/reportar-error`
  - `frontend/src/componentes/Configuracion.tsx` L28 — `/api/admin/*`
  - `frontend/src/componentes/plano-sala/usePlanoSala.ts` L192 — `/api/plano-sala/export`
  - `frontend/src/components/app-sidebar.tsx` L76 — `/api/reportar-error`
- **Problema:** Sin JWT interceptor, sin 401 redirect, sin baseURL config
- **Fix:** Reemplazar con axios instance o Orval hooks

#### A2 — Admin endpoints sin role validation
- **Archivo:** `src/handlers/admin.rs` L33-86
- **Problema:** Cualquier usuario autenticado puede ejecutar seed/reset. El seed crea datos para el demo user, no para el caller. Mitigado por single-tenant pero inseguro si se agrega otro usuario
- **Fix:** Validar que el user_id sea el demo user, o agregar flag `is_admin` al user

#### A3 — Sin graceful shutdown
- **Archivo:** `src/main.rs`
- **Problema:** SIGTERM no cierra conexiones de pool ni detiene scheduler
- **Fix:** Agregar `tokio::signal::ctrl_c()` + `axum::serve().with_graceful_shutdown()`

### 🟡 MEDIOS (mejoras importantes)

#### M1 — Token JWT en query param para SSE
- **Archivos:** `frontend/src/hooks/useNotificaciones.ts` L38, `src/handlers/notificaciones.rs` L135
- **Problema:** Token visible en logs del servidor, browser history, DevTools
- **Mitigación:** Token expira en 24h, es HTTPS en producción, documentado como limitación de EventSource
- **Fix factible:** No hay alternativa limpia con EventSource. Documentar riesgo

#### M2 — Sin rate limiting en login/forgot-password
- **Archivo:** `src/handlers/auth.rs`
- **Problema:** Se puede bruteforcear login o spamear forgot-password emails
- **Fix:** Agregar tower::limit::RateLimitLayer o middleware custom

#### M3 — Sin timeout en SMTP
- **Archivo:** `src/services/email.rs`
- **Problema:** Conexión SMTP puede colgar indefinidamente si el server no responde
- **Fix:** `.timeout(Duration::from_secs(30))`

#### M4 — Scheduler sin deduplicación
- **Archivo:** `src/main.rs` (scheduler loop)
- **Problema:** Si ejecutar_ciclo tarda >60s, ciclos se solapan
- **Fix:** Agregar Mutex o flag de "executing"

#### M5 — usePlanoSala export sin validar response.ok
- **Archivo:** `frontend/src/componentes/plano-sala/usePlanoSala.ts` L192-200
- **Problema:** Hace `resp.json()` sin verificar status → parse error en 5xx
- **Fix:** Agregar `if (!resp.ok) throw`

#### M6 — Error handling genérico en useNotificaciones SSE
- **Archivo:** `frontend/src/hooks/useNotificaciones.ts` L48
- **Problema:** JSON parse error silenciado completamente (`catch { /* ignorar */ }`)
- **Fix:** Logear con `console.warn()` al menos

### 🟢 BAJOS (limpieza/convenciones)

#### B1 — Frontend: directorio `componentes/` debería ser inglés
- **Archivo:** `frontend/src/componentes/`
- **Violación:** Regla 9 del protocolo (nombres en inglés)
- **Fix:** Renombrar no recomendado ahora (toca 30+ imports). Documentar deuda técnica

#### B2 — Frontend: directorio `estilos/` debería ser inglés
- **Archivo:** `frontend/src/estilos/`
- **Fix:** Mismo caso que B1

#### B3 — `unwrap()` en `turno_a_rango` handlers/plano_sala.rs
- **Archivo:** `src/handlers/plano_sala.rs` L307-316
- **Invariante probada:** `NaiveTime::from_hms_opt(7,0,0)` siempre es Some → safe
- **Fix:** Cambiar a `.expect("hora literal válida")` para documentar la invariante

#### B4 — Frontend: notificaciones count endpoint inconsistente
- **Archivo:** `frontend/src/hooks/useNotificaciones.ts` L24
- **Problema:** Hace GET `/api/notificaciones/count` pero backend define como `/api/notificaciones/contar`
- **Verificar:** Confirmar la ruta real del endpoint en backend

---

## Orden de Ejecución
1. C1 — CORS (seguridad crítica)
2. C2 — Roadmap URLs
3. A1 — Raw fetch → axios
4. A2 — Admin role check
5. A3 — Graceful shutdown
6. M1-M6 — Mejoras medias
7. B1-B4 — Limpieza

## Estado
- [x] Investigación completada
- [x] C1 — CORS restringido (CORS_ORIGINS env var)
- [x] C2 — Roadmap URLs actualizadas
- [x] A1 — Raw fetch → axios (4 archivos)
- [x] A2 — Admin DEMO_MODE check
- [x] A3 — Graceful shutdown + pool close
- [x] M3 — SMTP timeout 30s
- [x] M4 — Scheduler mutex anti-solapamiento
- [x] M5 — usePlanoSala export usa axios (ya no necesita validar response.ok)
- [x] M6 — SSE parse error logueado con console.warn
- [x] B3 — unwrap → expect en turno_a_rango
- [ ] M1 — Token SSE en query param (limitación EventSource, documentado)
- [ ] M2 — Rate limiting login/forgot-password (pendiente)
- [ ] B1/B2 — Renombrar carpetas españolas (deuda técnica, no prioritario)
