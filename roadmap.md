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
| Validación    | validator 0.19                 |
| Auth          | jsonwebtoken 10 + argon2       |
| Frontend      | React 18 + TypeScript + Vite   |
| State         | React Query + Zustand          |
| Codegen       | Orval 8                        |
| Deploy        | coolify-manager-rs             |

# Glory Rest — Roadmap

## Notas de infraestructura

- **Sentinel (2026-08-10)**: re-pinado a la release coordinada **0.7.0** (`a804c0d`, `main` + tag `v0.7.0`); lock regenerado, `quality:lock --check` y doctor PASS, suite 232 pass. La release integra la auditoría 108A-1 (contratos CLI, init, ADR 0001, seguridad/concurrencia) sobre 0.6.4. El stage `custom` local fue retirado (commit `f13d0e16`): 15 reglas migradas al Core, 2 observe-only P1 con destino declarado en roadmap-sentinel.
- **Sentinel 0.7.23 (2026-10-11)**: pin `02d82d7` (v0.7.23) en la rama `chore/sentinel-0.7.23` (commit `9a21d66`); lock regenerado, `quality:lock --check` y doctor PASS, evidencia compile + suite 776 pass en staging limpio. **Pendiente de decisión de la usuaria:** integrar en `main`. Corrección 2026-10-11: la nota anterior (2 commits sin pushear en `main`) era obsoleta. `main` local coincide con `origin/main` (`43c3c74`) y `3747902`/`05b8a41` no son alcanzables desde ninguna rama.
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
| **Pagos completos** (AddOrderPayment)                               | Ventas → "Pagar en BDP"                              | ✅ Implementado (verificado en simulador). **Verificación real 2026-09-18**: pago-en-creación dentro del `CreateOrder` inicial funciona en gratuita (comanda 6338, 0,11 € con `Payments`, `bdp_synced=true`). El `Payment/Add` a comanda **existente** sigue pendiente de suscripción (prueba 2.3, 2026-08-05). |
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
| **Sincronización bidireccional automática** (escritura ciega simultánea) | Riesgo de bucles y conflictos; bloqueada por diseño | ❌ Rechazado (matiz 2026-09-19: el mecanismo de vuelta **Avisos de Acciones** sí existe en BDP —ver nota `Agente/documentacion/bdp/guia-bidireccional-avisos-2026-09-19.md`—; pendiente verificar en TPV v36 y decidir fase D en bloque 199A-1) |
| **CancelOrder**                                 | Cancelación de comanda 6338 contra BDP real 2026-09-18 con auditoría `exito`; **prueba local hecha el 2026-10-17** (`PRUEBA-1`, pedido 15): el push sale, el BDP responde `301000` y la comanda desaparece de `Comandas_Cabecera`. Faltó corregir dos defectos del guard de armado (`92f2afb`, `46e49eb`). Queda sin verificar el efecto en cocina (D09a, sin impresora auxiliar) | 🟢 Verificado en local |
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

### Bloque 301A-1 — Sentinel RESTAURANTE: corregir todos los hallazgos (2026-10-18)

Objetivo: que `workspace-manager` no liste hallazgos de Sentinel en RESTAURANTE (broadcast, sqlite-carga-N, funcion-larga, parametros-excesivos, limite-lineas, god-object, usestate, objeto-mutable). Un commit por bloque, con `Arranque gate "RESTAURANTE" --inicio/--cierre` a 0 nuevos.

- [x] NotifHub: broadcast → mpsc por suscripción (`src/services/notificacion.rs`).
- [ ] sqlite-carga-N: `handlers/ventas.rs`, `bin/seed.rs`, `bdp_article_map/conteos.rs`.
- [ ] funcion-larga: `importar_clientes_bdp`, `conciliar_purchase_note`, `cambiar_bdp_sync_mode`.
- [ ] parametros-excesivos: structs de parámetros (8 funciones).
- [ ] limite-lineas, god-object, frontend usestate-excesivo y objeto-mutable.

### Bloque 10AA-3-B — Diez alcances de escritura BDP no se pueden armar (resuelto el 2026-10-17)

Origen: `10AA-3-A` (resuelto el 2026-10-17) destapó que el emparejamiento alcance↔objetivo de `src/handlers/configuracion.rs` solo admite dos familias: `create_customer` → objetivo `cliente`, y `create_order | add_payment | invoice | cancel_order` → objetivo `venta`. De los **15** alcances de `VALID_BDP_WRITE_SCOPES` (`src/services/bdp_write_guard.rs`) solo **5** se podían armar por lote; los otros **10** —`create_article`, `modify_article`, `modify_prices`, `create_department`, `create_family`, `regularize_stock`, `transfer_stock`, `inventory`, `add_tip`, `add_points`— respondían **422** siempre.

**Resuelto:** el emparejamiento ya no usa el vocabulario humano («cliente»/«venta») sino el **dominio literal** que pasa el worker (`articulo`, `departamento`, `familia`, `stock`, `propina`, `cliente_puntos`), declarado en `tipo_destino_de_alcance`. Y el segundo obstáculo —el armado exigía `target_entity_id = entidad_uuid(dominio, entidad_id)`, un SHA-256 que la API no podía calcular— se resuelve aceptando el **código local** y derivando el UUID con la misma función que el worker: `entidad_uuid` se movió de `bdp_push.rs` a `bdp_write_guard.rs` para que no haya dos copias que diverjan. `target_entity_id` pasa de `Uuid` a texto en el contrato (compatible: un UUID sigue siendo una cadena válida). Cobertura: test de tabla sobre los 15 alcances × 8 tipos de destino, más dos de derivación.

Alcance deliberadamente **no** tocado: la ruta de escritura y el fail-closed (el armado sigue exigiendo entidad exacta, snapshot vigente y cupo).

### Bloque 14AA-3 — Deuda previa del gate de RESTAURANTE (Rust, Sentinel y secretos)

Origen: gate completo `14AA-1` (2026-10-14, FAIL; `.quality-reports/check/14AA-1/latest.md`). Alcance acordado por la usuaria (2026-10-14): los cinco grupos siguientes.

- **god-object** (error > 800 líneas): `src/handlers/ventas.rs` (828), `src/services/bdp_backup.rs` (858), `src/services/bdp_sync_venta.rs` (1154), `src/services/bdp_weblink_catalog.rs` (977). Partir por dominio.
- **clippy** (error que bloquea la lib): `src/services/modo_operacion.rs:169`. Nace en `331392d`; último toque `12c1e85`.
- **secreto-en-log**: `src/bin/remote_access_bootstrap.rs:322` imprime la contraseña de RustDesk con `println!`. Redactar o no emitirla.
- **quality-timeout**: los tests unitarios de `src/bin/remote_access_bootstrap.rs` superan el timeout del gate (649 s en la etapa Rust).
- **ruta-post-sin-rate-limit**: **42** rutas POST en 19 ficheros de `src/handlers/` sin límite de tasa visible (el recuento anterior, 7, era parcial). Decidir por ruta: límite (`tower_governor`) o waiver con `sentinel-disable-file`, con su motivo.

Siguiente paso: partir los cuatro god-object; corregir clippy; redactar la contraseña en log; fijar la política de límite de tasa para las 42 rutas. Verificar con `npm run gate:check -- 14AA-3`.

### Bloque 11AA-2 — Gate: checklist en planes activos (preexistente)

Hallazgo `docs-plan-no-checklist` del gate de 11AA-1 en `Agente/planes/plan-deploy-desde-cero-2026-09-18.md` y `Agente/planes/plan-fase-d-bdp-local-2026-10-10.md`. Ya existe en `main` (43c3c74), no lo causa 11AA-1. Siguiente paso: añadir a cada plan su checklist de cierre y repetir `npm run gate:check -- <id>`.

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
**D3 borde hecho 2026-09-17** (con sesión CF del usuario): proxy naranja en
`perf` (cf-ray EWR), regla `perf-compartido-cache` (GET
`/api/bdp/article-maps*|article-stock*` → elegible + TTL del origen;
rutas reales `/api/bdp/*`, no `/api/*`), MISS→HIT verificado, negativo OK
(sin token 401, con token DYNAMIC siempre + ETag estable).
Autorizaciones pendientes: publicar cifra (tramos ya ejecutados).
**D4/D5 hechos 2026-09-17:** harness `scripts/perf/consulta50k.mjs` (mix-60 +
mix-90, auto-stop p95>800 ms, `cf-cache-status` por op); u100 VERDE 180 s
(560 req/s, p95 349 ms, 0 err) + repetición 120 s VERDE; u150 VERDE 60 s
(628 req/s); u250/u500 ROJO (techo origen ~230 DYNAMIC/s, pg ~1 núcleo,
pool 8/10 — no es cuello de pool); control `--origen-directo` inválido como
bypass (CF siguió dando HIT). **Claim: ≥5.000 personas consultando
(lecturas); 50k NO alcanzado.** Informe:
`Agente/completados/informe-50k-2026-09-17.md`. Límite abierto:
`rust-test` ~350 s supera el budget 300 s del gate (solución pendiente).

### Bloque 179A-1 — Abaratar origen y llegar a 50k (plan activo 2026-09-17)

Plan: `Agente/planes/plan-origen50k-2026-09-17.md`. Origen: techo 230 DYNAMIC/s
(informe-50k-2026-09-17): cada listado = 2 queries pesadas
(`src/repositories/venta.rs:158-219`), solo el dashboard tiene caché en app.
Fases: F1 caché en app para listados default (TTL 15-30 s + invalidación),
F2 recorte por query (índice default, vía sin JOIN, COUNT estimado), F3
dashboard en paralelo (`tokio::join`), F4 pool/pg con compuerta de medición,
F5 re-medición y claim (hito 1: 2.500 req/s ≈ 25k; hito 2: 5.000 req/s = 50k).
Descartado explícito: caché CF para privados, réplica/VPS mayor, subir TTLs.
**Estado 2026-09-17:** F1 hecho (`524aaa0`, en vivo verificado por cabeceras)
pero 2× u100 por debajo del baseline — causa: harness 1 sola cuenta (los
escritores invalidan su caché) + tablas en ~17k filas vs base saneada del
baseline. F2 hecho (`fc787e2`: índices clientes/reservas + COUNT barato;
F2a EXPLAIN y F2c vía sin JOIN aparcados/descartados con motivo). F3 hecho
(`2fdee6a`: `resumen_mes` en paralelo). F4 decidido NO (pg saturada 1 núcleo,
pool sin agotar). F5 DESBLOQUEADO: manager `1.0.0` reconstruido, EXPLAIN F2a OK
(<5 ms), base saneada, deploy F2/F3 en vivo, índices F2 aplicados.
Tramos 2026-09-17 (base limpia, mix-90): u100 629 req/s p95 259 ms;
u500 1490 req/s p95 795 ms VERDE ×2 (techo origen 231→419 DYNAMIC/s);
u1000 colapsa (529 req/s, auto-stop 50 s). Muestreo pg en u500:
app p50 47 %/max 139 %, pg p50 40 %/max 55 % → **F4 REVISADO: pool 10→20**
(`src/main.rs:22-23`, compuerta cumplida; revertir si pg >80 %).
F4 commiteado (`ca03e9c`) + desplegado (app reiniciada 13:50 UTC, pool 20 en
vivo). **Freno externo 2026-09-17:** tras el primer u1000 los tramos se
degradan en cascada (u1000 pool-10 529 req/s → u1000 pool-20 13 req/s →
u500 47 req/s) con app/pg IDLE y sondas secuenciales OK (sin
`cf-mitigated`) = throttle por IP concurrente aguas arriba (CF), no
regresión. Protocolo: pausa de inundaciones + re-sondeo en escalera
u100→u250→u500; rampa hacia u1000, no 1000 VU de golpe.
**Cierre F5 2026-09-17:** el throttle persiste/agravado con cada sonda
(u100 92 req/s) pero el egress local está sano (100 paralelos a google
p95 628 ms, 0 fallos) = throttle CF por IP, no código ni PC.
**Claim provisional: ≥14.000 personas (u500 mix-90 1.490 req/s VERDE,
techo origen 419 DYNAMIC/s, +59 % vs baseline).** Pool 20 sin probar
(desplegado en ventana throttled). Informe:
`Agente/completados/informe-50k-f5-2026-09-17.md`. Reintento u1000 con
rampa o IP rotada a decisión del usuario.

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
operación**; pago-a-existente/factura-a-existente `⏸` hasta activar la suscripción WebLink —
**matiz 2026-09-16:** ver bloque 169A-2, el pago dentro del `CreateOrder` inicial puede funcionar en gratuita y se
prueba de noche; el `⏸` sigue valiendo para `Payment/Add` a comanda existente.
**Matiz 2026-09-18:** `CancelOrder` verificado funcionando contra el BDP real
—comanda 6338 cancelada vía API, auditoría `exito`—; el `⏸` de cancelación queda levantado; prueba local pendiente desde 10/10, l.118).
**Historia que no se borra:** bajo 049A-1 sí se ejecutó **W-Q2.1 real** (alta de `90000003` →
INCIDENTE `200109`) y su fix local `sync_catalog`; el residuo `90000003` sigue en el BDP real
(pendiente 1g) y `ModifyArticleAndUpdateProfile` sigue sin funcionar contra el BDP real con payload
mínimo.
**Siguiente paso:** Fase 3 ejecutada 2026-09-18 (Q2.7/Q2.9/Q2.11 VERDES, Q2.5
bloqueo vigente, Q2.6 no intentada; detalle en `Agente/completados/tareas-2026-09-18.md`).
Queda: S9 visual + TPV restaurante (anular 6258, depto 901, crear almacén 1).
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

**Siguiente paso:** 169A-2 P2 HECHO 2026-09-18 (ver bloque; incluye verificación
empírica de `CancelOrder`). 169A-3 HECHO 2026-09-16.

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

En cristiano: la comanda 6258 se creó sin cobrar y al intentar cobrarla después nuestro
propio sistema de seguridad lo frenó (para cobrar necesita leer la comanda y la versión
gratuita no deja leerla). Guillermo leyó el manual y vio que la gratuita sí permite crear
comandas ya cobradas desde el inicio —lo bloqueado es cobrar después—. Lo probamos esa
noche (comanda 6338, ya cobrada al crearla) y funcionó. Moraleja: no se puede cobrar la
6258 (quedó pendiente para anular en TPV), pero sí crear comandas cobradas.

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
**Hecho noche 2026-09-17:** P0 PASS (36.2 Hostelería, 557 art., 10 tenders, cero
errores licencia) y P1 PASS (`?tender_id=1` → OnlyCheck con pago aceptado:
`EndType=1`, `Payments=[{T1, Amount=total}]`, `Invoice=true`, cero creación).
La gratuita **valida** pago-en-creación. Detalle en
`Agente/completados/tareas-2026-09-17.md`.
**P2 HECHO madrugada 2026-09-18** (detalle en `Agente/completados/tareas-2026-09-18.md`):
venta `PRUEBA-169A2-PAGO-20260917` 0,10+0,01 € con `Payments` e `Invoice=false`
(seam temporal `BDP_P2_SIN_FACTURA`, ya retirado) → comanda BDP **6338**
(`bdp_synced=true`, `bdp_sync_error=null`); modo `read_only`/`standalone` restaurado.
**Hallazgo: `CancelOrder` FUNCIONA en esta conexión** (el usuario tenía razón; el
"Subscripción no activada" de 2026-08-05 ya no aplica): anulación local +
`POST /api/bdp/push/:id/reintentar` → fila `venta/6338/cancelar` = `sincronizado`,
auditoría `cancel_order` = `exito` (`cfde8212-…`). Documentación viva corregida
(`bdp_sync.rs`, `ConfigBdp.tsx`, filas de este roadmap); el historial fechado se conserva.
**Supersede:** deja obsoleta la hipótesis "pago/factura ⏸ hasta suscripción" del bloque 149A-2
para pagos (la factura vía `Invoice=true` también entra en la prueba).
**Seguimiento propuesto (hecho el 2026-10-17):** el armado manual `cancel_order` vía
`PUT sync-mode` ya funciona (`92f2afb`; el validador objetivo/alcance ya lo clasifica como `venta`,
y `46e49eb` hizo que un armado manual vigente autorice el push en vez de bloquearlo).
Queda decidir con el cliente si se retira el flag `ff_bdp_cancel_order`.

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

### Bloque 247A-9 — Decisiones pendientes del cliente

| ID | Item | Pregunta al cliente | Esfuerzo estimado |
| --- | --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- || D2 | **Compras** (albaranes) | ✅ Fases 1-3 implementadas y testeadas sin llamar a BDP: lectura, borradores locales y conciliación con gastos. Protegidas por feature flags `ff_bdp_purchase_notes_*`. | ~12h || D5 | **CancelOrder** | 🟢 Verificado contra el BDP real 2026-09-18 (comanda 6338) y en local el 2026-10-17 (l.118). El armado manual de `cancel_order` ya funciona (`92f2afb`, `46e49eb`). Pendiente decidir con el cliente: retirar el flag `ff_bdp_cancel_order` | ~1h limpieza/contrato |

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

## Pendientes reales

| #   | Item                                                                                                                                                                                                                              | Bloqueo                                                                   | Esfuerzo          |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------- |
| 1   | **4 pruebas de escritura real** contra BDP (cliente, comanda, pago, factura) — plan activo: `Agente/planes/plan-pruebas-escritura-bdp-real-2026-08-04.md` (pruebas locales 2026-08-04, producción después). **Estado 2026-08-05: 2/4 verificadas** (cliente 900001 ✅, comanda 5330 ✅); **pago (2.3) y factura (2.4) PENDIENTES** — `Payment/Add` devuelve "Subscripción no activada"; el cliente afirma que la suscripción estaba activa → pendiente de verificación con cliente/proveedor WebLink. | Suscripción WebLink REST API de pago a verificar con el cliente | ~1h investigación + retomar pruebas |
| 1c  | **Verificar con cliente/proveedor WebLink** por qué `Payment/Add` responde "Subscripción no activada" si la suscripción estaba activa (¿instalación IP_BDP:8068 sin módulo REST de pago? ¿CodigoIntegrador sin permiso? ¿suscripción en otro entorno?). Documentado en plan 048A-11. | Cliente / proveedor WebLink | ~1h |
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
| 3   | **CancelOrder** — 🟢 verificado contra el BDP real 2026-09-18 (comanda 6338) y en local el 2026-10-17 (l.118); armado manual operativo. Queda limpieza/contrato: retirar flag `ff_bdp_cancel_order` | BDP verificado | ~1h |
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
| 189A-3 | **Auditoría profunda modo BDP `automatic`** — cerrada 2026-09-18 (informe en `Agente/completados/tareas-2026-09-18.md`; plan en `Agente/planes/completados/`). Activación cerrada (11 validaciones), 5 gates + push por guard, E2E `read_only→automatic→read_only` sin escrituras al TPV. 3 defectos reales → 189A-4/5/6. Límite: escritura real en `automatic` no probada (requiere escribir en TPV). | Ninguno (auditoría) | Hecho |
| 189A-4 | **`call_waiter` escribe al TPV sin pasar por ningún gate** — `src/handlers/plano_sala.rs:515-529` solo distingue standalone/bdp: ni `modo_permite_escritura`, ni backup flag, ni guard; funciona incluso en `read_only` (contradice su promesa). Decidir: gatearlo por modo o declararlo excepción documentada + aviso en UI. | Decisión de diseño | ~1-2h |
| 189A-5 | **PATCH `invalida_armado_bdp` no incluye `bdp_auto_backup_before_write`** — `src/handlers/configuracion.rs:197-209`: desactivar el backup pre-write en `automatic` no fuerza `read_only` (mitigado hoy porque los 5 gates lo comprueban por escritura, pero el modo visible miente y `authorize_automatic` no lo re-chequea). Añadir el flag a la lista + test. | Ninguno (código) | ~1h |
| 189A-6 | **Cierre de auditoría silencioso (`let _ = actualizar_resultado`)** — 5 sitios (`bdp_sync_pago.rs:344,468`, `bdp_sync_factura.rs:303`, `bdp_customer_sync.rs:660`, `bdp_sync_venta.rs:381`), todos en paths de error/ambiguo: si el UPDATE falla, la entrada queda `pendiente` y bloquea futuras ops sin rastro. Propagar o al menos loguear + reconciliar pendientes. | Ninguno (código) | ~2h |
| 199A-1 | **Fase D — Receptor Avisos de Acciones BDP→web** (nuevo 2026-09-19, plan activo `Agente/planes/plan-fase-d-avisos-bdp-2026-09-19.md`, nota `Agente/documentacion/bdp/guia-bidireccional-avisos-2026-09-19.md`). La guía bidireccional + ZIP de Avisos (04/09/2026, sin abrir hasta hoy) aportan la pieza de vuelta que faltaba: 7 eventos (crear/modificar/estado/borrar/finalizar/traspasar/línea) → receptor idempotente + worker `GetOrder` + reconciliación. F0 decisiones (sin preguntas al distribuidor, decisión 2026-09-20); **F1 receptor+inbox hecha el 2026-10-17** (`/api/bdp/avisos/{accion}`, inbox con idempotencia, secreto compartido, límites; contrato real del ZIP del distribuidor y config del TPV en `Agente/documentacion/bdp/avisos-receptor-199A-1-2026-10-17.md`; 19 pruebas verdes y sonda HTTP real contra el servidor de la 89: 404/401/202/200/422/401 como toca y el secreto censurado en el buzón); F2 worker 7 eventos; F3 reconciliación+XML distribuidor; F4 TPV v36 real + piloto 7 días. No urgente (el restaurante ya opera sin ella). Estimación ~2.200–3.400 líneas, 7–12 laborables. | Decisión cliente + prueba en TPV local (10AA-3) para F2/F4 | F0 y F1; resto tras decisión |
| 01AA-1 | **Alcance real con TPV local licenciado** (nuevo 2026-10-01, plan activo `Agente/planes/plan-alcance-local-bdp-2026-10-01.md`). TPV 36.2 local licenciado (`122417` + `751080-84`) tras fix de registro (`MAC ADDRESS PUENTEADA` = `NetworkAddress` huérfano; borrado con backup `C:\tmp\backup-NetworkAddress-0000.reg`; detalle en `Agente/documentacion/bdp/error-mac-puenteada-2026-10-01.md` §19). Fija el alcance de la fase D con pruebas locales reversibles: P0 configurar BDP (serie pruebas, almacén, perfiles, avisos), P1 API (modificar/detalle/abiertas/camarero), P2 pantalla+receptor local (payloads de avisos, reintentos, exportación/stock), P3 mínimo pagos/facturas (cierra pendiente #1: 2.3/2.4), P4 tabla del borrador + F0 de 199A-1 + mini-guía de manejo. | Distribuidor (serie pruebas, almacén, perfiles, avisos) | P0 completo; siguiente P1 (API reversible, `Agente/planes/plan-alcance-local-bdp-2026-10-01.md:144`) |
| 09AA-1 | **4ª tanda Guillermo (cerrada 2026-10-09, `Agente/completados/tareas-2026-10-09.md`)** — B8 separado con 6 campos por ficha; equivalencias A05→D23-m, A14→D21a+D22a, A19→D32, A07→D23a, A08→D23b, A17→D29, A01→D03+D03a; D18c con guarda `Status=3`. PDF 6p 318676 bytes (`C:\Users\Owner\Downloads\alcance-fase-d-2026-10-09.pdf`, SHA256 `AF91F92535BD0A0D4B689744D7044A9E3831B37C23B1182D6FE57C5C5FAAED5C`). | Ninguno | — |
| 09AA-2 | **Base T2 autorizada por Guillermo** — buzón, outbox, id-map, cursores, libro stock, receptor, monitorización, conciliación. **Tablas hechas el 2026-10-17** (L5.0 del plan 10AA-3): `migrations/20261017120000_bdp_base_t2.{up,down}.sql` crea buzón, id-map, cursor, agotados y libro de stock (5, no 6: *outbox* ya es `bdp_push_pendientes`, no se duplica) con el bloqueo por artículo y almacén de D26g, y `tests/bdp_base_t2.rs` (5 tests). El receptor ya las escribe (F1, 2026-10-17); falta el worker, que es F2 de 199A-1, monitorización y conciliación son F3. Sin marcar pruebas superadas hasta B8 completo + equivalencias corregidas (ya entregado en 09AA-1). | Ninguno (código) para las tablas; el resto va con 199A-1 | ~1d (tablas) |
| 10AA-2 | **Enviar la 5ª tanda a Guillermo (pendiente de permiso de la usuaria)** — alcance `Agente/usuario/alcance-fase-d-2026-10-02.md` y anexo B8 `Agente/planes/anexo-b8-aceptacion-2026-10-08.md` ya corregidos (rev. 10/10; registro en `Agente/completados/tareas-2026-10-10.md`). Antes de enviar, regenerar el PDF: `Downloads/alcance-fase-d-2026-10-09.pdf` es de la 4ª tanda. CancelOrder decidido: pendiente de prueba local (roadmap l.118). | Permiso de la usuaria | ~0,5h |
| 12AA-1 | **Frontend a Vite 8 + TypeScript 6** (plan `area-trabajo/Agente/planes/plan-mejora-flujo-herramientas-2026-10-10.md`, F6). Hecho: `frontend/package.json` (plugin-react 6.1.2, TS 6.0.3, Vite 8.3.4, override `@rolldown/plugin-babel` ^0.1.7 por conflicto de peers con shadcn 4.14.1) y `frontend/tsconfig.json` (sin `baseUrl`, deprecado en TS 6). Pendiente: `vite build` y comprobación visual. El gate local solo hace type-check (PASS); el contrato declara `build: -`; `--ci` exige script `test:full`, que el frontend no tiene. | Decidir cómo cubrir el build en el contrato (`Arranque comandos`) | ~0,5h |
| 12AA-3 | **Sentinel warnings preexistentes en archivos tocados por 12AA-2** (refactor aparte, fuera del alcance del commit). `src/handlers/configuracion.rs`: `cambiar_bdp_sync_mode` 156 líneas (funcion-larga, máx. 100), 606 efectivas (god-object y limite-lineas controlador 500). `src/services/bdp_sync_preflight.rs`: 723 efectivas (limite-lineas servicio 700, god-object). `src/bin/seed.rs`: 731 efectivas (god-object) y `sqlite-carga-N-consultas`. En los dos primeros solo se tocó el formato (rustfmt). Dividir por dominio. | Ninguno (código) | ~1-2d |
| 12AA-4 | **7× `sqlx::query()` sin macro en `src/bin/seed/operativa.rs`** (`sqlx-query-sin-macro`; código movido desde `seed.rs` sin cambios, el hallazgo ya existía). Pasar a `sqlx::query!` exige regenerar `.sqlx/` contra una BD migrada. | BD `glory_backend` recreada y migrada el 2026-10-12 (74/74, 47 tablas): desbloqueado | ~1h |
| 12AA-5 | **Fallos preexistentes del gate de alcance completo.** El incremental no los ve. `--full`: Sentinel marca ~45 errores fuera del cambio (`ventas.rs`, `bdp_backup.rs`, `bdp_sync_venta.rs`, `secreto-en-log` en `src/bin/remote_access_bootstrap.rs:322`, ~40× `ruta-post-sin-rate-limit` en handlers); clippy y tests estaban bloqueados por la BD desfasada (BD recreada 2026-10-12; aún sin ejecutar). | BD migrada desde 2026-10-12 (desbloqueado). Sentinel no | ~1-2d |
| 10AA-3-F3 | **Fase D (plan `Agente/planes/plan-fase-d-bdp-local-2026-10-10.md`): pendientes.** F3 reconciliación, perfiles y XML: espera respuesta del distribuidor. L6–L10 DEMO: bloqueados por suscripción y licencia 84 (confirmar). L3.a2: falta impresora de cocina. `cambiar-estado`: requiere Monitor de Cocina instalado; no hay instalador en el laptop (el monitor va en otro PC de la red local; el zip de Avisos no lo trae), así que hace falta el instalador de la usuaria. Descripción de salones: el texto no entra por SendInput. Plano de mesas: `pc.mjs capturar` no lo ve. Gate 10AA-3 (2026-10-18, `gate:check -- 10AA-3`, sin `--full`, tras cc91203, 8f15b5c, 55af31d y bc15a38): clippy de `modo_operacion.rs` corregido (adcb6d3) y rustfmt de los archivos de F1/F2 corregido; `sentinel.lock.json` pasa (`quality:doctor --lock`, match); varsense y docs PASS. Lo que falla es heredado, fuera de 10AA-3: sentinel `god-object-rs` en `bdp_push.rs` (882 líneas), `bdp_sync_venta.rs` (1150) y `bdp_weblink_catalog.rs` (1117), ya registrados en 12AA-5 y 267A-9; rust-fmt en unos 25 archivos que la fase D no tocó (p. ej. `src/main.rs`, `src/handlers/admin.rs`; comprobado con `rustfmt --check` en la 89); frontend `tsc` no instalado en el laptop (`node_modules` va en la 89, así que el gate local no puede pasar de ese stage). Informe: `.quality-reports/check/10AA-3/latest.md`. | Decidir con la usuaria el alcance de F3 y la suscripción | ~2-3d |
| 12AA-6 | **Contraseña demo en el historial de Git.** Commits anteriores a `22fa84b` contienen la contraseña demo en claro y un doc antiguo con otra contraseña de rotación. Retirarla del árbol no la borra del historial. El repo es público, así que la cuenta demo de producción es accesible con esa contraseña. Decidido: no rotar ahora (2026-10-12, respuesta de la usuaria). Pendiente de nueva decisión: rotar la cuenta demo y/o reescribir historia (force-push, reservado). | Decidido: no rotar ahora; pendiente de decisión reservada | ~0,5h |
| 12AA-7 | **Excepción de riesgo 1 mes: `braces` (GHSA-vfj7-8cjw-p6xm, DoS por patrones anidados, `<=3.0.3`).** `npm audit` da 7 high en `frontend/`, todas por la cadena `shadcn` (devDependency `^4.1.0`, CLI de componentes) → `fast-glob`/`micromatch` → `braces@3.0.3`. Sin versión parcheada en npm. Riesgo aceptado: solo build/desarrollo, sin código de servidor ni entrada externa. Decidido: excepción documentada, vence **2026-11-12** (decisión de la usuaria, 2026-10-12). Al vencer: renovar con motivo o retirar `shadcn`. `npm audit` no tiene lista de excepciones: la alerta 74 de Dependabot sigue abierta en GitHub hasta que se descarte. | Vencimiento 2026-11-12 | ~0,5h |
| 10AA-3 | **Fase D sobre BDP local (plan activo `Agente/planes/plan-fase-d-bdp-local-2026-10-10.md`)** — conmutar la app de BDP real a local (configuración en BD, no `.env`; con volcado previo), luego lectura L2, escritura controlada por lote L3 (`PRUEBA-*`), F0 empírica de los 7 avisos en TPV local (L4) y después F1–F4 de 199A-1. Sin preguntas al distribuidor (decisión 2026-09-20). Confirmar que `bdp_pos_id=1` es su terminal (aplicado, reversible; plan §9.5). L2.2: conteos con el TPV. Gap de stock: `article-stock` lee caché de 2026-09-15 (plan §9.7). Decidir las 4 filas `error` (recomendado: no tocarlas; plan §9.8) | L0, L1, L2.1, L2.3, **L3.a y L3.c hechos**. Config BD: `modo_operacion=auto`, `push_modalidad=manual`, `bdp_sync_enabled=true`, `read_only`. **F1 hecha el 2026-10-17** (receptor + inbox, plan §9.19). **Bloqueos:** L3.b (modificar venta sin ruta BDP, plan §9.9), L3.a2 y D09a (sin impresora de cocina), L2.2 (conteos con la usuaria). L4.2 ya no está bloqueado por configuración: el 2026-10-17 el lote `20261017170735` apuntó `CONFIG_WEBHOOKS.XML` al receptor real (`127.0.0.1:3105` + `/api/bdp/avisos/*`, los 7 eventos activos, TpvNet relanzado después) y la pata receptora se probó de punta a punta con un aviso sintético por el túnel hasta `bdp_avisos_inbox` (fila borrada después). Lo que falta es que el TPV **emita**: provocar una comanda `PRUEBA-2`. La herramienta pc-control ya no bloquea: `pulsar-local` (píxel local a la ventana, con el TPV tapado o no) hecho y probado el 2026-10-18. **Entorno provisional, parar al cerrar:** túnel inverso `ssh -f -N -R 127.0.0.1:8070:127.0.0.1:8070 dev89` (el WebLink solo escucha en el loopback del portátil) y `C:\tmp\fase-d-local\` (contiene secretos). **Siguiente:** L4.2 — provocar el primer aviso real en el TPV (comanda `PRUEBA-2`); después L4.3–L4.4 y F2 |

---

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
