# Borrador del correo a BDP (18AB-9) — 20/11/2026

**Para:** soporte de BDP (dirección habitual de soporte técnico)
**CC:** `guillermo.autoia@gmail.com`
**Estado:** borrador. **No enviado**: lo envía la usuaria.

Cuerpo listo para copiar:

---

Asunto: WebLinkRestAPI 1.2 — dudas sobre comandas abiertas, facturación y servicio a domicilio

Buenos días:

Somos Nakomi Studio y estamos integrando el TPV de un restaurante con BDP a través de
WebLinkRestAPI 1.2 (BDP-Net 36.2, aplicación 01 Hostelería, servicio 84). Todas las
pruebas están hechas sobre un BDP de demostración, nunca sobre el TPV del cliente, y
las comandas se han cancelado o facturado al terminar.

Planteamos seis dudas funcionales y cinco hechos que ya hemos comprobado por nuestra
cuenta, para que no haga falta reproducirlos.

## Dudas

1. **Dividir y unir cuentas.** ¿Existe una operación oficial en WebLinkRestAPI 1.2?
   Si no, ¿qué procedimiento admiten: contrapartidas, varias comandas, juntar mesas?

2. **Retirar un pago parcial de una comanda abierta** y **anular o devolver un cobro ya
   emitido.** ¿Hay alguna operación, documentada o no, o se hace obligatoriamente en el
   terminal?

3. **`NotAllowedAsInvitation`.** El campo existe en la ficha del artículo y en
   `Articles/Get`, pero `Orders/Create` lo ignora: invitar un artículo marcado como no
   invitable se acepta con `Invitation: true` y total 0. ¿Es el comportamiento previsto
   en la 36.2, o el campo solo lo aplica el terminal y no la API? Además,
   `Articles/ModifyAndUpdateProfiles` acepta el campo y lo devuelve en `Articles/Get`,
   pero **no lo escribe en la base**: queda en una caché del servicio.

4. **Cambio de mesa.** No encontramos operación de cambio de mesa en la API. Reenviar la
   comanda con `OrderId` y la mesa nueva **no libera la mesa de origen**: la comanda
   queda colocada en las dos. Y contra una mesa **ocupada**, si el total que enviamos
   coincide con el teórico, las líneas se **fusionan en la comanda del destino** sin
   aviso (las dos comandas siguen existiendo). ¿Cuál es el procedimiento oficial?

5. **`GetOrder` de una comanda facturada** devuelve `{"Order": null, "Status": 3}`: la
   comanda deja de ser legible por la API. Además, `Orders/Payment/Add` sobre una
   comanda ya facturada responde «El índice estaba fuera del intervalo…». ¿Es el
   comportamiento previsto? Afecta a cualquier consulta de estado posterior a la
   facturación y a nuestro sondeo periódico.

6. **Comandas de servicio a domicilio (`Type=2`).** En la aplicación 01 Hostelería,
   `Orders/Create` con `Type=2` responde siempre
   `[300009]-LA APLICACIÓN NO ADMITE COMANDAS DE SERVICIO A DOMICILIO`, con y sin
   `Customer` relleno y también en `OnlyCheck`. El manual (`CreateOrder`) dice que esas
   comandas solo se crean en la aplicación **Servicio a Domicilio**. Si el restaurante
   va a vender a domicilio, ¿qué aplicación y qué suscripción necesitamos para
   probarlo y para operarlo?

## Hechos ya comprobados por nuestra parte

1. **Quitar una línea no la borra**: hay que enviar una línea con `Units: -1` y el total
   final de la comanda. La contrapartida aparece en el ticket con importe negativo.
2. **`Items` lleva solo las líneas del cambio** y `Order.Total`, el total final de la
   comanda. No es un reemplazo del contenido.
3. **El comentario de una línea no se puede reescribir.** Reenviar la misma `Lin` con
   otro texto y `Units: 0` añade una línea nueva y deja la vieja: quedan las dos. Lo
   único que deja el ticket coherente es la contrapartida de −1 **con el comentario
   viejo** más el alta con el nuevo.
4. **`PaymentId` no es idempotente.** Repetir `Orders/Payment/Add` con el mismo
   `PaymentId` y hueco en el total **cobra dos veces**; el `[301201]` solo salta cuando
   la suma excede el total. Y `Invoice: true` en un pago parcial se rechaza con
   `[301301]` **aunque el pago sí se guarda**, así que un reintento a ciegas duplicaría.
5. **`Orders/Create` no deduplica por `MarketplaceOrderId`**: dos peticiones idénticas
   crean dos comandas con el mismo `MarketId` + `MarketplaceOrderId`. Y `GetOrder` por
   ese par devuelve **solo una**, ocultando la otra. ¿Hay alguna garantía de unicidad
   del lado de BDP, o la debe sostener el integrador?

## Un fallo que creemos que conviene corregir

`Orders/Create` **acepta** una línea con `Units: 0` (también añadida a una comanda que ya
tiene líneas válidas) y el total no cambia. Después, al cobrar con
`Orders/Payment/Add` e `Invoice: true`, BDP responde **«Intento de dividir por cero.»** y
**no factura**. La comanda queda cobrada pero sin documento, y el mensaje no menciona la
línea, así que desde el terminal no se sabe qué corregir. Con la misma sesión, una
comanda sin esa línea factura correctamente. Entendemos que una línea de 0 unidades
debería rechazarse en `Orders/Create`, o al menos no impedir la facturación.

## Detalle de las pruebas

- BDP de demostración: BDP-Net 36.2, aplicación `01` Hostelería, WebLinkRestAPI 1.2
  (`/Service/GetApplicationVersion`), terminal 1, caja abierta.
- Artículos del catálogo de prueba: 1001 COCA-COLA 1,05 €, 1002 SCHWEPPES LIMÓN 1,20 €.
- Salón 1, mesas 4 a 8. Todas las comandas canceladas o facturadas al terminar; ninguna
  prueba sobre el TPV del cliente ni sobre producción.

Quedamos a la espera de vuestra respuesta. Si conviene, podemos preparar un guion de
reproducción con las peticiones exactas de cada punto.

Gracias y un saludo,

Nakomi Studio

---

## Notas de envío (para la usuaria, fuera del correo)

- Los seis puntos de las dudas son los que fijó Guillermo el 18/11, más la cuestión de
  qué aplicación hace falta para `Type=2` (que él mismo planteó en el punto 7).
- Los cinco hechos van con la evidencia del 18/11
  (`Agente/documentacion/bdp/pruebas-comandas-guillermo-2026-11-18.md`) y del 20/11
  (`ticket-lineas-comentarios-2026-11-20.md`, `tipo2-domicilio-hosteleria-2026-11-20.md`).
- Falta la dirección de soporte: no está en el repo ni en el `.env`. Se pone al enviar.
- No se menciona ninguna credencial, ni el nombre del restaurante, ni importes reales.
