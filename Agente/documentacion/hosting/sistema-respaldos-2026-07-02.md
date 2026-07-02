# Sistema de Respaldos — coolify-manager-rs
> **Fecha:** 2026-07-02  
> **Alcance:** Documentación completa del sistema de backups del manager y del servidor  

---

## 1. Visión general

El sistema de respaldos tiene **tres capas** independientes que se complementan:

```
┌──────────────────────────────────────────────────────┐
│  Capa 1: Manager CLI (coolify-manager-rs)            │
│  Backup/restore completo vía API + SSH                │
│  → DB dumps, uploads, compose, manifests SHA-256      │
├──────────────────────────────────────────────────────┤
│  Capa 2: Cron VPS (backup-server.sh)                 │
│  Backup automático diario de DBs en el servidor       │
│  → Solo bases de datos, sin uploads                   │
├──────────────────────────────────────────────────────┤
│  Capa 3: Compose backups locales                     │
│  Respaldo del docker-compose.yml antes de cada deploy │
│  → Máximo 5 por sitio, en ~/.coolify-manager/        │
└──────────────────────────────────────────────────────┘
```

---

## 2. ¿Qué se respalda?

| Tipo | Contenido | Método |
|------|-----------|--------|
| **Bases de datos** | Dumps SQL completos (PostgreSQL: `pg_dump --no-owner --no-privileges`; MariaDB: `mariadb-dump --single-transaction --routines --triggers`) | `export_database_binding()` / `export_database_binding_to_host()` |
| **Archivos persistentes** | Directorios de `site.backup_policy.source_paths` (ej: `/app/uploads`, `/app/content`) empaquetados como `.tar.gz` | `archive_container_path()` / `archive_container_path_to_host()` |
| **Compose YAML** | Copia del `docker-compose.yml` antes de sobrescribir en deploy | `backup_compose_locally()` en `deploy_service.rs` |
| **Sitios lightweight** | Todo `/srv/hosting/{site}` (código, compose, site.env, contenido) | `create_lightweight_site_backup()` |

**Lo que NO se respalda automáticamente:**
- Bind mounts de Coolify (`/data/uploads/{site}`) — Coolify no los incluye en sus backups
- Volúmenes Docker nombrados — solo se respalda el contenido via `source_paths`
- El binario compilado del servicio Rust — se reconstruye en cada deploy

---

## 3. Disparadores de backup

| Disparador | Cuándo | Comando/Flujo |
|------------|--------|---------------|
| **CLI directo** | Manual desde terminal | `coolify-manager backup --name <sitio> --tier daily/weekly/manual` |
| **Pre-deploy automático** | Antes de cada `deploy-service` (si `backup_policy.enabled`) | Crea backup `Manual` con label `"pre-deploy-service"` |
| **Pre-restore safety** | Antes de restaurar un backup | Crea backup `Manual` con label `"pre-restore"` |
| **Cron VPS** | Diario a las 03:00 UTC | `backup-server.sh` via crontab |
| **Windows Task Scheduler** | Tareas diarias/semanales escalonadas | `schedule-backup` crea tareas con `StartWhenAvailable` |

### Pre-deploy: flujo completo
```
deploy-service ejecuta:
  1. [pre] Backup automático (si enabled y no --skip-backup)
  2. [1/6] Sync compose con Coolify API (con guard E19)
  3. [2/6] ensure_postgres_auth_and_hostname (con guard E20)
  4. [3/6] Build imagen nueva
  5. [4/6] Swap contenedor
  6. [5/6] Health check
  7. [6/6] Seed (opcional)
```

---

## 4. Dónde se almacenan

### Remoto principal (SSH a VPS2)

```
/backups/coolify-manager/
  {site_name}/
    daily/
      {YYYYmmdd_HHMMSS}_{backup_id}.tar.gz
      {YYYYmmdd_HHMMSS}_{backup_id}.tar.gz
    weekly/
      {YYYYmmdd_HHMMSS}_{backup_id}.tar.gz
    manual/
      {YYYYmmdd_HHMMSS}_{backup_id}.tar.gz
```

**Configuración en `settings.json`:**
```json
{
  "backupStorage": {
    "localDir": "backups",
    "remote": {
      "type": "ssh_remote",
      "host": "VPS2_IP",
      "user": "root",
      "sshKey": "~/.ssh/id_rsa",
      "baseDir": "/backups/coolify-manager",
      "directTransferKey": "/root/.ssh/vps2_key"
    }
  }
}
```

### Direct transfer (optimización)

Si `directTransferKey` está configurado, el backup se crea **directamente en VPS1** y se transfiere por SCP a VPS2 a velocidad de datacenter (~100 Mbps), evitando pasar por la conexión doméstica (~2 Mbps). Esto reduce tiempos de 35+ minutos a ~2 minutos para backups grandes.

### VPS server-side (cron)

```
/data/backups/
  {stack_uuid}/
    daily/
      {timestamp}.sql.gz
    weekly/
      {timestamp}.sql.gz
```

### Compose backups locales

```
~/.coolify-manager/compose-backups/
  {site_name}/
    compose-{YYYYmmdd-HHmmss}-{hash}.yml   ← máx. 5 por sitio
```

---

## 5. Políticas de retención

| Tier | Keep (manager) | Keep (lightweight) | Poda automática |
|------|----------------|--------------------|--------------------|
| **Daily** | 2 (configurable: `daily_keep`) | 3 | Sí, elimina los más antiguos |
| **Weekly** | 2 (configurable: `weekly_keep`) | 3 | Sí |
| **Manual** | Sin límite | N/A | No |

**Cron VPS (`backup-server.sh`):**
- 2 daily + 2 weekly por defecto
- Si el dump supera 500MB → clasificado como `heavy`, solo weekly, keep=1
- Verifica ≥1GB de espacio libre antes de empezar

---

## 6. Formato del backup (manifest)

Cada backup del manager empaqueta un `.tar.gz` con:

```
{backup_id}.tar.gz
  manifest.json          ← metadatos + checksums SHA-256
  {site_name}_db.sql.gz  ← dump de base de datos
  uploads.tar.gz         ← archivos de source_paths (si aplica)
```

**`manifest.json`:**
```json
{
  "backup_id": "abc123",
  "site_name": "studio",
  "tier": "daily",
  "created_at": "2026-07-02T03:00:00Z",
  "label": null,
  "artifacts": [
    {
      "kind": "database",
      "filename": "studio_db.sql.gz",
      "checksum_sha256": "a1b2c3...",
      "size_bytes": 1048576
    },
    {
      "kind": "files",
      "filename": "uploads.tar.gz",
      "source_path": "/app/uploads",
      "checksum_sha256": "d4e5f6...",
      "size_bytes": 5242880
    }
  ]
}
```

---

## 7. Flujo de restore

### Restore de sitio Coolify (`restore_backup`)

```
1. Descargar backup remoto a ~/.restore-{backup_id}/
2. Extraer tar.gz, leer manifest.json
3. Validar checksums SHA-256 de cada artifact
4. [Opcional] Crear backup de seguridad ("pre-restore")
5. Para cada artifact:
   - database: ejecutar psql/mysql con el dump
   - files: tar -xzf dentro del contenedor
6. Verificar health del sitio post-restore
7. Si falla → rollback automático con el safety snapshot
```

### Restore de sitio lightweight (`light_restore`)

```
1. Snapshot de seguridad opcional
2. Descargar backup remoto
3. Validar manifest + checksums
4. Subir tar.gz al VPS destino
5. Ejecutar script bash que:
   - docker compose down
   - Limpiar directorio del sitio
   - Extraer tar.gz
   - Recrear usuario SFTP
   - Reescribir Caddyfile
   - Restaurar site.env
   - docker compose up -d
6. Verificar con curl
7. Reportar FQDN + credenciales
```

### Rollback automático (E11)

Si el health check falla post-deploy:
1. Buscar último compose backup en `~/.coolify-manager/compose-backups/{site}/`
2. Restaurar compose anterior vía API Coolify
3. `docker compose up -d --no-build --force-recreate`
4. Re-verificar health

---

## 8. Guardas de seguridad (E19 + E20)

Implementadas el 2026-07-02 como respuesta al incidente glory-rest (pérdida de datos por regeneración de compose).

### E19 — Credential Drift Detection

**Ubicación:** `sync_compose()` en `deploy_service.rs`  
**Cuándo se ejecuta:** Antes de cada PATCH del compose a Coolify

```
Compose actual en Coolify ──→ Extraer POSTGRES_USER, POSTGRES_DB
Compose que se va a deployear ──→ Extraer POSTGRES_USER, POSTGRES_DB

Si difieren → ABORTAR con error:
  "E19: Credenciales PostgreSQL cambiaron en el compose de '{site}'.
   Esto causaría pérdida de datos al crear una base nueva vacía."
```

También verifica coherencia interna: `DATABASE_URL` vs `POSTGRES_USER`/`POSTGRES_DB`.

### E20 — Database Existence Verification

**Ubicación:** `ensure_postgres_auth_and_hostname()` en `deploy_service.rs`  
**Cuándo se ejecuta:** Antes de `ALTER USER` en cada deploy

```
¿Existe la base de datos objetivo en el contenedor postgres?
  SELECT 1 FROM pg_database WHERE datname = '{db_name}'

Si NO existe → ABORTAR con error:
  "E20: Base de datos '{db}' no existe en postgres-{uuid}.
   Bases existentes: {lista}.
   Posible causa: Coolify regeneró el compose con credenciales distintas."
```

### Escenario que previenen

```
1. Deploy inicial: POSTGRES_USER=glory_app, POSTGRES_DB=glory
2. Coolify regenera compose (bug/edición manual/sync):
   POSTGRES_USER=rust_app, POSTGRES_DB=rust_db
3. Deploy siguiente usa el compose corrupto:
   - postgres crea DB "rust_db" (vacía)
   - Migraciones corren sobre la DB vacía
   - Datos originales en "glory" quedan huérfanos (DROP DATABASE)
   
Con E19: el deploy se aborta en paso 3, antes de llegar a Coolify.
Con E20: si E19 falla (compose cambiado por otro mecanismo), el ALTER USER
         aborta porque "rust_db" no existe en el contenedor.
```

---

## 9. Script backup-server.sh (cron VPS)

**Instalación:** `coolify-manager install-backups --server-ip {ip}`  
**Cron:** `0 3 * * *` en el VPS

**Características:**
- **Zero hardcoding**: descubre automáticamente containers `postgres-*` y `mariadb*` en ejecución
- Extrae credenciales de env vars del container (`POSTGRES_USER`, `POSTGRES_DB`, etc.)
- Clasifica tier por día: dom=weekly, lun-sáb=daily
- **Throttle**: dumps >500MB → solo weekly, keep=1
- Organización: `/data/backups/{stack_uuid}/{daily|weekly}/{timestamp}.sql.gz`
- Rotación con `find` + `head -n` basado en mtime
- Verifica ≥1GB de espacio libre antes de empezar

---

## 10. Limitaciones conocidas

| Issue | Impacto | Workaround |
|-------|---------|------------|
| Cron VPS solo respalda DB | Uploads no incluidos en backup automático del servidor | Usar manager `backup --name` para incluir `source_paths` |
| Coolify no respalda bind mounts | `/data/uploads/` es responsabilidad propia | Incluir en `backup_policy.source_paths` |
| Compose backup limitado a 5 | Si haces muchos deploys, se pierden compose antiguos | Los 5 más recientes siempre disponibles |
| Google Drive es legacy | OAuth puede expirar | Migrar a `ssh_remote` |
| Lightweight restore no restaura volúmenes | Si había datos en volúmenes nombrados, se pierden | Usar manager restore para sitios Coolify |
| Direct transfer requiere clave en VPS1 | Sin clave, backup viaja PC→VPS2 (lento) | Configurar `directTransferKey` |

---

## 11. Comandos de referencia

```powershell
# Backup manual de un sitio
coolify-manager backup --name studio --tier manual

# Backup pre-deploy (automático, pero forzable)
coolify-manager backup --name studio --tier manual --label "manual-test"

# Restore desde backup más reciente
coolify-manager restore --name studio

# Restore desde backup específico
coolify-manager restore --name studio --backup-id abc123

# Ver backups disponibles
coolify-manager backups --name studio

# Instalar cron de backups en VPS
coolify-manager install-backups --server-ip 66.94.100.241

# Programar backups en Windows
coolify-manager schedule-backup --name studio --tier daily --time 03:00

# Backup de sitio lightweight
coolify-manager light-backup --target hosting --name mi-sitio --tier daily
```

---

## 12. Incidente de referencia

Ver: [Incidente glory-rest 2026-07-01](incidente-glory-rest-2026-07-01.md)
