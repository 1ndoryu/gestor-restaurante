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

- [ ] A-Q2.1 Alta de artículo — 13 dimensiones + visual del estado en pantalla
- [ ] A-Q2.2 Modificar artículo / precios
- [ ] A-Q2.3 Alta de departamento
- [ ] A-Q2.4 Comanda (create_order)
- [ ] A-Q2.5 Pago (bloqueada por suscripción → se simula)
- [ ] A-Q2.6 Factura (bloqueada por suscripción → se simula)
- [ ] A-Q2.7 Propina
- [ ] A-Q2.8 Puntos de fidelidad
- [ ] A-Q2.9 Stock (UpdateStock / masivo)
- [ ] A-Q2.10 Llamada a camarero
- [ ] A-Q2.11 Cancelación (bloqueada por suscripción → se simula)
- [ ] A-Q2.12 Reintento manual desde la cola
- [ ] A-Q2.13 Arming/modalidad (automático vs manual)

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

- [ ] S1 Baseline de suites: `lib` (Vía T / contrato), fail-closed, push/cola, guard, simulador
- [ ] S2 Happy path de las 13 operaciones contra el simulador, con verificación local por operación
- [ ] S3 Suscripción inactiva por operación → `pendiente_suscripcion`, sin reintentos en bucle
- [ ] S4 Timeout a mitad de escritura → estado honesto, sin doble envío, reintento acotado
- [ ] S5 Payload inválido → rechazo definitivo honesto, cero filas fantasma
- [ ] S6 Duplicado deliberado → un solo efecto (artículo, comanda, departamento) + cola sin repetir
- [ ] S7 Inventario masivo borde (0 ítems, stock negativo, motivo vacío)
- [ ] S8 Cola: pendiente → reintento manual único → error visible → sin auto-flush
- [ ] S9 **Visual**: cada estado de la cola y cada aviso mostrado en pantalla y confirmado por ti

## 5. FASE 3 — Escrituras reales (una a una, autorizadas por ti)

- [ ] W-Q2.1 … W-Q2.13 una por una (misma numeración que §3). Pago/factura/cancel quedan `⏸` hasta
      activación de la suscripción.
- [ ] W-S3.1 Polling/reconciliación justo después de la operación origen
- [ ] W-S3.2 Limpieza: cero datos de prueba en el BDP, estado local coherente, config restaurada
      (`bdp_sync_enabled=false`, `read_only`, poll off)
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
| — | — | — | — |

## 9. Próximo paso

Fase 1 (auditoría) puede empezar sin BDP y sin red; Fase 3 no se toca hasta que lo autorices.
