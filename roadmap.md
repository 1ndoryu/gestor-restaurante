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
- `237A-2`: fixes post-247A-1 — tests compilables + MDs actualizados. Añade campos `ff_bdp_*` faltantes en constructores de test y `stock_actual` en `tests/bdp_article_map.rs`.
- `247A-3`: fix `ON CONFLICT` en `bdp_audit_log` para índice parcial `WHERE idempotency_key IS NOT NULL`.
- `247A-4`: evaluación de riesgos BDP en producción. Documento `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` con 12 riesgos priorizados y mitigaciones. Tests backend/frontend pasan.

---

## Tareas pendientes

### 🟦 BDP — Estado actual

**Documentación reciente:**
- `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` — evaluación de riesgos priorizados antes de producción.
- `Agente/planes/plan-pendientes-bdp-2026-07-23.md` — plan detallado de funcionalidades pendientes de decisión del cliente (C1, C2, D1-D5, XT1, XT2).
- `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md` — mapeo visual de cada funcionalidad BDP en el frontend.

#### Mitigaciones críticas BDP en curso

| ID | Riesgo | Estado | Siguiente paso |
| --- | --- | --- | --- |
| R2 | Transacción abierta durante llamadas HTTP a BDP |  Parcialmente mitigado (commit temprano, pérdida de lock cross-instance) | Decidir: lock de sesión `pg_advisory_lock` o columna `bdp_sync_status` |
| R3 | Throttling tratado como error permanente | ✅ Mitigado (`Throttled → AmbiguousTransport`) | Añadir retry/reconciliación automática |
| R1 | Reconciliación de comandas ambiguas (`AmbiguousTransport`) | ⏳ Pendiente | Implementar `reconcile_ambiguous_orders` en `bdp_order_poller` |
| R5 | Timeout global en `sync_venta` |  Pendiente | Envolver fase HTTP en `tokio::time::timeout` |
| R4 | Cliente sin mapeo bloquea comanda sin feedback claro | 🟡 Backend mejorado (mensaje descriptivo) | Mejorar UI/UX de ventas (badge + toast) |
| R11/R12 | IVA/precio por defecto hardcodeados y aritmética `f64` | ⏳ Pendiente evaluación | Revisar si BDP valida totales y pasar cálculo a `Decimal` |
| R13 | `SYNC_LOCKS` mutex poisoning | ✅ Mitigado (`unwrap_or_else(|e| e.into_inner())`) | Considerar migración a `parking_lot`/`DashMap` |
| R14 | Limpieza manual de `SYNC_LOCKS` | ⏳ Pendiente | Refactorizar a guard RAII |
| R15 | `Throttled` en pagos/facturas no se trata como ambiguo | ⏳ Pendiente | Aplicar mismo mapeo en `add_order_payment` e `invoice_order` |

### ✅ Completado recientemente

#### BDP — Fase 9: Catálogo, Plano de Sala y Menús
- **Fase 9.1 — ExportArticles: Sync de catálogo BDP → Glory.** ✅ 157A-7+157A-9
- **Fase 9.2 — GetArticle: Consulta individual de artículo.** ✅ 157A-9
- **Fase 9.3 — GetPricesArticles: Refresh de precios.** ✅ 157A-9
- **Fase 9.4 — GetRoomTables: Sync de mesas BDP → Glory.** ✅ 157A-9
- **Fase 9.5 — GetMenuDefinition: Lectura informativa de menús.** ✅ 157A-9

#### Mejoras de UX y seguridad
- **C1 — Auto-arming** ✅ Implementado en 247A-1.
- **C2 — Toggle rápido de modo escritura en navbar** ✅ Implementado en 247A-1.
- **XT1 — Throttling/semáforo BDP** ✅ Implementado en 247A-1.
- **XT2 — Feature flags por restaurante** ✅ Implementado en 247A-1.
- **D1 — Verificación de stock (parser defensivo)** ✅ Implementado en 237A-4.

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

