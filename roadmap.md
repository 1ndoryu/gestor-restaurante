Objetivo: Sistema de restaurante con integración BDP (WebLink). Backend Rust (Axum) + React SPA.
Rama: glory-rs-rest

**Seguimiento de quality gate (2026-08-12):** Sentinel `0.7.1` (`b22c8484`) y VarSense
`2.2.1` (`88f281f9`) están publicados, fijados en `quality-tools.json` y verificados por
`quality:lock --check` + `sentinel doctor --json`. `gate:check` es ahora el wrapper canónico
que delega en `sentinel check --stages`; `task:check` queda como alias de compatibilidad. La
primera comprobación docs de este checkout mostró 5 hallazgos preexistentes de
`broadcast-mutex-riesgo-rs` en `src/`. El proyecto los mantiene como warning explícito porque
`tokio::sync::broadcast` es la abstracción intencional para fanout SSE; los hallazgos siguen visibles
en el reporte y la regla no se borra ni se desactiva.

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

- **Sentinel (2026-08-10)**: re-pinado a la release coordinada **0.7.0** (`a804c0d`, `main` + tag `v0.7.0`); lock regenerado, `quality:lock --check` y doctor PASS, suite 232 pass. La release integra la auditoría 108A-1 (contratos CLI, init, ADR 0001, seguridad/concurrencia) sobre 0.6.4. El stage `custom` local fue retirado (commit `f13d0e16`): 15 reglas migradas al Core, 2 observe-only P1 con destino declarado en roadmap-sentinel.
- **restaurante.wandori.us**: Coolify service `glory-rest`, UUID `b8s0cks444o0sogo8kg8wcgw`
- **Deploy**: Siempre via coolify-manager-rs (`deploy --name glory-rest --update`), nunca desde Coolify UI
- **Branch**: `glory-rs-rest`
- **SSH PROHIBIDO**: PowerShell profile bloquea SSH/SCP/SFTP en agentes VS Code (ver `Agente/prevencion/ssh-prohibicion-completa-2026-06-30.md`)

## Deploy con coolify-manager-rs

**coolify-manager-rs** es una CLI Rust que centraliza toda operación contra Coolify. Reemplaza SSH directo, scp, y la UI web de Coolify para tareas operativas.

### Comandos principales

| Comando                                            | Uso                                                   |
| -------------------------------------------------- | ----------------------------------------------------- |
| `deploy --name <sitio> --update`                   | Deploy completo: actualiza código, rebuild si aplica  |
| `deploy --name <sitio> --update --skip-backup`     | Deploy rápido (cambios de código sin migraciones BD)  |
| `redeploy --name <sitio>                           | Fuerza redeploy via API Coolify (sin cambios locales) |
| `health --name <sitio>`                            | Health check remoto + HTTP. Obligatorio post-deploy   |
| `logs --name <sitio>`                              | Logs del contenedor remoto                            |
| `restart --name <sitio>`                           | Reinicia servicios del sitio                          |
| `backup --name <sitio>` / `restore --name <sitio>` | Backup/restore externo                                |
| `exec --name <sitio> -- <cmd>`                     | Ejecuta comando en el contenedor                      |

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

---

## Contexto

Sistema de restaurante con integración BDP (WebLink REST API). Backend Rust (Axum) sirve API + SPA. Frontend React integrado en `frontend/src/`. La integración BDP permite sincronizar clientes, comandas, pagos y facturas entre Glory y el sistema de punto de venta del restaurante.

---

## Resumen ejecutivo — Integración BDP (para respuesta al cliente)

### ✅ Lo que ya está operativo

| Funcionalidad BDP                                                   | Dónde se ve en la web                                | Estado                                                                           |
| ------------------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------- |
| **Catálogo de artículos** (sync, precios, stock)                    | Configuración → BDP → "Catálogo de artículos BDP"    | ✅ Implementado; la tarifa real se elige en pantalla si BDP devuelve cero        |
| **Mapeos técnicos** (tender, canales, artículo/cliente por defecto) | Configuración → BDP → "Correspondencias Glory ↔ BDP" | ✅ Visible (colapsable)                                                          |
| **Clientes BDP** (importar/sincronizar)                             | Clientes → "Importar BDP"                            | ✅ Funcional; lista clientes de BDP                                              |
| **Plano de Sala** (mesas BDP)                                       | Plano de Sala → "Sync BDP"                           | ✅ Funcional                                                                     |
| **Comandas** (crear orden en BDP)                                   | Ventas → "Enviar a BDP"                              | ✅ Funcional, requiere autorización temporal                                     |
| **Pagos completos** (AddOrderPayment)                               | Ventas → "Pagar en BDP"                              | ✅ Implementado (verificado en simulador). **Verificación real pendiente**: `Payment/Add` responde "Subscripción no activada" (2026-08-05, prueba 2.3) — requiere suscripción WebLink de pago activa en la instalación. |
| **Pagos parciales** (AddOrderPayment parcial)                       | Ventas → icono de tarjeta en fila de venta           | ✅ Implementado bajo feature flag `ff_bdp_partial_payments`. Configurable desde UI. (Misma dependencia de suscripción de pago para BDP real). |
| **Facturas** (InvoiceOrder)                                         | Ventas → "Facturar en BDP"                           | ✅ Implementado (verificado en simulador). **Verificación real pendiente**: misma dependencia de suscripción WebLink de pago (prueba 2.4). |
| **Estado BDP**                                                      | Navbar (badge BDP: lectura/escritura)                | ✅ Visible e interactivo                                                         |
| **Polling de estados**                                              | Configuración → BDP → "Actualización de estados"     | ✅ Configurable                                                                  |
| **Explorador de menús/packs/fastfoods**                             | Configuración → BDP → sección inferior               | ✅ Visible y funcional                                                           |
| **Stock (solo lectura)**                                            | Tabla de mapeos de artículos, columna "Stock"        | ✅ Visible si BDP devuelve stock                                                 |

### ❌ Lo que NO está integrado (por decisión de alcance o pendiente del cliente)

| Funcionalidad                                   | Motivo                                                                                                                                           | Estado               |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------- |
| **Compras** (albaranes/facturas de proveedores) | **Fases 1-3 implementadas.** Lectura, borradores y conciliación. Protegidas por feature flags `ff_bdp_purchase_notes_*`. Configurables desde UI. | ✅ Implementado      |
| **Pagos parciales**                             | Implementado bajo feature flag `ff_bdp_partial_payments`. Ledger local `bdp_pagos` con idempotency_key. Configurable desde UI.                   | ✅ Implementado      |
| **Sincronización bidireccional automática**     | Riesgo de bucles y conflictos; no soportada por BDP                                                                                              | ❌ Rechazado         |
| **CancelOrder**                                 | BDP responde "Subscripción no activada"                                                                                                          | ❌ Bloqueado por BDP |
| **Modificación de stock**                       | Alcance solo lectura en integración actual                                                                                                       | ❌ Fuera de alcance  |

### 🔒 Autorización temporal para escrituras

**Cómo funciona hoy:**

- Por defecto, Glory está en **modo solo lectura** respecto a BDP. Puede consultar e importar, pero no escribir.
- Para enviar una comanda, pagar o facturar, se requiere una **autorización temporal** (arming).
- Esta autorización se puede hacer de dos formas:
    1. **Manual**: Configuración → BDP → Permiso de operación (escritura temporal).
    2. **Automática**: Si se activa el feature flag `ff_bdp_auto_arm`, al pulsar "Enviar a BDP" / "Pagar" / "Facturar" se solicita confirmación dinámica y el sistema arma/desarma solo para esa operación.
- Tras cada escritura exitosa o fallida, el sistema **vuelve automáticamente a solo lectura**.

**Respuesta al cliente:** No es necesario cambiar manualmente el modo cada vez si se activa el auto-arming. La confirmación se pide dentro del flujo de la operación.

### 📦 Importaciones de catálogo vs stock

- **Importación de catálogo**: se refiere a artículos, precios, familias, departamentos, códigos de barras y, si BDP lo devuelve, **stock actual**. Es decir, el stock es parte del catálogo, no algo separado.
- **Stock**: se muestra en la tabla de mapeos si el módulo de almacén de BDP está activo y devuelve `CurrentStock`. Es solo lectura; no se puede modificar desde Glory.

---

## Tareas pendientes

- **128A-1 — Independencia total del BDP (funcionar con o sin BDP)**: plan activo
  `Agente/planes/plan-independencia-bdp-2026-08-12.md`. Auditar lo no completado de la integración BDP
  (N1–N14), inventariar las dependencias del BDP y planificar modo `standalone` para catálogo, stock,
  compras, anulación de ventas, historial y explorador, con conmutador `standalone`/`bdp` y degradación
  automática. F0 (auditoría), F1 (conmutador de modo + badge) y F2 (catálogo local:
  origen/local_dirty, CRUD sin BDP, resolve_article M5, import M6/M7) **completados**
  con gate PASS (2026-08-13). En curso — siguiente bloque: **F3** (stock local).

- Automatizar la detección de credenciales literales en documentación según `Agente/prevencion/prevencion-secretos-documentacion-bdp-2026-07-28.md`.
- Automatizar la inmutabilidad de migraciones aplicadas según `Agente/prevencion/prevencion-inmutabilidad-migraciones-2026-07-28.md`.
- **287A-8 — Corregir health/rollback Rust en coolify-manager-rs:** el gestor sustituye el healthcheck seguro por `hostname -i`; al conectar `coolify`, la primera dirección es IPv6 y la URL sin corchetes hace que `curl` falle, Docker marque `unhealthy` y Traefik responda `503`. Además, el rollback puede perder la red externa y evaluar salud después de restaurar el compose anterior. Debe conservar `localhost`, persistir la red y validar la versión activa antes de decidir rollback. Ver `Agente/documentacion/hosting/incidente-red-traefik-glory-rest-2026-07-28.md`.

### Bloque 247A-7 — Mitigaciones críticas BDP (implementadas)

| ID      | Riesgo                                                       | Estado          | Qué se hizo                                                                                                  | Archivos clave                                                  |
| ------- | ------------------------------------------------------------ | --------------- | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------- |
| R1      | Reconciliación periódica de comandas/pagos/facturas ambiguas | ✅ Implementado | Worker `reconcile_ambiguous_orders` en `bdp_order_poller`; consulta `GetOrder` y cierra auditorías `ambiguo` | `src/services/bdp_order_poller.rs`                              |
| R5      | Timeout global en fase HTTP de `sync_venta`                  | ✅ Implementado | Fase HTTP envuelta en `tokio::time::timeout(Duration::from_secs(45))`                                        | `src/services/bdp_sync.rs`                                      |
| R14     | Limpieza manual de `SYNC_LOCKS`                              | ✅ Implementado | Guard RAII `SyncLockGuard` que llama `cleanup_lock` en `Drop`                                                | `src/services/bdp_sync.rs`                                      |
| R2-nota | Lock distribuido perdido tras early commit (cross-instance)  | Documentado     | Evaluar `pg_advisory_lock` de sesión o columna `bdp_sync_status` si se despliega multi-instance              | `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` |

### Bloque 247A-9 — Decisiones pendientes del cliente

| ID | Item | Pregunta al cliente | Esfuerzo estimado |
| --- | --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- || D2 | **Compras** (albaranes) | ✅ Fases 1-3 implementadas y testeadas sin llamar a BDP: lectura, borradores locales y conciliación con gastos. Protegidas por feature flags `ff_bdp_purchase_notes_*`. | ~12h || D5 | **CancelOrder** | BDP responde "Subscripción no activada". ¿Pueden activar el módulo? | ~12-16h si BDP lo activa |

### Bloque 247A-11 — Modo Demo y Refuerzos Compras BDP (completado)

| ID  | Item                                                   | Estado   | Notas                                                                                                                                         |
| --- | ------------------------------------------------------ | -------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| DM1 | Frontend: Modo demo en las 4 páginas BDP               | ✅ Hecho | Stock, Explorador, Historial y Compras usan datos simulados, bloquean hooks reales y deshabilitan sync.                                       |
| C1  | Backend: Refuerzos BDP Compras Fase 1                  | ✅ Hecho | Simplificación `validar_rango_fechas`, filtrado de claves naturales vacías y mapeo seguro de errores BDP sin filtrar URLs.                    |
| T6  | Pruebas: Simulación segura y tests unitarios BDP --lib | ✅ Hecho | Verificado con simulación sin llamar a API real. Tests unitarios en validación de fechas y clave natural. `cargo test bdp --lib` 57 tests OK. |

### Bloque 247A-9 — Pruebas y validación antes de producción

| ID  | Tarea                                                                           | Esfuerzo |
| --- | ------------------------------------------------------------------------------- | -------- |
| T1  | Validar flujo completo con simulador BDP local (crear comanda, pagar, facturar) | ~4h      |
| T2  | Validar flujo con BDP real del restaurante en entorno controlado                | ~4h      |
| T3  | Probar auto-arming y toggles de seguridad                                       | ~2h      |
| T4  | Revisar logs de ambigüedad y reconciliación                                     | ~2h      |
| T5  | Documentar procedimiento de rollback y restauración                             | ~2h      |

**Plan de pruebas propuesto al cliente:**

1. Fijar una sesión de 2 horas con acceso al BDP del restaurante (o simulador).
2. Crear una venta de prueba en Glory y enviarla a BDP.
3. Verificar que la comanda aparece en el TPV/BDP.
4. Registrar un pago completo y facturar.
5. Verificar que el estado se refleja en Glory (polling o consulta manual).
6. Probar el modo de autorización temporal y auto-arming.
7. Revisar auditoría en "Historial BDP".

---

### Bloque 267A-4 — Feature flags UI + backlog técnico + tests (implementados)

| ID     | Item                                           | Estado          | Qué se hizo                                                                                                     | Archivos clave                                                                                                                  |
| ------ | ---------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| FF-UI  | Feature flags BDP configurables desde frontend | ✅ Implementado | 6 toggles con descripciones en Configuración BDP. Tipos, sync y save actualizados.                              | `ConfigBdp.tsx`, `configuracion-types.ts`, `useConfiguracion.ts`, `useConfiguracionSync.ts`, `gestionRestauranteAPI.schemas.ts` |
| S16-H3 | Tests unitarios para canonical_target          | ✅ Implementado | 8 tests: acepta HTTP/HTTPS limpio, rechaza path/query/fragment/credenciales, strip trailing slash, fingerprint. | `src/services/bdp_backup.rs`                                                                                                    |
| S16-H4 | Tests adicionales para allowlist               | ✅ Implementado | 4 tests: rechaza query string, fragment, URL vacía, acepta localhost con puerto.                                | `src/services/bdp_weblink.rs`                                                                                                   |
| R4     | Test delay_ms timeout handling                 | ✅ Implementado | Test integración: inyecta 25s delay, verifica que cliente HTTP (20s timeout) mapea a error.                     | `tests/bdp_simulator_integration.rs`                                                                                            |
| Docs   | Roadmap actualizado                            | ✅ Hecho        | Secciones de bloque 267A-4 y pendientes actualizadas.                                                           | `roadmap.md`                                                                                                                    |

### Bloque 267A-1 — Mitigaciones y mejoras BDP (implementadas)

| ID        | Riesgo/Mejora                                        | Estado          | Qué se hizo                                                                                                   | Archivos clave                                                   |
| --------- | ---------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| S7-H1     | Dos ventas mapeadas al mismo bdp_order_id            | ✅ Implementado | UNIQUE partial index `idx_ventas_bdp_order_id_unique` en `ventas(user_id, bdp_order_id)`                      | `migrations/20260726200000_bdp_unique_constraints.up.sql`        |
| S7-H3     | Misma orden facturada dos veces                      | ✅ Cubierto     | El índice único general por `(user_id, bdp_order_id)` ya impide el duplicado; el índice parcial redundante se elimina en una migración posterior e inmutable | `migrations/20260728080000_bdp_remove_redundant_invoice_index.up.sql` |
| S16-H2    | Sin límite de tamaño de body en peticiones HTTP      | ✅ Implementado | `RequestBodyLimitLayer::new(2MB)` en el router Axum + feature `limit` en tower-http                           | `src/handlers/mod.rs`, `Cargo.toml`                              |
| R12       | IVA hardcodeado 10.0 en fallbacks de resolve_article | ✅ Implementado | Fallbacks usan `config.iva_por_defecto` en vez de `10.0` literal                                              | `src/services/bdp_sync.rs`, `src/services/bdp_sync_preflight.rs` |
| R16       | Conversión Decimal→f64 sin precisión documentada     | ✅ Documentado  | Enfoque vía string mantenido (más preciso); redondeo en call-sites cuando se necesite                         | `src/services/bdp_sync.rs`, `src/services/haddock.rs`            |
| Tests-R12 | Test para IVA por defecto cuando BDP no lo devuelve  | ✅ Implementado | `first_article_uses_default_iva_when_missing` + tests actualizados con parámetro `default_iva_pct`            | `src/services/bdp_sync_preflight.rs`                             |

### Bloque 267A-5 — Tests de servicios de negocio BDP (implementados)

| ID    | Item                                                       | Estado          | Qué se hizo                                                                    | Archivos clave                     |
| ----- | ---------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------ | ---------------------------------- |
| SVC-1 | 8 guard tests (rechazos sin simulador)                     | ✅ Implementado | read_only, disabled, missing order_id, zero amount, invoice guards             | `tests/bdp_service_integration.rs` |
| SVC-2 | sync_venta E2E contra simulador + PostgreSQL               | ✅ Implementado | Crea orden en BDP, verifica bdp_synced=true y bdp_order_id en BD               | `tests/bdp_service_integration.rs` |
| SVC-3 | add_order_payment E2E contra simulador + PostgreSQL        | ✅ Implementado | Registra pago, verifica ledger local (bdp_pagos) y audit log                   | `tests/bdp_service_integration.rs` |
| SVC-4 | invoice_order E2E contra simulador + PostgreSQL            | ✅ Implementado | Factura orden pagada, verifica bdp_invoiced=true y bdp_order_status='invoiced' | `tests/bdp_service_integration.rs` |
| SVC-5 | Helper seed_arming con snapshot FK + authorize IS NOT NULL | ✅ Implementado | Crea snapshot dummy + armado vigente para que authorize() no bloquee           | `tests/bdp_service_integration.rs` |
| SVC-6 | marketplace_order_id hecho público                         | ✅ Implementado | Cambiado de pub(crate) a pub para poder testear desde integration tests        | `src/services/bdp_sync.rs`         |

### Pendientes que ya NO son pendientes (implementados previamente)

| Item                 | Estado             | Evidencia                                                                       |
| -------------------- | ------------------ | ------------------------------------------------------------------------------- |
| R4: Feedback UI      | ✅ Ya implementado | `venta-row-actions.tsx`: toasts descriptivos, historial pagos, estados ambiguos |
| Stock pantalla       | ✅ Ya implementado | `BdpStock.tsx`: filtros, sorting, paginación, CSV, demo mode, ruta `/bdp/stock` |
| S6-H1: Redirect      | ✅ Ya cerrado      | `redirect(Policy::none())` en `bdp_weblink.rs:44`                               |
| S7-H2: Tx post-HTTP  | ✅ Ya cerrado      | `pool.begin()` + `tx.commit()` en add_order_payment/invoice                     |
| S14-H1: restaurar tx | ✅ Ya cerrado      | `pool.begin()` + `tx.commit()` en `bdp_backup.rs:574`                           |
| R1: Reconciliación   | ✅ Ya implementado | `reconcile_ambiguous` en `bdp_order_poller.rs`                                  |
| R3: Throttling       | ✅ Ya mitigado     | `Throttled→AmbiguousTransport` en `bdp_sync.rs:506`                             |

---

## Pendientes reales

| #   | Item                                                                                                                                                                                                                              | Bloqueo                                                                   | Esfuerzo          |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------- |
| 1   | **4 pruebas de escritura real** contra BDP (cliente, comanda, pago, factura) — plan activo: `Agente/planes/plan-pruebas-escritura-bdp-real-2026-08-04.md` (pruebas locales 2026-08-04, producción después). **Estado 2026-08-05: 2/4 verificadas** (cliente 900001 ✅, comanda 5330 ✅); **pago (2.3) y factura (2.4) PENDIENTES** — `Payment/Add` devuelve "Subscripción no activada"; el cliente afirma que la suscripción estaba activa → pendiente de verificación con cliente/proveedor WebLink. | Suscripción WebLink REST API de pago a verificar con el cliente | ~1h investigación + retomar pruebas |
| 1c  | **Verificar con cliente/proveedor WebLink** por qué `Payment/Add` responde "Subscripción no activada" si la suscripción estaba activa (¿instalación 100.83.196.35:8068 sin módulo REST de pago? ¿CodigoIntegrador sin permiso? ¿suscripción en otro entorno?). Documentado en plan 048A-11. | Cliente / proveedor WebLink | ~1h |
| 1d  | **Comanda 5330 en BDP real** (creada en prueba 2.2, Status=0 abierta, sin ticket): avisar al cliente para que la **anule desde el TPV** si le afecta (instrucciones en plan, sección follow-up). | Cliente (TPV) | ~5 min |
| 1e  | **Deploy a producción + correcciones de intuitividad** (dudas de Guillermo: demo, snapshots, botones pago/factura, conciliar, cancelar, mapeo, importar/enviar) — plan activo: `Agente/planes/plan-deploy-produccion-intuitividad-2026-08-08.md` (incluye borrador de respuesta al cliente). **Estado 2026-08-09: U1–U8 implementadas y validadas** (typecheck + build Vite OK; bloque 048A-12). Pendiente: gate `task:check`, commit/push, deploy vía coolify-manager-rs y verificación en producción. La activación de escrituras BDP queda sujeta a validación del cliente (paso 5 del plan). | Ninguno (la suscripción WebLink la activa el dueño/cliente en paralelo) | ~2-4h deploy + verificación (UI hecha) |
| 1f  | **048A-22 — Reproducibilidad de Sentinel y del gate coordinado**: plan P0–P3 para identidad única, repin/repair transaccional, gate autónomo, worktrees listos, cleanup recuperable y rendimiento. Plan: `Agente/planes/plan-correccion-auditoria-sentinel-2026-08-08.md`. | Implementación pendiente por fases; gate base conserva deuda separada que no debe ocultarse | Plan listo |
| 1b  | **Lecturas reales BDP, sin escrituras** — conexión, acceso y formas de pago verificados. Catálogo y Compras ya muestran configuración guiada y persistente cuando BDP devuelve cero artículos o rechaza la plantilla. El Explorador queda fuera del criterio de entrega. No se efectuó ningún cambio en BDP. | Cliente: elegir la tarifa que devuelva artículos y aportar un código de plantilla de Compras existente | ~30 min |
| 1b  | **Tests E2E servicio contra simulador con DB** (sync_venta, add_payment, invoice)                                                                                                                                                 | ✅ Hecho (267A-5) — 11 tests: 8 guard + 3 E2E contra simulador+PostgreSQL | ✅ Hecho          |
| 2   | **Activar 6 feature flags** en producción                                                                                                                                                                                         | ✅ UI implementada — se puede activar desde Configuración BDP             | ~1h verificación  |
| 3   | **CancelOrder**                                                                                                                                                                                                                   | BDP: "Subscripción no activada"                                           | ~12-16h           |
| 4   | **S16-H3/H4**: Tests para allowlist y canonical_target                                                                                                                                                                            | ✅ Hecho (267A-4)                                                         | ✅ Hecho          |
| 5   | **Tests simulador** (pagos parciales + compras)                                                                                                                                                                                   | ✅ Hecho — 92 Python + 23 Rust pasando                                    | ✅ Hecho (267A-2) |
| 6   | **Runbook operativo BDP**                                                                                                                                                                                                         | ✅ Hecho                                                                  | ✅ Hecho          |
| 7   | **Feature flags doc**                                                                                                                                                                                                             | ✅ Hecho                                                                  | ✅ Hecho          |
| 8   | **Badge "BDP: off" interactivo** — que permita activar BDP directamente si hay credenciales, o redirigir a Configuración                                                                                                          | ✅ Hecho (267A-6)                                                         | ✅ Hecho          |
| 9   | **Planificar pruebas reales de lectura BDP** — verificar Stock, Explorador, Historial y Compras contra BDP conectado. Actualmente no hay procedimientos documentados para estas 4 páginas; solo existen para escritura (item #1). | ✅ Hecho (267A-6)                                                         | ✅ Hecho          |

---

### Bloque 267A-6 — Badge interactivo + Plan pruebas lectura (implementados)

| ID     | Item                                        | Estado          | Qué se hizo                                                                                                                                               | Archivos clave                                                           |
| ------ | ------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| BADGE-1| Badge BDP:off interactivo                   | ✅ Implementado | Dropdown con "Activar BDP" (si credenciales) o "Configurar credenciales BDP". PATCH + invalidación React Query cache.                                   | `frontend/src/components/site-header.tsx`                                |
| BADGE-2| Cache invalidation post-PATCH               | ✅ Implementado | `useQueryClient().invalidateQueries(['configuracion'])` tras activar BDP para refrescar badge inmediatamente.                                              | `frontend/src/components/site-header.tsx`                                |
| PLAN-1 | Plan pruebas reales lectura BDP             | ✅ Implementado | Procedimientos detallados para Stock, Explorador, Historial y Compras contra BDP real. Checklist previo, campos, filtros, errores, rendimiento, responsive. | `Agente/documentacion/bdp/plan-pruebas-lectura-bdp-2026-07-26.md`        |

## Referencias rápidas

- `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` — riesgos y mitigaciones.
- `Agente/planes/plan-pendientes-bdp-2026-07-23.md` — plan detallado de funcionalidades pendientes.
- `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md` — dónde se ve cada funcionalidad en el frontend.
- `Agente/completados/tareas-2026-07-24.md` — tareas BDP completadas recientemente.
- `Agente/documentacion/bdp/maestro-organizacion-bdp-2026-07-26.md` — inventario y verificación de toda la documentación BDP.
- `Agente/documentacion/bdp/maestro-auditoria-bdp-2026-07-26.md` — consolidación de auditorías y capas de defensa.
- `Agente/documentacion/bdp/feature-flags-bdp-2026-07-26.md` — documentación de los 6 feature flags BDP.
- `Agente/documentacion/bdp/runbook-operativo-bdp-2026-07-26.md` — procedimientos de incidente.
- `Agente/documentacion/bdp/auditoria-testing-simulator-2026-07-26.md` — auditoría de testing del simulador (92 Python + 23 Rust).
