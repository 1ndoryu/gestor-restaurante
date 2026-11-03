# Sondas en el BDP demo: modificar comanda y mesas (02/11/2026)

Origen: puntos 1 y 3 del mensaje de Guillermo del 02/11. BDP demo del portátil (`127.0.0.1:8070`, servicio 84), mesa 6 de salón 1, artículo 1001 (COCA-COLA, 1,05 €). Sondas en `C:/tmp/probe-bdp` (no versionadas, solo escriben en el BDP demo).

## 1. Modificar una comanda existente: SÍ se puede (D05–D07)

`CreateOrder` con `OrderId` = comanda existente (y el mismo `MarketplaceOrderId`, salón y mesa) **añade las líneas enviadas a la misma comanda**. El manual solo lo bloquea en la versión gratuita (300041); con la 84 funciona.

| Prueba | Resultado |
|---|---|
| Crear con 1 línea (mesa 6) | OrderId 83, total 1,05 |
| Reenviar la comanda entera con una unidad más | **300033**: se interpreta como líneas nuevas, el total no cuadra |
| Enviar solo la línea nueva con `Total` = total final de la comanda (2,10) | OK, mismo OrderId 83, 2 líneas, total 2,10 |
| Enviar solo la línea nueva con `Total` = suma de la línea nueva (1,05) | 300033 |
| Línea con unidades negativas (-1) y `Total` final | OK: la línea queda como -1 ud / -1,05 y el total baja a 1,05 (anulación por contrapartida) |
| Línea nueva con comentario (`Comments`) | OK, el comentario se guarda en la línea |
| Cancelar con `/API/Orders/Cancel` | OK; la mesa vuelve a quedar libre |

Regla: **`Items` = solo las líneas añadidas; `Order.Total` = total final de la comanda** (total anterior + suma de las líneas enviadas). Cambiar unidades o quitar una línea = enviar una línea de ajuste (unidades negativas o positivas). Cambiar el texto de una línea existente no se puede: no hay forma de reescribirla.

Pendiente en la aplicación: calcular ese delta contra `GetOrder` y enviarlo (hoy `update_order` no lo hace así). No se ha probado aún que la línea negativa se vea bien en el TPV ni cómo la imprime cocina.

## 2. Mesa 1 rechazada: no era el tipo de comanda

El rechazo de la mesa 1 era **300033 (total no cuadra)**, no «tipo de comanda no válido». Causa: las mesas 1, 3 y 7 del demo ya tenían una comanda abierta (`Orders/Get` por salón y mesa: ids 33, 23 y 71). Crear sobre una mesa ocupada se trata como añadir líneas y el total no coincide. Las mesas 2, 4, 5, 6 y 8 estaban libres.

Con `OrderOperationType = 1` (OnlyCheck) todas las combinaciones pasan, incluso con un total incorrecto: **OnlyCheck no valida totales ni ocupación**, así que no sirve para predecir el 300033.

| Combinación (OnlyCheck) | Resultado |
|---|---|
| Type 0, salón 0, mesa 0 (barra) | OK |
| Type 2 (domicilio) | 300009: la aplicación no admite servicio a domicilio |
| Type 1, salón 1, mesas 1/5/7 | OK |
| Type 0 con salón 1 y mesa 5 | OK |
| Type 1, salón 0, mesa 0 | 300008: número de salón incorrecto |

Regla para la aplicación: antes de crear en una mesa, `GetOrder` por `RoomNumber` + `TableNumber`. Si existe comanda, añadir líneas con la regla de arriba (o avisar de que la mesa está ocupada); si no, crear. Pendiente de implementar y de mostrar un error comprensible.

## 3. GetOrder por mesa sí devuelve el contenido

`Orders/Get` con `RoomNumber` + `TableNumber` (o `OrderId`) devuelve líneas, unidades, precios y total en el demo. Es la base para detectar cambios hechos en el TPV sin aviso (02AB-2).
