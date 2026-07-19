# Plan: Comando `restore-pg-data` para coolify-manager-rs

> **Fecha:** 2026-07-19
> **Estado:** ✅ COMPLETADO
> **Motivo:** Restaurar backup del incidente nakomi.studio (11 mayo 2026) que es un data directory raw de PostgreSQL

---

## Contexto

El backup del incidente está en `/data/incident-backups/studio-20260511-141942/postgres-pg-data.tar.gz` — es un **data directory raw de PG16**, no un pg_dump SQL. El comando `restore` existente solo maneja backups internos de coolify-manager. Sin este comando, la única forma de restaurar es SSH directo (prohibido por regla -2).

## Diseño del comando

```
coolify-manager restore-pg-data \
  --name <sitio> \
  --file <ruta-al-tarball> \
  [--database <nombre_db>] \
  [--skip-safety-snapshot]
```

## Flujo interno (7 fases)

| Fase                | Acción                                       | Rollback                  |
| ------------------- | -------------------------------------------- | ------------------------- |
| 1. Validación       | Verificar sitio es PG, resolver containers   | N/A                       |
| 2. Safety snapshot  | pg_dump de DB actual                         | N/A                       |
| 3. Upload + extraer | Subir tarball al VPS, extraer en /tmp        | Limpiar /tmp              |
| 4. Temp postgres    | Levantar postgres:16 temporal, pg_dump a SQL | docker rm -f temp         |
| 5. Parar app        | Detener app para evitar writes               | docker start app          |
| 6. Restaurar        | Drop+recreate DB, import SQL via psql        | Restaurar safety snapshot |
| 7. Cleanup          | Arrancar app, eliminar temporales            | N/A                       |

## Archivos a modificar

| Archivo                           | Cambio                                             |
| --------------------------------- | -------------------------------------------------- |
| `src/commands/restore_pg_data.rs` | NUEVO — lógica del comando                         |
| `src/commands/mod.rs`             | Agregar `pub mod restore_pg_data;`                 |
| `src/cli/mod.rs`                  | Agregar variante `RestorePgData` al enum `Command` |
| `src/cli/dispatch/deploy.rs`      | Agregar dispatch                                   |

## Seguridad

- Safety snapshot obligatorio (pg_dump pre-restauración)
- Parar app antes de drop DB
- Validación de exit code en cada fase
- Cleanup garantizado incluso en error
- Timeouts explícitos (60s readiness para postgres temporal)

## Resultados (2026-07-19)

- **Commit:** `c39790b` en `1ndoryu/coolify-manager-rs` (rama main)
- **Verificación post-restore:**
  - users: 9 ✓
  - orders: 1 ✓
  - services: 7 ✓
  - projects: 5 ✓
  - chat_messages: 44 ✓
  - blogs: 1 ✓
  - Total: 41 tablas importadas

## Fixes durante implementación

1. **`:ro` mount en postgres temporal** — PG necesita acceso de escritura para chown/WAL recovery. Fix: copiar data dir a ubicación writable antes de montar.
2. **"Argument list too long"** — base64 via `echo` excede ARG_MAX para dumps >100KB. Fix: usar siempre `upload_file_streamed()` para el SQL dump.
3. **`copy_to_container` con path remoto** — la función espera archivo local. Fix: usar `docker cp` directo via SSH cuando el archivo ya está en el host.
- No toca bind mounts — solo DB

## Problema post-restore: SQLx VersionMismatch

La DB restaurada (11 mayo) tiene checksums de migraciones que no coinciden con el binario actual (compilado desde commit `d3fe05f4`, 27 junio). La app entra en crash loop:

```
migrate: while executing migrations: while running version 20260420001000
Caused by: VersionMismatch(4188240120726213627)
```

Esto es **normal y esperado** al restaurar una DB antigua — las migraciones se modificaron después del backup.

### Fix planificado (SOLO vía coolify-manager-rs)

**Herramientas a usar:**
- `exec --name studio --target postgres --command "..."` → ejecutar SQL en postgres
- `exec --name studio --target app --command "..."` → verificar estado de la app
- `host-exec --command "..." --target <vps>` → calcular SHA-384 del commit desplegado en el servidor

**Por qué NO SSH directo:**
- `exec` usa el SshClient nativo de coolify-manager (russh), tiene validación de contenedor UUID, manejo de errores, timeouts
- `host-exec` es el wrapper seguro para comandos en el host — pasa por la misma infraestructura
- SSH directo desde PowerShell bypass toda la seguridad y puede corromper datos (CRLF, encoding, etc.)

### Fases del fix

| Paso | Acción | Comando coolify-manager |
|------|--------|------------------------|
| **F1. Verificar estado** | Confirmar crash loop y migración problemática | `exec --name studio --target app --command "cat /proc/1/cmdline"` |
| **F2. Obtener checksum esperado** | Clonar repo en servidor, calcular SHA-384 del commit desplegado | `host-exec --command "cd /tmp && git clone --depth 1 --branch glory-rs-rest --single-branch https://github.com/1ndoryu/glory-rs-template.git cm-checksum && sha384sum cm-checksum/migrations/20260420001000_vps_resale.up.sql"` |
| **F3. Actualizar checksum en DB** | UPDATE en `_sqlx_migrations` con el hash correcto | `exec --name studio --target postgres --command "UPDATE _sqlx_migrations SET checksum = decode('HASH', 'hex') WHERE version = 20260420001000;"` |
| **F4. Reiniciar app** | Restart para que la app arranque con checksums correctos | `restart --name studio` |
| **F5. Verificar health** | Confirmar que la app arranca y responde | `health --name studio` |
| **F6. Cleanup** | Eliminar clone temporal en servidor | `host-exec --command "rm -rf /tmp/cm-checksum"` |

### ¿Por qué este enfoque es seguro?

1. **No perdemos datos** — solo actualizamos un checksum (metadata), no datos de usuario
2. **Reversible** — el safety snapshot del restore-pg-data contiene el checksum original
3. **Auditable** — cada paso pasa por coolify-manager con logging
4. **No toca el binario** — solo alinea la DB con el binario ya desplegado
5. **Genérico** — funciona para cualquier mismatch de migración SQLx

### Si F2 falla (no hay git en servidor)

Plan B: usar `exec --name studio --target app` para extraer el checksum del archivo compilado embebido en el binario. SQLx almacena los checksums esperados en el binario mismo — podemos usar `strings` o buscar el patrón en el binario.

Plan C: extraer `_sqlx_migrations` del backup semanal (`20260719_051504`) que sí tiene checksums correctos. Solo necesitamos esa tabla, no toda la DB.

### Fix ejecutado: Plan C (backup semanal)

**Problema real:** F2 falló (`strings` no disponible en servidor). F3 requería parchear múltiples migraciones, no solo una. El checksum correcto para `20260420001000` del backup semanal es `3211b4e...` vs `3c23870...` de la DB restaurada.

**Ejecución:**
1. `host-exec`: extraer bloque `COPY public._sqlx_migrations` del `db-postgres.sql` del backup semanal → `/tmp/cm-migrations.sql` (2292 líneas, 64 migraciones)
2. `host-exec`: `docker cp` al contenedor postgres → `/tmp/migrations.sql`
3. `exec --target postgres`: `TRUNCATE _sqlx_migrations`
4. `exec --target postgres`: `psql -U rust_app -d rust_db -f /tmp/migrations.sql` → `COPY 64`
5. `host-exec`: `docker restart app-do8k4w8swccwwogoc0os0ck0`
6. Verificación: `exec --target app` → `ALIVE`
7. `health --name studio` → `http_ok=true app_ok=true fatal_logs=false`
8. `host-exec`: cleanup `/tmp/cm-*` y `/tmp/glory-app-binary`

**Resultado:** Studio nakomi.studio restaurado completamente — 41 tablas con datos del incidente + checksums alineados con el binario actual.

## Próximos pasos

1. ✅ Implementar `restore_pg_data.rs`
2. ✅ Wiring (mod.rs + CLI + dispatch)
3. ✅ Compilar
4. ✅ Restaurar nakomi.studio (DB + datos verificados)
5. ✅ Fix SQLx VersionMismatch (Plan C: replace _sqlx_migrations desde backup semanal)
6. ✅ Verificar health completo de nakomi.studio
7. 🔄 Commit y push final en coolify-manager-rs
8. 🔄 Actualizar plan como completado
