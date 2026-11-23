# Modificaciones de comanda en el TPV demo: mesa sí, barra no (2026-11-23)

Sondas del 23/11 contra el TPV demo 36.2 (`TpvNet.exe` del portátil, túnel
`89:8071`) y contra el simulador. Complementa
`sondas-modificar-comanda-y-mesas-2026-11-02.md`, cuyo contrato **solo vale para
comandas de mesa** (`Type = 1`).

## 1. La restricción es de la licencia demo, no del payload

El manual del WebLink (`Manual-WebLink.pdf`, pág. 40) limita la licencia demo:

- **Máximo 6 líneas por comanda.**
- **Solo las mesas 1, 2 y 3.**

`Type`: 0 Barra, 1 Mesa, 2 Domicilio; `RoomNumber`/`TableNumber` solo tienen
sentido con `Type = 1`.

Consecuencia observada: **una comanda de barra (`Type = 0`) no admite ninguna
modificación**. Todas las variantes devuelven `[300033]` (o `300008`):

| Variante probada (comanda de barra) | Resultado |
|---|---|
| Contrato del 02/11 (líneas delta + total final) | 300033 |
| Comanda entera con una unidad más | 300033 |
| `OrderOperationType = 1` (OnlyCheck) | 200 sin aplicar |
| Mismo `MarketplaceOrderId` / otro distinto | 300033 / 300033 |
| `Type 1` con salón 0 y mesa 0 | 300008 |

No es deduplicación por `MarketplaceOrderId`: el TPV rechaza igual con id nuevo.

Una comanda de **mesa** (`Type = 1`, salón 1, mesas 1–3) sí acepta el contrato
del 02/11 sin cambios.

## 2. BDP puede contestar 200 sin aplicar nada

Observado en las comandas de barra 174 y 176: `Orders/Create` con `OrderId`
devuelve **HTTP 200, `OrderId` y `ErrorMessage` vacío**, y la comanda queda
exactamente igual (mismo total, mismas líneas).

Un 200 no demuestra una escritura. La aplicación ahora relee `GetOrder` y solo
da la edición por hecha si el total coincide con el esperado.

## 3. Dos defectos corregidos (`21AB-5d`)

### a) Un rechazo de negocio dejaba la venta bloqueada para siempre

`BdpWeblinkError::Remote` (HTTP 200 con `ErrorMessage`, p. ej. el 300033) se
mandaba a `reconciliar_modificacion`, se anotaba `ambiguo` y a partir de ahí
`ensure_no_unresolved` bloqueaba toda escritura de esa venta — con un error que
no había cambiado una línea. Ahora va a `error`: no escribió nada, no deja
auditoría abierta.

### b) Un 200 sin aplicar se daba por hecho

La web registraba un cambio que el TPV no tenía, y el siguiente delta se
calculaba contra un estado falso. Ahora `confirmar_total_en_bdp` relee el total
tras el 200 y revierte con un mensaje claro si no coincide.

### c) Una modificación ambigua bloqueaba la venta para siempre

El poller reconciliaba `add_payment` e `invoice`, y `create_order` lo resolvía
`recuperar_lista_en_vuelo`, pero **`modify_order` no lo miraba nadie**. Además,
una modificación ambigua de una venta ya facturada no aparece en ninguna de las
tres listas de ventas, así que el corte temprano de `poll_pending` ni siquiera
llegaba a reconciliar.

Ahora el poller incluye `modify_order`: si `GetOrder` devuelve el total final, la
escritura se aplicó (`exito`); si devuelve otro, no llegó (`error`, la web nunca
cambió); si no se pudo preguntar, sigue `ambiguo` y se reintenta. El corte
temprano consulta las auditorías abiertas.

## 4. Lo que se puede y no se puede probar en el demo

- **Sí:** tres pagos parciales con identificadores distintos, reinicio real de
  `TpvNet.exe` con una operación en el aire, modificaciones encadenadas y en
  paralelo sobre una **mesa**, comandas de **más de 6 líneas** (comanda 180,
  siete `Items`) y mesas **fuera de 1–3** (comanda 158 en la mesa 4).
- **No:** modificaciones de comandas de **barra** (rechazo del TPV demo, no
  nuestro) y abrir una segunda comanda en una mesa que ya tiene una abierta.

**Corrección del 23/11 (20:43):** la sonda que llamé «límite de seis líneas»
nunca lo probó. Lo que envió fue **una línea con 7 unidades**, y el TPV la
aceptó sin problema (comanda 180, 1 línea × 7 uds, 7,35).

**Segunda corrección del 23/11 (21:05), ya con `GetOrder` delante:** la comanda
180 **sí tiene siete `Items` distintos** (siete `1001` × 1 ud, `Id` 1–7), no una
línea de siete unidades. Es decir, el TPV demo **aceptó una comanda de siete
líneas**, y el tope de «6 líneas» del manual (pág. 40) **no lo aplica este
ejecutable**.

El tope de mesas 1–3 tampoco: la comanda **158 está abierta en la mesa 4**
(salón 1, 2,50, 1 línea), así que el TPV demo admite mesas fuera de 1–3. Lo que
sí es un rechazo real es abrir una segunda comanda en una mesa que ya tiene una
abierta: eso lo comprobó la aplicación al intentar la mesa 4 con la 158 viva
(«La mesa 4 del salón 1 ya tiene una comanda abierta en el TPV (nº 158)»).

Conclusión: **de las dos restricciones del manual demo no se ha observado
ninguna**. Las dos siguen siendo afirmaciones del PDF sin confirmar en este
ejecutable.

Con la licencia real del restaurante no consta ninguna de estas dos
restricciones; hay que verificarlo en el piloto.

## 5. Evidencia

- `tests/bdp_modificaciones_rapidas_simulador.rs` — 5 tests (serialización,
  corte tras aplicar, rechazo de negocio, 200 sin aplicar).
- `tests/bdp_modificacion_ambigua_poller.rs` — 4 tests (cierre en éxito, cierre
  en error, sin respuesta sigue ambiguo, venta facturada no salta el turno).
- `tools/bdp-weblink-simulator/server.py` — modo de fallo `ok_without_apply`.
- Sondas: `dev89:~/.local/share/RESTAURANTE/21ab5d-probe/` (`modificaciones-mesa.mjs`,
  `sonda-barra-matrix.mjs`, `sonda-dedupe.mjs`, `sonda-releer2.mjs`).
