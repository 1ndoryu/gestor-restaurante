# Plan 049A-1 — Seguridad de escrituras BDP: auditoría anti-desastre + simulación antes de escribir

> **Plan aislado** para la etapa S3 del plan padre
> `Agente/planes/plan-revision-integral-bdp-2026-09-03.md` (039A-1, bloque Q2 §7 / fase F5).
> **Preparado:** 2026-09-04 · **Rama:** `glory-rs-rest` · **Estado:** creación registrada en
> `roadmap.md` (bloque activo) y `Agente/completados/tareas-2026-09-04.md`.
> **Contexto del cliente:** la **suscripción WebLink de pago NO está activa** (confirmado
> 2026-09-04; bloqueo externo ya documentado en roadmap pendientes 1c). Por tanto Q2.5/Q2.6/Q2.11
> (pago/factura/cancel) se esperan como `pendiente_suscripcion` — se simulan, se documentan y NO
> se ejecutan reales hasta que el cliente/proveedor la active.

## 1. Objetivo y estructura

El usuario pidió: *"primero vamos a trabajar en revisar que sean seguras, que no vayan a cometer
fallas graves en el bdp o dañar el sistema, una auditoría profunda y detallada enfocada en evitar
desastre, hacer simulaciones primero antes de escribir realmente"*.

Por tanto este plan NO empieza por escribir: empieza por una **auditoría de seguridad por
operación** (Fase 1), sigue con **simulaciones completas** (Fase 2, simulador Python +
wiremock + suite fail-closed) y SOLO al final, cuando cada operación esté auditada y simulada,
con suscripción activa cuando aplique y con **autorización explícita por operación**, se ejecutan
las escrituras reales (Fase 3). Regla dura: **ninguna escritura real sin auditoría + simulación
verde de esa operación**.

| Fase | Contenido | Regla de avance |
| --- | --- | --- |
| Fase 1 (A1–A13) | Auditoría profunda anti-desastre de las 13 operaciones Q2.1–Q2.13 | cada ítem auditado con hallazgo cerrado o clasificado |
| Fase 2 (S1–S8) | Simulaciones antes de escribir: simulador `tools/bdp-weblink-simulator/server.py` + suite wiremock Vía T + suite fail-closed | cada operación con su escenario simulado verde (o bloqueo externo documentado) |
| Fase 3 (W1–W13) | Escrituras reales contra BDP — una a una, autorización por operación | 0 escrituras hasta cerrar Fases 1–2 de esa operación |

**Regla de escrituras (heredada del padre):** cada escritura real: autorización explícita del
usuario para ESA operación → arming (`push_modalidad`) → operación → verificación de vuelta en
BDP (lectura) → registro local (cola vacía / `bdp_synced` / ledger / auditoría) → desarme →
limpieza del dato de prueba (Q0.5). Si el usuario no autoriza una operación queda `⏸` y se
continúa con la siguiente autorizada o se detiene, a elección del usuario.

## 2. No-alcance

- Escrituras reales contra el BDP antes de cerrar la Fase 1 y la Fase 2 de esa operación.
- Escrituras dependientes de suscripción (Q2.5 pago, Q2.6 factura, Q2.11 cancel) mientras la
  suscripción WebLink esté inactiva — se documentan como bloqueo externo (roadmap 1c), nunca
  como fallo del producto ni se reintentan en bucle.
- Cambios de código ajenos a la auditoría (salvo la excepción de la regla de oro: defecto real y
  pequeño de la clase H-W, con su prueba, corregido y documentado en §10b del padre).
- Deploy / producción web, migraciones destructivas, git remoto, Haddock.
- Ninguna llamada directa (curl/psql/SSH) a `100.83.196.35`: la única vía es la app
  (`BdpWeblinkClient` vía sus endpoints).

## 3. Contexto operativo (verificado 2026-09-04)

- **BDP online** vía Tailscale (lecturas Q1 ejecutadas); **suscripción de pago NO activa**.
- Parámetros reales: `bdp_base_url=http://100.83.196.35:8068`, `bdp_pos_id=31`,
  `bdp_employee_id=1`, `bdp_items_profile_id=1`, `bdp_catalog_price_type=1`,
  `bdp_almacen_default=1`; credenciales solo en config del entorno (nunca en evidencia).
- Estado seguro actual: `bdp_sync_enabled=false`, `bdp_sync_mode=read_only`, poll off,
  `push_modalidad=automatico` por defecto (D1).
- Métodos de escritura del cliente inventariados: `create_customer`, `create_order`,
  `cancel_order`, `add_order_payment`, `invoice_order`, `add_order_tip`, `call_waiter`,
  `add_points`, `create_articles_and_update_profiles`, `modify_article_and_update_profile`,
  `modify_prices_articles`, `create_department`, `create_department_and_update_profiles`,
  `create_family`, `create_subfamily`, `regularize_stock` (+ `UpdateMassiveInventory`,
  `UpdateStock`) — `src/services/bdp_weblink.rs` / `bdp_weblink_catalog.rs`.
- Clasificación de errores BDP ya implementada: "Subscripción no activada" →
  `pendiente_suscripcion` SIN reintento (`src/services/bdp_push.rs:8`, `bdp_backup.rs:925`).
- Simulador: `tools/bdp-weblink-simulator/server.py` (Python, puerto 18765, admin API con
  reset; lanzado por `tests/bdp_simulator_integration.rs`, E2E 267A-5). Suite wiremock:
  dev-dependency `wiremock 0.6` (contrato Vía T, 153 tests en F0/F2). Guards:
  `src/handlers/bdp_guard.rs::exigir_modo_bdp` (fail-closed, 8 tests en
  `tests/bdp_modo_standalone_fail_closed.rs`).

## 4. FASE 1 — Auditoría de seguridad por operación (anti-desastre)

Por cada operación Q2.1–Q2.13 se audita el recorrido completo
`handler → service → cliente → payload → respuesta → registro local`, con estas **13
dimensiones de desastre**:

| # | Dimensión | Pregunta de auditoría |
| --- | --- | --- |
| A1 | Guards fail-closed | ¿La operación exige modo `bdp` efectivo y rechaza en `standalone`/`read_only` sin red (≤10 ms)? |
| A2 | Arming | ¿Respetan `push_modalidad` (manual exige armado previo; auto pide confirmación dinámica y arma/desarma solo en la operación)? |
| A3 | Payload | ¿IDs referenciados existen en BDP (POS/empleado/perfil/almacén/tipo precio)? ¿precios ≥0, IVA en rango, códigos numéricos sin colisión? |
| A4 | Idempotencia | ¿Qué impide el envío doble (duplicado de artículo/comanda/pago)? ¿claves de dedup en la cola y en el cliente? |
| A5 | Cola | ¿Semántica de la cola `bdp_push_pendientes`: sin auto-flush, reintento SOLO manual, estados por ítem? |
| A6 | Fallo parcial | ¿Timeout a mitad de escritura (20 s, `bdp_weblink.rs:52`)? ¿BDP aceptó pero la respuesta se perdió? ¿cómo se detecta y reconcilia sin duplicar? |
| A7 | Referencias cruzadas | ¿Qué pasa si el artículo/departamento/orden referenciado no existe en BDP? ¿error honesto vs. datos inventados? |
| A8 | Aislamiento de datos | ¿Prefijos de prueba, sin colisión con datos reales, limpieza Q0.5 definida por operación? |
| A9 | Rollback/undo | ¿Qué mecanismo existe si una escritura real deja un dato de prueba (borrar artículo, anular orden, revertir stock)? |
| A10 | Timeout/throttle | ¿Timeouts acotados y throttle BDP respetado (2 concurrentes por base_url)? |
| A11 | Peor caso | ¿Cuál es el daño máximo de UNA llamada mal formada y qué la impide (validación, serde estricto, fallo alto)? |
| A12 | Suscripción | ¿La operación clasifica `pendiente_suscripcion` sin reintento ni cola infinita? |
| A13 | Auditoría/registro | ¿Ledger, `bdp_audit_log` y estados `bdp_synced` registran la operación sin secretos? |

### Operaciones a auditar (checklist)

- [ ] A-Q2.1 Alta artículo → `CreateArticlesAndUpdateProfiles` (código devuelto → mapeo; doble alta)
- [ ] A-Q2.2 Modificación artículo + precios → `ModifyArticleAndUpdateProfile`/`ModifyPricesArticles` (precio erróneo masivo)
- [ ] A-Q2.3 Departamento/familia → `CreateDepartment`/`CreateDepartmentAndupdateProfiles` (código secuencial D7)
- [ ] A-Q2.4 Comanda → `create_order` (líneas, `bdp_order_id`, polling; doble envío por timeout A6)
- [ ] A-Q2.5 Pago → `AddOrderPayment` (A12: suscripción inactiva → `pendiente_suscripcion` documentado)
- [ ] A-Q2.6 Factura → `InvoiceOrder` (A12: suscripción inactiva → documentado)
- [ ] A-Q2.7 Propina → `AddOrderTip` (D8; suma sobre total, decimales)
- [ ] A-Q2.8 Puntos → `AddPoints` (gating por módulo, `pendiente_suscripcion`)
- [ ] A-Q2.9 Stock/inventario → `UpdateStock`/`UpdateMassiveInventory` (motivos/almacén; riesgo de sobreescribir stock real → A11/A8)
- [ ] A-Q2.10 `CallWaiter` desde plano (solo modo bdp)
- [ ] A-Q2.11 `CancelOrder` push (A12: suscripción inactiva; anular la orden equivocada → A7/A9)
- [ ] A-Q2.12 Reintento manual desde Sincronización (estado por ítem, ver error, sin ráfaga)
- [ ] A-Q2.13 `push_modalidad` + arming real (A2 verificado en al menos una escritura de prueba)

**Salida de la Fase 1:** tabla de hallazgos por operación (seguro / riesgo mitigado / defecto
H-W clase con prueba). Cero escrituras reales mientras haya un defecto abierto que afecte a esa
operación.

### Rondas 1–2 completadas (2026-09-04) — inventario y hallazgos

**Ronda 1 (cruce de fuentes):** plan padre 039A-1 §7 (Q2.1–Q2.13) y paquete Q1 — sin desviaciones
(13 ops). Plan histórico de escrituras reales `plan-pruebas-escritura-bdp-real-2026-08-04.md` —
técnicas reutilizables: probe de contrato antes de escribir, runbook de mitigaciones, datos de
prueba existentes (cliente `900001`, comanda `5330`) y lecciones de la sesión del 2026-08-04.

**Ronda 2 (inventario de código, sin cambios):**

- **Cadena de guard** (`bdp_write_guard.rs`, 471 líneas): allowlist `VALID_BDP_WRITE_SCOPES` (16
  scopes) · `check_idempotency` · `try_auto_arm` (interactivo, exige `confirmation_text` = destino
  canónico) · `armar_push` (D1 `push_modalidad=automatico` / D2 `forzar_manual`) · `auto_arm_inner`
  (advisory lock, snapshot completo <24 h, expiry 5 min, cupo 1, upsert) · `ensure_no_unresolved` ·
  `authorize` (atómico: lock → check de ambigüedad → consume arming verificando config intacta →
  INSERT audit `pendiente` con idempotencia `ON CONFLICT` → cierra `bdp_sync_mode=read_only` y
  borra el arming **antes** del HTTP → devuelve `audit_id`).
- **Ejecución de cola** (`bdp_push.rs`): `flush` → por fila `armar_push → preparar_snapshot_escritura
  → authorize → dispatch (payload parseado a struct tipado) → actualizar_resultado + marcar_resultado`.
  Estados `pendiente/pendiente_suscripcion/sincronizado/error`; `REINTENTOS_MAX=5`; `clasificar_error`
  (match exacto `"Subscripción no activada"` → `pendiente_suscripcion`; `Http|Api|Throttled` →
  transitorio → audit `ambiguo` + reintento; resto → `error`). Standalone: no-op total.
- **Escrituras directas (fuera de cola):** `create_order` (ventas.rs `reintentar_sync_bdp`:
  `check_idempotency` + `try_auto_arm` + `retry_bdp_sync` con `authorize`+idempotency_key y lock
  distribuido `bdp-order:{id}`) · `add_payment`/`invoice` (bdp_sync.rs:527/636, con idempotency_key
  y snapshot pre-write obligatorio de la orden) · `create_customer` (bdp_customer_sync.rs:470–530,
  cadena completa) · **`call_waiter` (plano_sala.rs:513–537): solo modo≠standalone + allowlist del
  cliente; SIN arming/authorize/auditoría** (hallazgo 1).
- **Cliente** (`bdp_weblink.rs`): timeout 20 s, redirects off, throttle 2 concurrentes/base_url,
  allowlist `BDP_WRITE_ALLOWED_ORIGINS` re-chequeada en cada método de escritura (287–674);
  error→`Api{status,body}`, reqwest→`Http` (timeout cae aquí → `ambiguo`, correcto).
- **Tests existentes:** `tests/bdp_write_guard.rs`, `bdp_push.rs`, `bdp_push_cola.rs` + suite
  wiremock Vía T (153).

#### Tabla de hallazgos Fase 1

| ID | Op | Dimensión | Hallazgo | Clase | Evidencia |
| --- | --- | --- | --- | --- | --- |
| H-W-1 | Q2.10 | A1/A13 | `call_waiter` no pasa por arming/authorize/auditoría: no hay `bdp_audit_log`, no hay bloqueo por entidad ni snapshot. El cliente sí valida la allowlist. Riesgo: cero corrupción (notificación sin estado), pero sin trazabilidad. **CORREGIDO (2026-09-04):** `BdpBackupService::auditar_escritura_directa` + uso en `llamar_camarero` (éxito/error con mensaje, sin secretos); 2 tests sqlx nuevos en `tests/bdp_write_guard.rs`. | Defecto real (bajo) → corregido | `bdp_backup.rs`, `plano_sala.rs` |
| H-W-2 | cola (Q2.1/2.2/2.3/2.7/2.8/2.9/2.11) | A6 | Crash a mitad de HTTP → audit queda `pendiente`; el poller solo reconcilia `create_order/add_payment/invoice` (`bdp_order_poller.rs:388`). Fail-closed ✓ (authorize bloquea la reescritura de la entidad) pero no hay recuperación automática: requiere runbook manual. | Limitación (proceso) | `bdp_order_poller.rs:388` |
| H-W-3 | Q2.5 | A12 | Clasificación de suscripción por match exacto `trim()=="Subscripción no activada"`. Variación de texto/casing → cae a `error` con reintentos hasta `REINTENTOS_MAX=5` (acotado, no desastre, sin bucle infinito). **CORREGIDO (2026-09-04):** match normalizado case-insensitive con ambas grafías (subscrip-/suscrip-) y texto incrustado; 6 tests unitarios en `bdp_push.rs`. | Mejora (baja) → corregida | `bdp_push.rs:773+` |
| H-W-4 | Q2.4 | A4/A6 | Cadena completa verificada: `check_idempotency` + `authorize` `ON CONFLICT (user_id, idempotency_key)` + lock distribuido + `ensure_no_unresolved` + poller de reconciliación. | OK | `ventas.rs:266`, `bdp_sync.rs:215–340` |
| H-W-5 | Q2.1–Q2.13 | A2/A10 | Arming completo: advisory lock, snapshot <24 h exigido, cupo 1, expiry 5 min, cierre a `read_only` antes del HTTP, config verificada intacta al consumir; timeout 20 s + throttle. | OK | `bdp_write_guard.rs:120–470` |
| H-W-6 | Q2.5/Q2.6 | A13 | `preparar_snapshot_escritura` exige y captura snapshot de la orden para `add_payment`/`invoice` (obligatorio, falla sin `order_id`); resto de ops cubiertas por el snapshot completo <24 h del arming. | OK | `bdp_backup.rs:412–440` |
| H-W-7 | Q2.12 | A5 | `reintentar_uno`: standalone no-op, suscripción reintentable solo manual, `REINTENTOS_MAX` no aplica a acción manual. | OK | `bdp_push.rs:533–574` |
| H-W-8 | Q2.1 | A8 (precond.) | `BDP_WRITE_ALLOWED_ORIGINS` debe listar el BDP real antes de Fase 3; en sandbox localhost pasa sin env. Verificar en preflight de Fase 3 (config, no código). | Precondición | `bdp_weblink.rs:764–794` |

**Estado Fase 1:** H-W-1 y H-W-3 **corregidos con prueba** (169 tests verdes: 163 lib + 6
integración del guard, incluidos los 8 nuevos); H-W-2 documentado como runbook en Fase 2 (S8);
H-W-4..8 OK (evidencia de código). El bloqueo de Q2.10 queda levantado: la escritura ahora deja
`bdp_audit_log` con su resultado. Cero escrituras reales.

## 5. FASE 2 — Simulaciones primero (antes de escribir realmente)

Nada de lo que se va a escribir se prueba primero en BDP real: se prueba en el **simulador
`server.py`** (puerto 18765, reset por admin API) y en la **suite wiremock Vía T** (contrato).
El simulador sirve como sanity de contrato; nunca como evidencia de "integración real" (regla
del padre §2).

| # | Escenario | Vía | PASS |
| --- | --- | --- | --- |
| S1 | Baseline: suite wiremock Vía T completa (153) y suite fail-closed (8) verdes sobre el código actual | `cargo test` (wrapper, offline) | 0 fallos |
| S2 | Cada escritura Q2.1–Q2.13 en happy path contra el simulador + verificación local (mapa/cola/ledger/auditoría) | `tests/bdp_simulator_integration.rs` + endpoints app | estado local consistente, sin duplicados |
| S3 | Suscripción inactiva: pago/factura/cancel responden "Subscripción no activada" → `pendiente_suscripcion`, cero reintentos | simulador + `bdp_push.rs` | clasificación correcta (test existente + 1 por operación) |
| S4 | Timeout a mitad de escritura (respuesta >20 s o caída tras aceptar): ¿se detecta, se reconcilia, NO se duplica? | wiremock/simulador con delay | sin doble envío, estado honesto |
| S5 | Payload inválido (tipos, precios negativos, IDs inexistentes, IVA fuera de rango): error honesto, cero daño | wiremock | 422/409 honesto, sin filas fantasmas |
| S6 | Duplicado deliberado (mismo artículo/comanda dos veces): idempotencia | simulador | una sola entidad en BDP simulado |
| S7 | Inventario masivo borde (0 ítems, stock negativo, motivo vacío): no sobreescribe stock real | simulador | rechazo o conteo honesto |
| S8 | Cola: fila pendiente → reintento manual único → error visible → sin auto-flush | simulador + app Sincronización | estado por ítem correcto |

**Salida de la Fase 2:** por operación, escenario simulado verde + registro de la vía. Las
operaciones bloqueadas por suscripción quedan simuladas y documentadas (`⏸` externo), listas
para ejecutar real cuando el cliente active la suscripción.

## 6. FASE 3 — Escrituras reales contra el BDP (una a una, autorización por operación)

Solo operaciones con Fase 1 cerrada (sin defectos abiertos que las afecten) y Fase 2 verde. Cada
una: **autorización explícita del usuario → arming → operación → verificación de vuelta →
registro local → desarme → limpieza del dato de prueba**. Evidencia redactada (Q5.3 del padre:
nunca credenciales/token; NIF/teléfonos/emails enmascarados; contadores + muestra ≤3 items).

- [ ] W-Q2.1 … W-Q2.13 una por una (misma numeración que §4); las de suscripción quedan `⏸`
      externo con su documento de bloqueo (roadmap 1c) — no se ejecutan hasta activación.
- [ ] W-S3.1 Polling/reconciliación justo tras la operación origen (Q3.1/Q3.2 del padre).
- [ ] W-S3.2 Cierre: cero datos de prueba en BDP, estado local consistente, config restaurada
      (`bdp_sync_enabled=false`, `read_only`, poll off), evidencia por operación en §10b del padre.

## 7. Rondas de revisión (mecanismo anti-huecos, igual que el padre)

Tres pasadas sobre este plan antes de ejecutar la Fase 3:

- **Ronda 1 — Cruce contra el padre (039A-1 §7 Q2 + §8 S3) y el histórico
  `plan-pruebas-escritura-bdp-real-2026-08-04.md`** (2/4 escrituras reales hechas: cliente 900001,
  comanda 5330; lecciones de la prueba real): que ninguna operación ni mitigación del histórico
  falte aquí.
- **Ronda 2 — Cruce contra el código real** (handlers → services → cliente → tests): que cada
  dimensión A1–A13 tenga su prueba o verificación real, sin inventar mecanismos.
- **Ronda 3 — Cruce contra la evidencia de Fases 1–2** antes de la primera escritura real:
  checklist §8 completo, hallazgos clasificados, bloqueos externos documentados.

## 8. Criterios de aceptación (Definition of Done)

- [ ] Fase 1: 13/13 operaciones auditadas con hallazgos cerrados o clasificados (tabla §4).
- [ ] Fase 2: S1–S8 ejecutados; cada operación con simulación verde o bloqueo externo documentado.
- [ ] Fase 3: escrituras reales ejecutadas solo con autorización por operación; las de
      suscripción `⏸` con documento de bloqueo; cero datos de prueba residuales en BDP.
- [ ] Evidencia redactada en `Agente/completados/tareas-2026-09-04.md` (+ bloque en §10b del padre)
      y checklist del padre §7 Q2 / §8 S3 marcado.
- [ ] Commit local del bloque (sin push) cuando la fase lo indique, estilo de rondas anteriores.

## 9. Riesgos y mitigaciones

| Riesgo | Mitigación |
| --- | --- |
| Escritura duplicada por timeout/reintento | A4/A6: claves de dedup, verificación de vuelta antes de reintentar, reintento manual |
| Dato de prueba colisiona con real | A8: prefijos de prueba + limpieza Q0.5 por operación + verificación de vuelta |
| Suscripción inactiva se intenta en bucle | A12: `pendiente_suscripcion` sin reintento; bloqueo externo documentado (roadmap 1c) |
| Fallo silencioso ("verde con 0") | fallo alto en sync (lección H-Q1-03); evidencia por operación, nunca contadores vacíos sin explicación |
| Daño irreversible en BDP real | Fases 1–2 obligatorias por operación; sin lotes; arming; limpieza; auditoría |
| Disco/entorno | compilaciones vía wrapper a `C:\tmp`, gate de disco; si el presupuesto falla, podar target propio |

## 10. Estado y siguiente paso verificable

- **Estado:** Rondas 1–2 completadas; Fase 1 con hallazgos H-W-1..8. H-W-1 y H-W-3 corregidos
  con prueba (169 tests verdes, 8 nuevos); H-W-2 pendiente de runbook en Fase 2; H-W-4..8 OK.
  Cero escrituras reales; bloqueo de Q2.10 levantado.
- **Siguiente paso verificable:** **Fase 2** — baseline de la suite wiremock Vía T (153) +
  fail-closed (8) + simulaciones S1–S8 contra el simulador Python en :18765 (happy path por
  operación, suscripción inactiva, timeout a mitad de escritura, payload inválido, duplicado,
  inventario masivo borde, cola/reintento manual, runbook H-W-2).