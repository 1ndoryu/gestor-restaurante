# Incidente glory-rest — 503 Traefik + Rollback destructivo + SSH timeout
> **Fecha del incidente:** 2026-07-21
> **Sitio afectado:** restaurante.wandori.us (glory-rest)
> **UUID Coolify:** `b8s0cks444o0sogo8kg8wcgw`
> **VPS:** 66.94.100.241 (Tailscale 100.91.173.54)
> **Gravedad:** ALTA — sitio caído, rollback destruyó contenedor
> **Estado:** PARCIALMENTE RESUELTO — contenedor restaurado, causa raíz del 503 identificada, fix pendiente

---

## 1. Resumen

Deploy de BDP (Branch `glory-rs-rest`) a producción. El build de Rust completó exitosamente (485s), el swap del contenedor se ejecutó, pero el health check devolvió HTTP 503. El rollback automático falló porque `docker compose up` del compose anterior tuvo exit code 1, **destruyendo el contenedor sin recuperación**. Subsecuentemente se descubrió que la causa raíz del 503 es la **falta del label `traefik.docker.network=coolify`** en el compose de glory-rest, y que `rewrite_rust_service_compose()` no verifica ni inserta este label.

---

## 2. Cronología

| Hora (UTC) | Evento |
|---|---|
| ~12:05 | Inicio deploy `deploy-service --name glory-rest --skip-backup`. |
| ~12:06 | SSH timeout durante build. **Causa:** russh sin TCP keepalive. Sesión se cierra por inactividad. |
| ~12:10 | Fix implementado: `Config.keepalive_interval(60s)` en `ssh_client.rs`. Binary recompilado. |
| ~12:12 | Reintento deploy. Build completa en 485s. Swap ejecutado. |
| ~12:18 | Health check → **HTTP 503**. App reports `app_ok=true` (contenedor corriendo, app escuchando en :3000) pero Traefik no puede enrutar. |
| ~12:18 | Rollback automático ejecuta: restaura compose anterior via API, `docker compose up --no-build` → **exit 1**. Contenedor destruido. |
| ~12:18-12:20 | Diagnóstico: `curl` directo a VPS:80 → 404, VPS:443 → 503 "no available server". App responde OK desde IPs internas del contenedor (10.0.1.9, 10.0.10.4). |
| ~12:20-12:30 | Mejoras al rollback: wait 10s post-API, re-ejecución de hostname fix, 3 intentos (no-build → build → API deploy). |
| ~12:38 | Deploy con tool mejorado (`--skip-build`). Mismo 503. Rollback esta vez SÍ funcionó (contenedor restaurado). |
| ~12:45 | **Causa raíz identificada:** compose de glory-rest NO tiene `traefik.docker.network=coolify` en labels. `rewrite_rust_service_compose()` solo reescribe 4 claves, no labels Traefik. |

---

## 3. Causa raíz

### 3.1 — 503 de Traefik: Label `traefik.docker.network` faltante

El compose de glory-rest (creado antes de la regla `[235A-4]`) **no tiene** el label:
```yaml
- "traefik.docker.network=coolify"
```

**¿Por qué causa 503?**
- Traefik descubre contenedores Docker por labels.
- Cuando un contenedor está en múltiples redes Docker, Traefik necesita saber cuál usar para comunicarse con él.
- Sin `traefik.docker.network=coolify`, Traefik puede intentar usar la red interna del stack (`b8s0cks444o0sogo8kg8wcgw_default`) donde la IP del contenedor es diferente.
- Resultado: Traefik tiene el router configurado (Host rule OK) pero el servicio tiene **0 servidores disponibles** → 503 "no available server".

**Evidencia:**
```
# curl directo al VPS con Host header → 503
< HTTP/1.1 503 Service Unavailable
Content-Length: 20   ← "no available server"

# curl desde dentro del contenedor → 200 OK
{"status":"ok","version":"0.1.0"}
```

**¿Por qué no estaba el label?**
- `rewrite_rust_service_compose()` solo reescribe 4 campos (`REPO_URL`, `BRANCH`, `APP_BIN`, `SERVICE_FQDN_APP`) + reglas `Host()`.
- **No verifica ni inserta labels Traefik.** Si el compose original no tenía el label (sitio creado antes de `[235A-4]`), nadie lo agrega.
- El template nuevo (`config/templates/rust-stack.yaml`) sí lo tiene, pero sitios existentes nunca se re-renderizan desde el template.

### 3.2 — Rollback destructivo: 3 bugs

**Bug R1 — Sin espera post-API:**
`update_stack_compose()` actualiza la API de Coolify, pero el compose on-disk se regenera **asíncronamente**. El rollback ejecutaba `docker compose up` inmediatamente → usaba el compose PRE-SWAP (nuevo, con 503) en vez del restaurado.

**Bug R2 — Sin fix de hostname postgres:**
El compose backup tiene `@postgres:` genérico (legacy). Tras restaurarlo, la app no puede conectar a la BD porque `postgres` resuelve al `coolify-db` equivocado en la red `coolify`. `ensure_postgres_auth_and_hostname()` no se re-ejecutaba en rollback.

**Bug R3 — Sin fallback:**
Si `docker compose up --no-build` falla (imagen podada, compose corrupto), el rollback abortaba sin intentar rebuild ni redeploy via API. Resultado: contenedor destruido, sitio caído.

### 3.3 — SSH timeout durante build

**Causa:** La biblioteca `russh` no tenía TCP keepalive configurado. Durante la compilación silenciosa de Cargo (~10 min sin output), el servidor SSH cerró la sesión por inactividad.

**Código problemático (antes):**
```rust
let config = client::Config::default();
// Sin keepalive → sesión muere tras ~5 min de silencio
```

**Fix aplicado:**
```rust
let mut config = client::Config::default();
config.keepalive_interval = Some(std::time::Duration::from_secs(60));
config.keepalive_max = 3;
```

---

## 4. Fixes implementados

### 4.1 — SSH keepalive (`ssh_client.rs`) ✅
- `Config.keepalive_interval = 60s`, `keepalive_max = 3`
- `execute_long_running()`: reconnection on failure, max 5 consecutive failures before abort, heartbeat every 120s
- Commit: `0a2cb2a`

### 4.2 — Rollback robusto (`deploy_service.rs`) ✅
- **R1:** Sleep 10s post-API para que Coolify regenere compose on-disk
- **R2:** Re-ejecución de `ensure_postgres_auth_and_hostname()` tras restaurar compose
- **R3:** 3 intentos de recuperación:
  1. `docker compose up -d --no-build --force-recreate --no-deps` (rápido)
  2. `docker compose up -d --force-recreate --no-deps` (con rebuild, por si imagen podada)
  3. `deploy_stack()` via Coolify API (último recurso, git pull + rebuild completo)
- Nuevo método `CoolifyApiClient::deploy_stack(uuid)` → `POST /api/v1/services/{uuid}/deploy`

### 4.3 — Fix de hostname postgres (`ensure_postgres_auth_and_hostname`) ✅ (ya existía)
- `sed -i 's|@postgres:|@postgres-{uuid}:|g'` corrige hostname en compose on-disk
- Se ejecuta ANTES del swap en deploy normal
- Ahora TAMBIÉN se ejecuta en rollback

---

## 5. Fix pendiente: Label `traefik.docker.network=coolify`

### Problema
`rewrite_rust_service_compose()` no inserta el label `traefik.docker.network=coolify` si el compose original no lo tiene. Sitios legacy (creados antes de `[235A-4]`) quedan sin este label permanentemente.

### Solución propuesta
Añadir verificación/inserción del label en una de estas ubicaciones:

**Opción A (preferida):** En `rewrite_rust_service_compose()` — después de las sustituciones de claves, verificar que el label existe y agregarlo si falta:
```rust
fn rewrite_rust_service_compose(...) -> Result<String, CoolifyError> {
    // ... reemplazos existentes ...
    
    // [235A-4] Asegurar traefik.docker.network=coolify
    if !compose.contains("traefik.docker.network=coolify") {
        compose = compose.replace(
            "- traefik.enable=true",
            "- traefik.enable=true\n      - traefik.docker.network=coolify"
        );
    }
    
    Ok(compose)
}
```

**Opción B:** En `validate_compose_before_deploy()` — como warning E18+ que sugiera agregar el label.

**Opción C:** En `ensure_traefik_connected()` — verificar el label on-disk después de sincronizar.

### Impacto
- Todos los sitios Rust legacy en el VPS (glory-rest, nakomi.studio, etc.) pueden tener este problema.
- Sites nuevos (creados con template actualizado) ya tienen el label.
- El fix en `rewrite_rust_service_compose()` lo resolvería para todos los deploys futuros.

---

## 6. Lecciones aprendidas

1. **El rollback debe ser más robusto que el deploy.** Si el deploy falla, el rollback es la última línea de defensa. No puede fallar con un simple `docker compose up` sin fallback.

2. **Labels Traefik son configuración crítica.** Un label faltante = sitio caído. `rewrite_rust_service_compose()` debe verificar TODOS los labels obligatorios, no solo los 4 campos que reescribe.

3. **Coolify regenera compose on-disk asíncronamente.** Después de `update_stack_compose()`, hay que esperar a que el archivo en disco se actualice antes de ejecutar `docker compose up`.

4. **Compose backups preservan el estado incluyendo bugs.** Si el compose original tenía un bug (label faltante), el backup también lo tiene. El rollback debe re-aplicar fixes críticos (hostname, labels) después de restaurar.

5. **SSH keepalive es obligatorio para builds largos.** Cualquier sesión SSH que pueda estar silenciosa más de 5 minutos necesita TCP keepalive.

---

## 7. Acciones de seguimiento

- [x] Implementar fix del label `traefik.docker.network=coolify` en `rewrite_rust_service_compose()`
- [x] Verificar que nakomi.studio y otros sitios Rust legacy tengan el label (inyectado en rewrite)
- [x] Deploy de glory-rest con el fix del label
- [x] Commit + push de todas las mejoras del tool (rollback + SSH keepalive + label fix)
- [x] **Causa raíz #2 CONFIRMADA:** Coolify regenera compose on-disk durante build y borra fixes sed. Fix: re-aplicación post-build de TODOS los fixes antes del swap.
- [ ] Documentar en `Agente/lecciones/lecciones-aprendidas.md`

## 8. Confirmación post-deploy (21 julio 2026)

El deploy de glory-rest con el nuevo tool confirmó la causa raíz #2:

```
traefik.docker.network=coolify no encontrado en compose on-disk, inyectando...
Label traefik.docker.network=coolify inyectado via sed.
Deploy exitoso! https://restaurante.wandori.us/api/health respondiendo (status=Some(200)).
```

Después de los ~8.5 minutos del build Docker, Coolify había regenerado el `docker-compose.yml` on-disk desde el API state, eliminando el label que había sido inyectado en el paso 1. El bloque de re-aplicación post-build lo detectó y lo volvió a inyectar, resultando en un deploy exitoso con status 200.
