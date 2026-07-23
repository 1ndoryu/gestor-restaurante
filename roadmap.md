Objetivo: Sistema de restaurante con integración BDP (WebLink). Backend Rust (Axum) + React SPA.
Rama: glory-rs-rest

## Stack

| Capa          | Herramienta                    |
| ------------- | ------------------------------ |
| Framework web | Axum 0.7                       |
| OpenAPI       | utoipa 4 + utoipa-swagger-ui 7 |
| Base de datos | SQLx 0.8 (PostgreSQL)          |
| Validación    | validator 0.18                 |
| Auth          | jsonwebtoken + argon2          |
| Frontend      | React 18 + TypeScript + Vite   |
| State         | React Query + Zustand          |
| Codegen       | Orval 8                        |
| Deploy        | coolify-manager-rs             |

# Glory Rest — Roadmap

## Notas de infraestructura

- **restaurante.wandori.us**: Coolify service `glory-rest`, UUID `b8s0cks444o0sogo8kg8wcgw`
- **Deploy**: Siempre via coolify-manager-rs (`deploy --name glory-rest --update`), nunca desde Coolify UI
- **Branch**: `glory-rs-rest`
- **SSH PROHIBIDO**: PowerShell profile bloquea SSH/SCP/SFTP en agentes VS Code (ver `Agente/prevencion/ssh-prohibicion-completa-2026-06-30.md`)

## Deploy con coolify-manager-rs

**coolify-manager-rs** es una CLI Rust que centraliza toda operación contra Coolify. Reemplaza SSH directo, scp, y la UI web de Coolify para tareas operativas.

### Comandos principales
| Comando | Uso |
|---|---|
| `deploy --name <sitio> --update` | Deploy completo: actualiza código, rebuild si aplica |
| `deploy --name <sitio> --update --skip-backup` | Deploy rápido (cambios de código sin migraciones BD) |
| `redeploy --name <sitio>` | Fuerza redeploy via API Coolify (sin cambios locales) |
| `health --name <sitio>` | Health check remoto + HTTP. Obligatorio post-deploy |
| `logs --name <sitio>` | Logs del contenedor remoto |
| `restart --name <sitio>` | Reinicia servicios del sitio |
| `backup --name <sitio>` / `restore --name <sitio>` | Backup/restore externo |
| `exec --name <sitio> -- <cmd>` | Ejecuta comando en el contenedor |

### Flujo deploy obligatorio
```
deploy → health → si falla → redeploy → health
```

### Protecciones integradas
- **Pre-validación**: `validate_compose_before_deploy()` detecta errores de sintaxis antes de aplicar
- **Backup pre-write**: `backup_compose_locally()` guarda el compose antes de modificarlo (rollback manual posible)
- **Post-verify**: `verify_container_env_vars()` y `verify_container_volumes()` confirman que entorno y volúmenes se inyectaron
- **Rollback automático E11**: si health falla post-deploy, restaura el compose anterior y recrea contenedores
- **Marcador CM_GUARD_v1**: todos los comandos SSH incluyen el marker para que el servidor identifique tráfico legítimo de coolify-manager-rs

### Dónde está
```
C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs
```
Binario: `target\release\coolify-manager.exe`
Config: `config\settings.json` (servidores, tokens, sitios)

### Reglas
1. **NUNCA** SSH directo ni scp — todo por coolify-manager-rs.
2. **Siempre** `health` después de `deploy`.
3. **Redeploy** para servicios Rust/Docker custom (deploy solo WordPress).
4. Si un comando no está cubierto, dejar constancia para mejorar la herramienta (no buscar alternativa manual).

## Contexto

Sistema de restaurante con integración BDP (WebLink REST API). Backend Rust (Axum) sirve API + SPA. Frontend React integrado en `frontend/src/`. La integración BDP permite sincronizar clientes, comandas, pagos y facturas entre Glory y el sistema de punto de venta del restaurante.

## Estado interno reciente

- `237A-1`: auditoría adversarial extendida BDP. 7 hallazgos nuevos verificados como true positives. 6 fixes aplicados: N1 (tx atómica CreateCustomer), N2 (reconciliación clientes huérfanos en polling), N3 (SYNC_LOCKS bounded cleanup), N4 (token cache BDP para evitar doble login), N5 (circuit breaker import batch), N6 (invoice reconciliación con tx). Tests: 128/128 pasando, clippy 0 warnings.

---

## Tareas pendientes

### 🟦 BDP — Fase 9: Catálogo, Plano de Sala y Menús

- **Fase 9.1 — ExportArticles: Sync de catálogo BDP → Glory.** ✅ 157A-7+157A-9
  - Lee catálogo completo de BDP (`ExportArticles`), sincroniza con `bdp_article_map`.
  - Campos nuevos en mapa: `descripcion`, `precio_tarifa1`, `iva_pct`, `departamento`, `familia`, `ultima_sync_at`.
  - Endpoint: `POST /api/bdp/article-maps/sync-catalog`.
  - Tests: 17 tests (12 existentes + 5 nuevos `upsert_from_bdp`).

- **Fase 9.2 — GetArticle: Consulta individual de artículo.** ✅ 157A-9
  - `resolve_article()` enriquece nombre, precio e IVA vía GetArticle. Fallback a config si falla.
  - Client method: `get_article(&BdpGetArticleRequest)`.

- **Fase 9.3 — GetPricesArticles: Refresh de precios.** ✅ 157A-9
  - `BdpSyncService::sync_prices()` actualiza `precio_tarifa1` de artículos mapeados.
  - Endpoint: `POST /api/bdp/article-maps/sync-prices`.

- **Fase 9.4 — GetRoomTables: Sync de mesas BDP → Glory.** ✅ 157A-9
  - `BdpSyncService::sync_tables()` → `PlanoSalaRepository` para crear zonas/mesas.
  - Mapeo: `RoomName`→`ZonaSala.nombre`, `Table`→`Mesa.numero`.
  - Endpoint: `POST /api/bdp/sync-tables`.

- **Fase 9.5 — GetMenuDefinition: Lectura informativa de menús.** ✅ 157A-9
  - Expone definiciones de menús/packs/fast-food de BDP como JSON raw.
  - Endpoints: `GET /api/bdp/menus/:id`, `GET /api/bdp/fastfoods/:id`, `GET /api/bdp/packs/:id`.

### ✅ Resuelto

- **065A-4 — Resolver bloqueo BDP `[300035]` fuera de horario (sin escrituras reales).** ✅ RESUELTO 2026-06-07
  - Causa real del 300035: campos `AlreadyInvoiced` e `Invoice` faltantes en payload (no series ni Order.Type).
  - Causa del 300005 (IVA): POS 31 usaba serie `00031TM` sin IVA incluido. Fix: nueva serie `00031TI` con IVA incluido.
  - `build_only_check_order()` ahora usa `Type=0` (Barra) — único tipo que pasa validación sin config extra.
  - Validación dry-run completa: artículo real (`1001`, "CAFE BOMBON") → `ErrorMessage: ""`.
  - Pendiente: commit, deploy a producción, y probar endpoint `/api/configuracion/bdp/sync-dry-run` en producción.

- **202A-2 — Fix glory-rest login + seed restoration.** ✅ RESUELTO 2026-07-02
  - restaurante.wandori.us no dejaba entrar. Coolify regeneró compose (credenciales cambiadas).
  - Datos originales perdidos permanentemente. Seed ejecutado para restaurar demo.
  - Documentación: `Agente/documentacion/hosting/incidente-glory-rest-2026-07-01.md`

- **202A-3 — Guards E19+E20 en coolify-manager-rs.** ✅ RESUELTO 2026-07-02
  - E19: Credential Drift Detection — aborta deploy si POSTGRES_USER/POSTGRES_DB cambian.
  - E20: Database Existence Verification — aborta ALTER USER si la DB no existe.
  - Commit `ea16100` en coolify-manager-rs.

- **202A-4 — Documentación sistema de respaldos + incidente.** ✅ RESUELTO 2026-07-02
  - Documentación completa del sistema de backups (3 capas, formato, retención, restore, guards).
  - Documentación detallada del incidente glory-rest (cronología, causa raíz, acciones).
  - Archivos: `Agente/documentacion/hosting/sistema-respaldos-2026-07-02.md`, `Agente/documentacion/hosting/incidente-glory-rest-2026-07-01.md`

