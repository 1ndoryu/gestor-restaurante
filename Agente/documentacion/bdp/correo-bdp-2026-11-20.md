# Borrador del correo a BDP (18AB-9) — 20/11/2026

**Para:** `bdp@bdpcenter.com` (Gael)
**CC:** `guillermo.autoia@gmail.com`
**Hilo:** continuación del de 22-23/09/2026 sobre entorno de desarrollo y versiones
**Firma:** Asley Crespo Navarro, autorizada por el distribuidor Guillermo García Sáez (n.º 4652)
**Estado:** borrador. **No enviado**: lo envía la usuaria.

Cuerpo listo para copiar (versión corta, 20/11):

---

Asunto: RE: Entorno de desarrollo BDP-NET — dudas de comportamiento de la API tras probar la integración

Buenos días, Gael:

Sigo con la integración del cliente de la conversación de septiembre, ya sobre el entorno de
demostración. Antes de seguir, una cuestión de versión que nos condiciona todo lo demás.

**Versión.** Nos dijisteis que la 36.2 ya no está disponible para descarga y que se desarrollara
en la 37.5, que además es la primera adaptada a VeriFactu y que la 36 no servirá cuando entre en
vigor la obligatoriedad. Nuestro entorno de pruebas y el del cliente están en 36.2
(BDP-Net 36.2, aplicación 01 Hostelería, WebLinkRestAPI 1.2). ¿Confirmáis que la integración debe
apuntar ya a la 37.x? Y si es así, ¿podemos disponer de un entorno 37.5 con Hostelería y
WebLinkRestAPI para validar antes de tocar el del cliente? Los puntos que siguen los hemos
reproducido en 36.2; decidnos cuáles están corregidos en la 37.5 y los volvemos a probar allí.

Seis dudas de comportamiento:

1. Dividir y unir cuentas: ¿hay operación oficial en la API? Si no, ¿qué procedimiento admiten?

2. Retirar un pago parcial de una comanda abierta y anular o devolver un cobro ya emitido:
   ¿hay alguna operación, o se hace obligatoriamente en el terminal?

3. NotAllowedAsInvitation: el campo está en la ficha del artículo y en Articles/Get, pero
   Orders/Create lo ignora (acepta invitar el artículo, con total 0). Además
   Articles/ModifyAndUpdateProfiles lo acepta y lo devuelve, pero no lo escribe en la base:
   queda en una caché del servicio. ¿Es lo previsto, o el campo solo lo aplica el terminal?

4. Cambio de mesa: no encontramos operación. Reenviar la comanda con OrderId y la mesa nueva
   no libera la de origen, y contra una mesa ocupada las líneas se fusionan en la comanda del
   destino sin aviso. ¿Cuál es el procedimiento oficial?

5. GetOrder de una comanda facturada devuelve {"Order": null, "Status": 3}, y Payment/Add
   sobre una facturada responde «El índice estaba fuera del intervalo». Nos afecta a cualquier
   consulta posterior a la facturación y a nuestro sondeo periódico. ¿Es lo previsto?

6. Servicio a domicilio (Type=2): en la aplicación 01 Hostelería responde siempre
   [300009]-LA APLICACIÓN NO ADMITE COMANDAS DE SERVICIO A DOMICILIO, con y sin Customer
   relleno y también en OnlyCheck. El manual dice que solo se crean en la aplicación Servicio
   a Domicilio. Si el cliente va a vender a domicilio, ¿qué aplicación y qué suscripción
   necesitamos para probarlo y para operarlo?

Cinco hechos que ya tenemos comprobados, para que no haga falta reproducirlos:

1. Quitar una línea no la borra: hay que enviar Units: -1 con el total final. La contrapartida
   aparece en el ticket con importe negativo.

2. Items lleva solo las líneas del cambio y Order.Total el total final de la comanda. No es un
   reemplazo del contenido.

3. El comentario de una línea no se puede reescribir: reenviar la misma Lin con otro texto y
   Units: 0 añade una línea nueva y deja la vieja.

4. PaymentId no es idempotente: repetir un pago con el mismo PaymentId y hueco en el total
   cobra dos veces; el [301201] solo salta al exceder el total. E Invoice: true en un pago
   parcial se rechaza con [301301] aunque el pago sí se guarda.

5. Orders/Create no deduplica por MarketplaceOrderId: dos peticiones idénticas crean dos
   comandas con el mismo MarketId + MarketplaceOrderId, y GetOrder por ese par devuelve solo
   una. ¿La unicidad la debe sostener el integrador?

Un fallo que creemos que conviene corregir:

Orders/Create acepta una línea con Units: 0 (también añadida a una comanda con líneas válidas)
y el total no cambia. Después, al cobrar con Orders/Payment/Add e Invoice: true, BDP responde
«Intento de dividir por cero.» y no factura: la comanda queda cobrada pero sin documento, y el
mensaje no menciona la línea. Con la misma sesión, una comanda sin esa línea factura
correctamente. Entendemos que una línea de 0 unidades debería rechazarse en Orders/Create, o al
menos no impedir la facturación.

Todo está probado sobre el BDP de demostración, nunca sobre el TPV del cliente: aplicación 01
Hostelería, terminal 1, salón 1 mesas 4 a 8, artículos 1001 y 1002. Las comandas quedaron
canceladas o facturadas al terminar.

Quedamos a la espera de vuestra respuesta. Si conviene, preparamos un guion de reproducción con
las peticiones exactas de cada punto.

Gracias y un saludo,

Asley Crespo Navarro
Nakomi Studio

---

## Notas de envío (para la usuaria, fuera del correo)

- Los seis puntos de las dudas son los que fijó Guillermo el 18/11, más la cuestión de
  qué aplicación hace falta para `Type=2` (que él mismo planteó en el punto 7).
- Los cinco hechos van con la evidencia del 18/11
  (`Agente/documentacion/bdp/pruebas-comandas-guillermo-2026-11-18.md`) y del 20/11
  (`ticket-lineas-comentarios-2026-11-20.md`, `tipo2-domicilio-hosteleria-2026-11-20.md`).
- **Dirección:** `bdp@bdpcenter.com`, de donde responde Gael. Está registrada también en
  `problema-instalacion-sql-2026-09-30.md:191`. Se envía como respuesta al hilo de septiembre
  para que quede el contexto de la 36.2 y de la 37.5.
- **Firma:** la del hilo anterior, Asley Crespo Navarro, autorizada por Guillermo García Sáez
  (n.º 4652). Así BDP no tiene que volver a comprobar quién pregunta.
- **Cuestión de versión adelantada a propósito:** el 23/09 Gael dijo que la 36.2 ya no se
  descarga y que se desarrolle en la 37.5 (primera adaptada a VeriFactu; la 36 no servirá
  cuando entre en vigor). Preguntar seis comportamientos de 36.2 sin mencionarlo invita a un
  «pruébenlo en la 37.5» y a perder el turno.
- No se menciona ninguna credencial, ni el nombre del restaurante, ni importes reales.
