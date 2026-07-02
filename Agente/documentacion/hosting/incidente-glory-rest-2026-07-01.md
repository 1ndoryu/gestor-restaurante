# Incidente glory-rest — Pérdida de datos por regeneración de compose
> **Fecha del incidente:** 2026-07-01  
> **Detectado:** 2026-07-02  
> **Sitio afectado:** restaurante.wandori.us (glory-rest)  
> **UUID Coolify:** `b8s0cks444o0sogo8kg8wcgw`  
> **Gravedad:** CRÍTICA — pérdida total de datos de usuario  

---

## 1. Resumen

El 1 de julio de 2026, Coolify regeneró el `docker-compose.yml` de glory-rest, cambiando las credenciales de PostgreSQL de `glory_app/glory` a `rust_app/rust_db`. Al reiniciar el contenedor postgres, se creó una nueva base de datos vacía `rust_db` y la base original `glory` (con todos los datos del cliente) quedó huérfana. Los backups automáticos ya estaban vacíos porque se instalaron después del incidente.

**Datos perdidos:** Todos los datos del restaurante del cliente "Wan" — reservas, clientes, ventas, configuración, planos de sala, campañas de marketing.

---

## 2. Cronología

| Fecha (UTC) | Evento |
|-------------|--------|
| **2026-06-15** | Primer deploy de glory-rest. Compose original: `POSTGRES_USER=glory_app`, `POSTGRES_DB=glory`, `POSTGRES_PASSWORD=zsh7Lgdn...`. Seed inserta datos demo + datos reales del cliente. |
| **2026-06-15 → 2026-07-01** | El sitio funciona normalmente. El cliente usa la app, crea reservas, clientes, ventas. |
| **2026-07-01 ~06:02 UTC** | **Coolify regenera** el `docker-compose.yml` del stack. Causa probable: resync, re-save desde UI, o regeneración automática. Las credenciales cambian a `rust_app/rust_db` (estándar del template). |
| **2026-07-01 ~06:14 UTC** | Contenedor postgres recreado. Volume persiste pero `POSTGRES_DB=glory` ya no existe → postgres crea automáticamente `rust_db` (vacía). |
| **2026-07-01 ~06:14+** | App se conecta a `rust_db` (vacía). Migraciones SQLx corren sobre la DB vacía. Seed se ejecuta e inserta datos demo. Los datos originales en `glory` quedan huérfanos. |
| **2026-07-01 → 2026-07-02** | Backups automáticos diarios se ejecutan, pero respaldan `rust_db` (ya vacía). |
| **2026-07-02** | Cliente reporta: "no puedo entrar en la página del restaurante". Agente investiga y descubre la pérdida. |

---

## 3. Causa raíz

### Mecanismo del fallo

```
Compose original (junio 15):
  POSTGRES_USER: glory_app
  POSTGRES_DB: glory
  DATABASE_URL: postgres://glory_app:{pw}@postgres:5432/glory

Compose regenerado (julio 1):
  POSTGRES_USER: rust_app
  POSTGRES_DB: rust_db
  DATABASE_URL: postgres://rust_app:{pw}@postgres:5432/rust_db

Al recrear el contenedor postgres con POSTGRES_DB=rust_db:
  1. El volume persiste (datos de "glory" intactos en disco)
  2. Pero postgres crea automáticamente "rust_db" (la de POSTGRES_DB)
  3. La app se conecta a "rust_db" → vacía → migraciones → seed
  4. La DB "glory" queda huérfana (no en pg_database activo)
```

### ¿Por qué Coolify cambió el compose?

Coolify puede regenerar el `docker_compose_raw` (y el procesado `docker_compose`) en varias situaciones:
- **Re-sync** desde la UI (Settings → Reconfigure)
- **Template update** si el stack fue creado con un template que luego cambió
- **API PATCH** con `docker_compose_raw` incorrecto
- **Edición manual** en la UI de Coolify

En este caso, el compose pasó de tener credenciales personalizadas (`glory_app/glory`) al estándar del template (`rust_app/rust_db`). Esto sugiere una regeneración automática o un re-sync.

### ¿Por qué no se detectó antes?

1. `coolify-manager-rs` solo reescribe 4 claves en el compose (`REPO_URL`, `BRANCH`, `APP_BIN`, `SERVICE_FQDN_APP`) — **no toca credenciales**. Si el compose ya estaba corrupto, el manager lo deployearía sin detectar el cambio.
2. No existía ningún guard que comparara las credenciales entre deploys.
3. Los backups automáticos se instalaron **después** del incidente, así que respaldaban la DB vacía.

---

## 4. Datos perdidos

| Tabla | Contenido estimado |
|-------|--------------------|
| `reservas` | Reservas del restaurante (histórico + futuras) |
| `clientes` | Base de datos CRM de clientes |
| `ventas` | Registro de ventas |
| `gastos` | Registro de gastos |
| `mesas` / `zonas` | Configuración del plano de sala |
| `campanas_marketing` | Campañas de marketing |
| `plantillas_whatsapp` | Plantillas de mensajes |
| `configuracion_restaurante` | Settings del restaurante |
| `trabajadores` | Cuentas de empleados |
| `notificaciones` | Notificaciones pendientes |
| `asignaciones_etiquetas` | Tags de clientes |
| `reglas_recordatorio` | Recordatorios automáticos |

**Estado:** No recuperable. Los datos originales en la DB `glory` no están en `pg_database`. El directorio `base/` del OID correspondiente fue modificado el 1 de julio. Los backups existentes ya contenían la DB vacía.

---

## 5. Acciones tomadas

### Inmediatas (2026-07-02)
1. **Seed ejecutado** en glory-rest → restauró datos demo (`demo@restaurante.com` / `demo1234`)
2. **Verificación de otros servicios:**
   - studio: `rust_db` ✅ (consistente con template)
   - kamples: `kamples` ✅ (custom, no afectado)
3. **Cliente informado** de la pérdida y credenciales demo proporcionadas

### Preventivas (2026-07-02)
4. **E19 implementado** — Credential Drift Detection en `sync_compose()`
5. **E20 implementado** — Database Existence Verification en `ensure_postgres_auth_and_hostname()`
6. **Compilado, testeado y pusheado** a `github.com/1ndoryu/coolify-manager-rs` (commit `ea16100`)

---

## 6. Guardas implementadas (E19 + E20)

### E19 — Credential Drift Detection

**Archivo:** `src/commands/deploy_service.rs` → `sync_compose()`  
**Función:** `validate_postgres_creds_stable(current_compose, desired_compose, site_name)`

Antes de enviar el compose a Coolify vía PATCH:
1. Extrae `POSTGRES_USER` y `POSTGRES_DB` del compose actual (en Coolify)
2. Extrae `POSTGRES_USER` y `POSTGRES_DB` del compose que se va a deployear
3. Si difieren → **aborta** con error que explica el riesgo
4. También verifica coherencia entre `DATABASE_URL` y `POSTGRES_USER`/`POSTGRES_DB`

**Helpers:**
- `extract_postgres_env_from_compose(compose)` — parsea YAML del servicio postgres
- `extract_database_url_from_compose(compose)` — parsea `DATABASE_URL` con regex

### E20 — Database Existence Verification

**Archivo:** `src/commands/deploy_service.rs` → `ensure_postgres_auth_and_hostname()`

Antes de ejecutar `ALTER USER ... WITH PASSWORD`:
1. Ejecuta `SELECT 1 FROM pg_database WHERE datname = '{db_name}'` en el contenedor
2. Si la DB no existe → **aborta** e informa qué bases sí existen
3. Solo procede con ALTER USER si la DB objetivo está confirmada

**Cobertura:**
- E19 detecta cambios hechos **por coolify-manager-rs** (compose drift entre deploys)
- E20 detecta cambios hechos **por cualquier mecanismo** (Coolify UI, API directa, regeneración automática)

---

## 7. Lecciones aprendidas

### Reglas nuevas

1. **POSTGRES_USER y POSTGRES_DB nunca deben cambiar** una vez que un stack tiene datos. Si el template se estandariza, los stacks existentes deben mantener sus credenciales originales.

2. **Coolify puede regenerar compose sin aviso.** No confiar en que el compose on-disk sea estable. Verificar siempre antes de deploy.

3. **Los backups deben instalarse ANTES de poner en producción.** Un backup que se instala después de la pérdida es inútil.

4. **Coolify no respalda bind mounts ni DBs automáticamente.** Es responsabilidad del operador configurar backups.

5. **El seed no es un backup.** El seed inserta datos demo, no datos reales. Si se pierden datos de usuario, el seed no los recupera.

### Mejoras pendientes

- [ ] Verificar que todos los stacks legacy (no solo glory-rest) tengan respaldos activos
- [ ] Implementar backup automático pre-deploy para TODOS los sitios (no solo los que tienen `backup_policy.enabled`)
- [ ] Agregar alerta si un compose tiene credenciales distintas al compose backup local
- [ ] Documentar en `coolify-manager-rs` el procedimiento de recovery cuando un stack pierde su DB

---

## 8. Mapa de servicios y estado

| Servicio | UUID | DB User | DB Name | Estado |
|----------|------|---------|---------|--------|
| studio | `do8k4w8swccwwogoc0os0ck0` | rust_app | rust_db | ✅ OK |
| glory-rest | `b8s0cks444o0sogo8kg8wcgw` | rust_app | rust_db | ⚠️ Datos restaurados con seed demo |
| kamples | `mo4so4440c488g8woow4cow0` | kamples | kamples | ✅ OK |
| guillermo | `owck8sww4ogk8gskgwcsk4w0` | — | WP | ✅ OK |
| padel | `zkcc040cc0scock4kcooowkc` | — | WP | ✅ OK |
| cap | `qgskgw8wwc08o444o08wko8o` | — | WP | ✅ OK |
| wandori | `csoc88c0gw8kc4cwcwosc48s` | — | WP | ✅ OK |
| nakomi | `u00gc8ss4csc4cckkg4g00ks` | — | WP | ✅ OK |
