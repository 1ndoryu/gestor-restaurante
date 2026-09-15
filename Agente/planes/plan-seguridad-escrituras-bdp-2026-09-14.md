# Plan 149A-2 — Escrituras BDP (REINICIO 2026-09-14): auditoría + simulaciones + reales autorizadas, con confirmación visual

> **Fecha:** 2026-09-14 · **Rama operativa:** `main` · **ID:** `149A-2`
> **Sustituye a:** `049A-1` (`Agente/planes/completados/plan-seguridad-escrituras-bdp-2026-09-04.md`,
> cerrado por reinicio).
> **Hermano:** `149A-1` (`plan-revision-integral-bdp-2026-09-14.md`) cubre la revisión
> independiente + lecturas reales. Este plan cubre **todo lo que escribe**.
> **Motivo del reinicio (cita del usuario):** "aunque lo de las escrituras así ese plan también
> vamos a repetirlo" — los refactors y cambios de CSS invalidaron la evidencia previa.

## 1. Regla de oro (no cambia)

**Ninguna escritura real sin auditoría cerrada + simulación verde de esa operación, y sin
autorización explícita del usuario para ESA operación.** Orden: Fase 1 auditoría → Fase 2
simulaciones (durante: una por una, con confirmación visual) → Fase 3 escrituras reales una a una.

Contexto vigente: la **suscripción WebLink de pago no está activa**, así que pago/factura/cancel
(Q2.5/Q2.6/Q2.11) se simulan y quedan como `pendiente_suscripcion` hasta que el cliente la active.

> **Actualización (sonda de solo lectura, 2026-09-14):** la suscripción de **salones/mesas ya está
> activa** (el endpoint dejó de responder "Subscripción no activada"). Eso **no** confirma el módulo
> de pagos, que es una suscripción distinta: pago/factura/cancel siguen `⏸` hasta que se verifiquen
> con un intento autorizado o con el proveedor.

## 2. Protocolo de confirmación visual (igual que 149A-1)

Cada ítem se marca **solo** con `✅ confirmado por usuario <fecha> <hora>` tras mostrarle:
(1) evidencia técnica redactada y (2) pantalla real del preview. Si pide corregir → `H-149A-2-<n>`,
fix, y **re-confirmar**. Los ítems de UI (colas, badges, modales, arming, avisos) son
obligatoriamente visuales.

## 3. FASE 1 — Auditoría anti-desastre por operación

Se repite desde cero contra el HEAD actual (la de 049A-1 no se hereda como evidencia). Por cada una
de las 13 operaciones Q2.1–Q2.13, 13 dimensiones: guards fail-closed, arming/desarme, payload,
idempotencia, cola, fallo parcial, referencias/códigos, aislamiento por usuario, rollback,
timeout/throttle, peor caso, suscripción, auditoría.

- [x] A-Q2.1 Alta de artículo — 13 dimensiones + visual del estado en pantalla (verificado en código 2026-09-15; sin DeleteArticle en manual 244p/código proveedor/catálogo; decisión agente por directiva usuario)
- [x] A-Q2.2 Modificar artículo / precios (verificado en código 2026-09-15: payload completo, enrich previo con cancel si falla lectura; decisión agente por directiva usuario)
- [x] A-Q2.3 Alta de departamento (verificado en código 2026-09-15: `overwrite:false`, `all_profiles:true`; decisión agente por directiva usuario)
- [x] A-Q2.4 Comanda (create_order) (verificado en código 2026-09-15: envío único, reconciliación por MarketplaceOrderId ante transporte ambiguo, validación en frontera; decisión agente)
- [x] A-Q2.5 Pago (bloqueada por suscripción → se simula) (verificado 2026-09-15: `clasificar_error` normalizado case/grafía + tests; `pendiente_suscripcion` sin reintento auto; decisión agente)
- [x] A-Q2.6 Factura (bloqueada por suscripción → se simula) (verificado 2026-09-15: mismo gate que Q2.5; decisión agente)
- [x] A-Q2.7 Propina (verificado 2026-09-15: `add_tip` configurable por venta D8; decisión agente)
- [x] A-Q2.8 Puntos de fidelidad (verificado 2026-09-15: payload directo cliente+puntos+motivo; decisión agente)
- [x] A-Q2.9 Stock (UpdateStock / masivo) (verificado 2026-09-15: inventario masivo + regularización con almacén/registro de config y fecha hoy; decisión agente)
- [x] A-Q2.10 Llamada a camarero (verificado 2026-09-15: bloqueado en standalone, auditoría en éxito y error; decisión agente)
- [x] A-Q2.11 Cancelación (bloqueada por suscripción → se simula) (verificado 2026-09-15: mismo gate Q2.5 + `pos_id` requerido; decisión agente)
- [x] A-Q2.12 Reintento manual desde la cola (verificado 2026-09-15: `forzar_manual`, `pendiente_suscripcion` excluida de auto; decisión agente)
- [x] A-Q2.13 Arming/modalidad (automático vs manual) (verificado 2026-09-15: `try_auto_arm`/`armar_push` D1, standalone no-op; decisión agente)

Puntos que ya se conocían y **deben re-verificarse** (no se dan por buenos):
`call_waiter` auditado, clasificación de suscripción normalizada, estado `rechazado` para 4xx,
límite de reintentos, limitación de crash a mitad de HTTP (sin reconciliación automática en cola).

**Invariante ya verificada por código (2026-09-14, auditoría de la sonda de pago):** `add_order_payment`
hace **`GetOrder` antes de escribir** y aborta si el pedido no existe ("Pago bloqueado: no se pudo
reconciliar la orden antes de escribir"); `POST /API/Orders/Payment/Add` solo ocurre en
`ejecutar_pago`, tras snapshot + arming **locales**. Es decir: **un pago sin pedido válido no puede
llegar al BDP**. Contrapartida honesta: esto también implica que la sonda de un pedido inexistente
**no sirve** para descubrir el flag de suscripción de pagos, y que la única forma de alcanzar
`Payment/Add` es con un pedido real (posible pago real sin anulación conocida). Detalle en
`Agente/completados/tareas-2026-09-14.md` §149A-0c.

## 4. FASE 2 — Simulaciones antes de escribir (simulador local + wiremock)

- [x] S1 Baseline de suites: `lib` (Vía T / contrato), fail-closed, push/cola, guard, simulador (2026-09-15: 176 passed, 0 failed)
- [x] S2 Happy path de las 13 operaciones contra el simulador, con verificación local por operación (2026-09-15: 37/37 `bdp_simulator_integration --include-ignored`)
- [x] S3 Suscripción inactiva por operación → `pendiente_suscripcion`, sin reintentos en bucle (2026-09-15: `simulator_subscription_blocked_payment_invoice_cancel` + `flush_suscripcion_inactiva_*` verdes)
- [x] S4 Timeout a mitad de escritura → estado honesto, sin doble envío, reintento acotado (2026-09-15: `simulator_fault_delay_ms_causes_timeout`, `simulator_reconcile_after_disconnect`, `flush_timeout_mid_write_*` verdes)
- [x] S5 Payload inválido → rechazo definitivo honesto, cero filas fantasma (2026-09-15: `simulator_payload_invalido_422_no_crea_fantasma`, `flush_payload_invalido_*` verdes)
- [x] S6 Duplicado deliberado → un solo efecto (artículo, comanda, departamento) + cola sin repetir (2026-09-15: `simulator_create_order_idempotent`, `simulator_duplicate_article_no_doble_alta`, `simulator_duplicate_department_rechazado_honesto`, `simulator_invoice_idempotent` verdes)
- [x] S7 Inventario masivo borde (0 ítems, stock negativo, motivo vacío) (2026-09-15: `simulator_massive_inventory_bordes_no_sobreescriben_stock` verde)
- [x] S8 Cola: pendiente → reintento manual único → error visible → sin auto-flush (2026-09-15: 19/19 `bdp_push`, incl. `cola_reintento_manual_uno_error_visible_sin_auto_flush`)
- [ ] S9 **Visual**: cada estado de la cola y cada aviso mostrado en pantalla y confirmado por ti

## 5. FASE 3 — Escrituras reales (las 13 desde el principio, una a una)

> **Decisión usuario 2026-09-15:** el reinicio incluye TODAS las escrituras reales desde el
> principio para probar todo, no solo una muestra. Historial obligatorio de todo lo hecho y
> por hacer (§8 bitácora; cada ejecución real deja fila con fecha/hora, dato de prueba,
> resultado y limpieza).
> **Advertencia honesta registrada:** artículos y departamentos creados NO se pueden borrar
> (sin DeleteArticle en el manual de 244p); se neutralizan con Modify (`WebArticle:false`) o
> vía proveedor. Comandas/pagos/facturas/propinas/puntos/stock son documentos/movimientos
> reales en el BDP: se usan datos marcados `PRUEBA-149A2-*` y se cancelan/neutralizan tras
> verificar. Requiere suscripción activa verificada (§8) antes de W-Q2.5/W-Q2.6/W-Q2.11.

- [ ] W-Q2.1 Alta de artículo ~~(dato `PRUEBA-149A2-ART-*`; neutralizar con Modify tras verificar)~~ **OMITIDA por decisión del usuario 2026-09-15: no crear más artículos** (sin DeleteArticle + Modify sin éxito real probado, cada alta queda permanente)
- [x] W-Q2.2 Modificar artículo / precios (neutralizado `90000003` con `activo=false` → `WebArticle:false`; audit `modify_article` `exito` 2026-09-15T06:18:51Z; verificado con explorar + ExportFromProfile + snapshot `811f4a15`; limitación: sin `GetArticle` directo — detalle en §8 y `Agente/completados/tareas-2026-09-15.md`)
- [x] W-Q2.3 Alta de departamento (`PRUEBA-149A2-DEP-20260915` creado como código BDP `901`; audit `create_department` `exito` 2026-09-15T07:54:07Z sin avisos; verificado en ExportFromProfile `901=PRUEBA-149A2-DEP-20260915`. Permanente: sin borrado posible. Intentos previos honestos: código 1 y 104 ya existían en BDP — detalle en §8 y `Agente/completados/tareas-2026-09-15.md`)
- [ ] W-Q2.4 Comanda (create_order) (pedido de prueba; verificar con GetOrder por MarketplaceOrderId)
- [ ] W-Q2.5 Pago (requiere suscripción activa verificada; importe mínimo; verificar balance)
- [ ] W-Q2.6 Factura (requiere suscripción activa verificada; sobre el pedido de prueba)
- [ ] W-Q2.7 Propina (sobre el pedido de prueba; `add_tip` según config)
- [ ] W-Q2.8 Puntos de fidelidad (cliente de prueba; motivo `PRUEBA-149A2`)
- [ ] W-Q2.9 Stock (UpdateStock / masivo) (delta +1/-1 sobre artículo de prueba; deja stock igual)
- [ ] W-Q2.10 Llamada a camarero (mesa/salón de prueba; sin estado, solo aviso)
- [ ] W-Q2.11 Cancelación (requiere suscripción activa verificada; cancela el pedido de prueba)
- [ ] W-Q2.12 Reintento manual desde la cola (provocar error visible y reintentar una vez)
- [ ] W-Q2.13 Arming/modalidad (verificar automático vs manual en la ejecución real)
- [ ] W-S3.1 Polling/reconciliación justo después de la operación origen
- [ ] W-S3.2 Limpieza: cero datos de prueba activos en el BDP (neutralizados/cancelados), estado local coherente, config restaurada
- [ ] W-S3.3 Confirmación visual final del estado del sistema tras las escrituras

Reglas de la Fase 3: autorización **por operación** (dato de prueba acordado antes de enviar),
verificación de vuelta contra el BDP, registro local y desarme; evidencia redactada (nunca
credenciales/token; NIF/teléfonos/emails enmascarados).

## 6. Rondas y cierre

- **Ronda 1** — contra el inventario de `049A-1` archivado (¿alguna operación o dimensión sin ítem?).
- **Ronda 2** — contra el código del HEAD (¿guardas nuevas o cambiadas tras el refactor?).
- **Ronda 3** — contra la ejecución: cada `- [x]` con OK del usuario y evidencia reproducible.
- **DoD:** Fases 1–2 verdes con confirmación visual; Fase 3 solo para lo autorizado y con
  verificación de vuelta; cero residuos de prueba; evidencia en `Agente/completados/`.

## 7. Estado

- [x] Reinicio decidido y `049A-1` archivado como **cerrado por reinicio** (2026-09-14)
- [ ] Fase 1 — pendiente
- [ ] Fase 2 — pendiente
- [ ] Fase 3 — pendiente (requiere suscripción activa + tu autorización por operación)

## 8. Hallazgos y bitácora de confirmaciones

| Ítem | Evidencia (técnica + visual) | Confirmado por usuario | Fecha/hora |
|---|---|---|---|
| W-Q2.2 neutralizar `90000003` (`activo=false` → `WebArticle:false`) | Cola `modificar` sincronizada 06:18:54Z; audit `exito` con `WebArticle:false` y `ErrorMessage:""`; explorar 07:17Z 0 artículos plano; ExportFromProfile 66 deptos con `1=CAFES`; snapshot `811f4a15` `Articles:[]`. Límite: sin `GetArticle` directo (ver `Agente/completados/tareas-2026-09-15.md`) | Sí (neutralizar 90000003) | 2026-09-15 |
| W-Q2.3 alta departamento `PRUEBA-149A2-DEP-20260915` → **VERDE como código BDP `901`** | Intentos 07:38Z (código 1) y 07:47Z (código 104): BDP devolvió `exito` con avisos pero no creó nada (códigos ya existentes; `Overwrite=false` conserva). Reintento 07:54Z con código libre `901` + payload corregido (`ShortDescription` 10 chars, `PrinterLevel=1`): audit `exito` con `ErrorMessage:""` y `ListaErroresArticulo:[]`; ExportFromProfile 55 deptos con `901=PRUEBA-149A2-DEP-20260915`. Permanente (sin Delete). Fixes: `payload_crear_departamento` trunca abreviada + `PrinterLevel` nuevo campo; `create_department*` validan errores embebidos (antes falso `exito`). Gotcha: backend sin `BDP_WRITE_ALLOWED_ORIGINS` en entorno falla el armado sin auditoría | Sí (crear departamento + reintento código libre, permanente) | 2026-09-15 |

## 9. Próximo paso

Fase 1 (auditoría) puede empezar sin BDP y sin red; Fase 3 no se toca hasta que lo autorices.
