Objetivo: Nakomi Studio — sitio web de agencia creativa. Migrado de WordPress a Rust (Axum) + React SPA.
Rama: glory-rust-nakomi

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

# Nakomi Studio — Roadmap

## Notas de infraestructura

- **nakomi.studio**: VPS1 (66.94.100.241), Coolify service `do8k4w8swccwwogoc0os0ck0`
- **VPS2 Coolify**: Configurado en settings.json
- **Deploy**: Siempre via coolify-manager-rs, nunca desde Coolify UI (ver doc de persistencia volúmenes)
- **Volúmenes**: Documentado en `Agente/documentacion/hosting/coolify-volumenes-persistencia-2026-04-12.md`

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

Proyecto migrado de WordPress a Rust (Axum) + React SPA. El frontend React se integra en frontend/src/. El backend Rust sirve API + SPA.

## Estado interno reciente

- `245A-9`: el runtime de hosting `lightweight` ya expone backup/restore remoto por manager y por API de suscripciones (`/api/hosting/subscriptions/{id}/backups`, `/api/hosting/subscriptions/{id}/restore`). Pendiente siguiente del frente: smoke operativo real del restore, observabilidad/panel lightweight y receta WordPress premium.
- `245A-10`: la compra de hosting ya deja fijado el runtime por plan en vez de depender del provider global; `normal-*` solo cae en `lightweight` cuando el target está configurado, WordPress sigue en Coolify, y en producción se corrigió `COOLIFY_BASE_URL` al alias interno de Coolify con el bypass de checkout test desactivado antes de la compra real.
- `255A-2`: `/api/hosting/deployments` ya no interpreta un fallo de Coolify/runtime como inventario vacío; reconstruye un fallback mínimo desde `hosting_subscriptions` y solo devuelve `503` si ni siquiera puede recomponer una lista útil. En el diagnóstico de VPS2 también se confirmó que un `500` global de Coolify 4.1.0 podía venir de `personal_access_tokens.abilities='[*]'` en vez de JSON válido.
- `255A-3`: los governors de auth y API se subieron a límites productivos con `SmartIpKeyExtractor` para evitar `429` cruzados detrás de Coolify/Traefik cuando la SPA abre polling y varias requests concurrentes.
- `255A-4`: el panel de infraestructura ya separa recursos del plan vinculado de los límites runtime reales detectados. El sampler guarda límites CPU/RAM efectivos por contenedor vía `docker inspect`, así que despliegues legacy sin caps Docker dejan de mostrar valores sintéticos del plan como si fueran enforcement real.
- `265A-1`: el primer burst dinámico de CPU para hostings Coolify ya corre en background. Usa snapshots del sampler para subir o restaurar el cap runtime del contenedor principal (`site` o `wordpress`) según holgura real de la VPS, manteniendo el plan como baseline contractual.
- `265A-2`: el sampler ya normaliza los `\t` literales de `docker inspect --format` antes de parsear límites runtime. Con eso, hostings legacy como `hosting-0fa1d5da` dejan de persistir `site_cpu_limit_cores = null` cuando Docker sí tiene caps reales, y el burst puede evaluar sitios existentes además de los nuevos.
- `265A-3`: la aplicación real del burst ya no resuelve el compose project por `coolify_site_name` a secas. En hostings legacy/runtime el project efectivo puede ser `deployment_uuid`, así que el executor ahora prueba primero ese identificador, cae al slug solo como fallback y no memoriza un target pedido cuando `docker update --cpus` falla.
- `265A-5`: `hosting_plan_configs` ahora persiste `cpu_scaling_policy` por plan para elegir entre `baseline_burst` y `contention_throttle`. El segundo queda como default comercial: el sitio queda sin cap fuera de contención usando `docker update --cpu-quota -1`, y el sampler ya interpreta `CpuQuota < 0` como runtime ilimitado aunque Docker deje `NanoCpus` stale.
- `265A-6`: el panel de hosting ya incluye la pestaña **Respaldos** con lista, crear, restaurar y eliminar backups vía SSH para Coolify y vía manager para Lightweight. Backend: SSH al VPS para listar archivos en volumen `backup-data` del compose project. Frontend: `TabBackups.tsx` con tabla responsive, confirmaciones y estados vacío/error. Además, el compose de WordPress ahora inyecta SMTP automáticamente (`WORDPRESS_SMTP_*` env vars + `phpmailer_init` en `WORDPRESS_CONFIG_EXTRA`) si las credenciales `GLORY_SMTP_HOST`/`SMTP_HOST` están disponibles en el servidor.
- `275A-3`: hotfix del listado de backups para WordPress/Coolify. El endpoint fallaba con 500 porque `alpine:3.20` usa BusyBox y no soporta `ls --time-style=long-iso`; ahora el listing usa `ls --full-time`, comprueba la existencia del volumen antes de montarlo y el parser acepta timestamps `HH:MM:SS +0000`. Validado con test unitario nuevo y smoke SSH contra el VPS del hosting de prueba.

---

## Tareas pendientes

### 🟦 Producto de Correo para Hosting

Ver análisis completo en `Agente/documentacion/hosting/producto-correo-proveedores-2026-05-26.md`.

**Decisión pendiente (bloqueante):** Elegir proveedor — MXroute ($59/año, más barato, sin API) vs Migadu ($9/mes, API REST). Esto define la arquitectura de provisioning.

- **265A-11 — Fase 1: Aliases/reenvíos gratis con Cloudflare Email Routing.**
  - Configurar MX/SPF/DKIM/DMARC del dominio del cliente apuntando a Cloudflare.
  - Solo reenvío a Gmail/Outlook del cliente (sin IMAP/SMTP).
  - Incluir 3 alias en plan Pro, 5 alias en Avanzado.
  - Sin costo operativo para Nakomi.
  - Backend: `POST /api/hosting/{id}/aliases`, `DELETE /api/hosting/{id}/aliases/{alias}`.
  - Frontend: TabCorreo con lista de aliases y estado DNS.
  - ~8-10h estimado.

- **265A-12 — Fase 2: Buzones IMAP (MXroute o Migadu).**
  - Contratar proveedor y configurar cuenta reseller.
  - Implementar provisioning: crear/suspender/eliminar mailbox vía API (Migadu) o automatización panel (MXroute).
  - Modelos BD: `mail_domains`, `mailboxes`, `mail_events`.
  - Backend: CRUD de buzones, reset password, DNS automático.
  - Frontend: TabCorreo completo con indicadores de estado.
  - Billing: Stripe add-on a $1.50/buzón/mes.
  - ~20-26h estimado.

- **265A-13 — Incluir 1 buzón IMAP gratis en plan Avanzado.**
  - Modificar `hosting_plan_configs` (nuevo campo `included_mailboxes`).
  - Actualizar pricing en frontend y catálogo.
  - Stripe: nuevo price para el add-on.
  - ~3-4h estimado.

### ✅ Resuelto

- **065A-4 — Resolver bloqueo BDP `[300035]` fuera de horario (sin escrituras reales).** ✅ RESUELTO 2026-06-07
  - Causa real del 300035: campos `AlreadyInvoiced` e `Invoice` faltantes en payload (no series ni Order.Type).
  - Causa del 300005 (IVA): POS 31 usaba serie `00031TM` sin IVA incluido. Fix: nueva serie `00031TI` con IVA incluido.
  - `build_only_check_order()` ahora usa `Type=0` (Barra) — único tipo que pasa validación sin config extra.
  - Validación dry-run completa: artículo real (`1001`, "CAFE BOMBON") → `ErrorMessage: ""`.
  - Pendiente: commit, deploy a producción, y probar endpoint `/api/configuracion/bdp/sync-dry-run` en producción.

