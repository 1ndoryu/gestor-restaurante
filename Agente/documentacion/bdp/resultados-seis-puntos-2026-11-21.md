# Los seis puntos de Guillermo — resultados al 21/11/2026

Todo lo de abajo se ha ejecutado contra el **TPV demo** de BDP (aplicación 01 Hostelería
36.2, servicio 84) conectado al entorno `rest-dev`. Nada se ha probado contra el BDP del
restaurante, y **no hay ninguna escritura automática conectada** a él.

## Resumen

| Punto | Estado |
|---|---|
| 1. Pruebas de pagos | Listo para el piloto |
| 2. Protección contra unidades cero | Listo para el piloto |
| 3. Regresión del flujo normal | Listo para el piloto, con dos huecos (abajo) |
| 4. Configuración de producción | Preparada y validada contra el demo; falta correrla contra el TPV del restaurante |
| 5. Despliegue por etapas | Listo para el piloto |
| 6. Funciones que esperan a BDP | Bloqueadas en el servidor y marcadas en la interfaz |

## 1. Pruebas de pagos — listo

- **Dos peticiones simultáneas del mismo cobro** escriben **una sola vez** en BDP. Tres tests
  automáticos en verde; uno de ellos reproduce la respuesta perdida (BDP aplica el cobro y
  corta la conexión): al recuperar, la web lo detecta por `GetOrder` y **no reenvía**.
- **Tope de tres pagos por comanda**: el cuarto se rechaza sin escribir.
- **Dato para usted**: BDP 36.2 **no deduplica** `AddOrderPayment` por `PaymentId` (dos
  pagos de 2,00 con el mismo identificador quedaron como dos cobros, prueba del 18/11). La
  protección es nuestra; si algún día otro sistema escribe cobros con el mismo `PaymentId`,
  BDP los sumará.

  **Corrección del 23/11** (la versión anterior de este informe decía «cada intento lleva un
  `PaymentId` distinto», y eso no es lo que hace el código): el `PaymentId` **identifica el
  pago, no el intento**. Sale de la clave de idempotencia (`P<venta>-<clave>`, con resumen
  SHA-256 si la clave es larga) y **todos los reintentos de ese mismo pago reutilizan
  exactamente el mismo identificador**. Solo un pago sin clave de idempotencia lleva un id
  propio por armado, porque sin clave no hay forma de reconocer que es el mismo cobro.

  Que el id se repita no protege por sí solo, y no se pretende: la protección es la lectura
  previa a BDP (`GetOrder`) antes de reenviar. Si el pago aparece con ese id, ese importe y
  esa forma de pago, no se reenvía; si aparece con otro importe o con otra forma de pago,
  bloquea. Lo que sí queda garantizado es que un reintento **no puede generar un id nuevo**,
  que es justo lo que convertiría un reintento en un segundo cobro.

## 2. Unidades cero — listo

El bloqueo está en el servidor, no solo en la pantalla: creación, modificación, comentarios y
reintentos de la cola. Un test comprueba que el simulador **no recibe ni un solo payload** con
0 unidades por ninguna de las vías. La razón es la que usted describió: BDP acepta la línea de
0 uds y después la factura falla con «intento de dividir por cero», dejando la comanda cobrada
y sin documento.

## 3. Regresión del flujo normal — listo, con dos huecos

Ejecutado entero sobre el demo. Comandas y tickets:

| Qué | Comanda | Resultado |
|---|---|---|
| Crear y cobrar | 155 | Creada, cobrada y facturada sola |
| Añadir, quitar y modificar productos | 156 | Deltas de línea en el TPV; total idéntico en web y TPV |
| Cambiar comentarios de línea | 156 | Cada producto lleva su comentario a `Comments[]`; cambiarlo después envía contrapartida con el texto viejo y alta con el nuevo |
| Descuento e invitación | 157 | Descuento 0,50 y línea de invitación; total 4,50 |
| Cobro parcial con dos formas de pago | 157 | 2,00 tarjeta + 2,50 efectivo, los dos en el TPV |
| Ticket | 157 | `00001TB00000065` |
| Cancelar sin pagos | 156 | Comanda anulada en el TPV |
| Apagar BDP y recuperar | 159 | La venta queda pendiente y **se recupera sola** al volver el TPV |

**Un fallo encontrado y corregido en esta pasada**: una venta creada con el TPV apagado
quedaba marcada como error definitivo y el sistema no volvía a intentarla nunca; había que
reintentarla a mano. Ahora se queda como «pendiente» y se reenvía sola cuando el TPV responde
(comprobado en vivo: comanda 159 sin intervención).

**Dos huecos que quedan** (no bloquean el piloto, pero hay que decidirlos):

1. **Cobro parcial**: tras cobrar el total, la web seguía mostrando 0,45 € pendientes, porque
   calcula el pendiente sobre la base más el IVA y no sobre el total que aceptó el TPV. En un
   restaurante real eso lleva a cobrar de más. **Es el más importante de los dos.**
2. **Invitación**: la línea invitada llega al ticket con el artículo por defecto en vez del que
   se pidió. El importe es 0, que es lo que importa, pero el nombre no es el correcto.

**Comentarios de línea, cerrado**: cada producto tiene su campo en la web; el texto viaja en
`Comments[]` y se recorta a 200 caracteres. Cambiarlo en una comanda ya enviada se hace con
contrapartida del texto viejo más alta con el nuevo, porque BDP no reescribe el comentario de
una línea existente y fundirlos dejaría el texto antiguo en el ticket.

Otro detalle operativo: si se modifican líneas muy seguidas (menos de un segundo entre
peticiones), BDP responde «throttled» y la web conserva el estado del TPV y avisa. Es correcto,
pero un camarero rápido lo verá.

## 4. Configuración de producción — preparada, falta el TPV real

Los seis checks de configuración (perfil de artículos, mapa de IVA, perfil de documentos,
cliente por defecto, artículo por defecto y tarifa del catálogo) están desplegados y se han
corrido contra el demo: **19 comprobaciones, 19 en verde**. Ninguna da por bueno un valor por
estar escrito: cada una lo busca en una lista real del TPV. El artículo por defecto es nuevo:
antes un código inventado pasaba la configuración y solo fallaba al crear la comanda.

Una salvedad para no darle una impresión falsa: el demo no tiene cliente por defecto
configurado, así que ese check pasa con un aviso y no con un valor. En el restaurante sí hay
que ponerlo, y hasta entonces la comanda sin cliente saldrá sin cliente en el ticket.

**No se ha rellenado ningún código inventado.** Los valores del restaurante salen de la
pantalla de diagnóstico de `restaurante.wandori.us`, que es el único sitio con ruta al BDP del
restaurante. Queda pendiente que usted o nosotros pasemos ese diagnóstico allí y rellenemos lo
que marque.

## 5. Despliegue por etapas — listo

Se pueden encender por separado: lectura, comandas, modificaciones, descuentos, pagos y
tickets, además del registro de actividad. Cada etapa se puede apagar, y volver a «solo
lectura» es inmediato y corta las escrituras en el servidor, no solo en la pantalla.

## 6. Funciones que esperan a BDP — bloqueadas

Conviene separar dos casos, porque no son lo mismo:

- **Pedidos a domicilio (`Type=2`)**: la opción existe en la web y está **deshabilitada** con la
  nota «pendiente de BDP»; además el **servidor rechaza** el envío antes de tocar la red, aunque
  alguien lo intente por API. Motivo: Hostelería 01 responde `300009`, y el demo no tiene la
  aplicación «Servicio a Domicilio» donde sí funcionaría.
- **Cambio de mesa, dividir y unir comandas, retirar pagos parciales, devoluciones y
  anulaciones después del ticket**: **no existen en la web**. No hay botón ni endpoint que
  pueda escribir en BDP por esas vías, así que no hay nada que desactivar. Devoluciones y
  rectificaciones figuran además en la lista de operaciones que BDP no expone por API y que se
  hacen en su TPV; la web se limita a indicarlo con el responsable asignado.

En resumen: lo único que la web podría escribir y está cortado es el pedido a domicilio, y lo
está en el servidor.

## Lo que hace falta de su parte

1. Validar la licencia del TPV demo (avisa «sin conexión con el servidor de subscripción»).
2. Pasarnos los valores reales de configuración del restaurante (formas de pago, canales,
   perfil de documentos, IVA, terminal, empleado, artículo y cliente por defecto) para correr
   el diagnóstico contra su BDP.
3. Decidir los dos huecos del punto 3, en especial de dónde sale el total en un cobro parcial.
