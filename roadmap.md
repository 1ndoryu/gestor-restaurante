# Roadmap — RESTAURANTE

Sistema de restaurante con integración BDP (WebLink REST API). Backend Rust (Axum) sirve la API y la SPA React (`frontend/src/`); rama `main`. Se compila, prueba y analiza en la VPS 89 (taller) y se despliega en producción (`restaurante.wandori.us`, servicio Coolify `glory-rest`) siempre vía `coolify-manager-rs`. Stack, deploy y resumen ejecutivo BDP, íntegros en `Agente/completados/roadmap-historico-2026-10-19.md`.

## Siguiente bloque

- [ ] `10AA-3` — **Fase D sobre BDP local.**
  Plan `Agente/planes/plan-fase-d-bdp-local-2026-10-10.md` (parte 05) y cierre `Agente/planes/plan-fase-d-cierre-2026-10-20.md`. Todo se comprueba contra el TPV BDP local (v36.2, WebLink en `127.0.0.1:8071` por el túnel de la 89), nunca contra el del cliente.
  **Hecho:** L0, L1, L2.1, L2.3, L3.a, L3.c; F1 (receptor + inbox) y F2 (worker) de 199A-1; fichas D01–D04, D09–D20, D22, D23a–b, D24a–c, D24-m, D24-p, D25, D26b, D26d, D26f, D26g, D28, D28-auto, D28a, D32 con evidencia viva. Automatismo completo desde el 2026-10-22: siete bucles (cola, estados, avisos, documentos, maestros, albaranes y cadencias de clientes/plano/departamentos, estas últimas con selector en Configuración). Detalle de cada cierre en `Agente/completados/tareas-2026-10-17..22.md`; resumen para el cliente en `Agente/completados/informe-fase-d-2026-10-22.md`.
  **Pendiente, y qué lo desbloquea:**
  - Presencial en el PC del TPV: **D29** (reinicio de `TpvNet.exe` y de BDP.NET), **D08** (caja y web a la vez) y la **licencia 84** (el TPV es DEMO y responde «sin licencia»).
  - **D31**: el aviso llega; el TPV lo rechaza con `[404602]` porque el terminal no tiene IP de Servidor de Mensajes (configuración del TPV).
  - Impresora de cocina: **D03a** y **D09a**; Monitor de Cocina: el 7.º aviso (**D21**) y **D21a**.
  - **D26** y **D26c** con escritura completa en el piloto; **D30** en F4.
  - **D05–D07** y **D24d** no tienen servicio WebLink: van por D32 (procedimiento y responsable) o por una función desactivada declarada.
  - F3 de 199A-1 (reconciliación, perfiles y XML) sigue esperando al distribuidor.
  Criterio de hecho: D01–D32 con evidencia o límite formal declarado, y `Arranque gate "RESTAURANTE" --cierre` a 0 nuevos.


## Pendientes

- [ ] `199A-1` — **Fase D — Receptor Avisos de Acciones BDP→web.**
  Plan `Agente/planes/plan-fase-d-avisos-bdp-2026-09-19.md`. 7 eventos → receptor idempotente + worker `GetOrder` + reconciliación. F0, F1 (receptor + inbox) y F2 (worker) hechas; F2 cerrada el 2026-10-18 y con bucle periódico desde el 2026-10-22.
  Pendiente: F3 reconciliación + XML del distribuidor, F4 TPV v36 real + piloto de 7 días.

- [ ] `01AA-1` — **Alcance real con TPV local licenciado.**
  Plan `Agente/planes/plan-alcance-local-bdp-2026-10-01.md`. TPV 36.2 local licenciado; fija el alcance de la fase D con pruebas locales reversibles: P0 configurar BDP, P1 API (modificar/detalle/abiertas/camarero), P2 pantalla + receptor local, P3 mínimo pagos/facturas (cierra pendiente `1`), P4 tabla del borrador + F0 de 199A-1.
  Hecho: P0. Pendiente: P1 (`plan-alcance-local-bdp-2026-10-01.md:144`) y P2–P4.

- [ ] `10AA-3-F3` — **Fase D: F3 reconciliación, perfiles y XML.**
  Plan `Agente/planes/plan-fase-d-bdp-local-2026-10-10.md`. F3 espera respuesta del distribuidor. `cambiar-estado` requiere Monitor de Cocina instalado (falta el instalador); la descripción de salones no entra por SendInput; `pc.mjs capturar` no ve el plano de mesas.
  Criterio de hecho: F3 cerrada y bloqueos resueltos o declarados.

- [ ] `149A-1` — **Revisión integral BDP REINICIO: ítem por ítem.**
  Plan `Agente/planes/plan-revision-integral-bdp-2026-09-14.md`. Sustituye a 039A-1 (archivado como cerrado por reinicio). El usuario confirma visualmente ítem por ítem; cada ítem exige `✅ confirmado por usuario <fecha> <hora>`. Parte 1 standalone (P0–P14), Parte 2 lecturas reales (Q0–Q5), Parte 3 simulación del cliente (S0–S2).
  Siguiente paso: P0 (baseline del repositorio) con evidencia técnica + primera captura.

- [ ] `149A-2` — **Escrituras BDP REINICIO: auditoría + simulaciones + reales autorizadas.**
  Plan `Agente/planes/plan-seguridad-escrituras-bdp-2026-09-14.md`. Sustituye a 049A-1 (archivado). Repite Fase 1 (auditoría de las 13 operaciones), Fase 2 (simulaciones + simulador Python) y Fase 3 (escrituras reales una a una, solo con autorización explícita por operación). Subplan de catálogo: `Agente/planes/plan-catalogo-bdp-front-honesto-2026-09-15.md`.
  Hecho: Fase 3 ejecutada 2026-09-18 (Q2.7/Q2.9/Q2.11 verdes; Q2.5 bloqueado). Pendiente: S9 visual + TPV restaurante (anular 6258, depto 901, crear almacén 1).

- [ ] `169A-5` — **Escalar a 50.000 personas consultando con evidencia.**
  Plan `Agente/planes/plan-50k-2026-09-16.md`. D1 perfilado, D2 origen, D3 borde CF, D4 harness `consulta50k`, D5 medición y claim. Claim actual: ≥5.000 personas (lecturas); 50k no alcanzado. Informe: `Agente/completados/informe-50k-2026-09-17.md`.
  Pendiente: autorizaciones para publicar la cifra.

- [ ] `179A-1` — **Abaratar origen y llegar a 50k.**
  Plan `Agente/planes/plan-origen50k-2026-09-17.md`. F1–F5 (caché en app, recorte por query, dashboard en paralelo, pool/pg, re-medición). Claim provisional ≥14.000 personas (u500 mix-90 1.490 req/s VERDE). Informe: `Agente/completados/informe-50k-f5-2026-09-17.md`.
  Pendiente: reintento u1000 con rampa o IP rotada (decisión del usuario).

- [ ] `099A-1` — **Paquete restaurante: hosting ES/UE + MFA + SaaS + seguridad.**
  Plan `Agente/planes/plan-saas-servidor-seguridad-2026-09-09.md`. Prioridad declarada por el cliente: autenticación y servidor. Siguiente paso: F1 (estructura + estimación); sin migrar ni desplegar hasta entonces.
  Sub-pasos: hosting ES/UE (migrar `glory-rest` a CPD en España/UE; destino sin elegir); MFA (TOTP local vs OAuth Google); SaaS multiempresa (en espera); seguridad y capas; entornos dev/pruebas + normativo; estructura para el cliente; registro en España (aviso, sin acción).

- [ ] `1` — **4 pruebas de escritura real contra BDP (cliente, comanda, pago, factura).**
  Plan `Agente/planes/plan-pruebas-escritura-bdp-real-2026-08-04.md`. Estado 2026-08-05: 2/4 verificadas (cliente, comanda); pago (2.3) y factura (2.4) pendientes por la suscripción WebLink.
  Criterio de hecho: las 4 pruebas verificadas contra el BDP real.

- [ ] `1c` — **Verificar con cliente/proveedor WebLink la suscripción de pago.**
  Por qué `Payment/Add` responde "Subscripción no activada" si la suscripción estaba activa (¿IP_BDP:8068 sin módulo REST de pago? ¿CodigoIntegrador sin permiso? ¿otro entorno?). Documentado en plan 048A-11.

- [ ] `1b` — **Lecturas reales BDP, sin escrituras.**
  Conexión, acceso y formas de pago verificados; Catálogo y Compras ya muestran configuración guiada. El Explorador queda fuera del criterio de entrega. El cliente debe elegir la tarifa que devuelva artículos y aportar un código de plantilla de Compras existente (~30 min).

- [ ] `1d` — **Comanda 5330 en el BDP real.**
  Creada en la prueba 2.2 (Status=0 abierta, sin ticket): avisar al cliente para que la anule desde el TPV.

- [ ] `1e` — **Deploy a producción + correcciones de intuitividad.**
  Plan `Agente/planes/plan-deploy-produccion-intuitividad-2026-08-08.md` (incluye borrador de respuesta al cliente). U1–U8 implementadas y validadas (2026-08-09).
  Pendiente: gate `task:check`, commit/push, deploy vía `coolify-manager-rs` y verificación en producción; la activación de escrituras BDP queda sujeta a validación del cliente.

- [ ] `1f` — **048A-22 — Reproducibilidad de Sentinel y del gate coordinado.**
  Plan `Agente/planes/plan-correccion-auditoria-sentinel-2026-08-08.md`. P0–P3: identidad única, repin/repair transaccional, gate autónomo, worktrees listos, cleanup recuperable y rendimiento.
  Criterio de hecho: fases P0–P3 implementadas sin ocultar la deuda del gate base.

- [ ] `1g` — **049A-1/S2 Residuo: artículo web 90000003 en el BDP real.**
  Primer y único artículo web del BDP real; `ExportArticles` falla `[200109]` y no existe `DeleteArticle`. Opciones: obtener el contrato `ModifyArticle` completo y enviar payload completo, o pedir al proveedor WebLink su eliminación.

- [ ] `1h` — **049A-1/S2 Escritura real Q2.2 — ModifyArticle contra BDP (diferido).**
  Read-modify-write implementado y simulado (`47b95ed`); requiere autorización explícita y el contrato completo. Depende de `1g`.

- [ ] `3` — **CancelOrder: retirar el flag `ff_bdp_cancel_order`.**
  Verificado contra el BDP real (comanda 6338, 2026-09-18) y en local (2026-10-17); armado manual operativo. Queda decidir con el cliente la retirada del flag (~1h limpieza/contrato).

- [ ] `10AA-2` — **Enviar la 5ª tanda a Guillermo.**
  Alcance `Agente/usuario/alcance-fase-d-2026-10-02.md` y anexo B8 `Agente/planes/anexo-b8-aceptacion-2026-10-08.md` corregidos (rev. 10/10). Antes de enviar, regenerar el PDF (el de `Downloads` es de la 4ª tanda). Pendiente del permiso de la usuaria.

- [ ] `09AA-2` — **Base T2 autorizada por Guillermo.**
  Buzón, outbox, id-map, cursores, libro stock, receptor, monitorización y conciliación. Tablas hechas el 2026-10-17 (`migrations/20261017120000_bdp_base_t2.*`, `tests/bdp_base_t2.rs`). Falta el worker (F2 de 199A-1), monitorización y conciliación (F3).

- [ ] `14AA-3` — **Deuda previa del gate de RESTAURANTE (Rust, Sentinel y secretos).**
  Origen: gate completo `14AA-1` (2026-10-14, FAIL). Grupos: god-object (`ventas.rs` 828, `bdp_backup.rs` 858, `bdp_sync_venta.rs` 1154, `bdp_weblink_catalog.rs` 977), clippy (`modo_operacion.rs:169`), secreto-en-log (`remote_access_bootstrap.rs:322`), quality-timeout (tests de `remote_access_bootstrap.rs`) y ruta-post-sin-rate-limit (42 rutas POST en 19 ficheros).
  Criterio de hecho: los cinco grupos resueltos y `npm run gate:check -- 14AA-3` verde.

- [ ] `12AA-3` — **Sentinel warnings preexistentes en archivos tocados por 12AA-2.**
  `configuracion.rs` (`cambiar_bdp_sync_mode` 156 líneas, 606 efectivas), `bdp_sync_preflight.rs` (723 efectivas), `seed.rs` (731 efectivas + `sqlite-carga-N-consultas`). Dividir por dominio.

- [ ] `12AA-4` — **7× `sqlx::query()` sin macro en `src/bin/seed/operativa.rs`.**
  Pasar a `sqlx::query!` exige regenerar `.sqlx/` contra una BD migrada (BD recreada y migrada el 2026-10-12: desbloqueado).

- [ ] `12AA-5` — **Fallos preexistentes del gate de alcance completo.**
  `--full`: Sentinel marca ~45 errores fuera del cambio (`ventas.rs`, `bdp_backup.rs`, `bdp_sync_venta.rs`, `secreto-en-log`, ~40× `ruta-post-sin-rate-limit`); clippy y tests estuvieron bloqueados por la BD desfasada (BD migrada 2026-10-12). Deuda: 3 avisos `unused axum::Json` y lints de `--all-targets`; 4 tests con `DATABASE_URL`.

- [ ] `12AA-1` — **Frontend a Vite 8 + TypeScript 6.**
  Hecho: `frontend/package.json` y `frontend/tsconfig.json` migrados. Pendiente: `vite build` y comprobación visual. Decidir cómo cubrir el build en el contrato (`Arranque comandos`).

- [ ] `12AA-6` — **Contraseña demo en el historial de Git.**
  Commits anteriores a `22fa84b` contienen la contraseña demo en claro y un doc con otra de rotación; el repo es público. Decidido: no rotar ahora (2026-10-12). Pendiente de nueva decisión: rotar y/o reescribir historia (reservado).

- [ ] `12AA-7` — **Excepción de riesgo 1 mes: `braces` (GHSA-vfj7-8cjw-p6xm).**
  7 high en `frontend/` por la cadena `shadcn` (devDependency); riesgo aceptado (solo build/desarrollo). Vence 2026-11-12: renovar con motivo o retirar `shadcn`.

- [ ] `189A-4` — **`call_waiter` escribe al TPV sin pasar por ningún gate.**
  `src/handlers/plano_sala.rs:515-529` solo distingue standalone/bdp; funciona incluso en `read_only`. Decidir: gatearlo por modo o declararlo excepción documentada + aviso en UI.

- [ ] `189A-5` — **PATCH `invalida_armado_bdp` no incluye `bdp_auto_backup_before_write`.**
  `src/handlers/configuracion.rs:197-209`: desactivar el backup pre-write en `automatic` no fuerza `read_only`. Añadir el flag a la lista + test.

- [ ] `159A-2` — **Unificar botones sync en Importar/Exportar + importar departamentos/familias del BDP.**
  Plan `Agente/planes/plan-sync-importar-exportar-bdp-2026-09-15.md`. 13 botones sync reales → 2 por sección + endpoint `importar-departamentos` + spike familias. Bloqueo: F0 spike (rango códigos 1..999, export familias, filtro flush).

- [ ] `318A-3` — **Evaluar reactivación de reglas de consistencia de formularios.**
  `html-nativo-en-vez-de-componente`, `componente-artesanal` y `componente-sin-hook-glory` están deshabilitadas. Revisar su reactivación tras la unificación de formularios de PT; requiere autorización explícita para tocar `sentinel.config.json`.

## Bloqueos

- `cliente-bdp-produccion` — **Decisiones del cliente y pruebas contra BDP real antes de producción** (viene de `247A-9`, 24-jul). Compras (fases 1-3) y CancelOrder ya están cerrados; el 2026-10-20 se retiró esa parte de la tarea. Queda lo que depende de terceros: las decisiones del cliente sobre compras y cancelación, y las pruebas T1 simulador BDP local, T2 BDP real en entorno controlado, T3 auto-arming y toggles, T4 logs de ambigüedad y reconciliación, T5 runbook de rollback y restauración.
- `perf-staging` — **Decisiones de la usuaria sobre el staging de rendimiento** (viene de `169A-4`, 16-sep). El trabajo técnico está cerrado: matriz 9/9 ejecutada el 2026-09-16 (~251k peticiones), veredicto T1 (1 vCPU/1 GB) sobrado con el cuello en postgres al pico, informe en `Agente/completados/informe-perf-vps-2026-09-16.md`, y el commit de cierre `00560f8` ya está en `origin/main`. Queda lo que decide la usuaria: pasar `1ndoryu/gestor-restaurante` a privado, y el destino del staging `restaurante-perf` + DNS `perf.wandori.us` con sus ~35k filas `perf-harness` (conservar o borrar).
- `suscripción-weblink` — **Escrituras reales BDP bloqueadas por la suscripción WebLink de pago.** `Payment/Add` a comanda existente y factura a existente en `⏸`; el pago dentro del `CreateOrder` inicial sí funciona en gratuita. Verificar con cliente/proveedor (tareas `1`, `1c`).
- `impresora-cocina` — **Sin impresora de cocina** ("pendiente de real"): bloquea L3.a2 y D09a (10AA-3).
- `sentinel-0.7.23` — **Pin de Sentinel 0.7.23 pendiente de decisión de la usuaria** para integrar en `main` (rama `chore/sentinel-0.7.23`, commit `9a21d66`).
- `guillermo-saas` — **SaaS multiempresa en espera** hasta confirmar si todos los clientes usarán BDP (099A-1).
- `modifyarticle` — **`ModifyArticleAndUpdateProfile` del BDP real no acepta payload parcial** (NRE): bloquea neutralizar el residuo 90000003.
- `tarifa-bdp` — **El cliente debe elegir la tarifa que devuelva artículos y aportar un código de plantilla de Compras existente** (lecturas reales `1b`).
- `comanda-5330` — **Comanda 5330 abierta en el BDP real**: avisar al cliente para anularla desde el TPV.
- `distribuidor` — **F3 de 199A-1 y 01AA-1 esperan respuesta del distribuidor** (serie pruebas, almacén, perfiles, avisos).
- `licencia-84` — **Licencia 84 (Application) sin ejecutar en el TPV DEMO.** El 2026-10-20 se comprobó que no bloqueaba L6–L10 en local como se creía: D01 ya está implementado en `src/handlers/configuracion.rs:726-780` y lo que falta es ejecutarlo contra el TPV local, no escribirlo.
- `demo-password` — **Contraseña demo en el historial de Git** (repo público): decidido no rotar ahora; rotar y/o reescribir historia es reservado a la usuaria.
- `braces` — **Excepción de riesgo `braces`** (GHSA-vfj7-8cjw-p6xm) vence 2026-11-12.

## Planes activos

- `Agente/planes/plan-fase-d-bdp-local-2026-10-10.md` — `10AA-3` — Fase D sobre BDP local: lectura L2, escritura L3, avisos L4 y F1–F4.
- `Agente/planes/plan-fase-d-avisos-bdp-2026-09-19.md` — `199A-1` — receptor de Avisos de Acciones BDP→web (F2–F4 pendientes).
- `Agente/planes/plan-alcance-local-bdp-2026-10-01.md` — `01AA-1` — alcance real con TPV local licenciado.
- `Agente/planes/plan-revision-integral-bdp-2026-09-14.md` — `149A-1` — revisión integral BDP ítem por ítem.
- `Agente/planes/plan-seguridad-escrituras-bdp-2026-09-14.md` — `149A-2` — auditoría, simulaciones y escrituras reales BDP.
- `Agente/planes/plan-catalogo-bdp-front-honesto-2026-09-15.md` — `149A-2` — front honesto de Catálogo BDP.
- `Agente/planes/plan-pruebas-escritura-bdp-real-2026-08-04.md` — `1` — 4 pruebas de escritura real contra BDP.
- `Agente/planes/plan-deploy-produccion-intuitividad-2026-08-08.md` — `1e` — deploy a producción + correcciones de intuitividad.
- `Agente/planes/plan-correccion-auditoria-sentinel-2026-08-08.md` — `1f` — reproducibilidad de Sentinel y del gate coordinado.
- `Agente/planes/plan-sync-importar-exportar-bdp-2026-09-15.md` — `159A-2` — unificar botones sync + importar departamentos/familias.
- `Agente/planes/plan-deploy-desde-cero-2026-09-18.md` — `11AA-2` — checklist de cierre en planes activos.
- `Agente/planes/plan-perf-vps-2026-09-16.md` — `169A-4` — staging en VPS + informe de rendimiento sin BDP real.
- `Agente/planes/plan-50k-2026-09-16.md` — `169A-5` — escalar a 50.000 personas consultando.
- `Agente/planes/plan-origen50k-2026-09-17.md` — `179A-1` — abaratar origen y llegar a 50k.
- `Agente/planes/plan-saas-servidor-seguridad-2026-09-09.md` — `099A-1` — hosting ES/UE, MFA, SaaS y seguridad.
- `Agente/planes/anexo-b8-aceptacion-2026-10-08.md` — `19AA-1` — fichas de aceptación D: 5 hitos (D01–D32) sin cerrar.
- `Agente/planes/plan-verificacion-guia-guillermo-2026-10-05.md` — `19AA-2` — verificación de la guía, 8 bloques (B1–B8) sin cerrar.
- `Agente/planes/fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-07-bitacora-parte-1.md` — parte de `plan-fase-d-bdp-local-2026-10-10.md` (`10AA-3`): bitácora con 8 casillas abiertas.
- `Agente/planes/plan-auditoria-completa-2026-03-30.md` — `19AA-3` — 23/26 hechas; quedan M1 (token SSE), M2 (rate limiting) y B1/B2 (carpetas en español).
- `Agente/planes/plan-glory-rs-template-2026-03-28.md` — `19AA-4` — Glory-RS como plantilla reutilizable (crate, auth, front, CI): sin empezar.
- `Agente/planes/plan-desbordamiento-plano-2026-03-31.md` — `19AA-5` — solución definitiva del desbordamiento del plano y validación en local.
- `Agente/planes/plan-compras-bdp-2026-07-25.md` — `19AA-6` — probar Fases 2 y 3 contra BDP real y activar sus flags (decisión del cliente).
- `Agente/planes/plan-pagos-parciales-bdp-2026-07-25.md` — `19AA-7` — tests de integración contra el simulador real de BDP.
- `Agente/planes/plan-stock-bdp-gestionable-2026-07-25.md` — `19AA-8` — elegir opción A (desglose por almacén) u B (escritura): decisión del cliente.
- `Agente/planes/plan-pendientes-bdp-2026-07-23.md` — `19AA-9` — pendientes de decisión del cliente: Compras (D2), pagos parciales (D4) y CancelOrder (D5).
- `Agente/planes/plan-visibilidad-bdp-frontend-2026-07-23.md` — `19AA-10` — pendientes de decisión: Compras, bidireccional (D4), pagos parciales (D5) y CancelOrder.
