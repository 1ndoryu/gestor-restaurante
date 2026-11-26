BORRADOR — respuesta al correo de Gael del 24/11/2026. Sin enviar; lo envía la usuaria.

**De:** Wan <andoryyu@gmail.com>
**Para:** `integraciones@bdpcenter.com`
**CC:** `guillermo.autoia@gmail.com`, `gael@bdpcenter.com`
**Asunto:** RE: WebLinkRestAPI 1.2 — seis dudas y un fallo de facturación
**Estado:** borrador. **No enviado.**

Va como respuesta al hilo del 20/11, no como correo nuevo: Gael pidió que usáramos
`integraciones@` y nos devolvió tres preguntas, que se contestan aquí. Versión corta: de las 17
preguntas que llegó a tener, quedan 7, las que condicionan el trabajo. El delivery va solo como
alcance para después, porque la usuaria y Guillermo acordaron aparcarlo.

---

Hola, Gael:

Gracias por la respuesta, que aclara casi todo. Antes de las dudas que nos quedan, contesto a las
tres preguntas que nos hacías.

*Quién usa la aplicación.* Está pensada para el personal del restaurante, no para el cliente final:
es el personal quien anota el pedido, gestiona la cuenta y cobra, con el cliente delante como hasta
ahora. Por eso las invitaciones y el cambio de mesa los trata el personal y no el comensal; la
restricción de invitación se aplica en nuestra interfaz, que no deja marcar como invitado un
artículo que BDP no permite invitar.

*Qué hace y cómo interactúa con BDP.* Es la aplicación de sala: plano de mesas, comandas, cuentas,
cobros y consulta de la venta del día. El TPV de BDP sigue siendo el dueño de los datos y el que
factura: la web envía comandas y pagos por WebLinkRestAPI y lee del TPV el estado de cada comanda y
el catálogo. No escribimos nunca directamente en la base de datos de BDP.

*Confirmado: trabajamos sobre BDP Net Hostelería.* Es la aplicación que tiene el restaurante, y la
integración la hemos desarrollado contra la demo 36.2.

Cuatro dudas:

1. Modificaciones de comandas de barra

En la demo, modificar una comanda de barra (`Type = 0`) contesta 200 con el mensaje de error vacío y
la comanda queda igual; las de mesa (`Type = 1`) sí se aplican. Lo hemos reproducido con las
comandas 222, 223, 225 y 227.

¿Hostelería 36.2 con licencia completa permite modificar comandas de barra por WebLinkRestAPI? Si es
así, ¿cuál es el procedimiento correcto?

2. Pagos parciales

Seguimos vuestro criterio: dividir la cuenta en nuestra aplicación y enviar los pagos parciales. El
manual fija un máximo de tres pagos y cortamos ahí.

¿`AddOrderPayment` (`/API/Orders/Payment/Add`) admite oficialmente más de tres pagos sucesivos sobre
la misma comanda, o ese límite es firme?

3. Devoluciones y pagos negativos

Nos proponéis devolver por la pasarela y registrar en BDP un pago negativo por el mismo importe, pero
el manual no dice que el importe pueda ser negativo.

¿Está admitido oficialmente? Cuando la comanda ya está facturada, ¿sobre qué comanda se registra y
qué documento genera? Si no lo está, ¿qué procedimiento autorizáis para dejar registrada una
devolución?

4. Servicio a domicilio (`Type = 2`)

Por ahora lo dejamos aparcado, como acordamos con el distribuidor. Solo para saber el alcance cuando
lo abordemos:

- ¿Qué suscripción o licencia de Servicio a Domicilio haría falta, y sustituye a Hostelería o
  conviven las dos?
- Al hacer el cambio, ¿se conservan los datos y la configuración actuales (artículos, mesas,
  clientes, series de facturación)?
- La licencia WebLinkRestAPI 84 que usamos ahora, ¿sigue siendo válida para esa aplicación?

Un último apunte: sigue apareciendo el aviso «sin conexión con el servidor de subscripción» y el TPV
funciona con él. ¿Esa licencia demo sigue vigente y hasta cuándo?

Si es más rápido, os llamamos en la franja de 9:00 a 11:00.

Gracias.

Un saludo,
Asley Crespo Navarro
Autorizada por el distribuidor Guillermo García Sáez (n.º 4652)
