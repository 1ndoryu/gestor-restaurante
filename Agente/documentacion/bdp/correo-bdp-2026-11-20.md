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

1. Dividir y unir cuentas: ¿hay operación en la API, o se hace en el terminal?

2. Retirar un pago parcial y anular un cobro ya emitido: ¿alguna operación, o solo en el terminal?

3. NotAllowedAsInvitation: `Orders/Create` lo ignora e invita el artículo igual. ¿Es lo previsto, o solo lo aplica el terminal?

4. Cambio de mesa: no hay operación; reenviar con la mesa nueva no libera la de origen. ¿Cuál es el procedimiento?

5. `GetOrder` de una comanda facturada devuelve `{"Order": null}` y `Payment/Add` sobre ella da «El índice estaba fuera del intervalo». ¿Es lo previsto?

6. Servicio a domicilio (`Type=2`): Hostelería 01 responde `[300009]` siempre. ¿Qué aplicación y suscripción hacen falta para operarlo?

Y un fallo: `Orders/Create` acepta una línea con `Units: 0` y el total no cambia, pero después `Orders/Payment/Add` con `Invoice: true` responde «Intento de dividir por cero.» y no factura. La comanda queda cobrada sin documento y el mensaje no dice qué línea es. Sin esa línea, la misma comanda factura bien.

Un par de cosas que ya comprobamos, por si ahorran tiempo: quitar una línea exige enviar `Units: -1` (no se borra), y `PaymentId` no es idempotente: repetirlo con hueco en el total cobra dos veces.

Si conviene, preparo un guion con las peticiones exactas de cada punto.

Gracias y un saludo,

Asley.
