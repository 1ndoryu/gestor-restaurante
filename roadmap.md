Objetivo: Sistema de restaurante con integración BDP (WebLink). Backend Rust (Axum) + React SPA.
Rama: main

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
- **Branch**: `main`
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

### Bloque 169A-4 — Staging en VPS + informe de rendimiento sin BDP real (plan activo 2026-09-16)

Plan: `Agente/planes/plan-perf-vps-2026-09-16.md`. **Decisión 2026-09-16:** alcance C
(standalone + mock BDP + 2-3 lecturas reales puntuales autorizadas), pruebas directo en
VPS, push de `main` autorizado. Staging separado `restaurante-perf` en `standalone`
(cero tráfico BDP por construcción); harness versionado con operaciones mínimas reales
(login, dashboard, venta+cobro, catálogo, stock, gasto/reserva); métricas con
`container-stats`; informe con p50/p95 + CPU/RAM + recomendación de recursos.
**Matriz 9/9 ejecutada 2026-09-16 (~251k peticiones, `https://perf.wandori.us`):
T1-sost p95 199 ms 0 errores; T2-pico 76 req/s p95 196 ms; T1-pico 70 req/s p95
232 ms con 1 error cliente transitorio en 42k; sin OOM ni leaks. Veredicto: T1
(1 vCPU/1 GB) sobrado para este perfil; cuello = postgres CPU en pico.**
Informe: `Agente/completados/tareas-2026-09-16.md` (§169A-4 PERF-VPS).
**Pendiente del usuario:** repo a privado; destino del staging (C.5 borrar/conservar);
push del cierre.

### Bloque 169A-5 — Escalar a 50.000 personas consultando con evidencia (plan activo 2026-09-16)

169A-5 (plan activo 2026-09-16)

Plan: `Agente/planes/plan-50k-2026-09-16.md`. Aritmética: 50k × 1 req/10s ≈
5.000 req/s en borde; con ≥95 % HIT en Cloudflare el origen ve ≤250 req/s.
Fases: D1 perfilado (EXPLAIN + clasificación cacheable), D2 origen (índices +
caché en app + headers), D3 borde (proxy naranja + reglas CF + test negativo
de privacidad), D4 harness `consulta50k` (rampa por tramos con parada
automática), D5 medición y claim (cifra = tramo verde redondeado abajo).
Hechos: cero caché hoy, sin CDN en path (gris), índices base OK, manager con
`cache` pero sin reglas CF visibles. **D1 hecho 2026-09-17:** SUMs dashboard
13–15 ms Seq Scan, `ventas_listar` 14,2 ms, maps/stock <0,3 ms; trabajadores
comparten `sub`=propietario → caché por URL segura en single-tenant (D3.3 la
confirma). **D2 código hecho 2026-09-17** (migración + caché resumen TTL 30 s
con invalidación + `Cache-Control`/`ETag` en 5 GETs; `check` limpio, tests
verdes). **D0 saneamiento base (2026-09-17, previo a D2, decisión usuario):**
7 items reparados — 3 planes con checklist/archivado, `type-check`
(`node_modules` en `glory-rs`), `clippy -D warnings` (17→0), `rustfmt`,
`npm_execpath` robusto (commit 27007ce). **D2 commit 49b56c2 + push 2026-09-17**
(verificado: fmt/check/clippy limpios, suite 33 binarios verde). **D2 deploy +
D2.4 2026-09-17:** `deploy --update` OK en `restaurante-perf`, `health` ok,
migración `20260917000000` aplicada + índices origen50k presentes en staging;
`GET /api/dashboard/resumen` con `Cache-Control: max-age=30, private` + `ETag`
estable + `304`; en origen 36,6 ms frío y ~7 ms p50 caliente (≤30 ms ✓);
`EXPLAIN` 7–12 ms (Seq Scan correcto para usuario demo 16,8k/17k filas).
Autorizaciones pendientes: proxy+reglas CF,
tramos de carga, publicar cifra. Límite abierto:
`rust-test` ~350 s supera el budget 300 s del gate (solución pendiente).

### Bloque 099A-1 — Paquete restaurante 2026-09-09: hosting ES/UE + MFA + SaaS + seguridad (plan activo 2026-09-09)

Origen: reunión del restaurante trasladada por Guillermo el 2026-09-09 (chat 04–09/09/2026 en
`C:\Users\Owner\Downloads\Guillermo-conversacion-2026-09-09\chat.md:121-167`). Plan detallado:
`Agente/planes/plan-saas-servidor-seguridad-2026-09-09.md`. Prioridad declarada por el cliente:
**autenticación y servidor**. El SaaS queda **en espera** hasta confirmar si todos los clientes
usarán BDP o habrá distintos softwares (pregunta abierta a Guillermo).
Trabajo adicional al alcance original (la app nació single-tenant, sin MFA, en un solo stack).

**Estado actual verificado (2026-09-09, solo lectura):** auth JWT+argon2 sin MFA
(`src/services/auth.rs`, `src/handlers/auth.rs:49`, `src/handlers/trabajadores.rs:269`);
sin entidad tenant/empresa (sin `tenant`/`empresa_id`/`organization` en `src/`);
HTTPS ya en el borde (Traefik + Let's Encrypt con redirect http→https en
`temp-compose-glory-rest.yml:44-53`); app y postgres ya en contenedores separados pero en el
mismo host/red Docker. Deploy siempre vía coolify-manager-rs y solo con autorización explícita.
**Dudas resueltas 2026-09-09:** MFA = OAuth Google delegado; hosting = mantener Coolify en
servidor ES/UE; SaaS en espera; estructura en ambos niveles (ejecutiva + código).
**Siguiente paso:** F1 (estructura + estimación); sin migrar ni desplegar nada hasta entonces.

### Bloque 149A-1 — Revisión integral BDP REINICIO: todo desde cero con confirmación visual ítem por ítem (plan activo 2026-09-14)

Plan activo: `Agente/planes/plan-revision-integral-bdp-2026-09-14.md`. Sustituye a `039A-1`
(archivado como **cerrado por reinicio**: `Agente/planes/completados/plan-revision-integral-bdp-2026-09-03.md`).
Motivo del reinicio: los refactors y los cambios de CSS posteriores invalidaron la evidencia previa,
así que se repite todo desde cero, incluidas las pruebas reales.
**Método nuevo:** el usuario confirma **visualmente ítem por ítem**; ningún ítem se marca sin
`✅ confirmado por usuario <fecha> <hora>`, y lo que pida corregir se abre como `H-149A-1-<n>`, se
corrige y se vuelve a mostrar. Partes: **Parte 1** independiente `standalone` (P0–P13 + P14 nuevo:
tokens, responsive, foco, zoom, temas, estados vacío/carga/error), **Parte 2** lecturas reales
(authorización previa obligatoria; Q0–Q5), **Parte 3** simulación del cliente (S0–S2). Las
escrituras no viven aquí: están en 149A-2.

**Siguiente paso:** P0 (baseline del repositorio) con evidencia técnica + primera captura para tu OK.

### Bloque 149A-2 — Escrituras BDP REINICIO: auditoría + simulaciones + reales autorizadas (plan activo 2026-09-14)

Plan activo: `Agente/planes/plan-seguridad-escrituras-bdp-2026-09-14.md`. Sustituye a `049A-1`
(archivado como **cerrado por reinicio**: `Agente/planes/completados/plan-seguridad-escrituras-bdp-2026-09-04.md`).
Repite desde cero Fase 1 (auditoría anti-desastre de las 13 operaciones Q2.1–Q2.13 × 13 dimensiones),
Fase 2 (simulaciones: suites + simulador Python `:18765`; escenarios S1–S9 con confirmación visual
de los estados de cola) y Fase 3 (escrituras reales una a una, **solo con autorización explícita por
operación**; pago/factura/cancel `⏸` hasta activar la suscripción WebLink — **matiz 2026-09-16:**
ver bloque 169A-2, el pago dentro del `CreateOrder` inicial puede funcionar en gratuita y se
prueba de noche; el `⏸` sigue valiendo para `Payment/Add` a comanda existente).
**Historia que no se borra:** bajo 049A-1 sí se ejecutó **W-Q2.1 real** (alta de `90000003` →
INCIDENTE `200109`) y su fix local `sync_catalog`; el residuo `90000003` sigue en el BDP real
(pendiente 1g) y `ModifyArticleAndUpdateProfile` sigue sin funcionar contra el BDP real con payload
mínimo.
**Siguiente paso:** Fase 1 (no toca red ni BDP).
**Subplan 2026-09-15 (navegador diferido):** `Agente/planes/plan-catalogo-bdp-front-honesto-2026-09-15.md`
— front honesto de Catálogo (código visible + estado push + error que distingue),
lecciones de W-Q2.3; F1 obligatorio, F2 recomendado, F3 diferible.

### Bloque 149A-3 — Permisos por rol y errores silenciosos (subplan de 149A-1) (plan activo 2026-09-14)

Plan: `Agente/planes/plan-permisos-y-errores-silenciosos-2026-09-14.md`. Nace de un reporte del
usuario en la revisión 149A-1: **el menú no filtra por rol** (el trabajador ve Configuración,
Trabajadores y Sincronización), los **403 dejan la pantalla en "Cargando..." sin aviso** y se
**reintentan 4–5 veces**, el backend solo protege `trabajadores.rs` con `require_owner`, y el gate
**no tiene ninguna regla** que detecte ese fallo silencioso (`sentinel.config.json` sin reglas de
error visible al usuario). Fases: F0 inventario rol × página × endpoint con **ambas cuentas**, F1
decisión de modelo (**requiere al usuario**), F2 UI honesta ante 403/401 sin reintentos, F3 menú por
rol, F4 guardas consistentes en backend, F5 propuesta de regla al gate (sin modificar el gate sin
autorización), F6 verificación con ambas cuentas y confirmación visual por ítem.

**Hecho:** F4 Tanda A (guardas `require_role` en los 9 endpoints críticos, probadas con las dos
cuentas, cero daño), **F4 Tanda B (2026-09-16, commit `c40de93`)**: `CatalogoEdicion` en
import-catalog/sync-catalog/sync-prices + `require_role(Admin)` en sync-tables, tests 3/3 y
sondas vivas (trabajador 403 ×4, dueño pasa guards) y **F2 (UI honesta)**: `EstadoError` compartido, `retry` que no repite 4xx y las
3 pantallas que mentían (Trabajadores, Sincronización, Chatbot) ya dicen "no tienes permiso". También
reparado el bloqueo preexistente de `cargo test --lib` (176 passed / 0 failed, imports del split).

**F1 RESUELTA por el usuario (2026-09-16)**: (a) modelo = `permisos_*` por acción
actual (fail-closed, solo dueño, delegable); (b) menú = deshabilitado con aviso, no oculto;
(c) alcance = configurable por el dueño. **Hecho 2026-09-16 (este commit):** **F0** (barrido del
dueño: las guardas dejan pasar a `Admin` por diseño; sondas vivas dueño 200/422 donde trabajador
403 — ningún endpoint bloquea al dueño), **F3** (menú deshabilitado con aviso: `authStore`
`rolEfectivo()`/`esTrabajador()` desde `effective_role` del JWT —verificado en vivo `"admin"` /
`"trabajador"` con `tid` + `permisos:[]`—; `soloAdmin` en Sincronización, Trabajadores y
Configuración; TODO [C2-3] del site-header cerrado; `check:front` cero errores en `frontend/src`),
**F5** (propuesta sin tocar el gate: 32 ficheros con `sentinel-disable-file sqlx-query-sin-macro`
justificados —las macros exigen BD viva en compilación y el proyecto compila offline—; propuesta:
exigir motivo citado + medición del patrón F2 con caso mínimo `Trabajadores.tsx` pre-F2).
**F6 HECHA 2026-09-16 (navegador automatizado :5183)**: menú trabajadora con las 3
entradas deshabilitadas + aviso, desplegable BDP con las 2 opciones admin deshabilitadas,
`/trabajadores` directo → 403 honesto, "Importar del BDP" como trabajadora → toast de error
honesto (cero escrituras), dueño todo habilitado + lista 3 trabajadores. **Subplan 149A-3
COMPLETO** (detalle en el plan §7 + `tareas-2026-09-16.md`).

**Siguiente paso:** 169A-2 nocturna (venta con pago en BDP real). 169A-3 HECHO 2026-09-16.

### Bloque 169A-3 — Permisos de menú por trabajador: qué ve cada rol, todo configurable (nueva 2026-09-16)

Pide el usuario: hoy el trabajador ve las 23 entradas del menú (WA, recordatorios, historial,
campañas, no-shows…). Lo lógico por defecto + configurable por el dueño (no se sabe qué querrá
el cliente). Estado actual verificado: existe `permisos_trabajador` + `SECCIONES_VALIDAS` (11
gruesas) + panel de checkboxes del dueño + `permisos` en el JWT, **pero nada lo aplica** (cero
`require_seccion` en backend, menú no lo lee).

**Propuesta de defecto (todo reconfigurable por el dueño y por trabajador):**
- SÍ por defecto (operativa diaria): Dashboard, Ventas, Gastos, Reservas, Calendario, Clientes,
  Canales, No-Shows, Plano de Sala.
- NO por defecto (técnico/propietario/encargado): Configuración, Sincronización, Trabajadores,
  Historial, Catálogo, Stock, Inventario, Compras, Menús y Packs, Campañas, Plantillas WA,
  Recordatorios, Reseñas, Inactividad.
- Las 3 admin por rol (Configuración, Sincronización, Trabajadores) siguen además con guard
  backend + deshabilitado-con-aviso F1 aunque se concedan por error.

**Fases:** M1 extender `SECCIONES_VALIDAS` a grano por entrada (~23 claves, conviviendo con las 11
actuales) + sembrar defecto por rol; M2 panel del dueño con las nuevas claves; M3 menú filtra por
`permisos` del trabajador (decidir: ocultar vs deshabilitar-con-aviso); M4 `require_seccion` en
backend para las sensibles (hoy el array es informativo); M5 verificación con ambas cuentas.
**Decisión del usuario 2026-09-16: deshabilitar con aviso (no ocultar) + luz verde (implementar ahora).**

**HECHA 2026-09-16 (M1-M5, sin commit):** `SECCIONES_VALIDAS` 24 por entrada de menú
(`campanas,plantillas_wa,recordatorios,resenas,inactividad` + `notificaciones` sin entrada pero
configurable; `marketing` gruesa migrada a hijas vía `20260916000000_*.sql` up/down); defecto
trabajador 9 claves operativa diaria (solo en creación; `Some([])` = sin acceso); JWT `secs`
firmado; `verificar_seccion` (desconocida→403, dueño siempre pasa) en crear/actualizar/eliminar/
enviar campañas+plantillas+recordatorios, solicitar reseña e inactividad (guard primero,
fail-closed); `GET /trabajadores/secciones` devuelve array 24; `PATCH /:id {permisos}` reemplaza.
Panel dueño con `ETIQUETAS_SECCION` + nota semántica defecto/re-login; menú filtra por
`tieneSeccion()` con deshabilitado+`AVISO_SECCION` (`disabled`+`aria-disabled`+`title`),
quick-actions Venta/Gasto/Reserva condicionadas. Tests `permisos_secciones_menu` 4/4 +
`cargo check --lib` OK. Verificado en vivo :3100+:5183: `/secciones`=24; trabajadora crear/
actualizar campaña válida→403, dueño actualizar inexistente→404; PATCH Sara=9 defecto →
re-login `permisos`=9 + `secs`=9; navegador: 9 links + 14 deshabilitados (`disabled=true`,
`title`=aviso) + `+Venta disabled=false`. Gotchas: preview :3100 necesita `.env` exportado
explícito (`dotenvy` no basta tras reinicio; `BDP_DEFAULT_ARTICLE_NAME` obligatorio) + `PORT=3100`
+ CWD=repo; Json-extractor 422 precede al handler (sondas 403 exigen body válido);
`PUT /trabajadores/:id` no existe (405) → es `PATCH`. Docs + commits hechos
(`9296119` M1-M5, `203cf8e` N1 NavUser, `3e19107` lote barato). Solo queda el
push (pendiente de autorización, igual que los previos).

### Bloque 169A-2 — Pago dentro del CreateOrder + prueba nocturna en gratuita (plan activo 2026-09-16)

Hallazgo 2026-09-16 (chat Guillermo): el `301010` es solo de `GetOrder`; nuestro pago de 0,11 €
nunca se envió (lo frenó el guard propio de reconciliación) y la sonda directa a `Payment/Add`
con importe 0 devolvió `[301201]` (validación de negocio, no de licencia). El manual solo quita
en gratuita "agregar pago/propina/factura a comanda **existente**" (`# WEBLINK RESTAPI.md:166-174`)
y documenta `Payments` (máx 3, total o parcial), `Tip` e `Invoice=true` dentro del `CreateOrder`
inicial. Hoy `build_order` (`bdp_sync_venta.rs:671-719`) envía sin `Payments`/`Tip` e
`Invoice=false`: ese flujo en dos pasos es el bloqueado. `GetApplicationVersion` 84 → WeblinkRestAPI
v1.2 sin errores; 89 → Hostelería v36.2; ninguna indica el tipo de suscripción.

**De día (sin red BDP) — HECHO 2026-09-16 (commit `263155b`):** `build_order` incluye
`Payments: [{TenderId, Amount, PaymentId}]` (PaymentId = MarketplaceOrderId estable), `Tip` si
propina positiva e `Invoice=true` solo con tender y total positivo; sin tender todo como antes.
Tests nuevos 3/3 + regresión `cargo test --lib` 182/182 + `build --bins` OK. Nota: se purgó el
target de rama `C:\tmp\glory-target\glory_backend_main` (4,56 GB, permitido por AGENTS.md) porque
el gate del wrapper exige 6 GB libres; el workflow actual compila en `C:\tmp\glory-target\debug`.
**Protocolo silencioso y cuidadoso (no romper nada, 2026-09-17):** `EndType=0` **PROHIBIDO**
(imprime en cocina; solo con autorización separada + aviso a cocina/TPV). Orden obligatorio:
P0 solo-lectura (`Health` + `GetApplicationVersion` + `sync-dry-run` existente, cero escrituras) →
P1 `OnlyCheck` (`OrderOperationType=1`) con el payload exacto de pago-en-creación
(`Payments/Tip/Invoice=true`, `EndType=1`, `MarketplaceOrderId PRUEBA-*`), cero creación →
P2 real mínima (0,11 €, artículo genérico, `EndType=1` pendiente en autocomanda, hueco muerto
16-18h, ver `OrderId`, anular en <1 min vía `CancelOrder`, verificar modo `read_only` restaurado).
Parar ante cualquier error de licencia/5xx/impresión inesperada. Prohibido tocar `.env`,
reiniciar el backend sin backup, tocar datos reales (solo `PRUEBA-*`), o dejar residuos.
Si BDP devuelve error de licencia en P1/P2, queda confirmado que hace falta suscripción de pago.
**Supersede:** deja obsoleta la hipótesis "pago/factura ⏸ hasta suscripción" del bloque 149A-2
para pagos (la factura vía `Invoice=true` también entra en la prueba).

### Bloque 039A-1 — Revisión integral BDP: independencia funcional + integración completa (3 rondas) (CERRADO POR REINICIO 2026-09-14)

Plan archivado: `Agente/planes/completados/plan-revision-integral-bdp-2026-09-03.md`. Cerrado con
**104 ítems hechos / 38 pendientes**; su evidencia (Parte 1 P0–P13, S1 de la Parte 3, lecturas Q1
reales) quedó obsoleta por los refactors y cambios de CSS y se rehace en 149A-1. Deuda que
sobrevive: el residuo `90000003` en el BDP real (pendiente 1g) y el `ModifyArticleAndUpdateProfile`
que revienta en el BDP real con payload mínimo.

### Bloque 049A-1 — Seguridad de escrituras BDP: auditoría anti-desastre + simulación antes de escribir (CERRADO POR REINICIO 2026-09-14)

Plan archivado: `Agente/planes/completados/plan-seguridad-escrituras-bdp-2026-09-04.md`. Cerrado con
Fases 1–2 ejecutadas (S1–S8 PASS) y **una escritura real ejecutada** (W-Q2.1 → incidente `200109`,
residuo `90000003` en el BDP real). Se repite desde cero, con confirmación visual, en 149A-2.



### Seguimiento 318A-3 — Evaluar reactivación de reglas de consistencia de formularios (2026-09-01)

Informe del cierre de PROYECTO TASKS (plan `PROYECTO TASKS/Agente/planes/` 318A-3): este proyecto
tiene `html-nativo-en-vez-de-componente`, `componente-artesanal` y `componente-sin-hook-glory`
deshabilitadas por otro agente. Acción propuesta (no ejecutada aquí, sin tocar el gate): revisar su
reactivación tras la unificación de formularios del sistema declarativo de PT; requerirá autorización
explícita para modificar `sentinel.config.json` de RESTAURANTE.

**Nota (318A-4, 2026-09-01):** el fix F2 (rutas workspace de components/ui/shared en
`reactComponentRules`) fue publicado en sentinel v0.7.6 (`fbb580f`) y v0.7.7 (`0559576`); los
hallazgos `html-nativo` que se reactiven al habilitar la regla serán reales (con seam de migración
si el sistema declarativo aplica). Solo doc, sin gate.

### Bloque 208A-2 — Corrección de la independencia BDP (H1–H8) (completado 2026-08-27)

Plan archivado: `Agente/planes/completados/plan-correccion-independencia-bdp-2026-08-27.md`.
Hereda la **auditoría 208A-1** y las decisiones D1–D6. **C1**: CRUD de artículos movido a la
página "Catálogo" (pestañas Artículos / Departamentos y familias); Configuración → BDP queda
solo con conexión/mapeos/permisos + enlace "Ir a Catálogo". **C2**: botón "Nuevo artículo" en
Stock (NuevoArticuloDialog) + empty state accionable + "Sync catálogo/precios" deshabilitados
fuera de modo bdp (H7). **C3**: conteo de inventario persistido (migración
`20260827000000_bdp_conteos_inventario` con idempotencia), endpoints `GET/POST
/bdp/inventario/conteos` + `GET /:id`; el guardado aplica la diferencia al stock local con
motivo "conteo" (D4) y encola solo líneas con código BDP; UI con "Guardar conteo", historial y
"Retomar" y mensaje honesto en standalone. **C4**: sección "Sincronización" en el menú con
`GET /bdp/push/pendientes` y `POST /bdp/push/:id/reintentar` (reintento manual D2); acciones
solo en modo bdp. **C5**: normalización standalone+sync → sync=false al guardar (H5). **C6**: empty
state de Compras con "Nuevo albarán" (H8). **Verificado**: `cargo check` exit 0; tests nuevos
12/12 (conteos 4, cola 5, normalización 3) + regresión bdp_inventario 3/3 y bdp_push 13/13,
ejecutados test por test (regla nueva `no-heavy-suites` en el AGENTS.md raíz); `tsc` limpio;
UI end-to-end (alta TEST-1, stock 5 tras conteo, cola visible sin envío, Configuración sin CRUD,
cero tráfico a BDP). Trabajo sin commitear.

### Bloque 208A-1 — Auditoría integral independencia BDP, revisión 1×1 (auditado 2026-08-27)

Plan archivado (2026-09-03, bloque 039A-1):
`Agente/planes/completados/plan-auditoria-independencia-bdp-2026-08-27.md`. El usuario detectó
que la UX no refleja la independencia (Stock no crea artículos, Inventario no persiste conteos,
el CRUD de artículos está escondido en Configuración, Compras bajo sospecha). **Regla: NO se
implementa nada durante la auditoría** — se verifica 1×1 cada dominio (modo operativo, catálogo,
stock, inventario, anulación, compras, pagos/factura local, menús, historial, permisos,
escrituras 198A-1, UX, integridad de datos), se anota hallazgo con evidencia y severidad, y al
final se decide el plan de corrección.
**Resultado 2026-08-27:** baseline verde (`cargo check` exit 0, 153 tests, `tsc` limpio). El
núcleo de independencia está implementado y testeado; la deuda es de UX/ubicación: **H1** (CRUD
de artículos solo en Configuración → BDP → "Catálogo de artículos BDP"; la página "Catálogo"
solo tiene departamentos/familias — **Alto**), **H2** (Stock sin "Nuevo artículo"; empty state
solo sugiere BDP/demo — **Alto**), **H3** (Inventario: conteo solo `useState`, no persiste;
en standalone "Enviar" es no-op con toast engañoso — **Alto**), **H4** (diferencia contada no se
aplica al stock local — **Medio**), **H5** (sin normalización al guardar standalone+sync —
**Bajo**), **H6** (sin visibilidad de la cola de push en UI; solo flush global — **Medio**),
**H7** ("Sync catálogo" habilitado en standalone — **Medio**), **H8** (empty state de Compras no
ofrece "Nuevo albarán" — **Bajo**). Verificado como correcto (no es bug): encolado local en
standalone (filas pendientes que se envían al conectar BDP), test de invariante de flush,
migraciones aditivas, serie L-, rango reservado. **Siguiente paso:** decidir el plan de
corrección (decisiones del usuario en §6 del plan) — NO se ha implementado nada todavía.

### Bloque 138A-2 — Verificación LECTURA REAL de las 24 lecturas BDP "en uso" (absorbido por 039A-1, 2026-09-03)

Plan archivado (bloque 039A-1): `Agente/planes/completados/plan-prueba-lecturas-bdp-2026-08-18.md`.
El objetivo queda encomendado a la Parte 2 del plan centralizado 039A-1 (bloque Q1). Verificar contra el **BDP
REAL del restaurante** (`100.83.196.35:8068`, solo lecturas, cero escrituras) que las 24 funciones
de lectura marcadas "en uso" en el inventario final (64 funciones) siguen respondiendo tras
F0–F10 (128A-1). Sin simulador (descartado por decisión del usuario), sin escrituras, sin deploy.
Pendiente de confirmar BDP online + credenciales — condición heredada por la Parte 2 del plan
centralizado 039A-1.

### Bloque 198A-2 — Pruebas de interfaz: independencia + escritura (sin BDP real) (completado 2026-08-19)

Plan archivado (2026-09-03, bloque 039A-1):
`Agente/planes/completados/plan-pruebas-interfaz-bdp-2026-08-19.md`. Verificar a nivel de
**interfaz** (navegador + backend local + BD de rama) que la independencia (128A-1) y los
efectos locales de la integración de escritura (198A-1) funcionan en `standalone`, y que
**ninguna** funcionalidad ofrece ni envía nada a BDP (cero tráfico a `100.83.196.35:8068`).
Cubre: conmutador/badge, catálogo/stock/anulación/compras/pagos-factura local/menús/permisos,
y los controles nuevos (artículo D3, departamento/familia D7, propina D8, puntos D9, inventario
D6=A, CallWaiter D10 oculto, "Sincronizar a BDP" oculto, CancelOrder F6). Sin BDP real, sin
deploy.
**Progreso 2026-08-19 (primera pasada):** stack aislado (:3100 backend / :5180 vite, BD de
rama, seed demo), login OK, `modo_efectivo=standalone` verificado; badge off ✅, Sincronizar
oculto ✅, CallWaiter oculto ✅, departamento creado local con código secuencial ✅, inventario
renderiza ✅, propina/anulación locales visibles ✅, banners de desactivación ✅, **cero tráfico
a BDP** ✅. **Bug corregido:** bucle infinito `Maximum update depth exceeded` en
`BdpStatusIndicator` (`site-header.tsx`) — memoizado `serverData` con `useMemo`.
**Progreso 2026-08-19 (segunda pasada, completa):** propina end-to-end (diálogo → `5.50 €`
persistido en `ventas.propina`) ✅, anulación local end-to-end (motivo + confirmación →
`anulada=true` + `anulacion_motivo`) ✅, Compras (4 albaranes, sync deshabilitado, nuevo albarán) ✅,
Explorador (menús/packs locales + 4 definiciones BDP) ✅, Historial (4 auditoría + 2 snapshots;
registra `anular_venta` Local) ✅, Stock (6 artículos, sync deshabilitado) ✅, **403 de permisos**
(trabajador → `POST /api/bdp/push/flush` y `GET /api/trabajadores` 403 claro; lectura de
configuración 200) ✅, **cero tráfico a BDP** (solo localhost) ✅.

### Bloque 198A-1 — Escrituras BDP completas (completado 2026-08-19)

Plan (archivado): `Agente/planes/completados/plan-escrituras-bdp-completas-2026-08-19.md`.
**Objetivo:** que todo lo que
BDP pueda aceptar como escritura se escriba (**15 nuevas** —catálogo, stock, departamentos, propina,
`CallWaiter` en el plano, puntos— + `CancelOrder` pendiente de suscripción), construido sobre los datos
locales de 128A-1 (catálogo, stock, departamentos, anulación, propina, puntos) con cola de push
unidireccional (`local_dirty` → BDP), guards/arming/backup/auditoría, y
**independencia intacta** (en `standalone` nada se envía). Incluye 5 lecturas de soporte
(`GetApplicationVersion`, perfiles para crear/modificar, `GetPoints`) y ampliación del simulador.
Fuera de alcance (BDP no lo expone): menús/fastfoods/packs y compras — permanecen locales.
**Revisiones profundas 2 y 3 (2026-08-19):** 16 conflictos nuevos (M11–M26: códigos no devueltos por
`CreateArticlesAndUpdateProfiles`, dependencia departamento→artículo, mapeo IVA `bdp_tav_map`, códigos
de familia, `AllProfiles`, identificadores BDP para propina/cancelación, simulación de suscripción por
módulo, migraciones aditivas, concurrencia con UNIQUE parcial + `FOR UPDATE`, colisión de rango
reservado, límites de recurso, `GetApplicationVersion` por módulo, `ErrorList` parcial de stock/inventario,
requisitos `WebArticle`/`Inventariable`, `OrderIdentifier { OrderId }`). D1 resuelta: `push_modalidad`
configurable (default `automatico`). D2 resuelta: bloqueo por suscripción → reintento **solo manual**
(el auto-reintento queda para errores transitorios). D3 resuelta: código de rango reservado automático
editable (default `90xxxxxxx`), `AutomaticCode` descartado (M11). D4 resuelta: perfiles = todos los POS
activos (`AllProfiles=true` en departamentos). D5 resuelta: almacenes y motivos configurables en
Configuración (defaults Store=1, CodReg=1). D6 resuelta: UI completa de inventario (conteo físico,
diferencias, envío por lotes). D7 resuelta: códigos de departamento/familia/subfamilia por asignación
secuencial automática. D8 resuelta: propina configurable por venta (sumar/sustituir, default sumar).
D9 resuelta: fidelización con gating por módulo (puntos en ficha de cliente, `pendiente_suscripcion`
si no hay módulo). D10 resuelta: incluir `CallWaiter` (botón "llamar camarero" en el plano).
**Progreso final (cerrado):** catálogo con 20 endpoints + cliente con 20 métodos + structs PascalCase;
migraciones `bdp_push_escrituras`, `bdp_push_estado_ancho`, `bdp_catalogo_propina_puntos` y
`bdp_write_arming_ampliar` (corrige las CHECK de scopes/dominios que bloqueaban el arming del push);
`BdpPushService` + `BdpPushFlushService` (worker con guards, no-op en standalone); wiring en handlers
locales (artículo D3, departamento/familia D7, propina D8, puntos D9, inventario D6=A) + UI completa
(CallWaiter D10, propina, puntos, catálogo, inventario) + botón "Sincronizar a BDP" (flush manual D1/D2);
`CancelOrder` como push (F6): `payload_cancelar` + encolado `venta/cancelar` desde la anulación local
con `bdp_order_id`. Tests (153 unit + 13 bdp_push + 3 bdp_inventario + 24 bdp_f8_permisos + 8
bdp_service_integration). **Pendiente diferido por diseño:** verificación real contra BDP
(suscripción/datos del cliente; BDP offline).

### Bloque 128A-1 — Independencia total del BDP (completado 2026-08-13)

F0–F10 implementados en `glory-rs-rest` (commits `821954c0`…`e12b3968`, gate `task:check 128A-1
--full` PASS en F7/F8/F9): conmutador `standalone`/`bdp` con badge, catálogo local, stock local,
anulación local, compras locales, historial/pagos parciales/factura local, menús/packs locales y
permisos operativos por acción con enforcement backend (403). Sin escrituras ni deploy: pendiente
de autorización del usuario para llevar a producción. **Deuda declarada (F10-1) — CERRADA:** M1 (invariantes del conmutador), M2 (histéresis) y M3
(cache/invalidación) estaban parcialmente abiertas al cierre. Se cerraron: M1 — se eliminaron los
checks redundantes de `bdp_sync_enabled`/`bdp_configurado` que contradecían el modo forzado `bdp`
en `bdp_sync.rs` (sync_venta/add_order_payment/invoice_order), `venta.rs` (retry_bdp_sync),
`ventas.rs` (obtener_bdp_status) y `bdp_write_guard.rs` (try_auto_arm/armar_push), y el poller dejó de
filtrar por `bdp_sync_enabled` (el modo efectivo es la única puerta); M2 — ya implementada en
`ServicioModoOperacion` (umbral 3 fallos, registrar_fallo/exito) y cableada al poller y a las
escrituras directas (`bdp_payment`/`bdp_invoice` bloquean en degradación y alimentan la histéresis);
M3 — `invalidar` se llama al guardar configuración y `modo_efectivo` (con TTL) tiene consumidores.
Plan cerrado en
`Agente/planes/completados/plan-independencia-bdp-2026-08-12.md`; evidencia por fase en
`Agente/completados/128A-1-F4-anulacion-local-ventas.md`, `128A-1-F5-compras-locales.md`,
`128A-1-F6-auditoria-local-pagos-factura.md`, `128A-1-F7-menus-packs-locales.md`,
`128A-1-F8-permisos-operativos.md` y `128A-1-F9-pruebas-bdp.md`; resumen en
`Agente/completados/tareas-2026-08-13.md`.

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
| 1g  | **049A-1/S2 Residuo: artículo web 90000003** — creado en W-Q2.1 como prueba `WebArticle:true`; es el primer y único artículo web del BDP real. `ExportArticles` falla con `[200109]` sin posibilidad de DeleteArticle (no existe en WebLink). `ModifyArticleAndUpdateProfile` del BDP real revienta con NRE (payload mínimo) y no neutraliza `WebArticle:false`. El fix local sync_catalog (fallback perfil H-Q1-03) ya está implementado y commiteado, pero **90000003 sigue en el BDP** como residuo. Pendiente de: (a) obtener el contrato ModifyArticle completo (~100 campos, ver `Agente/documentacion/bdp/contrato-modifyarticle-2026-09-07.md`) y enviar payload completo para neutralizar; o (b) solicitar al proveedor WebLink la eliminación del artículo en BDP. | BDP real: ModifyArticle no acepta payload parcial (NRE) | ~2-4h preparación payload completo + autorización escritura |
| 1h  | **049A-1/S2 Escritura real Q2.2 — ModifyArticle contra BDP** (diferido). El read-modify-write está implementado y simulado (commit `47b95ed`), pero la escritura real de modificación de artículo (p.ej. sobre `90000003` con payload completo) requiere autorización explícita y confirmación del contrato completo con el BDP. Diferido hasta tener resuelto el residuo 1g. | Depende de 1g | ~2h escribir + verificar |
| 10  | **099A-1 Hosting ES/UE** — migrar `glory-rest` desde EE.UU. a CPD en España (o UE) de empresa que no sea Amazon/Google/Azure. Incluye elegir destino, recrear servicio vía coolify-manager-rs, verificar health+TLS y DNS. | Destino sin elegir; deploy nuevo requiere autorización explícita | ~1-2d (según proveedor) |
| 11  | **099A-1 MFA** — doble autenticación (prioritaria). Opciones: TOTP local o delegar en OAuth Google / Google Workspace. Sin MFA hoy (JWT+argon2 en `src/services/auth.rs`). | Decidir TOTP vs OAuth delegado | ~1-2d |
| 12  | **099A-1 SaaS multiempresa** — una plataforma, datos separados por cliente (empresa/tenant, aislamiento, alta/suscripción, página inicio). EN ESPERA hasta confirmar si todo va con BDP o multi-software. | Pregunta abierta a Guillermo | ~1 semana tras confirmación |
| 13  | **099A-1 Seguridad y capas** — solo puertos necesarios, cifrado entre máquinas (texto plano solo intra-máquina), separar capas app/modelos/BD y plan de escalado. | Diseño pendiente de dudas | ~2-3d |
| 14  | **099A-1 Entornos + normativo** — entorno dev/pruebas separado de producción; figuras responsables seguridad/protección datos + documentación. | Alcance doc por confirmar | ~1-2d |
| 15  | **099A-1 Estructura para el cliente** — entregar descripción de la estructura (nivel por confirmar: código vs arquitectura ejecutiva) + plan + estimación. | Duda abierta | ~2-4h |
| 16  | **099A-1 Registro en España (aviso, sin acción)** — pedirán código fuente y más; Guillermo pregunta condiciones/pago por facilitarlo. Lejos aún, solo registrado. | — | — |
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
| 267A-7 | **F2 DIP handlers→repositories** (plan `PROYECTO TASKS/Agente/planes/plan-deuda-restante-039A-1-2026-09-11.md`): 8 sitios con SQL directo migrados — `admin.rs` (reset demo → `AdminRepository`), `bdp_customer_sync.rs` (tx post-create_customer → variantes `_tx` en `ClienteRepository`/`BdpAuditLogRepository`), `configuracion.rs` (arming snapshot/armar/desarmar → `BdpWriteArmingRepository`), `ventas.rs` (lookup InvoiceNumber idempotente → `BdpAuditLogRepository`). Sin cambio de comportamiento: SQL y mensajes idénticos. | Ninguno (código; sin BDP real) | Hecho + gate local-light PASS + commit/push `ef6b830` (2026-09-13). Pendiente `--full` (cooldown) y cierre |
| 267A-8 | **F3 splits `funcion-larga-rs` >200** (mismo plan): `sincronizar_cliente_bdp` (264), `sync_venta` (291), `add_order_payment` (276), `invoice_order` (223) en `services/bdp_sync.rs` + `handlers/bdp_customer_sync.rs`. Las de 102–185 quedan exceptuadas con trigger 200. | Después de 267A-7 | Hecho + gate local-light PASS + commit/push `ef6b830` (2026-09-13). Pendiente `--full` (cooldown) y cierre |
| 267A-9 | **F4 monolitos** (mismo plan + sub-plan `Agente/planes/plan-f4-split-bdp-sync-2026-09-12.md`): `services/bdp_sync.rs` 3606 → 1041 líneas (hub + `bdp_sync_venta.rs` + `bdp_sync_pago.rs` + `bdp_sync_factura.rs` + `bdp_sync_catalogo.rs`; `limite-lineas-nivel-3` resuelto). F4.4 medido 2026-09-12/13: sentinel PASS 0 errores (`build_order` 130 ef. warning EXC, 0 funciones >200 propias); fix clippy `AuditoriaDirecta` (error preexistente `bdp_backup.rs:482` cerrado con autorización); 2 planes cerrados movidos a `completados/` (docs PASS). Gate 267A-9 PASS (sentinel/rust/docs) + commit/push `ef6b830` (2026-09-13). Pendiente: `services/bdp_weblink_catalog.rs` (1708 líneas, deuda aparte) + `--full` tras cooldown + cierre. | Después de 267A-7/8 | En curso (tomada T-1789288127359; código + gate + push hechos) |
| 159A-2 | **Unificar botones sync en Importar/Exportar + importar departamentos/familias del BDP** — plan: `Agente/planes/plan-sync-importar-exportar-bdp-2026-09-15.md`. 13 botones sync reales inventariados (catálogo, stock, clientes, ventas, plano, sincronización, config); objetivo 2 botones por sección + endpoint nuevo importar-departamentos (factible: ExportDepartments ya cableado) + spike familias. | F0 spike (rango códigos 1..999, export familias, filtro flush) | ~1-2d |
| 159A-3 | **Modo demo BDP centralizado en Configuración** — quitar `BdpDemoToggle` de las toolbars (Stock/Compras/Explorador/Historial: el badge ámbar + botones rompen el layout y sobrecargan), switch único en pestaña BDP de Configuración, demo OFF por defecto (hoy `useBdpDemoMode` arranca ON en dev) + fix wrap toolbar a la derecha. | Ninguno | Hecho 2026-09-15 (código + type-check limpio + verificado en navegador :5182; OK visual usuario pendiente) |
| 159A-4 | **Importar snapshot de stock desde BDP** — CERRADO 2026-09-15: spike contra BDP real demuestra NO VIABLE (todo artículo → "NO ES DEL TIPO WEB", almacén 1 inexistente, `ExportArticles` vacío; sin tienda web no hay stock BDP). Fuente de stock = 100 % local. Cierre: subtítulo honesto en Stock + paginación 50/pág en Inventario (verificada). Subplan: `Agente/planes/completados/plan-import-stock-bdp-2026-09-15.md`. | — | Hecho |
| 169A-1 | **Auditar ciclo de compras locales en `bdp_audit_log`** — hallazgo P9.1 (2026-09-16): crear/pasar-a-borrador/conciliar albarán (serie L) y crear gasto por conciliación NO escriben auditoría. Cierre 2026-09-16: `auditar_ciclo_local` + tx en crear/borrador/conciliar (fail-closed), idempotencia `albaran-local-crear/borrador/conciliar-{id}` + `gasto-local-albaran-{id}`, 4 etiquetas en Historial; `cargo test bdp_purchase_notes_lifecycle` 21/21 + ciclo vivo en :3100 (L-4 borrador→conciliado, audit 6 filas nuevas) + Historial :5182 47 registros con etiquetas; flags restaurados False; commit `e48c8dc`. | Ninguno (local) | Hecho |

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
