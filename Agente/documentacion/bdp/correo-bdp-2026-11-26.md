BORRADOR — correo a BDP (soporte / integraciones), 2026-11-26. Sin enviar. Cuatro preguntas
sacadas de los siete puntos de Guillermo del 25/11 y de las pruebas contra el demo 36.2.
Todo lo aquí descrito se ha reproducido contra la instalación demo; no hay nada supuesto
salvo lo que se marca expresamente como hipótesis.

Asunto: Cuatro dudas de la integración por WebLinkRestAPI (demo 36.2): entrega, modificaciones, pagos y devoluciones

Hola:

Estamos terminando la integración de nuestra aplicación de sala con vuestro TPV a través de
WebLinkRestAPI, contra la instalación demo 36.2. Hemos cerrado casi todo; nos quedan cuatro
puntos donde la API no llega o su comportamiento no es el que esperábamos. Los describimos con
lo que hemos medido, para que no haya nada supuesto.

1. Servicio a domicilio (`Type = 2`)

Necesitamos crear comandas de reparto. En la estructura `Order` que documenta la API, `Type`
admite 0 (barra) y 1 (mesa); no encontramos descrito el valor 2 ni una operación para el
servicio a domicilio. En la respuesta del 24/11 se nos dijo que eso necesita otra aplicación de
BDP y probablemente otra licencia.

- ¿Qué aplicación y qué licencia son?
- ¿Hay alguna vía por WebLinkRestAPI para crear una comanda de reparto con dirección y cliente?

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
- ¿En un TPV con licencia completa funciona igual que en las de mesa?
- ¿`[300033]` es solo una validación del total, y basta con enviar el total recalculado?

3. Pagos parciales

Confirmado vuestro criterio de dividir la cuenta en nuestra aplicación y enviar pagos parciales
hasta darla por pagada. Nuestra aplicación corta en tres pagos por cuenta (regla nuestra, no
vuestra).

- ¿Cuántos pagos parciales admite una comanda en BDP? ¿Hay un máximo o un límite por importe
  mínimo?
- ¿`OrderPayment/Add` tiene alguna condición de cierre de la comanda que debamos conocer (por
  ejemplo, que el último pago deba igualar exactamente el saldo)?

4. Devoluciones y pagos negativos

Para una devolución con tarjeta, nuestra idea es devolver por la pasarela y anotar en BDP un
pago negativo del mismo importe, ya que no hay operación de retirada de cobro en la API.

- ¿`OrderPayment/Add` acepta un importe negativo? ¿Lo admite sobre una comanda ya facturada o
  cerrada?
- Si no lo acepta, ¿qué procedimiento autorizáis para dejar registrada una devolución en el TPV?
  Rectificativa, anulación con la comanda ya facturada, u otro.

Un apunte sobre el entorno de pruebas: sigue apareciendo el aviso de «sin conexión con el
servidor de subscripción» y el TPV funciona con él. Nuestras pruebas se han hecho todas así.
Sigue pendiente de respuesta la consulta del 23/11 sobre la vigencia de esa licencia demo.

Gracias.
