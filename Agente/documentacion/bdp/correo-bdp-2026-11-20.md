# Borrador del correo a BDP (18AB-9) — 20/11/2026

**Para:** `bdp@bdpcenter.com` (Gael)
**CC:** `guillermo.autoia@gmail.com`
**Remitente:** Asley Crespo Navarro, autorizada por el distribuidor Guillermo García Sáez (n.º 4652)
**Estado:** borrador. **No enviado**: lo envía la usuaria.

Cuerpo listo para copiar (20/11):

---

Asunto: WebLinkRestAPI 1.2 — dudas de comportamiento de la API tras probar la integración

Buenos días:

Soy Asley Crespo Navarro, autorizada por el distribuidor Guillermo García Sáez (n.º 4652), y estoy integrando el TPV del cliente con BDP mediante WebLinkRestAPI 1.2 (BDP-Net 36.2, aplicación 01 Hostelería, servicio 84). Todo lo que sigue está probado sobre un BDP de demostración, nunca sobre el TPV del cliente, y las comandas se han cancelado o facturado al terminar.

Planteamos seis dudas y cinco hechos que ya hemos comprobado por nuestra cuenta, para que no haga falta reproducirlos.

1. Dividir y unir cuentas: ¿hay una operación oficial en la API? Si no, ¿qué procedimiento admiten: contrapartidas, varias comandas, juntar mesas?

2. Retirar un pago parcial de una comanda abierta y anular o devolver un cobro ya emitido: ¿hay alguna operación, documentada o no, o se hace obligatoriamente en el terminal?

3. `NotAllowedAsInvitation`: el campo existe en la ficha del artículo y en `Articles/Get`, pero `Orders/Create` lo ignora y acepta invitar un artículo marcado como no invitable con `Invitation: true` y total 0. Además, `Articles/ModifyAndUpdateProfiles` acepta el campo y lo devuelve en `Articles/Get`, pero no lo escribe en la base: queda en una caché del servicio. ¿Es el comportamiento previsto en la 36.2, o el campo solo lo aplica el terminal y no la API?

4. Cambio de mesa: no encontramos operación en la API. Reenviar la comanda con `OrderId` y la mesa nueva no libera la mesa de origen, y contra una mesa ocupada las líneas se fusionan en la comanda del destino sin aviso. ¿Cuál es el procedimiento oficial?

5. `GetOrder` de una comanda facturada devuelve `{"Order": null, "Status": 3}` y `Orders/Payment/Add` sobre una facturada responde «El índice estaba fuera del intervalo». ¿Es el comportamiento previsto? Nos afecta a cualquier consulta de estado posterior a la facturación y a nuestro sondeo periódico.

6. Comandas de servicio a domicilio (`Type=2`): en la aplicación 01 Hostelería, `Orders/Create` responde siempre `[300009]-LA APLICACIÓN NO ADMITE COMANDAS DE SERVICIO A DOMICILIO`, con y sin `Customer` relleno y también en `OnlyCheck`. El manual dice que esas comandas solo se crean en la aplicación Servicio a Domicilio. Si el restaurante va a vender a domicilio, ¿qué aplicación y qué suscripción necesitamos para probarlo y para operarlo?

Hechos ya comprobados por nuestra parte:

1. Quitar una línea no la borra: hay que enviar una línea con `Units: -1` y el total final de la comanda. La contrapartida aparece en el ticket con importe negativo.

2. `Items` lleva solo las líneas del cambio y `Order.Total` el total final de la comanda; no es un reemplazo del contenido.

3. El comentario de una línea no se puede reescribir: reenviar la misma `Lin` con otro texto y `Units: 0` añade una línea nueva y deja la vieja.

4. `PaymentId` no es idempotente: repetir `Orders/Payment/Add` con el mismo `PaymentId` y hueco en el total cobra dos veces, y el `[301201]` solo salta cuando la suma excede el total. Además, `Invoice: true` en un pago parcial se rechaza con `[301301]` aunque el pago sí se guarda.

5. `Orders/Create` no deduplica por `MarketplaceOrderId`: dos peticiones idénticas crean dos comandas con el mismo `MarketId` + `MarketplaceOrderId`, y `GetOrder` por ese par devuelve solo una. ¿Hay alguna garantía de unicidad del lado de BDP, o la debe sostener el integrador?

Un fallo que creemos que conviene corregir: `Orders/Create` acepta una línea con `Units: 0`, también añadida a una comanda que ya tiene líneas válidas, y el total no cambia. Después, al cobrar con `Orders/Payment/Add` e `Invoice: true`, BDP responde «Intento de dividir por cero.» y no factura, de modo que la comanda queda cobrada pero sin documento. El mensaje no menciona la línea, así que desde el terminal no se sabe qué corregir; con la misma sesión, una comanda sin esa línea factura correctamente. Entendemos que una línea de 0 unidades debería rechazarse en `Orders/Create`, o al menos no impedir la facturación.

Detalle de las pruebas: BDP de demostración, BDP-Net 36.2, aplicación 01 Hostelería, WebLinkRestAPI 1.2, terminal 1, caja abierta, salón 1 mesas 4 a 8 y artículos 1001 y 1002 del catálogo de prueba. Todas las comandas quedaron canceladas o facturadas al terminar; ninguna prueba sobre el TPV del cliente ni sobre producción.

Quedamos a la espera de vuestra respuesta. Si conviene, podemos preparar un guion de reproducción con las peticiones exactas de cada punto.

Gracias y un saludo,

Asley.

---

## Notas de envío (para la usuaria, fuera del correo)

- Los seis puntos de las dudas son los que fijó Guillermo el 18/11, más la cuestión de qué aplicación hace falta para `Type=2` (que él mismo planteó en el punto 7).
- Los cinco hechos van con la evidencia del 18/11 (`Agente/documentacion/bdp/pruebas-comandas-guillermo-2026-11-18.md`) y del 20/11 (`ticket-lineas-comentarios-2026-11-20.md`, `tipo2-domicilio-hosteleria-2026-11-20.md`).
- **Dirección:** `bdp@bdpcenter.com`, de donde responde Gael. Está registrada también en `problema-instalacion-sql-2026-09-30.md:191`.
- **Firma:** la del contacto habitual con BDP, Asley Crespo Navarro, autorizada por Guillermo García Sáez (n.º 4652). Así BDP no tiene que comprobar quién pregunta.
- Correo nuevo, no continuación del hilo de septiembre: la cuestión de versión ya está resuelta y no se menciona.
- No se menciona ninguna credencial, ni el nombre del restaurante, ni importes reales.
