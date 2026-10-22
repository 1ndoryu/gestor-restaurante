# Fase D — Informe de cierre

**Integración web ↔ TPV BDP.** Qué se ha comprobado, qué funciona solo y qué queda pendiente.
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
del TPV físico, y están enumerados sin adornos en la sección 4.

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

Seis procesos automáticos, uno por necesidad. Todos respetan el **modo de la
cuenta**: en solo lectura no se escribe nada en el TPV.

| Proceso | Cada cuánto | Qué hace |
|---|---|---|
| **Envío de la cola** | 15 s | Manda al TPV lo que la web tiene pendiente (ventas, artículos, stock, pagos, facturas) |
| **Actualización de estados** | 60 s | Relee el estado de las comandas enviadas: si el TPV la cerró, la web lo sabe |
| **Avisos del TPV** | 30 s | Procesa los avisos que el TPV manda a la web (crear, modificar, borrar, finalizar, traspasar…) |
| **Documentos de venta** | por cuenta | Relee facturas y albaranes del TPV: cabecera, líneas y pagos |
| **Maestros** | por cuenta (15 min recomendado) | Relee artículos y precios: un cambio hecho en el TPV llega solo a la web |
| **Albaranes de compra** | 5 min | Importa los albaranes del TPV sin pulsar nada |

La regla que se ha corregido en este cierre es la más importante de todas: **elegir
«enviar en automático» enciende también la lectura del resultado**. Antes, la
comanda se enviaba y se quedaba congelada en la web, porque nadie volvía a
preguntar cómo acabó. Eso era exactamente lo contrario de lo que pide un
restaurante que elige automático.

---

## 3. Resultado de cada prueba

### Bloque 1 — Base y seguridad (Hito 1)

| Ficha | Qué comprueba | Resultado |
|---|---|---|
| D01 | Salud, versión, licencia 84 y acceso registrados | **Cerrada.** El TPV respondió versión 36.2 y la ficha quedó registrada. La licencia 84 se consulta ya en el diagnóstico de la pantalla; en el TPV DEMO responde «sin licencia», que es lo esperado |
| D02 | La contraseña vieja no entra, la nueva sí, sin secretos legibles | **Cerrada.** Rotación que exige la contraseña actual y registro de intentos que nunca guarda la contraseña ni su huella cifrada |

### Bloque 2 — Comandas y cocina (Hito 2)

| Ficha | Resultado |
|---|---|
| D03 — una venta web crea una sola comanda en caja, vinculada | **Cerrada.** Comanda 15, presente en el TPV y en la web, con el vínculo |
| D03a — crear con envío a cocina | **Pendiente.** Necesita la impresora de cocina |
| D04 — respuesta perdida al crear: se recupera la misma, no otra | **Cerrada en vivo.** Se cortó la respuesta con un proxy: la app no reenvió a ciegas, la relectura recogió la misma comanda (nº 60) y un reintento devolvió el mismo identificador con una sola entrada de auditoría |
| D05–D07 — añadir, cambiar o retirar línea de una comanda | **No aplicable.** El TPV no ofrece servicio web para modificar líneas. Se declara, no se disimula |
| D08 — toque simultáneo en caja y web | **Pendiente.** Exige dos personas a la vez: una en la caja y otra en la web |
| D09 — anular desde la web anula en caja | **Cerrada.** La anulación llegó al TPV y la comanda desapareció de su tabla |
| D09a — efecto en cocina de esa anulación | **Pendiente.** Necesita testigo de impresora auxiliar |
| D10 — anular en caja se refleja en la web | **Cerrada en vivo.** La comanda 46 pasó a anulada en la web con fecha y motivo; un aviso repetido no la vuelve a tocar; el sondeo también la anula |
| D21 — llegan los siete avisos | **6 de 7 cerrados.** El séptimo («cambiar estado») necesita el Monitor de Cocina instalado |
| D21a — aviso de traspaso de comanda completa | **Pendiente** de la prueba presencial |
| D22 / D22a — aviso repetido o desordenado | **Cerradas.** Repetido = ignorado; desordenado = aceptado y ordenado. El traspaso de línea real (comanda 47) se reenvió: idéntico → «duplicado», con fecha anterior → aceptado |
| D23a — trabajador caído con receptor operativo | **Cerrada.** El aviso espera en el buzón y se procesa al arrancar |
| D23b — receptor inaccesible | **Cerrada con límite declarado:** solo se recupera lo que se puede localizar por su identificador de pedido |
| D23-m — comanda manual con aviso perdido | **Cerrada con límite declarado.** El TPV no ofrece listado de comandas por API, así que una comanda de caja sin aviso no se recupera sola; queda el procedimiento escrito para que una persona la resuelva |

### Bloque 3 — Pagos, propina, puntos y factura (Hito 3)

| Ficha | Resultado |
|---|---|
| D11a / D11b / D11c — propina que suma, que sustituye y que se queda a cero | **Cerradas** con lectura entre pasos. Se corrigió una conclusión anterior falsa: se había leído una comanda ya anulada |
| D12a / D12b / D12c — puntos que suman, que restan y respuesta perdida | **Cerradas.** La pérdida de respuesta se reconcilió leyendo el saldo, sin reenviar. Y dos sumas seguidas ya no se pisan: se acumulan sobre la misma fila |
| D13 / D13a — pago parcial, con y sin factura | **Cerradas.** Dejaron ver un defecto real: un número de factura vacío marcaba la venta como facturada. Corregido |
| D14 — pago total liquida una sola vez | **Cerrada.** Un pago repetido se rechaza porque excede el saldo |
| D14s — dos pagos a la vez | **Cerrada.** Una petición entra y la otra recibe «ya hay un pago en proceso». No hay cobro doble |
| D15 / D15t — respuesta perdida o sin respuesta al pagar | **Cerradas.** La app consulta antes de repetir y encuentra el pago que sí se aplicó |
| D16 — pago hecho en la caja se ve en la misma venta web | **Cerrada** con un lote automatizado contra el TPV (4 pasos en verde, documento cerrado) |
| D17 — tope de tres pagos | **Cerrada.** Defecto encontrado y corregido: no había tope |
| D18 / D18a / D18b / D18c — número único, no re-facturar | **Cerradas.** Una segunda factura devuelve la existente en vez de emitir otra |
| D18d — factura completa con NIF | **Cerrada** sin escribir nada nuevo: la exportación del TPV ya trae el CIF y el nombre del cliente en cada documento |
| D19 / D19t — respuesta perdida al facturar | **Cerradas.** Número único, sin segunda factura |
| D20 — aviso y documento enlazados en una sola venta | **Cerrada.** Se enlazan por el número de factura, con índice único que impide duplicar |

### Bloque 4 — Maestros, stock y documentos (Hito 4)

| Ficha | Resultado |
|---|---|
| D24a / D24b / D24c — alta, modificación y desactivación de artículos | **Cerradas** contra el TPV, con el artículo leído después en su maestro |
| D24d — borrado | **Cubierta por D32.** El TPV no tiene servicio de borrado: la web muestra el procedimiento y el responsable |
| D24-p — plano leído sin mover posiciones | **Cerrada.** Comprobación del terminal, empleado y perfil contra el TPV |
| D24-m — formas de pago y empleados leídos | **Cerrada.** Tarjeta de solo lectura; no crea registros |
| D25 — clientes, con y sin sobrescritura | **Cerrada.** La sobrescritura es explícita y solo se aprobó en el TPV local |
| D26 / D26a — ajuste de stock desde la web y consulta unitaria | **D26a cerrada con límite:** la lectura es correcta y el caso de error con almacén inexistente se confirma en el TPV del cliente. **D26 (ajuste web = un solo movimiento)** pendiente: exige una corrida completa de escritura de stock en el piloto |
| D26b — consulta en lista por almacén | **Cerrada** con botón propio y llamador en producción |
| D26c — regularización con motivo | **Pendiente** de la corrida con motivo inválido contra el TPV del cliente |
| D26d — traspaso entre almacenes | **Cerrada** con traspaso real de ida y vuelta |
| D26e — ajuste por diferencia | **Cubierta** por la regularización, que es el mismo servicio con motivo (y por eso auditable). El TPV ofrece un servicio menor que no se usa a propósito |
| D26f — inventario absoluto | **Cerrada.** Nunca envía cantidad vacía; la misma cantidad da diferencia cero |
| D26g — sin respuesta en stock | **Cerrada.** Si hay duda, no se reintenta a ciegas; si el TPV rechazó con un motivo claro, ya no se marca «ambiguo» |
| D27 — la venta de caja no descuenta dos veces | **Declarada no verificable hoy:** no existe un proceso que mueva stock por venta. La protección queda escrita y probada para cuando exista |
| D28 / D28-auto / D28-auto-R — documentos, exportación programada y reinicio a mitad | **Cerradas y vivas.** Cadencia y perfil por cuenta. Se descubrió que un ajuste del TPV devolvía vacío y se fijó correctamente |
| D28a — albaranes automáticos | **Cerrada.** Importación completa sin botón |
| D32 — procedimiento y responsable de lo que la API no hace | **Cerrada en pantalla.** Responsable por defecto y por operación, editable por el cliente |

### Bloque 5 — Excepciones y piloto (Hito 5)

| Ficha | Resultado |
|---|---|
| D29 — tres reinicios en mitad de una operación | **2 de 3 cerradas** por inyección (proceso de sincronización y caída del servicio web), sin pérdida ni duplicado. Los dos reinicios que quedan exigen tocar el PC |
| D31 — llamar al camarero desde una mesa | **Cerrada técnicamente:** el aviso llega al TPV y el TPV lo rechaza por su propia configuración (falta la dirección del servidor de mensajes). Queda configurarlo en el terminal |
| D30 — piloto y despliegue por etapas | **Fuera de esta fase.** Requiere permiso y un TPV en producción |
| D33 — Configuración legible y con un solo guardado | **Cerrada este cierre** (ver sección 5) |

---

## 4. Limitaciones encontradas

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

## 5. Qué se ha cambiado en este último tramo

Además de cerrar fichas, se han corregido cuatro cosas que no se veían en las
pruebas pero que hacían la aplicación incómoda de usar. Las cuatro nacen de lo que
se ve en pantalla, no de una lista de tareas:

**a) Configuración parecía seis aplicaciones distintas.** Unas ayudas eran cajas
dentro de una caja y otras cajas sueltas; unas pestañas tenían dos botones
«Guardar» y otras ninguno; el aviso «esto se guarda al instante» flotaba entre dos
tarjetas sin decir a cuál pertenecía. Ahora las seis pestañas son iguales: un
título de grupo, sus tarjetas separadas por el mismo hueco, una sola barra de
guardado abajo a la derecha y el aviso de guardado instantáneo pegado al título del
grupo al que pertenece.

**b) La ayuda estaba donde no se busca.** Antes había una ayuda por pestaña, con
siete temas mezclados: para entender un interruptor había que leer siete bloques.
Ahora cada tarjeta tiene la suya, en su cabecera.

**c) Se podía perder lo escrito.** Al cambiar de pestaña con cambios sin guardar,
se perdían sin avisar. Ahora se avisa, no se cambia de pestaña y el botón
«Deshacer» es la salida. Salir de la página con cambios también avisa.

**d) El stock podía quedar bloqueado para siempre.** Si una operación de stock
quedaba en duda, ese artículo y ese almacén no se podían volver a tocar nunca: ni
reintentar ni dejarlo estar. Ahora hay una pantalla que enseña las operaciones en
duda y una persona decide qué pasó —se aplicó (no se reenvía) o se reintenta—.

Y la lectura de maestros, que antes solo ocurría al pulsar «Importar del TPV»,
tiene ahora su propia cadencia por cuenta.

---

## 6. Cómo se ha probado

Ninguna afirmación de este informe viene de leer el código y suponer. Cada una tiene
su prueba:

- **Contra el TPV real en local**, por su API, con el túnel de la máquina de
  pruebas: lecturas, escrituras, avisos, pagos, facturas, stock y documentos.
- **Con fallos provocados a propósito:** proxies que cortan la respuesta, que la
  retrasan 75 segundos, que duplican avisos o los llegan desordenados. Sirven para
  comprobar que la app no duplica nada cuando la red falla.
- **Con el TPV automatizado** (teclado y pantalla del propio TPV): alta de
  artículos, cobro y cierre de documento.
- **En el navegador**, recorriendo las seis pestañas de Configuración y midiendo el
  resultado en pantalla, no solo mirando capturas.
- **Suite automática del proyecto:** 636 pruebas en verde, 0 fallos y 47 omitidas
  a propósito (las que exigen el simulador del TPV o el TPV real del cliente).
- **Comprobación de calidad** sobre el código cambiado: sin avisos nuevos.

---

## 7. Estado final y siguiente paso

La integración está lista para el piloto en el TPV del cliente, con dos
condiciones:

1. **La suscripción de pago del TPV**, que habilita pagos y facturas sobre
   comandas existentes.
2. **Una sesión presencial de una hora** con el TPV delante para cerrar las cuatro
   pruebas que lo exigen (impresora de cocina, doble toque y reinicios).

Fuera de eso, lo que queda abierto depende de terceros —el distribuidor del TPV— o
es decisión del cliente: qué hacer con la comanda manual sin aviso y quién firma
el procedimiento de las operaciones que la API no expone.
