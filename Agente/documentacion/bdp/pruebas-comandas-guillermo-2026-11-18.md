# Las seis pruebas de Guillermo sobre comandas abiertas (18/11/2026)

Origen: mensaje de Guillermo del 17/11. BDP local 36.2 (licencia `DESARROLLO ASLEY - LA`, servicio 84), aplicación 84, terminal 1, `CONEXION LOCAL`, caja abierta. Todas las pruebas sobre comandas abiertas y sin facturar (`AlreadyInvoiced = false`), salón 1, artículos del catálogo web (1001 COCA-COLA 1,05 €, 1002 SCHWEPPES LIMÓN 1,20 €). Sondas en `C:/tmp/probe-bdp-18aa` (no versionadas; solo escriben en el BDP demo). Las cuentas y comandas se cancelaron al terminar; el artículo 1002 se devolvió a su estado original.

Convención de las tablas: **Enviado** es el cuerpo relevante de la petición, **Respuesta** el `ErrorMessage` literal de la API, **TPV** lo comprobado en el terminal.

## Resumen: qué falla

| Nº | Prueba | Resultado |
|---|---|---|
| 1 | Añadir y quitar líneas | Pasa |
| 2 | Cambiar cantidades y comentarios | Pasa |
| 3 | Descuentos e invitación | Pasa, salvo `NotAllowedAsInvitation`, que **BDP no comprueba** |
| 4 | Cambiar la comanda de mesa | **Falla**: no existe operación; reenviar deja la comanda en las dos mesas, y contra una mesa ocupada fusiona las líneas en la comanda del destino sin avisar |
| 5 | Pagos parciales y facturación | Pasa con tres salvedades: `Invoice:true` con pago parcial se rechaza; repetir `PaymentId` **duplica el cobro**; y ni `Orders/Create` ni `GetOrder` deduplican por `MarketplaceOrderId` |
| 6 | Cancelar comanda abierta | Pasa, pero anula también el pago parcial que tuviera, sin dejar rastro por API |

---

## 1. Añadir y quitar líneas de una comanda existente

Comanda 88, mesa 4, `MarketplaceOrderId` `S18A3940896`.

| Paso | Enviado | Respuesta | TPV / `GetOrder` |
|---|---|---|---|
| 1.1 Crear con 2 líneas (1 ud × 1,05 + 2 ud × 0,75) | `Order.Total` 2,55 | vacío | `OrderId` 88, total 2,55, 2 líneas |
| 1.2 Añadir una línea (1 ud × 1,05) | `OrderId` 88, `Items` = solo la línea nueva, `Order.Total` 3,60 | vacío | Misma comanda, 3 líneas, total 3,60 |
| 1.3 Quitar esa línea | `OrderId` 88, línea con `Units` −1, `Order.Total` 2,55 | vacío | Total 2,55. La línea **no desaparece**: queda una contrapartida de −1 ud en la línea 4 |

Regla confirmada: `Items` lleva **solo las líneas del cambio** y `Order.Total`, **el total final de la comanda**. Quitar es una contrapartida negativa, no un borrado.

## 2. Cambiar cantidades y comentarios

Misma comanda 88.

| Paso | Enviado | Respuesta | TPV / `GetOrder` |
|---|---|---|---|
| 2.1 Subir 1 ud (COCA-COLA) | Línea con `Units` 1, `Order.Total` 3,60 | vacío | Total 3,60 |
| 2.2 Bajar 1 ud | Línea con `Units` −1, `Order.Total` 2,55 | vacío | Total 2,55 |
| 2.3 Añadir línea con comentario | `Comments: [{Id: 0, Name: "SIN HIELO", Units: 1}]` | vacío | Comentario guardado en la línea 7, total 3,60 |

Cambiar el texto de una línea ya enviada no está soportado: no hay forma de reescribirla, solo de añadir o restar.

## 3. Descuentos e invitación

| Prueba | Enviado | Respuesta | `GetOrder` |
|---|---|---|---|
| 3.1 Descuento de línea del 50 % | `OrderItem.Discount: 50`, `DiscountPct: true`, `Total` 1,05 (2 ud × 1,05 = 2,10 menos 50 %) | vacío | `Discount` 50, `DiscountPct` true, `DiscountAmount` 1,05, línea 1,05 |
| 3.2 Descuento general del 10 % | `Order.Discount: 10`, `DiscountPct: true`, `Order.Total` 1,89 | vacío | `Discount` 10, `DiscountAmount` 0,21, total 1,89, líneas intactas a 2,10 |
| 3.3 Marcar una línea como invitación | `OrderItem.Invitation: true`, `Total` 0 | vacío | `Invitation` true, precio 0, total 0 |
| 3.4 Invitar un artículo con `NotAllowedAsInvitation` | Artículo 1002 con el campo a `true` (comprobado en `Articulos/Get` **y** en la tabla del TPV), `Invitation: true` | **vacío: lo acepta** | `Invitation` true, total 0 |
| 3.4 bis Control: el mismo artículo sin invitar | Sin `Invitation` | vacío | Normal, 1,20 |

**Hallazgo:** `NotAllowedAsInvitation` es solo un campo de ficha del artículo; `Orders/Create` **no lo comprueba**. El rechazo que pide la prueba no ocurre. Además `Articles/ModifyAndUpdateProfiles` acepta el campo y lo devuelve en `Articles/Get` sin escribirlo en la base: lo deja en una caché del servicio. Para 3.4 se forzó el valor directamente en la tabla `Articulos` y `Articulos_Perfil` del demo (restaurado al terminar).

## 4. Cambiar la comanda de mesa — **falla**

No hay operación de cambio de mesa en WebLinkRestAPI 1.2 (lista completa de rutas revisada). Las variantes probadas con `Orders/Create`:

| Prueba | Enviado | Respuesta | Efecto real |
|---|---|---|---|
| 4.1 Reenviar con `OrderId` y la mesa nueva, sin líneas | `OrderId` 97, `RoomNumber` 1, `TableNumber` 8, `Items: []`, `Total` 2,10 | `[300012]-COMANDA SIN LÍNEAS NI PAGOS` | Nada |
| 4.2 Reenviar con `OrderId`, la mesa nueva y la misma línea | `OrderId` 100, `TableNumber` 8, 1 línea de 2,10, `Total` 2,10 | vacío | **La mesa de origen NO se libera**: la comanda 100 queda colocada en las mesas 5 **y** 8. Las líneas no se duplican (total 2,10 en ambas) |
| 4.3 Cancelar desde la mesa de origen tras 4.2 | `Orders/Cancel` con `TableNumber` 5 | vacío | Quita la colocación de la mesa 5; la comanda sigue viva en la 8 |
| 4.4 Mover a una mesa ocupada, total distinto | Mesa 4 con la comanda 103 (1,20); reenvío de la 102 con total 2,10 | `[300033]-EL IMPORTE TOTAL INDICADO DE LA COMANDA NO COINCIDE CON EL TEÓRICO` | Correcto: no altera nada |
| 4.5 Mover a una mesa ocupada, **total que cuadra** | Mesa 4 con la 105 (1,20); reenvío de la 104 (`OrderId` 104) con total 3,30 | vacío | **Grave**: las líneas de la 104 se **fusionan en la 105** (2 líneas, total 3,30) y la 104 sigue existiendo en la mesa 5. No es un cambio de mesa, es una inyección silenciosa de líneas en otra comanda |

Comprobación en el TPV: el plano de sala se refresca a su ritmo y marcaba ocupadas las mesas 5 y 8 a la vez tras 4.2, pero también seguía marcando ocupadas mesas sin comanda (1, 3, 7), así que **no vale como testigo**. La prueba de que la comanda está en dos mesas es `Orders/Get` por `RoomNumber` + `TableNumber`, que devuelve el mismo `OrderId` en ambas. El modelo de datos del TPV que usa la API no es el de `Comandas_Cabecera` de la base `TpvNet` (esa tabla está vacía con comandas abiertas), así que la comprobación en base de datos tampoco sirve.

**Consecuencia para la aplicación:** hoy `update_order` no debe usarse para mover mesas. Si se hace, o se duplica la colocación o, peor, se fusionan líneas en la comanda de la mesa ocupada cuando el total coincide por casualidad. Es el supuesto que Guillermo contemplaba para su cuarta pregunta a BDP.

## 5. Dos pagos parciales, mismo `PaymentId` y facturación

Comanda 106 y 107, mesa 5, 4 ud × 1,05 = 4,20.

| Prueba | Enviado | Respuesta | Efecto |
|---|---|---|---|
| 5.1 Pago de 2,00 efectivo con `Invoice: true` | `TenderId` 1, `Amount` 2,00, `PaymentId` `PAGO-A1` | `[301301]-EL IMPORTE DE LOS PAGOS ES INFERIOR AL TOTAL DE LA COMANDA` | El **pago sí se guarda**; lo que se rechaza es facturar. `Invoice: true` no puede usarse en un pago parcial |
| 5.2 Pago de 2,20 tarjeta con `Invoice: true` (suma exacta) | `TenderId` 2, `Amount` 2,20, `PaymentId` `PAGO-B1` | vacío, `InvoiceNumber` `00001TB00000043` | Facturada al cuadrar |
| 5.3 `GetOrder` de una comanda facturada | `OrderId` de la 106 ya facturada | **vacío** | Devuelve `{"Order": null, "Status": 3}` y nada más: la comanda facturada deja de ser legible por la API. **Corregido el 18/11**: la versión anterior de esta fila decía que `GetOrder` rompía con «El índice estaba fuera del intervalo…». Ese error lo da `Payment/Add` **sobre una comanda facturada**, no la lectura |
| 5.4 Repetir el mismo `PaymentId` sin facturar | Dos veces `PAGO-A1`, 2,00 cada una, sobre la comanda 116 (4,20) | vacío las dos veces | **Duplica el cobro**: dos pagos de 2,00 con el mismo `PaymentId`, y `GetOrder` los devuelve como dos entradas. No hay idempotencia. El `[301201]` de 5.5 solo salta cuando la suma **excede** el total: con hueco, el duplicado entra |
| 5.4 bis `Orders/Create` duplicado: mismo `MarketplaceOrderId` | Dos `Create` idénticos, mismo `MarketplaceOrderId` `GPRUEBAT12`, 1 ud de 1,05, sin pagos | vacío las dos veces | **Crea dos comandas** (117 y 118), las dos con el mismo `MarketplaceOrderId` y las dos con su pago de 1,05. La comanda tampoco se deduplica por id de mercado |
| 5.4 ter `GetOrder` por `MarketplaceOrderId` con duplicados | `{MarketId: 9900, MarketplaceOrderId}` con 119 y 120 existiendo | vacío | Devuelve **una sola** (la 119) y oculta la otra. La consulta que usa la reconciliación no detecta el duplicado |
| 5.5 Pagar de más | 3,00 con 2,00 + 2,00 ya pagados | `[301201]-IMPORTE INCORRECTO` | Correcto |
| 5.6 Cuadrar exacto sin facturar y facturar después | 0,20 hasta 4,20; luego `Orders/Invoice` | vacío, `InvoiceNumber` `00001TB00000044` | Correcto |

**Para la aplicación:** `PaymentId` debe ser único por intento (nunca el `MarketplaceOrderId`, que se repite si se reenvía la comanda) y no se puede reintentar un pago con el mismo identificador: BDP lo cobraría dos veces. Y no se debe mezclar `Invoice: true` con un pago que no cuadre: devuelve error aunque el pago ya esté guardado, así que un reintento a ciegas duplicaría. Aplicado en `18AA-8`: `payment_id_intento` genera un id por armado y la reconciliación de un ambiguo lo busca en la auditoría, no por tender y monto.

## 6. Cancelar una comanda abierta

| Prueba | Enviado | Respuesta | Efecto |
|---|---|---|---|
| 6.1 Cancelar con un pago parcial de 1,00 encima (comanda 108) | `Orders/Cancel`, `PosId` 1, `OrderId` 108 | vacío | Cancelada. El pago parcial **desaparece sin rastro**: no hay abono ni documento |
| 6.1 bis `GetOrder` después | `OrderId` 108 | `[301000]-NO EXISTE NINGUNA COMANDA CON ESTE ID` | La comanda deja de existir por API |
| 6.1 ter Mesas tras cancelar | `Orders/Get` por mesa | `[301003]-NO EXISTE NINGUNA COMANDA EN LA MESA ESPECIFICADA` | Mesas 4, 5 y 8 libres |
| 6.2 Cancelar dos veces | Mismo `OrderId` | `[301000]-NO EXISTE NINGUNA COMANDA CON ESTE ID` | Correcto: idempotente en el resultado |
| 6.3 Cancelar sin pagos (comanda 109) | — | vacío | Igual: deja de existir |

**Consecuencia:** un pago parcial cobrado en efectivo o tarjeta y luego anulado con la comanda no deja documento fiscal. Es la segunda pregunta que Guillermo quería llevar a BDP.

---

## Lo que hay que preguntar a BDP

1. **Dividir y unir cuentas.** ¿Existe una operación oficial en WebLinkRestAPI 1.2? Si no, ¿qué procedimiento admiten (contrapartidas, varias comandas, mesa juntada)?
2. **Retirar un pago parcial de una comanda abierta** y **anular o devolver un cobro ya emitido.** ¿Hay operación no documentada o se hace en el TPV?
3. **`NotAllowedAsInvitation`** (añadido a la petición, no a la pregunta original): el campo existe en `Articles/Get` y en la ficha, `Orders/Create` lo ignora. ¿Es un defecto de la versión 36.2 o el campo solo lo aplica el terminal y no la API?
4. **Cambio de mesa**: procedimiento oficial, dado que el reenvío duplica la colocación y la fusión silenciosa en mesa ocupada.
5. **`GetOrder` de una comanda facturada** devuelve `{"Order": null, "Status": 3}`: la comanda deja de ser legible por la API, y `Payment/Add` sobre ella rompe con «El índice estaba fuera del intervalo…». ¿Es el comportamiento previsto? Afecta a cualquier consulta de estado posterior a la facturación, y a nuestro sondeo.
6. **Duplicados sin control**: `Orders/Create` no deduplica por `MarketplaceOrderId` (dos `Create` idénticos → dos comandas) y `GetOrder` por `MarketplaceOrderId` devuelve solo una, ocultando la otra. ¿Hay alguna garantía de unicidad del lado de BDP, o la debe sostener el integrador?

## Limpieza

Comandas 89 a 120 canceladas; mesas 1, 2, 3, 5, 7 y 8 libres al terminar. Artículo 1002 devuelto a `NotAllowedAsInvitation = false` por API y en la base del demo. La 116 se facturó y cerró.
