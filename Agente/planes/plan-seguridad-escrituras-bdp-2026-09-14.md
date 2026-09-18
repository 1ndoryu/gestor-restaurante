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
- [x] W-Q2.4 Comanda (create_order) (`PRUEBA-149A2-COMANDA-20260915` → BDP OrderId **`6258`** 0,11€; audit `create_order` `exito` 2026-09-15T08:41:38Z; `bdp_synced=true`, `bdp_order_status` nulo — GetOrder-contenido devuelve `[301010]` en API gratuita, también sobre 5330; la comanda queda ABIERTA: anular desde el TPV. Modo restaurado a `read_only`. Detalle en §8 y `Agente/completados/tareas-2026-09-15.md`)
- [ ] W-Q2.5 Pago (INTENTADO 2026-09-15 con autorización: sonda directa `Payment/Add` amount=0 → `[301201]-IMPORTE INCORRECTO`, ya no "Subscripción no activada" → módulo de pagos responde; intento real 0,11€ Contado orden 6258 vía Glory BLOQUEADO por el guard de reconciliación —correcto— con `422 [301010]`; cero pagos registrados; modo `read_only`; RE-VERIFICADO 2026-09-18 con autorización fase 3: armado manual `unidirectional` (`add_payment`↔venta 6258, snapshot vigente) + intento real → mismo `422 [301010]` pre-escritura; `bdp-payments` 0,11 pendiente / 0 pagado / 0 pagos, sin entrada `add_payment` en auditoría; desarmado + `standalone` restaurado. Bloqueo 100% vigente)
- [ ] W-Q2.6 Factura (NO INTENTADA 2026-09-18: requiere suscripción activa verificada —una factura exitosa sería un documento fiscal real irreversible—; sigue pendiente del tier de pago)
- [x] W-Q2.7 Propina (EJECUTADA 2026-09-18 con autorización fase 3: `POST propina` 0,01€ `add_tip:true` sobre venta 6258 → fila `propina/propina` `05d61e7f` + modo `bdp` + reintento individual → 1/1 `sincronizado`; audit `add_tip` `exito` 10:24:44Z `ErrorMessage:""`; modo restaurado `standalone`. Hallazgo de guard: en `standalone` el reintento devuelve `omitidos_standalone:1` — propina/pago/stock exigen modo `bdp`, a diferencia de `cancel_order` que se auto-arma)
- [x] W-Q2.8 Puntos de fidelidad (EJECUTADA 2026-09-16 con autorización: cliente `PRUEBA` BDP 900001, +1 motivo `PRUEBA-149A2-Q2.8` y -1 neutraliza; audit `add_points` `exito` sin error 05:36:15Z/05:36:17Z; saldo local 1.00 → 0; neto BDP cero; detalle en §8)
- [x] W-Q2.9 Stock (EJECUTADO 2026-09-18 con autorización fase 3: ajustes +1/-1 sobre `1001` motivo `PRUEBA-149A2-Q2.9`, stock local 1.0 → 0 neto cero; flush en modo `bdp` → BDP responde `[201600]-EL ALMACÉN 1 NO EXISTE` — el canal funciona, el BDP no tiene almacén 1; ver punto 5 del §9. Solo se encoló un `regularizar` (+1): el -1 no generó fila —posible dedupe por (dominio,entidad,operacion) pendiente—; sin efecto porque el +1 fue rechazado. Modo restaurado `standalone`)
- [x] W-Q2.10 Llamada a camarero (EJECUTADA 2026-09-16 con autorización: zona `PRUEBA-149A2` + mesa 99 locales, push directo `Table=99 Room=1`; BDP respondió `[404602]-No se ha establecido una IP de Servidor de Mensajes... del terminal por defecto` — el canal funciona, falta config del terminal en el BDP; audit `call_waiter` `error`; zona/mesa de prueba eliminadas, plano restaurado; detalle en §8)
- [x] W-Q2.11 Cancelación (EJECUTADA 2026-09-18 como W-Q2.11 formal: cancelación de la comanda de prueba 6338 vía `POST reintentar` individual —fila `venta/6338/cancelar` `sincronizado`, audit `cancel_order` `exito` `cfde8212` 07:30:41Z, venta `anulada=true`—; refuta el "Subscripción no activada" de agosto)
- [x] W-Q2.12 Reintento manual desde la cola (EJECUTADO 2026-09-16: reintento fila `stock/inventario` → 1 procesada / 1 error, mismo `[201500]`; mecanismo + auditoría verificados; detalle §8)
- [x] W-Q2.13 Arming/modalidad (automático verificado vía Q2.8; MANUAL verificado 2026-09-16: armado `create_customer`↔cliente PRUEBA 200 con lease y snapshot, desarmado inmediato 200, cero tráfico BDP; detalle §8)
- [x] W-S3.1 Polling/reconciliación (EJECUTADO 2026-09-16: `POST bdp-poll` 200 `updated:0` — mecanismo OK, nada que reconciliar sin tier de pago)
- [ ] W-S3.2 Limpieza: PARCIAL 2026-09-18 (parte local OK: `sync_mode=read_only`, `modo=standalone`, `sync_enabled=false`, cola 13 filas coherente —solo errores preexistentes conocidos + filas de hoy en estado final—; stock local `1001`=0; pagos 6258 en 0). Pendiente del restaurante en TPV: anular comanda 6258 + depto 901 `PRUEBA-149A2-DEP-20260915` (la 6338 ya está anulada vía API)
- [x] W-S3.3 Revisión final de estado (2026-09-16: cola 10 filas 7 sincronizado/3 error preexistentes; plano intacto; pagos 0; puntos 0; modo `read_only`. Límite honesto: confirmación visual en TPV imposible desde aquí — la hace el restaurante)

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
- [x] Fase 1 — hecha (auditoría A-Q2.1–Q2.13 verificada en código 2026-09-15)
- [ ] Fase 2 — pendiente solo S9 visual
- [ ] Fase 3 — parcial 2026-09-18: VERDES Q2.2/Q2.3/Q2.4/Q2.7/Q2.8/Q2.10/Q2.11/Q2.12/Q2.13/S3.1; Q2.5 bloqueado vigente; Q2.9 bloqueado por almacén 1; Q2.6 no intentada (tier pago); Q2.1 omitida; S3.2 local OK + TPV pendiente

## 8. Hallazgos y bitácora de confirmaciones

| Ítem | Evidencia (técnica + visual) | Confirmado por usuario | Fecha/hora |
|---|---|---|---|
| W-Q2.2 neutralizar `90000003` (`activo=false` → `WebArticle:false`) | Cola `modificar` sincronizada 06:18:54Z; audit `exito` con `WebArticle:false` y `ErrorMessage:""`; explorar 07:17Z 0 artículos plano; ExportFromProfile 66 deptos con `1=CAFES`; snapshot `811f4a15` `Articles:[]`. Límite: sin `GetArticle` directo (ver `Agente/completados/tareas-2026-09-15.md`) | Sí (neutralizar 90000003) | 2026-09-15 |
| W-Q2.3 alta departamento `PRUEBA-149A2-DEP-20260915` → **VERDE como código BDP `901`** | Intentos 07:38Z (código 1) y 07:47Z (código 104): BDP devolvió `exito` con avisos pero no creó nada (códigos ya existentes; `Overwrite=false` conserva). Reintento 07:54Z con código libre `901` + payload corregido (`ShortDescription` 10 chars, `PrinterLevel=1`): audit `exito` con `ErrorMessage:""` y `ListaErroresArticulo:[]`; ExportFromProfile 55 deptos con `901=PRUEBA-149A2-DEP-20260915`. Permanente (sin Delete). Fixes: `payload_crear_departamento` trunca abreviada + `PrinterLevel` nuevo campo; `create_department*` validan errores embebidos (antes falso `exito`). Gotcha: backend sin `BDP_WRITE_ALLOWED_ORIGINS` en entorno falla el armado sin auditoría | Sí (crear departamento + reintento código libre, permanente) | 2026-09-15 |
| W-Q2.4 comanda `PRUEBA-149A2-COMANDA-20260915` → **BDP OrderId `6258`** | venta `155c6d74-…` creada (base 0,10 + IVA 10% 0,01; artículo `1001` map; pago PROPINA/exento) tras corregir endpoint `sync-mode` (PUT, no POST) y reintentar snapshot completo con body string (mapa → error deserialización). Armado `unidirectional` `200` + snapshot `f3ae93bd-…`; sync `200` `bdp_synced=true` `bdp_order_id=6258` `bdp_sync_error` nulo; audit `create_order` `exito` 08:41:38Z; log `[065A-5] sincronizada OrderId=6258`. Verificación parcial: `bdp-status` 200 pero `bdp_order_status` nulo — GetOrder con contenido falla `[301010]` en API gratuita (idéntico sobre la comanda 5330: limitación del plan gratuito, no del pedido). Confirmación visual en TPV pendiente (anular comanda 6258 + depto 901). Modo restaurado `read_only` | Sí (comanda real 0,11€ comanda-de-prueba) | 2026-09-15 |
| W-Q2.5 pago 0,11€ orden 6258 → **BLOQUEADO por el guard (correcto), suscripción probablemente ACTIVA** | Autorización usuario 2026-09-15. Previo: `bdp-payments` pendiente 0.11/pagado 0/sin pagos. Sonda directa (curl, amount=0, nada escribe): `Payment/Add` sobre 5330 devuelve `[301201]-IMPORTE INCORRECTO`, ya NO "Subscripción no activada" (agosto) → el módulo de pagos responde validación de negocio. Intento real vía Glory (arming manual `unidirectional` 1 op `add_payment`, confirmación exacta, idempotency nueva): `422 "no se pudo reconciliar la orden antes de escribir: [301010]"` — el guard `GetOrder`-antes-de-escribir funciona y aborta porque la API gratuita no devuelve Total/Payments. Cero pagos registrados; modo restaurado `read_only`. Conclusión: para pagar hay que resolver la reconciliación (suscripción de pago completa o reconciliación local aceptada por el cliente); el endpoint de pago en sí ya no rechaza por licencia | Sí (intento pago real 0,11€ Contado orden 6258) | 2026-09-15 |
| VERIFICACIÓN PROFUNDA acceso BDP (todas las pruebas paradas hasta resolver) → **el mismo código integrador que funcionaba hoy ahora es rechazado; causa sin confirmar** | Auditoría local: `modify_article` 06:18:51 + `create_department` 07:38/07:47/07:54 + `create_order` 08:41:38, todos `exito` = logins BDP válidos esta mañana con el código de la BD. Intento de pago (tarde): fresh login OK → `[301010]` (el backend crea cliente nuevo por llamada: `BdpWeblinkClient::new` por operación, sin caché entre llamadas). Sonda `C:\tmp\pgq\sonda-149a2.js`: config BD == `.env` byte a byte (`DB_INT_EQ_ENV=true`, 8 chars). Intentos directos posteriores con forma CORRECTA (`Login/Password/TiempoSession/CodigoIntegrador` PascalCase, igual que el backend) → `{"ErrorMessage":"[5]-EL CÓDIGO DE INTEGRADOR...NO ES VÁLIDO","AuthSession":null}` persistente (00:18), servidor vivo (`/Service/Health` `IsAlive:true`). Certeza: el VALOR no se tocó (solo lecturas) y funcionó hoy; el rechazo es estado del lado BDP. Investigación código (00:30): el código en BD es el documentado desde agosto (`EQ_DOC_AGOSTO=true`, 8 chars; `.env` reescrito por bootstrap 16:06 desde la BD; `UPDATED 23:59` coincide con mis propios PUT de armado/desarmado). Conclusión: SÍ es el código correcto — el mismo que autenticó hoy 06:18–08:41 y en el intento de pago. Sin confirmar si el rechazo es cambio del restaurante, caducidad o bloqueo por mis propias sondas malformadas previas (2-3 logins con forma incorrecta: van en mi debe como posible contribuyente; no más sondas hasta aclarar). Sin login válido NINGUNA prueba más es posible | — (verificación de solo lectura) | 2026-09-15 |

| CORRECCIÓN `.env` (01:00) → **acceso confirmado OK, era copia local mala, no cambio del BDP** | El usuario confirmó que nunca cambió el código; el valor correcto es el de la BD (8 chars, login OK vía backend todo el día). Causa encontrada: mi copia anterior BD→`.env` perdió el primer carácter (`.env` quedó con 7 chars; `comparar.js`: `ENV_EQ_DB=false`, `ENV_EQ_DB_SIN_PRIMER=true`). Por eso mis sondas directas daban `[5]` mientras el backend (usa BD) autenticaba bien. Añadido: lecturas locales de `.env` bajo OneDrive salen rasgadas de forma intermitente (0/7/8 chars según lectura; el backend nunca escribe `.env`, solo lo lee al arrancar). Reparación: reescritura solo de esa línea desde BD con backup en `C:\tmp\pgq\.env.bak-20260916`; verificación 10/10 lecturas espaciadas len 8 + igualdad con BD, y `sync-dry-run` 13/13 checks OK incluyendo "Sesión y versión: Login y version correctos". El punto 2 del §9 queda anulado (no hubo revocación de acceso). Riesgo que hubo: reiniciar el backend con el `.env` malo habría pisado el código bueno de la BD vía bootstrap | Sí (copiar valor BD al env; usuario confirmó no haberlo cambiado) | 2026-09-16 |

| W-Q2.10 llamada a camarero → **canal VERDE, falta config del terminal BDP** | Autorización usuario 2026-09-16. Zona `PRUEBA-149A2` (orden 1) + mesa 99 creadas locales (201) y eliminadas tras la prueba (204); plano verificado restaurado (solo Salón principal 1-6). Push directo sin cola (solo exige allowlist de destino, sin armado): `POST llamar-camarero` → 500 + audit `call_waiter` `error` con `[404602]-No se ha establecido una IP de Servidor de Mensajes para la Configuración de Servicios Web del terminal por defecto`. Lectura: login OK, petición bien formada (`mesa:99,sala:1`), el BDP la entiende y responde validación de negocio/config — igual patrón que `[301201]` en pagos. Sin fix de código; el aviso funcionará cuando el restaurante configure la IP del servidor de mensajes en su terminal (punto para el informe §9) | Sí (Q2.10 sobre mesa/salón de prueba) | 2026-09-16 |

| W-Q2.8 puntos → **VERDE extremo a extremo, neto cero** | Autorización usuario 2026-09-16. Cliente `PRUEBA` (BDP 900001): `POST puntos` +1 (200) y -1 neutraliza (200), motivo `PRUEBA-149A2-Q2.8`; audit `add_points` `exito` sin error 05:36:15Z y 05:36:17Z; saldo local 1.00 → 0. Hallazgo de guard: el armado MANUAL (PUT sync-mode) rechazó `add_points`+`cliente` con 422 — la matriz manual solo admite (create_customer↔cliente) y (create_order/add_payment/invoice↔venta) (`configuracion.rs:384-394`); el `flush` aplicó su propio auto-armado (`armar_push`+backup+auditoría) y ejecutó dentro de guardarraíles. Esto además verifica Q2.13-modalidad-automática. Sin fix de código (la matriz manual podría ampliarse a add_points en el futuro, mejora opcional) | Sí (Q2.8 cliente de prueba) | 2026-09-16 |

| W-Q2.12 reintento → **VERDE, error conocido re-confirmado** | Autorización usuario 2026-09-16 (todo lo ejecutable). `POST /api/bdp/push/<id>/reintentar` sobre `stock/inventario` en error → 200 `procesados:1 errores:1`; la fila conserva el mismo `[201500]-EL ALMACÉN 1 NO EXISTE`. Sin datos nuevos, sin residuos | Sí (todo lo ejecutable ahora) | 2026-09-16 |
| W-Q2.13 manual → **VERDE sin tráfico BDP** | Armado `create_customer`↔cliente PRUEBA: PUT 200 (puerta completa: destino+alcance+objetivo+snapshot+huella) + desarmado inmediato PUT `read_only` 200. Cero llamadas al BDP. El automático ya había quedado verificado vía Q2.8 | Sí (todo lo ejecutable ahora) | 2026-09-16 |
| W-S3.1 polling → **mecanismo OK, nada que reconciliar** | `POST /api/ventas/bdp-poll` 200 `{"updated":0}` — esperado sin tier de pago (6258 sin contenido vía `[301010]`) | Sí (todo lo ejecutable ahora) | 2026-09-16 |
| W-Q2.11 formal (cancelación 6338) → **VERDE, `CancelOrder` funciona** | Autorización fase 3 2026-09-18. Anulación local + `POST reintentar` individual (modo `bdp`): 1/1 `sincronizado`, audit `cancel_order` `exito` (`cfde8212`, 07:30:41Z), venta `anulada=true`. Reintento individual, no flush (10 filas ajenas en error no tocadas) | Sí (fase 3) | 2026-09-18 |
| W-Q2.7 propina 0,01€ sobre 6258 → **VERDE extremo a extremo** | Autorización fase 3 2026-09-18. `POST propina` (`add_tip:true`) → fila `05d61e7f` pendiente; en `standalone` el reintento devuelve `omitidos_standalone:1`; con modo `bdp` → 1/1 `sincronizado`; audit `add_tip` `exito` 10:24:44Z `ErrorMessage:""`. Modo restaurado `standalone` | Sí (fase 3) | 2026-09-18 |
| W-Q2.9 stock +1/-1 sobre `1001` → **canal OK, BDP sin almacén 1** | Autorización fase 3 2026-09-18. Ajustes locales +1 (stock 1.0) y -1 (stock 0, neto cero) motivo `PRUEBA-149A2-Q2.9`; flush en modo `bdp` → `[201600]-EL ALMACÉN 1 NO EXISTE` (igual que `[201500]` del inventario). Solo se encoló el +1 (posible dedupe del -1); sin efecto neto. Incidencia propia: un filtro PowerShell defectuoso reintentó TODA la cola — las sincronizadas las rechazó el servidor con 422 y las viejas en error repitieron su error conocido; cero escrituras indebidas. Lección: imprimir el conteo filtrado antes de iterar. Modo restaurado `standalone` | Sí (fase 3) | 2026-09-18 |
| W-Q2.5 re-verificación 2026-09-18 → **bloqueo vigente, cero pagos** | Autorización fase 3. Armado manual `unidirectional` (`add_payment`↔venta 6258) + intento 0,11€ Contado (`Q2.5-20260918-6258`) → `422 [301010]` pre-escritura; `bdp-payments` 0,11 pendiente/0 pagado/0 pagos, sin `add_payment` en auditoría; desarmado + `standalone`. Auto-arm rechazado (`ff_bdp_auto_arm=false`): el camino manual sí arma | Sí (fase 3) | 2026-09-18 |

## 9. INFORME PENDIENTE AL RESTAURANTE (al cierre de las pruebas)

1. **Falta el "WebLink REST API de pago"**: activaron salones/mesas (el endpoint de pagos ya responde validación de negocio en vez de "Subscripción no activada"), pero la conexión WebLink sigue en tier gratuito: `Orders/Get` con contenido devuelve `[301010]-...PASAR A WEBLINK RESTAPI DE PAGO`. Sin contenido de comanda nuestro guard se niega a pagar a ciegas (correcto).
2. **Acceso WebLink verificado OK el 2026-09-16 ~01:00** (punto anterior anulado): los `[5]` de la noche eran sondas locales con una copia truncada del código en `.env`, no una revocación del BDP. Login vía backend (valores de BD) funcionó todo el día y sigue verde (`sync-dry-run` 13/13). Solo falta el tier de pago del punto 1.
3. **Pendiente de ellos tras reactivar**: reintentar pago 0,11€ orden 6258, factura, y limpieza (anular 6258 + depto 901 `PRUEBA-149A2-DEP-20260915`).
4. **Configurar IP de Servidor de Mensajes en su terminal** (Q2.10): el aviso de camarero llega al BDP pero no se entrega por `[404602]` — falta esa IP en la Configuración de Servicios Web del terminal por defecto.
5. **Crear el almacén 1 en el BDP** (Q2.9 2026-09-18): `UpdateStock` y el inventario masivo llegan al BDP pero son rechazados con `[201500]/[201600]-EL ALMACÉN 1 NO EXISTE`. Sin almacén 1 no hay escritura de stock posible (no es licencia: es configuración del BDP).
6. **NOVEDAD 2026-09-18: `CancelOrder` y `AddTip` funcionan en esta conexión** (comanda 6338 cancelada vía API, propina 0,01€ sobre 6258 sincronizada): el "Subscripción no activada" de agosto ya no aplica a cancelación ni propina. Solo pago posterior y factura siguen bloqueados por el tier gratuito.

## 10. Próximo paso (protocolo silencioso 2026-09-17 — no romper nada)

Fase 1 (auditoría) puede empezar sin BDP y sin red; Fase 3 no se toca hasta que lo autorices
**por operación**. Para 169A-2 rige el protocolo silencioso del roadmap: P0 solo-lectura →
P1 `OnlyCheck` con payload de pago-en-creación (`EndType=1`, cero creación) → P2 real mínima
(0,11 €, `EndType=1`, hueco muerto, anulación <1 min). **`EndType=0` PROHIBIDO** (imprime en
cocina; requiere autorización separada + aviso). Prohibido tocar `.env`, reiniciar sin backup,
tocar datos reales (solo `PRUEBA-*`) o dejar residuos. Parar ante licencia/5xx/impresión
inesperada. Detalle y estado en bloque 169A-2 del roadmap.
