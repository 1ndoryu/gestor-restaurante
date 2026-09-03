# Plan — Revisión integral BDP: independencia funcional + integración completa (3 rondas)

> **Fecha:** 2026-09-03
> **Rama:** `glory-rs-rest`
> **ID de bloque:** `039A-1`
> **Motivo (cita del usuario):** "crea un plan centralizado de esos 6 planes, para testear todo
> lo que hizo — una parte para la funcionalidad independiente y otra para la integración
> completa — una revisión detallada y profunda con pruebas reales contra el BDP… planifica
> revisarlo 3 veces para asegurarnos de que no dejamos nada por fuera".
> **Regla de oro:** verificación profunda, **sin parches**. Cada caso se verifica y se anota con
> evidencia (`✅/⚠️/❌/⏸` + vía). Un fallo abre un hallazgo con severidad en la tabla §10b; las
> correcciones salen en un bloque aparte después de la revisión. Única excepción: un fix trivial
> de bajo riesgo que bloquee la propia prueba (se documenta como hallazgo y se aplica).

---

## 1. Objetivo y estructura

Probar **todo lo que se construyó** entre 128A-1 y 208A-2 (independencia, escrituras completas,
pruebas de interfaz, lecturas reales pendientes, auditoría 1×1 y sus correcciones H1–H8) en una
revisión final **detallada y profunda**, organizada en tres partes:

| Parte | Qué prueba | Contra qué | Base |
| --- | --- | --- | --- |
| **Parte 1 — Funcionalidad independiente** | El producto es 100 % operacional **sin BDP** (`standalone`): catálogo, stock, inventario, anulación, compras, pagos/factura local, menús/packs, historial, permisos; controles BDP ocultos/deshabilitados; **cero tráfico a BDP** | Stack local aislado (`:3100`/`:5180`, BD de rama, seed demo), sin credenciales BDP | 128A-1, 198A-1 (efectos locales), 198A-2, 208A-1 (R1–R13), 208A-2 (C1–C7) |
| **Parte 2 — Integración completa (lecturas)** | Todo lo que Glory hace **contra el BDP real del restaurante** **sin escribir**: las 24 funciones de lectura "en uso", convivencia con datos locales, estados/degradación y seguridad | **BDP real** `100.83.196.35:8068` (Tailscale `restaurante-bdp`) cuando esté online + credenciales | 138A-2 (24 lecturas), 198A-2 (matriz de red), 208A-1 (R14) |
| **Parte 3 — Tarea final: simulación del cliente (aceptación)** | Recorrer el producto **como lo usa el cliente final** (dueño/trabajador) usando **todo lo disponible** (no solo BDP): Etapa S1 todas las funcionalidades **independientes** (día de operación completo), Etapa S2 el mismo día **con BDP conectado (solo lecturas)** y Etapa S3 las **escrituras BDP, al final del plan y solo con autorización del usuario por operación** | Navegador real + stack de la Parte 1/2, persona cliente (sin leer código) | Toda la app navegable (no solo BDP) desde la experiencia del cliente; escrituras = bloque Q2, diferido a S3 (§8) |

Y un mecanismo de **3 rondas de revisión** (§9) cuyo objetivo explícito es **no dejar nada por
fuera**: Ronda 1 contra los 6 planes fuente, Ronda 2 contra la implementación real y Ronda 3
contra la ejecución con su evidencia. Cada ronda amplía el checklist con lo que encuentre. **Al
inicio de cada ronda se relee este plan completo** (regla del proyecto) antes de marcar nada.
La **Parte 3 (§8)** añade la capa final de aceptación: el agente **simula al cliente final**
usando la aplicación completa — todo lo disponible en el producto, no solo el alcance BDP — en
tres etapas: S1 (todas las funcionalidades independientes), S2 (integradas con BDP, solo
lecturas) y S3 (escrituras BDP, al final del plan, autorizadas una a una).

**Regla de escrituras (orden de ejecución):** en TODO el plan, las escrituras al BDP real se
ejecutan **solo en la etapa final S3** (Parte 3, fase F5), después de cerrar las Partes 1–2 y
las etapas S1/S2, **una operación a la vez y cada una con autorización explícita del usuario**
y con arming. Nada que escriba en BDP se ejecuta antes.

## 2. No-alcance

- Cambios de código (salvo la excepción documentada en la regla de oro).
- **Escrituras al BDP real sin autorización explícita del usuario por operación** y sin arming:
  se ejecutan **solo en la etapa final S3 (fase F5)**. Las lecturas reales sí están autorizadas por
  este plan (bloque 138A-2) siempre que el BDP esté online con credenciales válidas.
- Deploy / producción web, migraciones destructivas, git remoto, Haddock.
- Simulador como sustituto de BDP real en la Parte 2: sirve solo como sanity de contrato antes de
  una llamada real, nunca como evidencia de "integración real".
- Sentinel/coolify/servidores: solo se usan para el gate de cierre (§10).

## 3. Los 6 planes fuente (todos archivados en `Agente/planes/completados/`)

| # | Plan fuente (bloque) | Aporte a esta revisión | Estado |
| --- | --- | --- | --- |
| 1 | `plan-independencia-bdp-2026-08-12.md` (128A-1) | F0–F10, decisiones D1–D8, mitigaciones M1–M18, Anexo A N1–N14 (lo NO completado de la integración) | Archivado 2026-08-13 |
| 2 | `plan-escrituras-bdp-completas-2026-08-19.md` (198A-1) | 15 escrituras nuevas + 5 lecturas de soporte, decisiones D1–D10, mitigaciones M1–M26, cola `bdp_push_pendientes`, guards/arming/backup/auditoría | Archivado 2026-08-19 |
| 3 | `plan-pruebas-interfaz-bdp-2026-08-19.md` (198A-2) | Matrices I0–I9 / W1–W8 / N1–N2 y entorno aislado; regresiones de UI ya cazadas | Archivado 2026-09-03 (039A-1) |
| 4 | `plan-prueba-lecturas-bdp-2026-08-18.md` (138A-2) | Inventario de las **24 funciones de lectura** "en uso" + pre-requisitos y riesgos de la llamada real | Archivado 2026-09-03 (039A-1) |
| 5 | `plan-auditoria-independencia-bdp-2026-08-27.md` (208A-1) | Checklist 1×1 R0–R14 y hallazgos H1–H8 (lo que la UX no reflejaba) | Archivado 2026-09-03 (039A-1) |
| 6 | `plan-correccion-independencia-bdp-2026-08-27.md` (208A-2) | Correcciones C1–C7 de H1–H8 con decisiones D1–D6 — **estado actual que esta revisión re-verifica** | Archivado 2026-08-27 |

## 4. Entorno

Mismo patrón que 198A-2 (stack aislado, sin chocar con otros proyectos):

| Componente | Comando | Puerto |
| --- | --- | --- |
| Backend (BD de rama) | `PORT=3100 node scripts/run-with-db.mjs run --bin glory-backend` | `127.0.0.1:3100` |
| Frontend Vite | `npm --prefix frontend run dev -- --port 5180` (proxy `/api → :3100` vía `VITE_API_TARGET`) | `127.0.0.1:5180` |
| PostgreSQL | ya corriendo (5432); BD `glory_backend_glory_rs_rest` | — |
| Seed demo | `node scripts/run-with-db.mjs run --bin seed` (usuario `demo@restaurante.com` / `demo1234`) | — |

- **Parte 1:** sin credenciales BDP en el entorno local → `modo_operacion=standalone`/
  `modo_efectivo=standalone` por defecto.
- **Parte 2:** solo cuando F0 confirme BDP online + credenciales; se apunta el cliente al BDP
  real mediante la configuración local (`BDP_BASE_URL=http://100.83.196.35:8068`,
  `BDP_POS_ID=31`, integrador `VBW2MBM5`), con snapshot de configuración previo y restauración
  posterior. Cada escritura real se arma y desarma en su propia operación.

## 5. Método de verificación (por caso)

| Vía | Descripción |
| --- | --- |
| **C** | Lectura de código (backend Rust / frontend TS): contrato, guards, wiring |
| **A** | Llamada API real contra el stack local (`:3100`, BD de rama, seed demo) |
| **U** | Recorrido en el navegador (preview `:5180`) con rol admin y trabajador |
| **T** | Tests existentes (`cargo test --lib` — suites bdp/inventario/push/permisos/integración; `tsc --noEmit`) |
| **B** | Consulta directa a la BD de rama (filas, estados, cola, auditoría) |
| **R** | Llamada real al BDP (solo Parte 2 y solo lecturas, salvo escrituras autorizadas) |

Resultado por ítem: `✅ OK` / `⚠️ Parcial (motivo)` / `❌ Fallo → hallazgo` / `⏸ Bloqueado por
dependencia externa (cuál)` — siempre con la vía y evidencia.

---

## 6. PARTE 1 — Checklist de funcionalidad independiente (`standalone`)

> Cruza R0–R13 (208A-1), 4.1–4.4 (198A-2) y C1–C7 (208A-2). El estado esperado es el **post-208A-2**;
> un ítem que regrese al estado pre-corrección es un fallo.
> Ejecutada el 2026-09-03 contra el stack aislado (`:3100`, BD `glory_backend_glory_rs_rest`
> migrada a HEAD, seed demo, frontend `:5180`). Evidencia completa: `Agente/completados/tareas-2026-09-03.md`
> (§2, y `/c/tmp/evidencia-p1.md` de trabajo).

### P0. Baseline del repositorio
- [x] P0.1 `cargo check --lib --tests` limpio (exit 0) — `SQLX_OFFLINE=true` vía wrapper (`C:\tmp\glory-target\glory_backend_main`), 2m48s, solo warnings
- [x] P0.2 `cargo test --lib` verde: **153 passed / 0 failed** (9.2 s) incl. suites `bdp_push`, `bdp_inventario`, `bdp_f8_permisos`, `bdp_service_integration`
- [x] P0.3 `npm run type-check`: **limpio para `frontend/src`** (cero errores propios); persisten 22 errores en `../glory-rs/frontend` (submódulo incluido en tsconfig pero NO importado por `src/` desde el rewrite shadcn 263A-16; preexistente en HEAD — verificado; entorno → nota en §10b)
- [x] P0.4 Estado git: solo los cambios esperados de 039A-1 (commit `a3b9a29`); árbol limpio tras la ejecución (probes externas, sin tocar el repo)

### P1. Modo operativo / conmutador / badge / degradación (R1, M1–M3)
- [x] P1.1 Sin credenciales → `standalone`, app 100 % operativa, badge "BDP: off" — API `GET /api/configuracion` (`modo_operacion:auto`, `bdp_sync_enabled:false` → efectivo `standalone`) y badge visible en Dashboard/Ventas/Configuración/Plano/Inventario con la app operativa
- [x] P1.2 `modo_operacion` es el switch maestro; `bdp_sync_enabled` solo aplica en modo bdp (M1) — `modo_operacion.rs`: `modo_efectivo_desde_config` (`bdp` exige `bdp_sync_enabled && bdp_configurado`); empírico: venta aceptada con `auto_arm=true` sin armar nada
- [x] P1.3 **C5 (H5):** guard de normalización al guardar en el PATCH — verificado en `src/handlers/configuracion.rs` (PATCH normaliza `standalone`+`sync=true`)
- [x] P1.4 Histéresis cableada — `modo_operacion.rs` (`UMBRAL_FALLOS_BDP`, `registrar_fallo_bdp`/`registrar_exito_bdp`, degradación en `modo_efectivo_sin_red`) alimentado por `bdp_order_poller.rs:286` (M2)
- [x] P1.5 Invalidación de caché del modo al guardar configuración — `modo_operacion.rs::invalidar` (M3) invocada en el PATCH
- [ ] P1.6 BDP caído → degradación con banner; operaciones locales sin error — requiere modo `bdp`/`auto` con credenciales y BDP caído; **se verifica en Parte 2 (Q3.1/Q3.2)**; en standalone el badge muestra "off" y la app opera sin errores
- [x] P1.7 Preflight ligero en auto (no dry-run completo) (F1) — en `auto` la derivación del modo es **solo configuración, sin red** (`bdp_configurado`); el dry-run completo vive aparte en `bdp_sync_preflight.rs` (OnlyCheck) y no se ejecuta en el flujo normal

### P2. Catálogo de artículos unificado (C1, R2, F2/D3, M5–M7)
- [x] P2.1 **C1 (H1):** CRUD en la página "Catálogo" (`/bdp/catalogo`, pestañas Artículos/Departamentos), no en Configuración — verificado en UI
- [x] P2.2 Configuración → pestaña BDP solo configuración (conexión, permisos, arming) + enlace "Ir a Catálogo" — verificado en UI
- [x] P2.3 CRUD local sin BDP — alta `T-P1-CAFE-01`/`90000001` con precio/IVA/familia/barcode, PATCH edición (familia→bebidas, barcode, activo=false), reactivación, todo 200 sin BDP
- [x] P2.4 Alta local asigna rango reservado — `articulo_bdp_codigo = 90000001` automático (`origen:local`), DB + API (D3)
- [x] P2.5 Origen visible en filas — columna "Origen" (local/bdp) en Catálogo y Stock (UI)
- [x] P2.6 `resolve_article` catálogo local antes del fallback (M5) — código de resolución en handlers de venta; verificado empíricamente: líneas de venta resueltas contra el map local (codigo `T-P1-*`)
- [x] P2.7 Import no pisa locales — PATCH local pone `local_dirty:true` (BD); mecanismo M6/M7 en `bdp_catalogo`/import (código + DB)
- [x] P2.8 En `standalone` "Sync catálogo" deshabilitado con motivo (UI Catálogo/Stock: botones Sync deshabilitados); **matiz de invariante**: el alta local SÍ inserta fila en la cola `bdp_push` con `estado=pendiente` (nunca se envía en standalone — flush reporta `omitidos_standalone`) — comportamiento local-first documentado en P12.4, no un fallo (ver §10b/H-N1)

### P3. Stock (C2, R3, D7)
- [x] P3.1 Ver stock local + ajuste con motivo/auditoría — UI Stock ("40 local") y API `POST /bdp/article-stock` (motivo, stock 40→50, `ajustado_local:true`, idempotente). **Defecto UI: H-P1-01** (la pantalla llama `/ajustar` → 405; el endpoint real existe y funciona)
- [x] P3.2 **C2 (H2):** botón "Nuevo artículo" en Stock (abre alta local con código/nombre/precio/IVA/familia) — UI
- [x] P3.3 **C2 (H7):** "Sync catálogo/precios" deshabilitado en `standalone` con tooltip "requiere BDP conectado" — UI Catálogo y Stock (botones disabled)
- [x] P3.4 Empty state accionable — verificado en UI Inventario/Stock (acciones locales como salida, no solo BDP)
- [x] P3.5 Origen del valor de stock visible — columna origen en Stock ("40 local")
- [x] P3.6 CSV disponible — botón de exportación CSV presente en Stock (UI)

### P4. Inventario con persistencia local (C3, R4, D6=A)
- [x] P4.1 **C3 (H3):** conteos persistidos localmente — tabla fechada `bdp_inventory_conteos`; 2 conteos visibles (3/9/2026 evidencia + 27/8/2026) con estado `aplicado` y botón "Retomar" en UI
- [x] P4.2 **C3 (H4):** guardar conteo ajusta stock con motivo "conteo" — conteo aplicado 10 vs esperado 20 → stock 50→40; DB `ajustado_local` + auditoría
- [x] P4.3 En standalone no hay envío y el mensaje es honesto — banner UI "Modo independiente… no se envía a BDP" (ya sin el toast "encolado" falso)
- [x] P4.4 Estado del envío visible cuando aplica — sección Conteos anteriores con columna Estado (aplicado) en UI
- [x] P4.5 "Retomar" un conteo previo — botón por fila en UI; cargó el conteo 27/8 en la vista (hook `useConteoInventario`)

### P5. Anulación + eliminación de ventas (R5, F4, D4/D5, M8–M11, F6)
- [x] P5.1 Anular venta local según modalidad — venta C anulada (estado `Anulada` en UI) y anulación por API sin BDP
- [x] P5.2 Confirmación dinámica + motivo + auditoría — confirmaciones dinámicas (`PAGO LOCAL {id}…`, `ANULAR…`), fila `anular_venta` con `origen_operacion=local` en Historial/audit (DB + UI)
- [x] P5.3 Venta facturada no anulable (M9) / anuladas no borrables (D5) / delete solo no sincronizadas — guards verificados en `venta.rs` (facturada→rechazo; anulada→409) y empírico (factura F-2026-0001 sin anular)
- [x] P5.4 Resumen diario excluye anuladas — `GET /api/ventas/resumen` cuadra con las sumas de BD excluyendo la anulada; liberación de mesa M11 en código de cierre de venta
- [x] P5.5 En `standalone` anulación 100 % local sin encolar — venta C: cero filas `cancel_order` en cola; solo auditoría local (DB)
- [ ] P5.6 Anulación con `bdp_order_id` en modo bdp → encola `cancel_order` y "pendiente BDP" — **requiere modo bdp con BDP real; se verifica en Parte 2 (Q2.6)**

### P6. Compras locales (C6, R6, F5, M18)
- [x] P6.1 **C6 (H8):** empty state ofrece "Nuevo albarán" como primera acción — UI Compras (botón "Nuevo albarán" + "Cargar modo demo")
- [x] P6.2 Crear albarán local serie `L-` — albarán `L-4` creado con líneas/IVA sin BDP (UI + API + DB `serie='L', numero=4`)
- [x] P6.3 Editar/eliminar (solo pendiente/borrador) y conciliar con gasto sin BDP — `L-4` → borrador → reconciliado (gasto creado, IVA por línea) vía API; guards de edición en estado conciliado verificados en código
- [x] P6.4 "Sync albaranes" deshabilitado en `standalone` — UI Compras (botón disabled); convivencia serie `L-` sin colisión (M18) — DB: `L-2` (sesión 27/8) y `L-4` coexisten bajo UNIQUE(user_id,serie,numero); comentario de migración documenta el prefijo reservado
- [x] P6.5 Flags `ff_bdp_purchase_notes_*` solo gatean en modo bdp (M12) — flags consultados en la capa de sync BDP (código), no en el CRUD local

### P7. Pagos y factura local (R7, F6, A6–A8)
- [x] P7.1 Pago completo local con `metodo_pago` — ventas A/B pagadas (Efectivo) sin BDP
- [x] P7.2 Pago parcial local + idempotencia (ledger `bdp_pagos`) — pagos parciales sobre ventas A/D/E: ledger 2+ filas, reintento NO duplicó; saldo pendiente correcto (DB + API)
- [x] P7.3 Factura local `F-{año}-{n:04}` — venta B facturada `F-2026-0001`, estado `Facturada` en UI, auditoría, guard de doble facturación en código
- [x] P7.4 En `standalone` botones BDP no se ofrecen — banner UI Ventas ("las columnas y acciones de BDP no se muestran") + código `useVentaRowActions`: `puedePagar` exige `bdpSyncEnabled && bdp_synced && bdp_order_id…` (falso en standalone); se ofrecen `puedePagoLocal`/`puedeFacturaLocal`

### P8. Menús y packs locales + Explorador (R8, F7, D2=A)
- [x] P8.1 CRUD local de menús/packs + precio recalculado — menú local creado/actualizado/borrado vía `/api/bdp/menus-locales` con precio recalculado por líneas (API + DB, sin BDP)
- [x] P8.2 Convivencia con definiciones BDP en el Explorador, origen visible — Explorador ("Menús y Packs") separa definiciones BDP (solo lectura/consulta) de los menús locales; origen visible (código + UI)

### P9. Historial / auditoría (R9)
- [x] P9.1 Operaciones locales con `origen_operacion='local'` — filas de anulación, ajuste stock, alta/edición catálogo, pagos parciales, factura local y conteos en tabla de auditoría con `origen_operacion=local` (DB + UI Historial con badge "Local")
- [x] P9.2 Snapshots de configuración sin BDP + filtros/badge — `/api/bdp/backup/snapshots` 200 en standalone; UI Historial con chips Todos/Local/BDP + búsqueda

### P10. Permisos operativos (R10, F8, M17)
- [x] P10.1 Enforcement backend real 403 — rol trabajador sin permiso recibe 403 en anulación/ajuste stock/flush; con `catalogo_edicion` puede alta de artículo (200) — verificado con token de trabajador creado ad hoc (API)
- [x] P10.2 6 permisos configurables, `catalogo_edicion` cubre el alta — selectores de 6 permisos en Configuración→BDP (UI) + enforcement por permiso en middleware/handlers (código)
- [x] P10.3 UI de permisos refleja/cambia acceso; trabajador lee configuración 200 — verificado (UI selectores + GET configuración con rol trabajador = 200)

### P11. Invariante central de red (N1–N2)
- [x] P11.1 Tras recorrer P1–P10 completo: **cero** peticiones a `100.83.196.35`/`:8068` — `netstat`: cero conexiones establecidas al host BDP (con Tailscale conectado); log backend sin hits BDP salvo el caso controlado de Plano de Sala (**H-P1-02**, solo ante clic explícito del usuario en "Sync BDP")
- [x] P11.2 Controles BDP ocultos/deshabilitados en `standalone`, uno a uno: CallWaiter D10 oculto (`onLlamarCamarero` solo con `modoEfectivoBdp`) · flush W7/badge: sin acciones en el badge (solo pill de estado) · "Sync catálogo" P3.3 y "Sync albaranes" P6.4 deshabilitados · envío inventario P4.3: no se ofrece (banner honesto) · pagar/facturar BDP P7.4: no se ofrecen · sección "Sincronización": visible con banner honesto + botones deshabilitados (C4) · botones de puntos/propina BDP en ficha de venta: no presentes (pago/factura/propina local) · **"Sync BDP" de Plano de Sala: NO cumple — visible y activo → H-P1-02**
- [x] P11.3 `POST /api/bdp/push/flush` forzado en standalone — respuesta `sincronizados:0`, `omitidos_standalone:1`, cola sin consumir (API + BD); invariante consistente con la suite `bdp_push` en verde

### P12. Integridad de datos e independencia real (R13)
- [x] P12.1 Migraciones aditivas con defaults (M15) — sin `DROP`/`RENAME` en las últimas 25 migraciones `.up` (solo los `.down` de reversión los contienen)
- [x] P12.2 Sin colisiones — serie `L-`: `L-2` y `L-4` bajo `UNIQUE(user_id,serie,numero)` (migración 20260816 documenta prefijo reservado); rango `90xxxxxxx` aplicado en el alta local `90000001` (M11)
- [x] P12.3 `venta::delete` considera Haddock (M14) — `VentaService::delete` bloquea con 409 mientras `haddock_sync_enabled` ("No se pueden eliminar ventas…")
- [x] P12.4 Cola local local-first — fila `bdp_push` `estado=pendiente` del alta/ajuste persiste en BD; flush la omite en standalone (`omitidos_standalone`) sin borrarla; se enviaría al conectar BDP (comportamiento documentado 208A-1)
- [x] P12.5 Persistencia tras recarga — tras reiniciar el backend (caída y arranque), artículos `T-P1-*`/`90000001`, albarán `L-4`, conteos, ventas A–E, pagos y factura `F-2026-0001` siguen en BD y UI (no era estado de UI)

### P13. Efectos locales de los controles de escritura 198A-1 en `standalone` (198A-2 §4.3, D7–D10)
> Añadido en la Ronda de creación 1 (cruce 198A-1/198A-2): los efectos *locales* de cada control de
> escritura tenían cobertura suelta en P5/P7; se agrupan aquí para que la Parte 1 no deje ninguno fuera.
- [x] P13.1 Alta local de departamento/familia con código secuencial local — departamento "P1-Departamento Local" creado desde Catálogo→Departamentos en UI standalone, código auto `2` (tras Bebidas=1); hint honesto "sin BDP, queda local"
- [x] P13.2 Propina (D8) configurable sumar/sustituir y persistida en `ventas.propina` — propinas 2,00/3,00 aplicadas (sumar) en ventas D/E sin envío; columna `propina` en BD (DB + UI acciones de fila)
- [x] P13.3 Puntos de cliente (D9) gating honesto — módulo de puntos sin suscripción activa no ofrece escritura (estado/oculto); verificado en código (gating por módulo) y sin filas de puntos escritas en standalone
- [x] P13.4 Arming `ff_bdp_auto_arm` fail-closed — venta con `auto_arm=true` aceptada localmente pero **cero filas** en `bdp_write_arming` y cero trazas BDP (guard `bdp_write_guard.rs` exige credenciales/modo efectivo); UI Configuración→BDP con switch de arming visible pero sin sugerir escritura sin BDP

---

## 7. PARTE 2 — Checklist de integración completa contra el BDP real (solo lecturas y verificación; **cero escrituras**)

> **Pre-condición de esta parte:** F0 confirma BDP online + credenciales válidas. Cada sub-bloque
> declara su propia dependencia; si una falta, el sub-bloque se marca `⏸` con el bloqueo concreto
> y NO se sustituye por simulador. El orden respeta la regla del cliente: lecturas primero, una a
> la vez, detenerse ante el primer resultado inesperado.

### Q0. Pre-requisitos de la Parte 2
- [ ] Q0.1 `tailscale status` → `restaurante-bdp` / `100.83.196.35` activo
- [ ] Q0.2 Credenciales del integrador `VBW2MBM5` y config local verificadas (lectura `ServiceHealth` + `Login`)
- [ ] Q0.3 Suscripción de pago WebLink activa (para Q2.4/Q2.5) — si no, se documenta bloqueo
- [ ] Q0.4 Snapshot de configuración + estado de BD previo; modo/armings revisados
- [ ] Q0.5 Checklist de datos de prueba: usar artículos/clientes/ventas de prueba y limpiar tras verificar

### Q1. Las 24 funciones de lectura "en uso" (fuente: plan 138A-2) — **solo lecturas, cero escrituras**
- [ ] Q1.1 `ServiceHealth` (health) — Q1.2 `GetVersion` — Q1.3 `Login`
- [ ] Q1.4 `GetArticle` — Q1.5 `GetPricesArticles` — Q1.6 `ExportArticles`
- [ ] Q1.7 `GetPOSArticlesList` — Q1.8 `ExportCustomers`
- [ ] Q1.9 `GetOrder` (documentar limitación API gratuita: solo `Status`)
- [ ] Q1.10 `ExportDepartment` — Q1.11 `DepartmentsExportFromProfile`
- [ ] Q1.12 `GetMenuDefinition` — Q1.13 `GetFastfoodDefinition` — Q1.14 `GetPackDefinition`
- [ ] Q1.15 `GetPOS` — Q1.16 `GetPOSes`
- [ ] Q1.17 `GetEmployee` — Q1.18 `GetEmployees` — Q1.19 `GetPOSEmployees`
- [ ] Q1.20 `GetPOSTenderList`
- [ ] Q1.21 `ExportPurchaseNotes` (Compras)
- [ ] Q1.22 `GetStock` — Q1.23 `GetListStock` (paths especulativos N6: marcar "especulativo" si el contrato real rechaza)
- [ ] Q1.24 `GetRoomTables` / `GetRoomsTables` (plano de sala)
- [ ] Q1.25 Verificación por flujo: catálogo (6), clientes (8), explorador (12–14), plano (24), compras (21), preflight (1/2/3/7/15/17/18/19/20)
- [ ] Q1.26 Limpieza: ninguna escritura ejecutada, ningún dato creado/modificado en BDP

### Q2. Escrituras reales — bloque **DIFERIDO a la etapa final S3** (Parte 3, §8; fase F5)
> Esta especificación queda aquí para trazabilidad (ids Q2.1–Q2.13 estables). **Durante la Parte 2
> no se ejecuta ninguna operación de Q2** (regla del cliente: escrituras al final del plan, una a
> la vez y con autorización explícita por operación). Ejecución y protocolo: §8 S3 y fase F5.
> **En S3**, cada una: autorización del usuario → arming → operación → verificación del efecto
> en BDP (lectura de vuelta) → registro local (cola vacía / `bdp_synced` / ledger / auditoría) →
> desarme → limpieza del dato de prueba (Q0.5).
> ⚠️ Escrituras de pago/factura requieren suscripción activa + autorización del usuario por
> operación; si BDP responde "Subscripción no activada" se documenta como bloqueo externo (no
> como fallo del producto).
- [ ] Q2.1 Alta artículo → `CreateArticlesAndUpdateProfiles` (código devuelto se guarda en el mapeo)
- [ ] Q2.2 Modificación artículo → `ModifyArticleAndUpdateProfile`; precios → `ModifyPricesArticles`
- [ ] Q2.3 Departamento/familia → `CreateDepartment` / `CreateDepartmentAndupdateProfiles` (código secuencial D7)
- [ ] Q2.4 Comanda: crear venta con líneas → enviar a BDP → verificar `bdp_order_id` y la orden en BDP (`GetOrder`/polling)
- [ ] Q2.5 Pago completo → `AddOrderPayment` (verificar ledger local + estado en BDP) — *depende de suscripción*
- [ ] Q2.6 Factura → `InvoiceOrder` — *depende de suscripción*
- [ ] Q2.7 Propina → `AddOrderTip` (D8)
- [ ] Q2.8 Puntos de cliente → `AddPoints` (D9; gating por módulo, `pendiente_suscripcion` si no hay módulo)
- [ ] Q2.9 Ajuste stock → `UpdateStock`; inventario → `UpdateMassiveInventory` (D6, motivos/almacén configurables)
- [ ] Q2.10 `CallWaiter` desde el plano (D10, visible solo en modo bdp)
- [ ] Q2.11 `CancelOrder` como push (F6): anular venta con `bdp_order_id` en modo bdp → encola `venta/cancelar` → reintento **solo manual** (D2); verificar estado en BDP — *depende de suscripción*
- [ ] Q2.12 Reintento manual desde la sección "Sincronización" (C4/H6: filas, estado por ítem, ver error) — verificar con un reintento real o documentar
- [ ] Q2.13 `push_modalidad` + arming real (D1/198A-1): modalidad `manual` exige armado previo en Configuración y `automatico`/`ff_bdp_auto_arm` pide confirmación dinámica y arma/desarma solo en la operación — verificado contra BDP real en al menos una escritura de prueba

### Q3. Estados, polling y reconciliación
> Los ítems que dependen de una escritura real se verifican en S3, justo después de la operación
> que los origina (Q3.1 tras Q2.4; Q3.2 si aplica).
- [ ] Q3.1 Polling de estados refleja la orden creada por Q2.4 (verificado en S3, fase F5): `bdp_order_status` cambia, badge correcto
- [ ] Q3.2 Reconciliación de órdenes ambiguas (worker `reconcile_ambiguous_orders`) con evidencia real o documentación de la limitación
- [ ] Q3.3 Degradación: cortar el BDP a mitad → histéresis (N=3, M2) → banner; operaciones locales intactas y **escrituras bloqueadas durante la degradación** con mensaje claro (el intento se rechaza localmente y no llega al BDP)
- [ ] Q3.4 Cola de push visible en UI solo en modo bdp (C4) con filas pendientes/sincronizadas/error

### Q4. Convivencia e integridad con datos reales
- [ ] Q4.1 Import de catálogo convive con artículos locales (origen visible, sin pisar ediciones locales)
- [ ] Q4.2 Stock BDP (`CurrentStock`) no pisa `stock_local` (N6) y el origen se distingue
- [ ] Q4.3 Albaranes importados (serie BDP) conviven con `L-` locales (M18)
- [ ] Q4.4 Clientes importados y plano de sala sincronizados sin duplicar
- [ ] Q4.5 Los datos de prueba creados en S3/Q2 se limpian/anulan al cierre (Q0.5) o quedan anotados para el TPV

### Q5. Seguridad y operación
- [ ] Q5.1 Timeout de fase HTTP (45 s, R5) verificado en una llamada lenta o documentado
- [ ] Q5.2 Allowlist/canonical target cumplen (S16-H3/H4) en el tráfico real
- [ ] Q5.3 Evidencia sin secretos ni datos personales completos (redacción obligatoria)
- [ ] Q5.4 Sin `bdp_order_id` duplicados ni doble facturación en las escrituras reales (índices S7-H1/H3)
- [ ] Q5.5 Throttling / límites de recurso del integrador (`bdp_throttle`, M-límites 198A-1) respetados en el tráfico real (sin ráfagas que rechace WebLink)

---

## 8. PARTE 3 — Tarea final: simulación del cliente (aceptación completa)

> El agente ejecuta esta parte **actuando como el cliente final del restaurante** (dueño/trabajador),
> no como revisor de ítems: se **usa la aplicación** como en un día real de operación y se recorre
> **todo lo disponible en el producto** — cada entrada del menú, cada página, cada modal accesible —
> no solo lo relacionado con BDP. Todo lo que el cliente notaría (roto, confuso, lento, mensaje
> raro, dato que no persiste) se reporta como hallazgo en la tabla §10b, aunque quede fuera del
> alcance BDP.
>
> Tres etapas. **S1 y S2 no escriben en BDP.** **S3 contiene todas las escrituras BDP del plan
> (bloque Q2, §7, diferido) y es la última etapa de la ejecución** (fase F5): ninguna operación de
> S3 empieza sin la autorización explícita del usuario para esa operación concreta.

### S0. Persona, guion e inventario del producto
- [ ] S0.1 Persona: dueño (admin) y trabajador (camarero); qué haría cada uno en un día real
- [ ] S0.2 Inventario de **TODA la app navegable**: menú/nav real → cada página → cada modal/diálogo
      accesible desde ella (incluye las pantallas no-BDP: ventas/POS, mesas, resumen diario,
      configuración general, usuarios/permisos, etc.)
- [ ] S0.3 Guion del día de operación armado sobre ese inventario (escenas de S1.2)
- [ ] S0.4 Datos de la simulación: usar el seed demo y datos identificables como de prueba; anotar
      lo que persista para no contaminar el estado real

### S1. Día de operación 100 % independiente (sin BDP) — todas las funcionalidades
- [ ] S1.1 Login como admin y como trabajador; permisos y navegación general
- [ ] S1.2 Escenas del día: (a) apertura y plano de sala / mesas; (b) venta en mesa y para llevar
      (líneas, cantidades, modificaciones); (c) pago (efectivo/tarjeta), propina local y factura
      local; (d) catálogo: alta/edición/desactivación de artículos, departamentos/familias;
      (e) stock: ajuste con motivo y export CSV; (f) inventario: conteo, guardar y retomar;
      (g) compras: albarán `L-` y conciliación con gasto; (h) menús/packs; (i) historial,
      auditoría y resumen diario; (j) configuración y permisos
- [ ] S1.3 Barrido de completitud: recorrer las páginas y controles que el guion no tocó (empty
      states, búsquedas, filtros, exportaciones, recarga persistente) y anotar hallazgo si algo
      no funciona — la meta es "recorrí la app completa", no solo las escenas
- [ ] S1.4 Cierre como cliente: recargar y verificar que lo operado persiste; resumen del día
      coherente
- [ ] S1.5 Cero tráfico a BDP en toda la simulación (re-verificación de P11.1 con la app completa)

### S2. Mismo día con BDP conectado — integración real, solo lecturas
- [ ] S2.1 Encender modo bdp con las credenciales reales (Q0) y ver el estado/badge como el cliente
- [ ] S2.2 Repetir el día con datos BDP visibles en los flujos: catálogo integrado (origen visible),
      clientes, plano de sala real, albaranes importados, stock BDP consultado, historial mixto
      — todo por lectura (Q1 cubre cada llamada; aquí se ve el flujo completo)
- [ ] S2.3 Los flujos que terminarían escribiendo se recorren **hasta el punto anterior a la
      escritura** y se completan en S3 con el mismo escenario (nada escribe antes de la etapa final)
- [ ] S2.4 Estados y degradación visibles al cliente (badge, banners, mensajes honestos) sin escribir
- [ ] S2.5 Evidencia redactada (sin secretos ni datos personales completos)

### S3. Escrituras BDP — etapa final, una a una y con autorización (bloque Q2, §7)
> **Regla dura:** S3 no empieza hasta cerrar S1 y S2. Dentro de S3 no hay lotes ni prisa: **una
> operación a la vez**, cada una precedida de la autorización explícita del usuario para esa
> operación, ejecutada con arming, verificada de vuelta en BDP y anotada en la tabla §10b antes
> de pasar a la siguiente. Si el usuario no autoriza una operación queda `⏸` y la etapa continúa
> con la siguiente autorizada o se detiene, a elección del usuario.

- [ ] S3.1 Ejecutar el bloque Q2 (§7) completo, en orden y con el protocolo de cada ítem:
      autorización → arming → operación → verificación de vuelta → registro local (cola vacía /
      `bdp_synced` / ledger / auditoría) → desarme → limpieza del dato de prueba (Q0.5)
- [ ] S3.2 Verificaciones dependientes de escritura: Q3.1 (polling de la orden creada) y Q3.2
      (reconciliación) justo tras su operación origen
- [ ] S3.3 Cierre: sin datos de prueba en BDP, estado local consistente, evidencia por operación

---

## 9. Las 3 rondas de revisión (mecanismo anti-huecos)

Cada ronda **relee este plan completo** y produce una tabla de cruce. El checklist se amplía con
lo que la ronda descubra; un ítem descubierto se añade a la sección correspondiente con su ID.

| Ronda | Cuándo | Fuente de cruce | Pregunta | Entregable | Criterio de salida |
| --- | --- | --- | --- | --- | --- |
| **Ronda 1 — Cobertura de fuentes** | Antes de ejecutar (F0) | Los 6 planes §3 (F0–F10, D1–D10, M1–M26, R1–R14, I/W/N, 24 lecturas, C1–C7, N1–N14) | ¿Cada ítem de los 6 planes tiene un caso P*/Q* (o exclusión justificada)? | Tabla cruce 6 planes → checklist | 100 % de ítems mapeados o exclusión documentada |
| **Ronda 2 — Cobertura de código** | Tras F1 (Parte 1 ejecutada) | Implementación real: `src/handlers`, `src/services`, `src/repositories`, migraciones, `frontend/src/componentes|paginas` por dominio | ¿Cada funcionalidad implementada tiene caso de prueba? ¿Hay código sin cubrir por el checklist? | Tabla cruce código → checklist (por dominio) | 100 % de funcionalidades mapeadas o exclusión documentada |
| **Ronda 3 — Cobertura de ejecución** | Al final (F6) | La ejecución real (resultados P*/Q*/S*) | ¿Cada ✅ tiene evidencia, cada ⚠️ su motivo, cada ⏸ su bloqueo, cada ❌ su hallazgo? ¿Algún caso sin ejecutar? | Auditoría caso a caso del checklist completo | Cero ✅ sin evidencia; cero ⚠️/⏸ sin motivo; hallazgos con severidad |

Reglas de las rondas:
- Un caso que una ronda invalide (por ejemplo, un flujo que ya no existe) se **reformula o se
  elimina con justificación**, nunca se deja huérfano.
- Cada ronda se cierra con su tabla de cruce adjunta al plan (o en el reporte de la completada),
  de modo que el "no dejamos nada por fuera" sea comprobable, no una afirmación.

## 10. Fases de ejecución

1. **F0 — Entorno + Ronda 1:** levantar stack aislado (§4), baseline P0, cruce contra los 6
   planes (§9) y ampliación del checklist.
2. **F1 — Parte 1:** ejecutar P1–P13 en `standalone` con evidencia por caso (vías U/A/B/T).
3. **F2 — Ronda 2:** cruce contra la implementación real; añadir casos faltantes y ejecutarlos.
4. **F3 — Parte 2 (sin escrituras):** Q0 (pre-requisitos) → Q1 (24 lecturas reales) → Q3/Q4/Q5
   verificables sin escribir. El bloque Q2 queda **diferido** (regla del cliente). Si el BDP no
   está online, esta fase queda `⏸` con el bloqueo registrado en el roadmap (no se cierra en falso).
5. **F4 — Parte 3, etapas S0–S2:** simulación del cliente (S0 inventario de la app, S1 día 100 %
   independiente, S2 mismo día con BDP solo lecturas) — app completa, no solo BDP.
6. **F5 — Parte 3, etapa S3 (escrituras):** bloque Q2 (§7) diferido, ejecutado **al final del
   plan**: una operación a la vez, cada una con autorización explícita del usuario por operación,
   con arming, verificación de vuelta y limpieza (Q0.5); anotación en la tabla §10b.
7. **F6 — Ronda 3:** auditoría final caso a caso (§9) + tabla de hallazgos final (§10b) + cierre
   documental (roadmap, completadas, este plan → `completados/`).
8. **F7 — Gate:** `cargo check`, `cargo test --lib`, `tsc --noEmit` y gate canónico del proyecto
   con resultado registrado en la completada.

## 10b. Tabla de hallazgos (se rellena durante la ejecución)

| ID | Área | Hallazgo (evidencia) | Severidad | Corrección propuesta | Estado |
| --- | --- | --- | --- | --- | --- |
| H-P1-01 | Stock (P3.1, UI) | "Ajustar" de Stock llama `POST /api/bdp/article-stock/ajustar` → **405**; el backend solo monta `POST /api/bdp/article-stock` (sin `/ajustar`). Evidencia: toast "Error al ajustar el stock" en UI + network log 405 + stock sin cambio (BD). El endpoint real funciona por API (stock 40→50, `ajustado_local:true`). Localizado: `frontend/src/api/bdp.ts:154` vs `src/handlers/bdp_article_map.rs` (mount sin `/ajustar`). | Alta (funcionalidad de UI rota) | Alinear ruta del frontend con el mount real (o montar también `/ajustar` con test) | Abierto |
| H-P1-02 | Plano de Sala (P11.1/P11.2, N1) | Botón "Sync BDP" de Plano de Sala **activo en `standalone`** (no deshabilitado/oculto como exige P11.2). Al hacer clic en modo standalone con credenciales presentes en el sandbox: `POST /api/bdp/sync-tables` → **intento real de `Auth/Login` contra `100.83.196.35:8068`** → error de transporte (Tailscale caído en ese momento) → 500 tras ~20 s. El handler `sync_tables` solo exige credenciales, **nunca consulta `modo_efectivo`/`bdp_sync_enabled`**. NO se re-disparó con Tailscale arriba (habría autenticado contra el BDP real). CallWaiter D10 en el mismo componente sí está gateado por `modoEfectivoBdp`. | Alta (viola el invariante de red N1 bajo clic explícito) | Gatear el botón y el handler por modo efectivo (ocultar/deshabilitar en standalone, como C4/P3.3/P6.4); test de regresión sin red | Abierto |
| H-N1 | Cola local-first (P2.8/P12.4) | Matiz de invariante, no fallo: el alta local SÍ inserta fila `bdp_push` `estado=pendiente` (comportamiento local-first de 208A-1); en standalone nunca se envía — `flush` reporta `omitidos_standalone:1`, `sincronizados:0`, la fila persiste y se enviaría al conectar BDP. | Info (documentado) | Ninguna; mantener documentado en P12.4 para que no se lea como "envío en standalone" | Cerrado (comportamiento esperado) |

## 11. Criterios de aceptación (Definition of Done)

- **Ronda 1 y Ronda 2 cerradas** con cruce 100 % mapeado (o exclusión justificada documentada).
- **Parte 1:** P1–P13 ejecutados con evidencia; **cero tráfico a BDP** (P11.1) en `standalone`.
- **Parte 2:** las 24 lecturas (Q1) ejecutadas contra BDP real o bloqueadas con motivo concreto;
  Q3–Q5 verificadas sin escritura; Q2 no ejecutado en esta parte (diferido).
- **Parte 3:** S1/S2 completadas como simulación del cliente con la **app completa**; S3 (bloque
  Q2) ejecutada **al final**, una operación a la vez y cada una con autorización del usuario
  documentada; las no autorizadas quedan `⏸` (no éxitos falsos).
- **Ronda 3:** sin ✅ sin evidencia; todos los hallazgos tienen severidad y dueño.
- **Gate F7 en verde** y evidencia registrada en `Agente/completados/`; roadmap actualizado en el
  mismo bloque; esta plan movida a `Agente/planes/completados/` al cierre.

## 12. Riesgos y mitigación

| Riesgo | Mitigación |
| --- | --- |
| BDP real offline / Tailscale desconectado | F0/Q0.1 confirman antes; si cae a mitad, detener y documentar (regla del cliente) |
| Suscripción de pago inactiva (Q2.5/Q2.6/Q2.11) | Documentar bloqueo externo; no marcar como fallo del producto; reintento manual (D2) |
| Escrituras reales sobre datos del restaurante | Solo datos de prueba (Q0.5); arming por operación; limpieza al cierre; autorización explícita del usuario; etapa final S3 aislada del resto (F5) |
| Puertos ocupados por otros proyectos | Stack aislado `:3100`/`:5180`; no tocar servidores ajenos |
| Cambios ajenos en el árbol durante la revisión | Verificar `git status` al inicio de cada fase; no commitear frentes ajenos |
| Casos descubiertos en Ronda 2 que alargan la sesión | Añadirlos al checklist y ejecutarlos en la misma o siguiente sesión; nunca omitirlos del reporte |
| Datos personales en respuestas reales | Redacción obligatoria en evidencia (Q5.3) |

## 13. Estado y siguiente paso verificable

- **Estado:** plan creado y revisado 3 veces al crearlo (§14 Rondas 1–3, 2026-09-03, 8 adiciones
  aplicadas) y re-revisado en una 2.ª pasada (§14 Ronda 4, 2026-09-03): escrituras BDP solo en la
  etapa final S3 (F5) con autorización por operación; Parte 3 (§8) simula al cliente con la app
  completa; 3 rondas de ejecución (§9); fases F0–F7. Los 3 planes activos fuente quedaron archivados.
- **F0 y Parte 1 EJECUTADAS (2026-09-03, stack aislado :3100/:5180, BD `glory_backend_glory_rs_rest`):**
  P0 baseline verde (`cargo check` offline 2m48s, `cargo test --lib` 153/0, type-check limpio en
  `frontend/src`, submodule glory-rs preexistente); P1–P13 ejecutados con evidencia por caso (vías
  A/U/B/T) — checklist completo marcado en §6; P11.1 invariante de red verificado con Tailscale
  CONECTADO (cero conexiones establecidas al host BDP); P12.5 persistencia tras reinicio del backend
  verificado. **Hallazgos: H-P1-01 y H-P1-02 (tabla §10b, ambos abiertos)** + matiz H-N1 cerrado.
  Diferidos con motivo explícito (no fallo): P1.6 y P5.6 → se verifican en Parte 2 (Q3.1/Q3.2, Q2.6).
- **Siguiente paso (F2 — Ronda 2):** cruce contra la implementación real (código Rust + frontend por
  dominio) para confirmar que ninguna funcionalidad implementada quedó sin caso, cerrar la Ronda 2 y
  pasar a F3 (Parte 2). La Parte 2 requiere además: confirmar con el usuario la disponibilidad del BDP
  real (online + credenciales integrador + suscripción de pago) y **autorización explícita por
  operación para cualquier escritura (S3/F5)** — mientras tanto se ejecuta solo la parte de lecturas
  (Q1) o queda `⏸` documentada.

## 14. Rondas de revisión de la CREACIÓN (2026-09-03, cumplidas al crear este plan)

> Las 3 rondas del §9 se ejecutan durante la revisión real (F0/F2/F6). Adicionalmente, y porque el
> usuario pidió "revisarlo 3 veces para asegurarnos de que no dejamos nada por fuera", este plan se
> cruzó 3 veces **al crearlo**. Resultado: 8 adiciones al checklist (P13.1–P13.4, ítems nuevos en
> P11.2, Q2.13, Q3.3, Q5.5). Una **4.ª pasada** (Ronda 4, al final de esta sección) se añadió después
> por pedido del usuario ("otra ronda").

**Ronda 1 — Cruce contra los 6 planes fuente.** Sección por sección de los 6 planes (headers
arriba) mapeada a casos P*/Q*:

| Fuente (plan) | Secciones → casos | Huecos encontrados |
| --- | --- | --- |
| 128A-1 (F0–F10, D1–D8, M1–M18, Anexo A N1–N14) | §4.1→P1 · §4.2→P2 · §4.3→P3 · §4.6→P7 · §4.7→P5 · §4.8→P6 · §4.9→P9 · §4.10→P8 · §4.11→P10 · §4.12→P1.4/P1.7/Q3.3 | N1–N14 se cruzan ítem a ítem en la Ronda 1 de F0 (mecanismo §9); M-item sueltos → Ronda 1 de F0 |
| 198A-1 (15 escrituras + 5 lecturas soporte, D1–D10, M1–M26) | §4.1→Q2.1–2.2 · §4.2→Q2.9 · §4.3→Q2.3 · §4.4→Q2.4/2.7/2.11 · §4.5→Q2.10 · §4.6→Q2.8 · §4.7→Q1/Q2.12 | `push_modalidad`/arming sin caso real → **Q2.13**; throttling M-límites → **Q5.5** |
| 198A-2 (I0–I9, W1–W8, N1–N2, entorno) | §4.1→P1 · §4.2→P2–P10 · §4.3→P5/P7 parcial + nuevos · §4.4→P11 | efectos locales de escritura dispersos → **sección P13** |
| 138A-2 (24 lecturas + flujos + riesgos) | Inventario → Q1.1–Q1.24 · flujos → Q1.25 · limpieza → Q1.26 | — |
| 208A-1 (R0–R14, H1–H8) | R0→P0 · R1→P1 · R2→P2 · R3→P3 · R4→P4 · R5→P5 · R6→P6 · R7→P7 · R8→P8 · R9→P9 · R10→P10 · R11→P11.3 · R12→P11.2/P2.1 · R13→P12 · R14→Q (Parte 2) · H1→P2.1 · H2→P3.2 · H3/H4→P4.1/P4.2 · H5→P1.3 · H6→Q3.4/Q2.12 · H7→P3.3 · H8→P6.1 | sección Sincronización (C4) y "Sync BDP" de Plano sin caso standalone → **P11.2 ampliado** |
| 208A-2 (C1–C7) | C1→P2.1–2.2 · C2→P3.2–3.4 · C3→P4.1–4.5 · C4→Q3.4/Q2.12/P11.2 · C5→P1.3 · C6→P6.1 · C7→F6/F7 | — |

**Ronda 2 — Cruce contra la implementación real (código).** Funcionalidades por archivo mapeadas a
casos: `modo_operacion.rs`→P1 · `bdp_weblink*.rs`/handlers `bdp_catalogo|article_map|customer_sync`→
P2/Q1/Q4 · `venta.rs`/`ventas.rs`→P5/P7/Q2.4–2.6 · `bdp_purchase_note*`→P6/Q4.3 · `bdp_menu_local*`→P8
· `bdp_backup`→P9.2 · `permisos.rs`→P10 · `bdp_push*`→P4.3/P11/Q3.4/Q2.12 · `bdp_sync.rs`→Q2.4–2.6/Q3.1 ·
`bdp_order_poller.rs`→Q3.1–3.2 · `bdp_write_guard.rs`→P13.4/Q2.13 · `bdp_throttle.rs`→**Q5.5** ·
`bdp_pago|bdp_punto_cliente`→P7.2/P13.3 · `haddock.rs`→P12.3 · frontend `BdpCatalogo|BdpStock|
NuevoArticuloDialog|BdpInventario|BdpCompras*|BdpExplorador|BdpHistorial|BdpSincronizacion|PlanoSala|
ListaVentas|site-header`→P*/Q* correspondientes · `BdpDemoToggle`/`bdp-mocks` = bloque 247A-11 anterior a
la cadena 128A-1→208A-2 → fuera de alcance de esta revisión (anotado). Huecos: mismos 8 ítems de la
Ronda 1 (ningún archivo de la cadena quedó sin caso).

**Ronda 3 — Consistencia interna y de ejecución.** Referencias H→C→P/Q cruzadas y coherentes (ver
tabla 208A-1/208A-2 arriba); IDs sin duplicados ni huérfanos; §5 vías (C/A/U/T/B/R) suficientes;
rangos de ítems P1–P12 actualizados a **P1–P13** en Fases y DoD; dependencias de Parte 2 declaradas
(Q0.1–Q0.5, suscripción en Q2.5/2.6/2.11); no hay caso sin método de verificación asignable.

**Ronda 4 — 2026-09-03 (2.ª pasada; pedido del usuario).** El usuario pidió "otra ronda" — que
volviera a revisar el plan — y aclaró el modelo: una tarea final donde el agente **simula ser el
cliente** usando la aplicación, con una etapa de funcionalidades independientes y otra integrada
con BDP, recorriendo **todo lo disponible** (no solo lo BDP), y dejando **todas las escrituras BDP
para la etapa final, con autorización del usuario por operación**. Hallazgos de esta ronda y cómo
se aplicaron:
1. La Parte 3 existía solo en la tabla del §1, sin sección propia ni etapas → **§8 PARTE 3** con
   S0 (persona/guion/inventario de toda la app), S1 (día 100 % independiente, todas las
   funcionalidades), S2 (mismo día con BDP, solo lecturas) y S3 (escrituras); barrido de
   completitud de la app entera (S0.2/S1.3).
2. Residuo del borrador anterior ("4 rondas / Ronda 4 de ejecución") → las rondas de ejecución
   vuelven a ser **3 (§9)**; la capa del cliente es la **Parte 3**, no una ronda.
3. El bloque Q2 (escrituras) seguía dentro de la Parte 2 con aire de ejecutarse ahí → marcado
   **DIFERIDO a S3 (fase F5)**; la Parte 2 (§7) queda explícitamente de solo lecturas; Q3.1/Q3.2
   pasan a verificarse en S3 tras su escritura origen; Q3.3/Q4.5 reescritos sin ambigüedad.
4. Referencias internas desactualizadas (F5b, "Ronda 3 en F4", §9b, §13, "Gate F5") → fases
   F0–F7 con la escritura en **F5** y el gate en **F7**; secciones renumeradas **§8–§14**; DoD,
   estado y checklist de cierre alineados con la nueva estructura.

## Checklist de cierre

- [x] Roadmap actualizado (bloques 208A-1/198A-2/138A-2 archivados → enlace a este plan; estado y
      siguiente paso en §13)
- [x] 6 planes fuente en `Agente/planes/completados/` con nota de archivado
- [x] Evidencia de la creación en `Agente/completados/tareas-2026-09-03.md` (bloque 039A-1)
- [x] Plan revisado 3 veces al crearlo (§14 Rondas 1–3, 2026-09-03) — adiciones P13, P11.2, Q2.13, Q3.3, Q5.5
- [x] Plan re-revisado en la 2.ª pasada (§14 Ronda 4, 2026-09-03): escrituras BDP → etapa final S3
      (fase F5) con autorización por operación; Parte 3 (§8) = simulación del cliente con la app
      completa; 3 rondas de ejecución (§9); secciones renumeradas §8–§14; fases F0–F7
- [ ] Releer este plan completo al inicio de cada ronda (regla §9)
