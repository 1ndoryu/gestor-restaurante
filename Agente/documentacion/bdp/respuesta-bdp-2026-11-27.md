# Respuesta de BDP al correo del 26/11 (`18AB-12`, `21AB-5c`, `18AB-6`)

Recibida el 27/11/2026 06:10 de `integraciones@bdpcenter.com`, dirigida a la usuaria con
Guillermo en copia. Contesta a los cuatro puntos de `correo-bdp-2026-11-26.md`.

## Lo que dicen, punto por punto

| # | Pregunta nuestra | Respuesta de BDP | Qué hacemos |
|---|---|---|---|
| 1 | ¿Hostelería 36.2 con licencia completa permite modificar comandas de barra? ¿Cuál es el procedimiento? | La licencia completa **no cambia el comportamiento de la API**. Piden **qué endpoint usamos y un ejemplo del JSON** de una comanda de barra «para revisar por qué la petición devuelve un 200 y la comanda queda igual». | Enviarles endpoint y JSON real. No es un límite de licencia: ellos no lo dan por cerrado. |
| 2 | ¿`AddOrderPayment` admite más de tres pagos sucesivos? | **Sí, admite más de tres.** «Podéis quitar el límite de tres en vuestra aplicación. Esa indicación del manual está desactualizada.» | Tope retirado hoy (`reglas.rs`); lo que corta es el saldo. |
| 3 | Pago negativo y devolución de una facturada | Se referían a una **comanda abierta**. Una vez emitida la factura, la comanda deja de admitir pagos: **la API no permite añadir ni modificar cobros de una factura emitida, ni tramitar su devolución**. Eso lo hace el personal de caja desde el TPV. | La web no ofrece devolución ni pago negativo sobre facturada. Sobre comanda abierta, el pago negativo sigue sin confirmarse explícitamente. |
| 4 | Servicio a Domicilio: licencia, datos, vigencia de la 84 | Son **dos aplicaciones distintas**. Hay que **instalar Servicio a Domicilio y liberar la suscripción en Hostelería**; una vez liberada, BDP **reasigna la aplicación a la suscripción** y ya se activa en Servicio a Domicilio. **Los datos se pueden trasladar** de una a otra. No contestan a la vigencia de la licencia demo ni a la 84. | Procedimiento claro y sin coste de migración de datos; falta que la usuaria decida con Guillermo cuándo. La vigencia del demo sigue sin respuesta. |

## Contexto que añaden

- «WebLinkRestAPI está pensada principalmente para que aplicaciones externas envíen
  comandas al TPV; la edición de comandas desde la API tiene un alcance más limitado.»
  Para las operaciones de camarero tienen **Handy** y el **TPV**.
- Esto encaja con lo que ya sabíamos: **no hay endpoint de modificación**. Modificamos
  con `POST /API/Orders/Create` y el `OrderId` de la comanda viva. El manual sí documenta
  `Order.OrderId` como campo de entrada (`# WEBLINK RESTAPI.md:11305`: «si no se especifica
  ninguno, o es cero, BDP asignará un número»), pero no dice en ningún sitio que enviarlo
  con el identificador de una comanda viva la modifique; el ejemplo de `Create` va a `0`, y
  `OrderOperationType` solo tiene 0 y 1. Esa es exactamente la duda que hay que ponerles por
  escrito junto al JSON.

## Corrección del 28/11: el «200 sin aplicar» en barra no está aislado

La evidencia que sostiene el «contesta 200 y no aplica» en barra es más débil de lo que
decían los documentos del 26/11. La matriz del 23/11
(`modificaciones-barra-vs-mesa-2026-11-23.md` §1) probó cinco variantes sobre comanda de
barra y **todas dieron `[300033]` o `300008`, no un 200**; el único 200 sin aplicar de esa
matriz es el de `OrderOperationType = 1` (OnlyCheck), y ahí el 200 es lo esperado: el manual
dice que OnlyCheck comprueba y **no crea** (`# WEBLINK RESTAPI.md:7213-7218`), así que no
demuestra ningún defecto de barra.

Lo que sí queda: las comandas de barra 222–227 contestaron 200 con `ErrorMessage` vacío
según la medición del 26/11, y para ellas no he encontrado el script ni la salida que lo
respalde. El correo a BDP ya no afirma reproducibilidad a partir de eso: pregunta, que es lo
que corresponde cuando no tenemos la traza.

## Lo que queda abierto

1. **Barra**: mandarles endpoint + JSON y esperar su revisión.
2. **Pago negativo sobre comanda abierta**: no lo confirman; el punto 3 cierra solo la
   facturada.
3. **Vigencia de la licencia demo** (`21AB-5e`): sin respuesta en este correo.
4. **`Type=2`**: falta la decisión de la usuaria.

## Ficheros que cambian

- `src/services/bdp_sync_pago/reglas.rs` y `bdp_sync_pago.rs`: fuera el tope de tres.
- `tests/bdp_pagos_idempotencia_simulador.rs`, `tests/bdp_saldo_remoto_simulador.rs`:
  el cuarto pago se registra; lo que corta es el saldo.
