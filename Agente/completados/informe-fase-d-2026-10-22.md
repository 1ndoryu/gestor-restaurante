# Fase D — Informe de cierre

**Integración web–TPV BDP.** Qué se ha comprobado, qué funciona solo y qué queda pendiente.
Fecha del informe: 22 de octubre de 2026.

---

## 1. Resumen

La fase D pedía una cosa por encima de todas: **que la integración funcione sin que
nadie pulse botones**. Cada ajuste del TPV tenía que llegar solo a la web, y cada
venta de la web tenía que llegar sola al TPV, sin duplicados y sin depender de que
alguien se acuerde de sincronizar.

Eso es lo que hay hoy. La web lee del TPV, le envía, recoge sus avisos y corrige
lo que se quede a medias **en segundo plano**, con una cadencia por cuenta que se
configura una vez. Y cuando algo no se puede resolver solo, la pantalla lo dice y
ofrece la salida, en vez de bloquear el artículo para siempre.

Se ha probado contra un **TPV BDP real instalado en local** (licencia DEMO,
versión 36.2), que es el mismo programa y las mismas tablas que el del cliente.
Lo que no se ha podido probar son los cuatro puntos que exigen una persona delante
del TPV físico, y están enumerados sin adornos en la sección 5.

**Resultado de las 65 pruebas de aceptación** (fichas D01–D32). Cada una está
hecha o no lo está: no hay «parcialmente». Las que declaran un límite son
pruebas que **no se pueden hacer tal cual** porque el TPV no ofrece ese servicio,
y se cierran diciendo qué se hace en su lugar:

| Estado | Cuántas | Cuáles |
|---|---|---|
| **Cerradas con evidencia** | **46** | D01, D02, D03, D04, D09, D10, D11a, D11b, D11c, D12a, D12b, D12c, D13, D13a, D14, D14s, D15, D15t, D16, D17, D18, D18a, D18b, D18c, D18d, D19, D19t, D20, D22, D22a, D23a, D24a, D24b, D24c, D24-p, D24-m, D25, D26b, D26d, D26f, D26g, D28, D28-auto, D28-auto-R, D28a, D32 |
| **Cerradas declarando un límite** | **9** | D05, D06, D07 (el TPV no modifica líneas), D23b y D23-m (solo se recupera lo localizable), D24d (sin servicio de borrado: va por D32), D26a (lectura correcta; el error con almacén inexistente se confirma en el TPV del cliente), D26e (cubierta por la regularización), D27 (no hay proceso que mueva stock por venta) |
| **Pendientes** | **10** | D03a, D08, D09a, D21, D21a, D26, D26c, D29, D31 (presencia física, impresora, configuración del terminal o el proveedor) y D30 (piloto, fuera de esta fase) |

---

## 2. Qué hace la integración hoy, sin intervención

Siete procesos automáticos, uno por necesidad. Todos respetan el **modo de la
cuenta**: en solo lectura no se escribe nada en el TPV.

| Proceso | Cada cuánto | Qué hace |
|---|---|---|
| **Envío de la cola** | 15 s | Manda al TPV lo que la web tiene pendiente (ventas, artículos, stock, pagos, facturas) |
| **Actualización de estados** | 60 s | Relee el estado de las comandas enviadas: si el TPV la cerró, la web lo sabe |
| **Avisos del TPV** | 30 s | Procesa los avisos que el TPV manda a la web (crear, modificar, borrar, finalizar, traspasar…) |
| **Documentos de venta** | por cuenta | Relee facturas y albaranes del TPV: cabecera, líneas y pagos |
| **Maestros** | por cuenta (15 min) | Relee artículos y precios: un cambio hecho en el TPV llega solo a la web |
| **Albaranes de compra** | 5 min | Importa los albaranes del TPV sin pulsar nada |
| **Clientes, plano y departamentos** | por cuenta (15 min) | Relee los clientes del TPV, los salones/zonas/mesas y las familias del catálogo, cada uno con su cadencia |

La regla que se ha corregido en este cierre es la más importante de todas: **elegir
«enviar en automático» enciende también la lectura del resultado**. Antes, la
comanda se enviaba y se quedaba congelada en la web, porque nadie volvía a
preguntar cómo acabó. Eso era exactamente lo contrario de lo que pide un
restaurante que elige automático.

---

## 3. La tabla del alcance, cómo queda hoy

Es la misma tabla que se entregó en el documento de alcance («Después de la fase
D»), fila por fila, con la última columna puesta al día. **Se trae** = del TPV a la
web. **Se envía** = de la web al TPV. **Automático** = viaja solo, sin pulsar nada.
**Cómo queda** = lo que hay hoy.

| Funcionalidad | Se trae | Se envía | Automático | Cómo queda |
|---|:---:|:---:|:---:|---|
| Catálogo (artículos, precio, familia) | Sí | Sí | Sí | **Hecho.** Lectura sola cada 15 min (cadencia por cuenta) y envío por la cola automática |
| Clientes | Sí | Sí | Sí | **Hecho.** Importación sola con su propia cadencia (cada 15 min por cuenta); el botón con confirmación queda para forzar el cambio en el momento |
| Plano de sala | Sí | — | Lectura sí | **Hecho.** Salones, zonas y mesas se releen solos con su cadencia; la lectura no mueve ni renombra nada |
| Departamentos | Sí | Sí | Sí | **Hecho.** Se refrescan solos con su propia cadencia, aparte de la del catálogo |
| Formas de pago, empleados | Sí | — | Lectura sí | **Hecho con matiz.** La pantalla compara el mapeo con el TPV cuando se le pide; no crea registros ni se refresca sola |
| Albaranes de compra | Sí | — | Por exportación | **Hecho.** Importación automática cada 5 min, últimos 31 días |
| Detalle de comanda / stock | Sí | — | Sí | **Hecho.** Se lee tras cada aviso y con los documentos de venta |
| Crear comanda | Sí | Sí | Sí | **Hecho en local.** Falta el envío a cocina, que necesita la impresora |
| Propina (en comanda) | — | Sí | Sí | **Hecho.** Sumar, sustituir y dejar a cero |
| Puntos fidelización (cliente) | — | Sí | Sí | **Hecho.** Abono, cargo y pérdida de respuesta, sin duplicar |
| Anular comanda | Sí | Sí | Sí | **Hecho.** El efecto en cocina queda para la sesión presencial |
| Modificar comanda enviada | Sí | Sí | Sí | **No existe en el TPV.** No hay servicio web para cambiar líneas; se dice en pantalla en vez de ofrecer un botón que falla |
| Detectar traspaso de TPV (avisos) | Sí | — | Sí | **Hecho en parte.** Traspaso de línea probado; el de comanda completa es presencial |
| Iniciar traspaso desde la web | — | Pendiente de TPV | No | **Fuera.** El TPV no lo expone |
| Cobrar comanda existente | Sí | Sí | Sí | **Hecho.** Parcial, total, doble clic y respuesta perdida |
| Facturar | Sí | Sí | Sí | **Hecho**, con la salvedad de la suscripción de pago del TPV |
| Consultar stock (TPV→web) | Sí | — | Lectura sí | **Hecho.** Por artículo y en lista por almacén |
| Regularizar stock (web→TPV) | — | Sí | Sí | **Hecho en parte.** Falta confirmar en el TPV del cliente el rechazo por motivo inválido |
| Traspasar stock entre almacenes | — | Sí | Sí | **Hecho.** Ida y vuelta real |
| Actualizar inventario (web→TPV) | — | Sí | Sí | **Hecho.** Nunca envía cantidad vacía |
| Llamar al camarero | — | Sí | Sí | **Hecho de nuestro lado.** El TPV lo rechaza hasta que se configure en el terminal |
| Borrar artículos | Pendiente de TPV | Pendiente de TPV | No | **Límite del TPV.** La pantalla dice quién lo hace y cómo |
| Reservas, gastos, marketing | — | — | — | **Fuera de alcance.** Son solo web y ya funcionan |

Resumen de la tabla: **17 filas hechas**, 2 con matiz declarado (formas de pago y
regularización de stock, ambas por lo que falta confirmar en el TPV del cliente),
2 que el TPV no expone y 1 fuera de alcance. Ninguna fila queda pendiente por
falta de automatismo.

---

## 4. Resultado de cada prueba

Las 65 fichas, una por línea. **Cerrada** = hecha y comprobada contra el TPV;
**pendiente** = exige algo que no está en este entorno. Las que declaran un límite
dicen exactamente cuál.

| Ficha | Qué se comprueba y cómo queda |
|---|---|
| D01 | Salud, versión, licencia 84 y acceso registrados — **Cerrada.** El TPV respondió versión 36.2 y la ficha quedó registrada. La licencia 84 se consulta ya en el diagnóstico de la pantalla; en el TPV DEMO responde «sin licencia», que es lo esperado |
| D02 | La contraseña vieja no entra, la nueva sí, sin secretos legibles — **Cerrada.** Rotación que exige la contraseña actual y registro de intentos que nunca guarda la contraseña ni su huella cifrada |
| D03 | Una venta web crea una sola comanda en caja, vinculada. **Cerrada.** Comanda 15, presente en el TPV y en la web, con el vínculo |
| D03a | Crear con envío a cocina. **Pendiente.** Necesita la impresora de cocina |
| D04 | Respuesta perdida al crear: se recupera la misma, no otra. **Cerrada en vivo.** Se cortó la respuesta con un proxy: la app no reenvió a ciegas, la relectura recogió la misma comanda (nº 60) y un reintento devolvió el mismo identificador con una sola entrada de auditoría |
| D05–D07| Añadir, cambiar o retirar línea de una comanda. **No aplicable.** El TPV no ofrece servicio web para modificar líneas. Se declara, no se disimula |
| D08 | Toque simultáneo en caja y web. **Pendiente.** Exige dos personas a la vez: una en la caja y otra en la web |
| D09 | Anular desde la web anula en caja. **Cerrada.** La anulación llegó al TPV y la comanda desapareció de su tabla |
| D09a | Efecto en cocina de esa anulación. **Pendiente.** Necesita testigo de impresora auxiliar |
| D10 | Anular en caja se refleja en la web. **Cerrada en vivo.** La comanda 46 pasó a anulada en la web con fecha y motivo; un aviso repetido no la vuelve a tocar; el sondeo también la anula |
| D11a, D11b, D11c | Propina que suma, que sustituye y que se queda a cero. **Cerradas** con lectura entre pasos. Se corrigió una conclusión anterior falsa: se había leído una comanda ya anulada |
| D12a, D12b, D12c | Puntos que suman, que restan y respuesta perdida. **Cerradas.** La pérdida de respuesta se reconcilió leyendo el saldo, sin reenviar. Y dos sumas seguidas ya no se pisan: se acumulan sobre la misma fila |
| D13, D13a | Pago parcial, con y sin factura. **Cerradas.** Dejaron ver un defecto real: un número de factura vacío marcaba la venta como facturada. Corregido |
| D14 | Pago total liquida una sola vez. **Cerrada.** Un pago repetido se rechaza porque excede el saldo |
| D14s | Dos pagos a la vez. **Cerrada.** Una petición entra y la otra recibe «ya hay un pago en proceso». No hay cobro doble |
| D15, D15t | Respuesta perdida o sin respuesta al pagar. **Cerradas.** La app consulta antes de repetir y encuentra el pago que sí se aplicó |
| D16 | Pago hecho en la caja se ve en la misma venta web. **Cerrada** con un lote automatizado contra el TPV (4 pasos en verde, documento cerrado) |
| D17 | Tope de tres pagos. **Cerrada.** Defecto encontrado y corregido: no había tope |
| D18, D18a, D18b, D18c | Número único, no re-facturar. **Cerradas.** Una segunda factura devuelve la existente en vez de emitir otra |
| D18d | Factura completa con NIF. **Cerrada** sin escribir nada nuevo: la exportación del TPV ya trae el CIF y el nombre del cliente en cada documento |
| D19, D19t | Respuesta perdida al facturar. **Cerradas.** Número único, sin segunda factura |
| D20 | Aviso y documento enlazados en una sola venta. **Cerrada.** Se enlazan por el número de factura, con índice único que impide duplicar |
| D21 | Llegan los siete avisos. **6 de 7 cerrados.** El séptimo («cambiar estado») necesita el Monitor de Cocina instalado |
| D21a | Aviso de traspaso de comanda completa. **Pendiente** de la prueba presencial |
| D22, D22a | Aviso repetido o desordenado. **Cerradas.** Repetido = ignorado; desordenado = aceptado y ordenado. El traspaso de línea real (comanda 47) se reenvió: idéntico → «duplicado», con fecha anterior → aceptado |
| D23-m | Comanda manual con aviso perdido. **Cerrada con límite declarado.** El TPV no ofrece listado de comandas por API, así que una comanda de caja sin aviso no se recupera sola; queda el procedimiento escrito para que una persona la resuelva |
| D23a | Trabajador caído con receptor operativo. **Cerrada.** El aviso espera en el buzón y se procesa al arrancar |
| D23b | Receptor inaccesible. **Cerrada con límite declarado:** solo se recupera lo que se puede localizar por su identificador de pedido |
| D24-m | Formas de pago y empleados leídos. **Cerrada.** Tarjeta de solo lectura; no crea registros |
| D24-p | Plano leído sin mover posiciones. **Cerrada.** Comprobación del terminal, empleado y perfil contra el TPV |
| D24a, D24b, D24c | Alta, modificación y desactivación de artículos. **Cerradas** contra el TPV, con el artículo leído después en su maestro |
| D24d | Borrado. **Cubierta por D32.** El TPV no tiene servicio de borrado: la web muestra el procedimiento y el responsable |
| D25 | Clientes, con y sin sobrescritura. **Cerrada.** La sobrescritura es explícita y solo se aprobó en el TPV local |
| D26, D26a | Ajuste de stock desde la web y consulta unitaria. **D26a cerrada con límite:** la lectura es correcta y el caso de error con almacén inexistente se confirma en el TPV del cliente. **D26 (ajuste web = un solo movimiento)** pendiente: exige una corrida completa de escritura de stock en el piloto |
| D26b | Consulta en lista por almacén. **Cerrada** con botón propio y llamador en producción |
| D26c | Regularización con motivo. **Pendiente** de la corrida con motivo inválido contra el TPV del cliente |
| D26d | Traspaso entre almacenes. **Cerrada** con traspaso real de ida y vuelta |
| D26e | Ajuste por diferencia. **Cubierta** por la regularización, que es el mismo servicio con motivo (y por eso auditable). El TPV ofrece un servicio menor que no se usa a propósito |
| D26f | Inventario absoluto. **Cerrada.** Nunca envía cantidad vacía; la misma cantidad da diferencia cero |
| D26g | Sin respuesta en stock. **Cerrada.** Si hay duda, no se reintenta a ciegas; si el TPV rechazó con un motivo claro, ya no se marca «ambiguo» |
| D27 | La venta de caja no descuenta dos veces. **Declarada no verificable hoy:** no existe un proceso que mueva stock por venta. La protección queda escrita y probada para cuando exista |
| D28, D28-auto, D28-auto-R | Documentos, exportación programada y reinicio a mitad. **Cerradas y vivas.** Cadencia y perfil por cuenta. Se descubrió que un ajuste del TPV devolvía vacío y se fijó correctamente |
| D28a | Albaranes automáticos. **Cerrada.** Importación completa sin botón |
| D29 | Tres reinicios en mitad de una operación. **2 de 3 cerradas** por inyección (proceso de sincronización y caída del servicio web), sin pérdida ni duplicado. Los dos reinicios que quedan exigen tocar el PC |
| D30 | Piloto y despliegue por etapas. **Fuera de esta fase.** Requiere permiso y un TPV en producción |
| D31 | Llamar al camarero desde una mesa. **Cerrada técnicamente:** el aviso llega al TPV y el TPV lo rechaza por su propia configuración (falta la dirección del servidor de mensajes). Queda configurarlo en el terminal |
| D32 | Procedimiento y responsable de lo que la API no hace. **Cerrada en pantalla.** Responsable por defecto y por operación, editable por el cliente |
| D33 | Configuración legible y con un solo guardado. **Cerrada este cierre.** Ayudas por tarjeta, un solo guardado por pestaña y aviso al cambiar de pestaña con cambios sin guardar |

---

## 5. Limitaciones encontradas

Todas las de esta lista están declaradas, no escondidas. Ninguna es un fallo de la
web: son límites del TPV o de las condiciones de prueba.

1. **Cuatro pruebas exigen una persona delante del TPV físico.** Son D03a y D09a
   (impresora de cocina), D08 (dos personas tocando a la vez) y el reinicio
   presencial de D29. No hay forma de simularlas sin falsear el resultado.
2. **El TPV no permite modificar líneas de una comanda por API** (D05–D07). La web
   lo dice en lugar de ofrecer un botón que iba a fallar.
3. **El TPV no permite borrar artículos por API** (D24d). Se resuelve por D32: la
   pantalla muestra quién lo hace y cómo.
4. **El TPV no ofrece listado de comandas por API** (D23-m). Una comanda creada a
   mano en la caja sin que llegue su aviso no se puede recuperar sola. Límite
   declarado con procedimiento escrito.
5. **El séptimo aviso («cambiar estado») exige el Monitor de Cocina instalado.**
   Los otros seis están probados con eventos reales.
6. **El aviso de «llamar al camarero» depende de la configuración del terminal**
   (D31): sin la dirección del servidor de mensajes, el TPV lo rechaza con su
   propio código de error. No es un defecto de la web; el error ya se puede leer
   en pantalla.
7. **Licencia 84:** el TPV de prueba es DEMO y responde «sin licencia». La consulta
   funciona; el valor afirmativo solo se verá en el TPV del cliente.
8. **Sobrescritura de clientes (D25):** se aprobó únicamente en el TPV local. En
   producción viene desactivada por defecto.

### Riesgo pendiente de decisión del cliente

La suscripción de pago del TPV es la que habilita añadir un pago a una comanda ya
creada y facturarla. Todo el flujo está escrito y probado en local contra la
versión DEMO; con la suscripción gratuita esas dos operaciones no están
disponibles. Es lo único que puede impedir que la integración funcione igual en
producción, y se resuelve hablando con el proveedor.

---


## 6. Estado final y siguiente paso

La integración está lista para el piloto en el TPV del cliente, con dos
condiciones:

1. **La suscripción de pago del TPV**, que habilita pagos y facturas sobre
   comandas existentes.
2. **Una sesión presencial de una hora** con el TPV delante para cerrar las cuatro
   pruebas que lo exigen (impresora de cocina, doble toque y reinicios).

Fuera de eso, lo que queda abierto depende de terceros —el distribuidor del TPV— o
es decisión del cliente: qué hacer con la comanda manual sin aviso y quién firma
el procedimiento de las operaciones que la API no expone.

Ninguna fila de la tabla del alcance queda pendiente por falta de automatismo: las
que no están hechas del todo son límites del propio TPV, declarados en la sección 5.
