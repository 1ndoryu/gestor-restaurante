BORRADOR — respuesta al correo de Gael del 24/11/2026. Sin enviar; lo envía la usuaria.

**De:** Wan <andoryyu@gmail.com>
**Para:** `integraciones@bdpcenter.com`
**CC:** `guillermo.autoia@gmail.com`, `gael@bdpcenter.com`
**Asunto:** RE: WebLinkRestAPI 1.2 — seis dudas y un fallo de facturación
**Estado:** borrador. **No enviado.**

Va como respuesta al hilo del 20/11, no como correo nuevo: Gael pidió en su respuesta que
usáramos `integraciones@` y nos devolvió tres preguntas que hay que contestar antes de las nuevas.

---

Hola, Gael:

Gracias por la respuesta, que aclara casi todo. Antes de las cuatro dudas que nos quedan, contesto
a las tres preguntas que nos hacías, porque de ellas dependen algunas respuestas.

*Quién usa la aplicación.* Está pensada para el personal del restaurante, no para el cliente final:
es el personal quien anota el pedido, gestiona la cuenta y cobra, con el cliente delante como hasta
ahora. Por eso las invitaciones y el cambio de mesa los trata el personal y no el comensal.

*Qué hace y cómo interactúa con BDP.* Es la aplicación de sala del restaurante: plano de mesas,
comandas, cuentas, cobros y consulta de la venta del día. El TPV de BDP sigue siendo el dueño de los
datos y el que factura: la web envía comandas y pagos por WebLinkRestAPI y lee del TPV el estado de
cada comanda y el catálogo. No escribimos nunca directamente en la base de datos de BDP.

*Confirmado: trabajamos sobre BDP Net Hostelería.* Es nuestra instalación, y la integración se ha
desarrollado contra la demo 36.2.

Ahora, los cuatro puntos:

1. Servicio a domicilio (`Type = 2`)

Necesitamos crear desde la web pedidos a domicilio. El manual documenta `Type = 2` en la estructura
`Order`, la dirección en `OrderCustomer` y la hora de reparto en `PreparationTime`, y nos decís que
la aplicación es BDP Servicio a Domicilio. Nos quedan estas dudas:

- ¿Qué suscripción de desarrollo (o licencia) hacen falta para poder probarlo?
- ¿Hay que sustituir la aplicación 01 Hostelería por Servicio a Domicilio, o conviven las dos?
- ¿La licencia WebLinkRestAPI 84 que usamos ahora sigue siendo válida para esa aplicación?
- ¿Cómo lo instalamos en nuestro entorno de pruebas, que es la demo 36.2?
- El cambio, ¿conserva los datos y la configuración actuales (artículos, mesas, clientes, series
  de facturación), o hay que volver a darlos de alta?

2. Modificaciones de comandas de barra

Al modificar una comanda abierta observamos dos comportamientos distintos en la demo:

- Comandas de barra (`Type = 0`): `SubOrder`/`Modify` contesta 200 con `ErrorMessage` vacío y la
  comanda queda exactamente igual. No es un rechazo: acepta y no aplica.
- Comandas de mesa (`Type = 1`): el cambio sí se aplica. Si el `Total` enviado no coincide con el
  teórico, responde `[300033] EL IMPORTE TOTAL INDICADO DE LA COMANDA NO COINCIDE CON EL TEÓRICO`.

Reproducido con las comandas 222, 223, 225 y 227 de la demo.

- ¿El entorno demo tiene limitada la modificación de comandas de barra?
- ¿En un TPV con licencia completa, Hostelería 36.2 permite modificar comandas de barra por
  WebLinkRestAPI, y cuál es el procedimiento correcto?
- ¿`[300033]` es solo una validación del total, y basta con enviar el total recalculado?

3. Pagos parciales

Seguimos el criterio que nos disteis: dividir la cuenta en nuestra aplicación y enviar los pagos
parciales. El manual dice «se podrán especificar un máximo de 3 pagos, igual que en el TPV táctil
de BDP-Net», y nuestra aplicación corta en tres por eso.

- ¿Ese máximo de tres es el mismo en `AddOrderPayment` sucesivos que en la lista `Payments` de
  `CreateOrder`, o sólo aplica al alta?
- ¿`AddOrderPayment` (`/API/Orders/Payment/Add`) admite oficialmente más de tres pagos sucesivos,
  o ese límite es firme?
- ¿`AddOrderPayment` tiene alguna condición de cierre de la comanda que debamos conocer (por
  ejemplo, que el último pago deba igualar exactamente el saldo)?

4. Devoluciones y pagos negativos

Nos proponéis devolver por la pasarela y registrar en BDP un pago negativo por el mismo importe. El
manual documenta `Amount` en `OrderPayment` sin decir que pueda ser negativo, y no queremos
desarrollarlo sin saber si está admitido.

- ¿`AddOrderPayment` acepta un importe negativo? ¿Lo admite sobre una comanda ya facturada o
  cerrada?
- ¿Sobre qué comanda se registra: la misma ya facturada, o hace falta una comanda nueva?
- ¿Qué documento genera (rectificativa, abono, nada)?
- Si no lo acepta, ¿qué procedimiento autorizáis para dejar registrada una devolución en el TPV?

Un apunte sobre el entorno de pruebas: sigue apareciendo el aviso «sin conexión con el servidor de
subscripción» y el TPV funciona con él; todas nuestras pruebas se han hecho así. La consulta sobre
la vigencia de esa licencia demo la mandamos el 23/11 y sigue sin respuesta. Si es más rápido por
teléfono, os llamamos en la franja de 9:00 a 11:00.

Gracias.

Un saludo,
Asley Crespo Navarro
Autorizada por el distribuidor Guillermo García Sáez (n.º 4652)
