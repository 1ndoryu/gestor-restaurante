# Borrador del correo a BDP (18AB-9) — 20/11/2026

**Para:** `bdp@bdpcenter.com` (Gael)
**CC:** `guillermo.autoia@gmail.com`
**Remitente:** Asley Crespo Navarro, autorizada por el distribuidor Guillermo García Sáez (n.º 4652)
**Estado:** borrador. **No enviado**: lo envía la usuaria.

Cuerpo listo para copiar (20/11):

---

Asunto: WebLinkRestAPI 1.2 — seis dudas y un fallo de facturación

Buenos días:

Soy Asley Crespo Navarro, autorizada por el distribuidor Guillermo García Sáez (n.º 4652). Integro el TPV de un restaurante con WebLinkRestAPI 1.2 (BDP-Net 36.2, aplicación 01 Hostelería) y todo está probado sobre el BDP de demostración, nunca sobre el TPV del cliente.

Seis dudas:

1. Dividir una cuenta en varias y unir varias en una: ¿hay operación en la API, o hay que hacerlo en el terminal? Si no existe, ¿qué procedimiento admiten?

2. Retirar un pago parcial ya metido en una comanda abierta y anular un cobro ya emitido: ¿hay alguna operación, o se hace solo desde el terminal?

3. `NotAllowedAsInvitation`: el artículo está marcado como no invitable en la ficha y `Articles/Get` lo devuelve, pero `Orders/Create` lo ignora y acepta `Invitation: true` con total 0. ¿Es lo previsto, o el campo solo lo aplica el terminal?

4. Cambio de mesa: no encontramos operación. Reenviar la comanda con el `OrderId` y la mesa nueva la coloca en las dos y no libera la de origen. ¿Cuál es el procedimiento oficial?

5. Consultar una comanda ya facturada: `GetOrder` devuelve `{"Order": null, "Status": 3}` y `Orders/Payment/Add` sobre ella responde «El índice estaba fuera del intervalo». ¿Es lo previsto? Lo necesitamos para consultar el estado después de cobrar.

6. Servicio a domicilio (`Type=2`): en la aplicación 01 Hostelería responde siempre `[300009]-LA APLICACIÓN NO ADMITE COMANDAS DE SERVICIO A DOMICILIO`, con y sin `Customer`. El manual dice que solo se crean en la aplicación Servicio a Domicilio: ¿qué aplicación y suscripción hacen falta para operarlo?

Y un fallo: `Orders/Create` acepta una línea con `Units: 0` y el total no cambia, pero después `Orders/Payment/Add` con `Invoice: true` responde «Intento de dividir por cero.» y no factura. La comanda queda cobrada sin documento y el mensaje no dice qué línea es. Sin esa línea, la misma comanda factura bien.

Un par de cosas que ya comprobamos, por si ahorran tiempo: quitar una línea exige enviar `Units: -1` (no se borra), y `PaymentId` no es idempotente: repetirlo con hueco en el total cobra dos veces.

Si conviene, preparo un guion con las peticiones exactas de cada punto.

Gracias y un saludo,

Asley.
