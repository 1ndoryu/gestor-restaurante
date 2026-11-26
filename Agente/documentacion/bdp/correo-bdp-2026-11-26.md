BORRADOR — correo a BDP (soporte / integraciones), 2026-11-26. Sin enviar. Cuatro bloques de
preguntas sacados de los siete puntos de Guillermo del 25/11 y de las pruebas contra el demo 36.2.
Todo lo aquí descrito se ha reproducido contra la instalación demo o está citado del manual;
no hay nada supuesto salvo lo que se marca expresamente como hipótesis.

**Para:** `integraciones@bdpcenter.com` (Gael)
**CC:** `guillermo.autoia@gmail.com`
**Remitente:** Asley Crespo Navarro, autorizada por el distribuidor Guillermo García Sáez (n.º 4652)
**Estado:** borrador. **No enviado**: lo envía la usuaria, igual que el del 20/11 al que BDP
contestó el 24/11.

Asunto: Cuatro dudas de la integración por WebLinkRestAPI (demo 36.2): entrega, modificaciones, pagos y devoluciones

Hola:

Estamos terminando la integración de nuestra aplicación de sala con vuestro TPV a través de
WebLinkRestAPI, contra la instalación demo 36.2. Hemos cerrado casi todo; nos quedan cuatro
puntos donde la API no llega o su comportamiento no es el que esperábamos. Los describimos con
lo que hemos medido, para que no haya nada supuesto.

1. Servicio a domicilio (`Type = 2`)

Necesitamos crear desde nuestra aplicación pedidos a domicilio (`Type = 2`). El manual documenta
`Type = 2` en la estructura `Order`, la dirección en `OrderCustomer` y la hora de reparto en
`PreparationTime`; lo que no detalla es qué necesitamos para tenerlo en marcha. En la respuesta
del 24/11 se nos dijo que hace falta otra aplicación de BDP y probablemente otra licencia.

- ¿Qué aplicación y qué suscripción de desarrollo (o licencia) hacen falta?
- ¿Hay que sustituir la aplicación 01 Hostelería por Servicio a Domicilio, o conviven?
- ¿La licencia WebLinkRestAPI 84 que usamos ahora sigue siendo válida para esa aplicación?
- ¿Cómo lo instalamos en nuestro entorno de pruebas?
- ¿El cambio conserva los datos y la configuración actuales (artículos, mesas, clientes, series
  de facturación)?

2. Modificaciones de comandas (#SubOrder / modificación)

Al modificar una comanda abierta observamos dos comportamientos distintos en el demo:

- Comandas de barra (`Type = 0`): `SubOrder`/`Modify` contesta 200 con `ErrorMessage` vacío y
  la comanda queda exactamente igual. No es un rechazo: acepta y no aplica.
- Comandas de mesa (`Type = 1`): el cambio sí se aplica. Si el `Total` enviado no coincide con
  el teórico, responde `[300033] EL IMPORTE TOTAL INDICADO DE LA COMANDA NO COINCIDE CON EL
  TEÓRICO`.

Reproducido con las comandas 222, 223, 225 y 227 del demo. Nuestra hipótesis es que es un
límite de la licencia demo, pero no la damos por buena sin confirmación.

- ¿El demo tiene limitada la modificación de comandas de barra?
- ¿En un TPV con licencia completa, Hostelería 36.2 permite modificar comandas de barra por
  WebLinkRestAPI, y cuál es el procedimiento correcto?
- ¿`[300033]` es solo una validación del total, y basta con enviar el total recalculado?

3. Pagos parciales

Confirmado vuestro criterio de dividir la cuenta en nuestra aplicación y enviar pagos parciales
hasta darla por pagada. El manual dice «se podrán especificar un máximo de 3 pagos, igual que en
el TPV táctil de BDP-Net» y nuestra aplicación corta en tres por eso.

- ¿Ese máximo de tres es el mismo en `AddOrderPayment` sucesivos que en la lista `Payments` de
  `CreateOrder`, o sólo aplica al alta?
- ¿`AddOrderPayment` (`/API/Orders/Payment/Add`) admite oficialmente más de tres pagos sucesivos,
  o ese límite es firme? Si lo admite, ¿por qué vía: la misma llamada, la consola, el TPV, otra
  aplicación?
- ¿`AddOrderPayment` tiene alguna condición de cierre de la comanda que debamos conocer (por
  ejemplo, que el último pago deba igualar exactamente el saldo)?

4. Devoluciones y pagos negativos

Para una devolución con tarjeta, nuestra idea es devolver por la pasarela y anotar en BDP un
pago negativo del mismo importe, ya que no hay operación de retirada de cobro en la API. El
manual documenta `Amount` en `OrderPayment` sin decir que pueda ser negativo.

- ¿`AddOrderPayment` acepta un importe negativo? ¿Lo admite sobre una comanda ya facturada o
  cerrada?
- Sobre qué comanda se registra: ¿la misma ya facturada, o hace falta una comanda nueva?
- ¿Qué documento genera (rectificativa, abono, nada)?
- Si no lo acepta, ¿qué procedimiento autorizáis para dejar registrada una devolución en el TPV?

Un apunte sobre el entorno de pruebas: sigue apareciendo el aviso de «sin conexión con el
servidor de subscripción» y el TPV funciona con él. Nuestras pruebas se han hecho todas así.
Sigue pendiente de respuesta la consulta del 23/11 sobre la vigencia de esa licencia demo.

Gracias.
