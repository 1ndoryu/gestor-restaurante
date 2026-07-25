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

| Funcionalidad BDP                                                   | Dónde se ve en la web                                | Estado                                       |
| ------------------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------- |
| **Catálogo de artículos** (sync, precios, stock)                    | Configuración → BDP → "Catálogo de artículos BDP"    | ✅ Visible y funcional                       |
| **Mapeos técnicos** (tender, canales, artículo/cliente por defecto) | Configuración → BDP → "Correspondencias Glory ↔ BDP" | ✅ Visible (colapsable)                      |
| **Clientes BDP** (importar/sincronizar)                             | Clientes → "Importar BDP"                            | ✅ Funcional; lista clientes de BDP          |
| **Plano de Sala** (mesas BDP)                                       | Plano de Sala → "Sync BDP"                           | ✅ Funcional                                 |
| **Comandas** (crear orden en BDP)                                   | Ventas → "Enviar a BDP"                              | ✅ Funcional, requiere autorización temporal |
| **Pagos completos** (AddOrderPayment)                               | Ventas → "Pagar en BDP"                              | ✅ Funcional, requiere autorización temporal |
| **Pagos parciales** (AddOrderPayment parcial)                       | Ventas → icono de tarjeta en fila de venta           | ✅ Funcional bajo feature flag `ff_bdp_partial_payments` |
| **Facturas** (InvoiceOrder)                                         | Ventas → "Facturar en BDP"                           | ✅ Funcional, requiere autorización temporal |
| **Estado BDP**                                                      | Navbar (badge BDP: lectura/escritura)                | ✅ Visible e interactivo                     |
| **Polling de estados**                                              | Configuración → BDP → "Actualización de estados"     | ✅ Configurable                              |
| **Explorador de menús/packs/fastfoods**                             | Configuración → BDP → sección inferior               | ✅ Visible y funcional                       |
| **Stock (solo lectura)**                                            | Tabla de mapeos de artículos, columna "Stock"        | ✅ Visible si BDP devuelve stock             |

### ❌ Lo que NO está integrado (por decisión de alcance o pendiente del cliente)

| Funcionalidad                                   | Motivo                                                   | Estado                        |
| ----------------------------------------------- | -------------------------------------------------------- | ----------------------------- |
| **Compras** (albaranes/facturas de proveedores) | Dominio complejo, fuera del alcance inicial              | ❌ Pendiente consulta cliente |
| **Pagos parciales**                             | Implementado bajo feature flag `ff_bdp_partial_payments` | ✅ Implementado (beta)        |
| **Sincronización bidireccional automática**     | Riesgo de bucles y conflictos; no soportada por BDP      | ❌ Rechazado                  |
| **CancelOrder**                                 | BDP responde "Subscripción no activada"                  | ❌ Bloqueado por BDP          |
| **Modificación de stock**                       | Alcance solo lectura en integración actual               | ❌ Fuera de alcance           |

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

### Bloque 247A-10 — Mejoras de stock BDP (en curso)

| ID | Item | Estado | Notas |
|---|---|---|---|
| S1 | Tests de stock (parsing `effective_stock` + upsert DB) | ✅ Hecho | `tests/bdp_article_map.rs` + unit tests en `src/services/bdp_weblink_catalog.rs` |
| S2 | Stock por almacén (solo lectura, almacén por defecto) | ✅ Hecho | Tabla `bdp_article_stock`, warehouse por defecto `"0"` / `"General"`; endpoint `/api/bdp/article-stock` preparado para futuro desglose |
| S3 | Mejorar exportación CSV de stock | ✅ Hecho | BOM para Excel, nombre dinámico con timestamp, columnas extendidas, fila de totales, opción filtrados/todos |
| S4 | Página individual de stock `/bdp/stock` | ✅ Hecho | Filtros, ordenación, paginación, sync catálogo |

### Bloque 247A-9b — Pagos parciales BDP (UI + ambiguos)

| ID | Item | Estado | Notas |
|---|---|---|---|
| P1 | Backend ledger `bdp_pagos`, feature flag e idempotencia | ✅ Hecho | `src/services/bdp_sync.rs`, `src/repositories/bdp_pago.rs` |
| P2 | UI de pagos parciales en `venta-row-actions.tsx` | ✅ Hecho | Diálogo con saldo, historial, añadir pago, generar `idempotency_key`; usa axios `instance` directamente |
| P3 | Reconciliación de pagos ambiguos (`bdp_pagos.resultado='ambiguo'`) | ✅ Hecho | `reconcile_ambiguous_pagos` en `bdp_order_poller.rs`; badge y aviso en UI |

### Bloque 247A-7 — Mitigaciones críticas BDP (implementadas)

| ID      | Riesgo                                                       | Estado       | Qué se hizo                                                                                                    | Archivos clave                                |
| ------- | ------------------------------------------------------------ | ------------ | -------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| R1      | Reconciliación periódica de comandas/pagos/facturas ambiguas | ✅ Implementado | Worker `reconcile_ambiguous_orders` en `bdp_order_poller`; consulta `GetOrder` y cierra auditorías `ambiguo`    | `src/services/bdp_order_poller.rs`            |
| R5      | Timeout global en fase HTTP de `sync_venta`                  | ✅ Implementado | Fase HTTP envuelta en `tokio::time::timeout(Duration::from_secs(45))`                                          | `src/services/bdp_sync.rs`                      |
| R14     | Limpieza manual de `SYNC_LOCKS`                              | ✅ Implementado | Guard RAII `SyncLockGuard` que llama `cleanup_lock` en `Drop`                                                  | `src/services/bdp_sync.rs`                      |
| R2-nota | Lock distribuido perdido tras early commit (cross-instance)  | Documentado  | Evaluar `pg_advisory_lock` de sesión o columna `bdp_sync_status` si se despliega multi-instance                | `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` |

### Bloque 247A-8 — Mejoras de UI/UX BDP (nuevas)

| ID  | Item                                                  | Estado        | Descripción                                                                                                  | Esfuerzo estimado |
| --- | ----------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------ | ----------------- |
| UI1 | **Página dedicada de historial BDP**                  | ✅ Implementado | Ruta `/bdp/historial` con pestañas de auditoría y snapshots. Acciones seguras (solo ver detalles).               | ~2h               |
| UI2 | **Página dedicada del explorador BDP**                | ✅ Implementado | Ruta `/bdp/explorador` para menús/packs/fastfoods con layout de página completa y tabla de líneas.              | ~2h               |
| UI3 | **Página dedicada de stock BDP**                      | ✅ Implementado | Ruta `/bdp/stock` con tabla de artículos, filtros y botón de sync catálogo. Solo lectura.                      | ~2h               |
| UI4 | **Página de stock BDP (solo lectura)**                | ✅ Implementado | Página individual `/bdp/stock` con filtros, ordenación, paginación, exportación CSV y banner de solo lectura. Ver plan en `Agente/planes/plan-stock-bdp-gestionable-2026-07-25.md`. Gestión/lectura por almacén pendiente de decisión. | ~4.5h             |

### Bloque 247A-9 — Decisiones pendientes del cliente

| ID  | Item                                    | Pregunta al cliente                                                                      | Esfuerzo estimado           |
| --- | --------------------------------------- | ---------------------------------------------------------------------------------------- | --------------------------- |
| D2  | **Compras** (solo lectura de albaranes) | ¿Necesita ver albaranes/facturas de proveedores en Glory? ¿El módulo está activo en BDP? | ~8h (fase 1 lectura)        |
| D4  | **Pagos parciales**                     | ✅ Implementado (backend + frontend + reconciliación de ambiguos). Ver `Agente/planes/plan-pagos-parciales-bdp-2026-07-25.md`. Ledger local (`bdp_pagos`), feature flag, idempotencia, prevención de sobrepago, tests de integración y UI en `venta-row-actions.tsx`. | ~18-22h (con lock + ledger) |
| D5  | **CancelOrder**                         | BDP responde "Subscripción no activada". ¿Pueden activar el módulo?                      | ~12-16h si BDP lo activa    |

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

## Referencias rápidas

- `Agente/documentacion/bdp/riesgos-produccion-bdp-2026-07-24.md` — riesgos y mitigaciones.
- `Agente/planes/plan-pendientes-bdp-2026-07-23.md` — plan detallado de funcionalidades pendientes.
- `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md` — dónde se ve cada funcionalidad en el frontend.
- `Agente/completados/tareas-2026-07-24.md` — tareas BDP completadas recientemente.
