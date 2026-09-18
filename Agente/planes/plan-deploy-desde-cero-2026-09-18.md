# Plan — Deploy a producción desde cero (BD limpia + env + usuario seguro)

> **Fecha:** 2026-09-18
> **Estado:** plan, sin ejecutar — cada fase con escrituras remotas requiere autorización explícita
> **Servicio:** `glory-rest` (`restaurante.wandori.us`, Coolify `my-first-project/production`)
> **Fuente base:** plan 2026-08-08 (`plan-deploy-produccion-intuitividad-2026-08-08.md` §5) + hallazgos 169A-2/149A-2

## Punto de partida verificado hoy

- Rama `main`, 3 commits por subir (`f4f76a9`, `6aaf77e`, `d9f9b99`).
- 148 migraciones SQL; el backend las aplica solo al arrancar (`sqlx::migrate!().run`, `src/main.rs:35`).
- Producción actual construye desde `glory-rs` (`temp-compose-glory-rest.yml`: `Dockerfile.rust` + `REPO_URL glory-rs.git`): el deploy debe apuntar el build a este repo (o confirmar que Coolify ya toma esta rama).
- `coolify-manager` **sin compilar** (única vía autorizada para operar producción).
- `C:\tmp` al 61% del techo (4,28/7 GB): hay que purgar antes de compilar.
- Registro público **siempre abierto** (`POST /api/auth/register`, sin flag para cerrarlo): el primer usuario sin trabajador es `Admin`.
- `.env` local verificado 2026-09-18 (solo presencia/longitud): `BDP_LOGIN`/`BDP_PASSWORD`/`BDP_INTEGRATOR_CODE` presentes, `BDP_BASE_URL` 25 chars, POS/employee/profile/article presentes. **OJO: `BDP_WRITE/CHECK_ORDER_ALLOWED_ORIGINS` SÍ están definidos en local (25 chars = el BDP)** — a producción van **vacíos** en F2; jamás copiar el `.env` tal cual. `CORS_ORIGINS`/`APP_URL` ausentes en local — definir en prod. `BDP_BOOTSTRAP_USER_EMAIL` local apunta al demo — en prod será el email del nuevo admin.
- Admin decidido: email `restaurante@nakomi.studio` (el usuario escribió `nakomi..studio` con doble punto — asumido typo, **confirmar antes de F5**), nombre `restaurante`, contraseña fuerte de 24 chars (mínimo del validador: 8, `src/models/user.rs:40`) generada en F5 y entregada una sola vez.

## Guardarraíles VPS (otros proyectos en producción) — lectura obligatoria antes de F0

> Riesgo real: el VPS alberga los servicios de varios proyectos bajo el mismo Coolify.
> Todo lo que sigue es para que ni un comando pueda salpicar a otro servicio.

### F0.0 — Inventario de solo-lectura (primer paso de la ejecución, cero escrituras)
1. Listar servicios del VPS (`list`/equivalente) y mostrar la tabla completa: nombre, dominio, estado.
2. Confirmar que `glory-rest` = `restaurante.wandori.us` = `serviceId 14` = contenedores `app-b8s0cks444o0sogo8kg8wcgw` + `postgres-b8s0cks444o0sogo8kg8wcgw`.
3. Ese inventario se te enseña y solo se sigue con tu visto bueno. Sin inventario verificado, no hay F1.

### Alcance acotado por nombre y por UUID
- Los recursos de restaurante llevan prefijo propio en todo: contenedores `*-b8s0cks444o0sogo8kg8wcgw`, volúmenes `b8s0cks444o0sogo8kg8wcgw_*` (`pg-data`, `app-data`, `uploads-data`), red `b8s0cks444o0sogo8kg8wcgw`. Cualquier comando que no mencione ese UUID o `--name glory-rest` no se ejecuta.
- El borrado de BD entra SOLO al contenedor `postgres-b8s0cks444o0sogo8kg8wcgw` (BD `rust_db`). Las BD ajenas viven en otros contenedores/volúmenes y no se listan ni se tocan.

### Prohibiciones explícitas en el VPS
- `restart --all` y cualquier variante `--all`: PROHIBIDO (además deja workloads Rust en `exited`).
- `prune` de cualquier tipo (contenedores, volúmenes, redes, imágenes, sistema): PROHIBIDO. Limpieza solo por nombre exacto de recurso propio.
- `exec`/shell en contenedores que no sean los dos de `glory-rest`: PROHIBIDO.
- Cambios de red Traefik globales: no se tocan; solo las labels del compose propio.
- SSH/Docker/curl directos al VPS: PROHIBIDO (solo `coolify-manager-rs`, que audita por servicio).

### Carga durante el build
- El build Docker corre en el VPS: compilar Rust+frontend puede picar CPU/RAM/disco. Mitigación: ejecutar en horario valle del restaurante, vigilar `health` de los vecinos antes y después, y abortar si algún servicio ajeno cambia de estado. Si el disco del VPS está justo, avisar antes de construir.

## Guardarraíles anti-accidentes (local y alcance) — siguen vigentes

> Contexto: lo que se cae "siempre" es por alcance implícito. Aquí todo alcance es explícito.
>
> 1. **`C:\tmp` es compartido por todos los proyectos.** La purga toca SOLO directorios objetivo verificados uno a uno (último acceso >24h + rama git inexistente o ajena confirmada). Jamás `glory-target` entero ni dirs con actividad reciente. Peor caso de error: un rebuild ajeno, nunca una caída (las producciones ajenas viven en remoto, no en `C:\tmp`).
> 2. **Toda operación remota lleva `--name glory-rest` explícito.** Prohibido `--all`, `restart --all` y cualquier comando sin `--name`. Antes del primer write: `list`/`status` de solo-lectura confirmando que `glory-rest` = `restaurante.wandori.us`.
> 3. **Reset de BD acotado al contenedor `postgres-b8s0cks444o0sogo8kg8wcgw`.** Antes del `DROP`, inspección read-only del nombre exacto del contenedor y de la BD (`rust_db`, usuario `rust_app`). Cada proyecto tiene sus propios contenedores/volúmenes (`b8s0cks444o0sogo8kg8wcgw_*`); no se toca ningún otro volumen.
> 4. **`sync-env` solo a `glory-rest`**, variable por variable según tabla F2 (nunca volcado del `.env` local: las allowlists locales irían escritas y el JWT local se reutilizaría).
> 5. **Git:** solo repo `RESTAURANTE`, solo rama `main`, push con rebase y `status` previo. Nada de otros repos.
> 6. **Red:** no se toca Tailscale ni firewall ni DNS en ninguna fase.
> 7. **Backend local `:3100`:** se detiene antes de F3 para no confundir verificaciones (es proceso local, sin efecto en producción).
> 8. **Doble confirmación humana:** F3 (borrado) no se ejecuta sin backup verificado + tu "sí" explícito en ese momento, aunque hayas autorizado el plan general.

## Fases

### F0 — Preflight (sin escrituras)
1. Purgar `C:\tmp` (targets viejos) y compilar `coolify-manager-rs` a `C:\tmp`.
2. `health --name glory-rest` + estado del servicio (lectura).
3. **Backup completo de la BD de producción** vía `coolify-manager-rs` (obligatorio antes de borrar nada).
4. BDP online en Tailscale (`100.83.196.35:8068`).
5. Recoger secretos del usuario (ver tabla): nunca van a git ni a logs.

### F1 — Subir código
- `git pull --rebase` + push de los 3 commits a `main` (requiere autorización: push).

### F2 — Variables de producción (vía `sync-env`, sin allowlists de escritura)
| Variable | Valor |
| --- | --- |
| `DATABASE_URL` | la gestiona Coolify (postgres `rust_db`, usuario `rust_app`) — no tocar |
| `JWT_SECRET` | **nuevo, aleatorio 64 chars** (nunca reutilizar el de local) |
| `BDP_BASE_URL` | `http://100.83.196.35:8068` (confirmar) |
| `BDP_LOGIN` / `BDP_PASSWORD` / `BDP_INTEGRATOR_CODE` | aportar (secretos del restaurante) |
| `BDP_POS_ID=31`, `BDP_EMPLOYEE_ID=1`, `BDP_ITEMS_PROFILE_ID=1` | confirmar |
| `BDP_DEFAULT_ARTICLE_CODE=1001`, `BDP_DEFAULT_ARTICLE_NAME=CAFE BOMBON` | confirmar |
| `BDP_BOOTSTRAP_USER_EMAIL` | email del admin que se creará en F5 |
| `BDP_WRITE_ALLOWED_ORIGINS` / `BDP_CHECK_ORDER_ALLOWED_ORIGINS` | **vacías** (modo seguro: cero escrituras hasta validación) |
| `DEMO_MODE` | **no definir** (los endpoints seed/reset quedan apagados) |
| `CORS_ORIGINS` / `APP_URL` | `https://restaurante.wandori.us` |

### F3 — Reiniciar la BD (DESTRUCTIVO, con backup de F0 verificado)
1. Parar `app`, vaciar esquema (`DROP SCHEMA public CASCADE; CREATE SCHEMA public;` vía `exec` en postgres) o recrear volumen `..._pg-data`.
2. `deploy --name glory-rest --update` → el arranque aplica las 148 migraciones (esquema cero).
3. `health` + `logs`: confirmar "Bootstrap BDP dirigido" y ceros (`bdp_sync_enabled=false`, modo `read_only`).

### F4 — Verificar base
- `GET /api/health` 200, login imposible aún (cero usuarios), `sync-dry-run` verde, portada del front carga.

### F5 — Usuario seguro (inmediato tras F4: el registro está abierto a todo el mundo)
1. `POST /api/auth/register` con email admin + contraseña fuerte (generada, ≥20 chars).
2. Fijar `BDP_BOOTSTRAP_USER_EMAIL` a ese email (dirige el bootstrap BDP a su cuenta).
3. Login de prueba + logout. Guardar credenciales en el gestor del restaurante, no en chat ni repo.
4. **Seguimiento:** el registro sigue abierto — tarea aparte: flag para cerrarlo o ligar registro a `Admin`.

### F6 — Conexión BDP en modo seguro
1. Smoke de lectura (preflight, catálogo) con allowlists aún vacías.
2. Solo tras validación contigo: activar allowlists + `bdp_sync_enabled` (eso ya es otro bloque, no este deploy).

## Rollback
- Si `health` falla: rollback automático E11 de `coolify-manager-rs` (recrea compose anterior).
- Si hay que volver a los datos viejos: restaurar backup de F0 (procedimiento del manager, se documenta al ejecutar).

## Riesgos
| Riesgo | Mitigación |
| --- | --- |
| Borrar la BD sin backup válido | F0.3 bloquea F3; verificar backup antes |
| Build actual apunta a `glory-rs`, no a este repo | Confirmar origen del build en F0; si apunta mal, corregir compose primero |
| Registro abierto entre F4 y F5 | Ventana mínima: F5 corre en el mismo bloque que F4 |
| `C:\tmp` lleno al compilar | Purgar en F0 (techo 7 GB) |
| BDP caído durante el deploy | No bloquea: el deploy deja todo en lectura; la conexión se valida en F6 |
