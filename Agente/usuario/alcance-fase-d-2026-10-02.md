# Fase D: que lo de caja llegue solo a la web — alcance y estado (02/10/2026, rev. 06/10/2026 tras correcciones + 07/10/2026 con 2ª tanda y resumen de pruebas + 08/10/2026 con 3ª tanda: matriz D, equivalencia y anexo B8 + 09/10/2026 con 4ª tanda: B8 separado, equivalencias corregidas y D18c + 10/10/2026 con 5ª tanda: correcciones técnicas de Guillermo verificadas contra manual y código)

**Aviso: este documento describe el alcance previsto y el estado actual, no un sistema terminado.** Ninguna función está terminada sin estar implementada, probada y registrada en la matriz de aceptación D y el anexo B8 (archivo separado con la ficha completa de cada prueba: previos, datos, pasos, esperado, fallo y evidencias). La guía operativa recibida (05/10/2026) es el plan principal de ejecución.

## Objetivo

Hoy casi todo va con botón: traer datos a la web, enviar comandas al BDP, y lo que se hace en caja hay que ir a buscarlo. La fase D quita los botones: **casi todo viaja solo en ambos sentidos** — lo de la web entra al BDP y lo de caja llega a la web al momento, salvo las excepciones que se listan más abajo.

Se prueba todo en un entorno local con la misma versión BDP.NET 36.2 (distinta instalación y configuración) y en producción solo se instala lo ya comprobado.

## Antes de la fase D (hoy, con botones)

| Funcionalidad                         |  Se trae  | Se envía  | Automático |
| ------------------------------------- | :-------: | :-------: | :--------: |
| Catálogo (artículos, precio, familia) |    Sí     |    Sí     | No (botón) |
| Clientes                              |    Sí     |    Sí     | No (botón) |
| Plano de sala                         |    Sí     |     —     | No (botón) |
| Departamentos                         |    Sí     |    Sí     | No (botón) |
| Formas de pago, empleados             |    Sí     |     —     | No (botón) |
| Albaranes de compra                   | Pendiente |     —     |     No     |
| Detalle de comanda / stock            |    Sí     |     —     | No (botón) |
| Crear comanda                         |     —     |    Sí     | No (botón) |
| Propina (en comanda)                  |     —     |    Sí     | No (botón) |
| Puntos fidelización (cliente)         |     —     |    Sí     | No (botón) |
| Anular comanda                        |     —     |    Sí     | No (botón) |
| Modificar comanda enviada             |     —     | Pendiente |     No     |
| Traspasar (mesa / líneas)             |     —     |     —     |     No     |
| Cobrar comanda existente              |     —     | Pendiente |     No     |
| Facturar                              |     —     | Pendiente |     No     |
| Mover stock                           |     —     | Pendiente |     No     |
| Llamar al camarero                    |     —     | Pendiente |     No     |
| Borrar artículos                      | Pendiente | Pendiente |     No     |
| Reservas, gastos, marketing           |     —     |     —     |     —      |

## Después de la fase D (objetivo, con excepciones)

| Funcionalidad                                                    |     Se trae      |     Se envía     |         Automático          | Estado actual |
| ---------------------------------------------------------------- | :--------------: | :--------------: | :-------------------------: | :-----------: |
| Catálogo (artículos, precio, familia)                            |        Sí        |        Sí        |             Sí              | Manual; auto pte. prueba |
| Clientes                                                         |        Sí        |        Sí        |             Sí              | Manual; auto pte. prueba |
| Plano de sala                                                    |        Sí        |        —         |         Lectura sí          | Manual; auto pte. prueba |
| Departamentos                                                    |        Sí        |        Sí        |             Sí              | Manual; auto pte. prueba |
| Formas de pago, empleados                                        |        Sí        |        —         |         Lectura sí          | Manual; auto pte. prueba |
| Albaranes de compra                                              |        Sí        |        —         |       Por exportación       | Manual; auto pte. prueba |
| Detalle de comanda / stock                                       |        Sí        |        —         |             Sí              | Manual; auto pte. prueba |
| Crear comanda                                                    |        Sí        |        Sí        |             Sí              | Aviso OK en local; pte. D03+D03a |
| Propina (en comanda)                                             |        —         |        Sí        |             Sí              | Pte. de prueba |
| Puntos fidelización (cliente)                                    |        —         |        Sí        |             Sí              | Pte. de prueba |
| Anular comanda                                                   |        Sí        |        Sí        |             Sí              | Pte. de prueba |
| Modificar comanda enviada                                        |        Sí        |        Sí        |             Sí              | Aviso OK en local; web pte. prueba |
| Detectar traspaso de BDP (avisos)                                    |        Sí        |        —         |             Sí              | Pte. de prueba |
| Iniciar traspaso desde la web                                        |        —         | Pendiente de BDP |             No              | Pte. de BDP |
| Cobrar comanda existente                                         |        Sí        |        Sí        |             Sí              | Manual OK en TPV; auto pte. prueba |
| Facturar                                                         |        Sí        |        Sí        |             Sí              | Simplificada OK en TPV; resto pte. prueba |
| Consultar stock (BDP→web)                                            |        Sí        |        —         |         Lectura sí          | Pte. de prueba |
| Regularizar stock (web→BDP)                                          |        —         |        Sí        |             Sí              | Pte. de prueba |
| Traspasar stock entre almacenes (web→BDP)                            |        —         |        Sí        |             Sí              | Pte. de prueba |
| Actualizar inventario (web→BDP)                                       |        —         |        Sí        |             Sí              | Pte. de prueba |
| Llamar al camarero                                               |        —         |        Sí        |             Sí              | Pte. de prueba |
| Borrar artículos                                                 | Pendiente de BDP | Pendiente de BDP |             No              | Pte. de BDP |
| Reservas, gastos, marketing                                      |        —         |        —         |              —              | Fuera de alcance |

Cómo leer las tablas. Se trae = del BDP a la web. Se envía = de la web al BDP. Automático = viaja solo, sin botón (objetivo). Estado actual = situación hoy de cada objetivo. Sí = incluido. Pendiente = falta un paso. Pendiente de BDP = falta respuesta o contrato de BDP. Pte. = pendiente. No = no se hace. Lectura sí = solo lectura automática. — = no aplica.

Regla: ninguna función pasa a Validado sin superar su prueba de aceptación. Matriz propia D (los códigos A01–A20 de la guía tienen otro significado: ver correspondencia). crear→D03+D03a; modificar→D05–D08; anular→D09+D10+D09a; propina→D11a–D11c; puntos→D12a–D12c; cobrar→D13–D17+D13a+D14s+D15t; facturar→D18–D20+D18a–D18d+D19t; avisos→D21+D22+D21a+D22a; caídas→D23a+D23b+D23-m+D29; maestros→D24a–D24d+D24-p+D24-m+D25; stock→D26+D26a–D26g; detalle→D27+D28+D28a; exportación auto→D28-auto+D28-auto-R; llamar→D31; excepciones→D32; piloto→D30. Detalle de cada prueba (previos, datos, pasos, esperado, fallo, evidencia): anexo B8, entregado como archivo separado. Sin prueba de ejecución: iniciar traspaso y borrar (ptes. BDP, cubiertos por D32); reservas, fuera.

## Pruebas de aceptación (resumen)

Cada función se valida con su prueba. El detalle de cada una (previos, datos utilizados, pasos exactos, qué se espera, qué error la detiene y qué evidencia deja) vive en el anexo B8, entregado como archivo separado.

| Prueba | Se comprueba |
| ------ | ------------ |
| D01 | Salud, versión, licencia 84 y acceso registrados. |
| D02 | La contraseña vieja no entra; la nueva sí, sin secretos legibles pero con registro de intentos. |
| D03 | Una venta web crea una sola comanda en caja, vinculadas. |
| D03a | Crear con OrderEndType=0: aparca la venta y sale por cocina. |
| D04 | Respuesta perdida al crear: se recupera la misma, no otra. |
| D05 | Añadir línea mantiene comanda y contenido exacto; la corrección sale por cocina. |
| D06 | Cambiar unidades o comentario: total correcto; la corrección sale por cocina. |
| D07 | Retirar línea exacta, o función desactivada si no soportada; la corrección sale por cocina. |
| D08 | Toque simultáneo en caja y web: conflicto visible, nada se pisa. |
| D09 | Anular desde la web anula en caja. |
| D09a | Anular con CancelOrder (solo PosId y OrderIdentifier; borra la comanda entera): comprobar qué sale por impresoras auxiliares. |
| D10 | Anular en caja se refleja en la web. |
| D11a | Propina sumada con AddTip=true: se suma a la base. |
| D11b | Propina sustituida con AddTip=false: fija el valor dado. |
| D11c | Propina a cero con AddTip=false: queda a cero sin residuo. |
| D12a | Puntos positivos: abono con ID único, saldo y motivo correctos. |
| D12b | Puntos negativos: cargo con ID único, saldo y motivo correctos. |
| D12c | Pérdida de respuesta en puntos: se consulta antes de repetir, sin duplicar. |
| D13 | Pago parcial único, saldo y caja correctos. |
| D13a | Parcial sin factura: la venta sigue abierta con el saldo. |
| D14 | Pago total liquida una sola vez. |
| D14s | Dos clics o dos pagos simultáneos: solo se realiza un cobro. |
| D15 | Respuesta perdida al pagar: se verifica antes de repetir. |
| D15t | Sin respuesta al pagar: se consulta antes de repetir, sin duplicar. |
| D16 | Pago de caja reflejado en la misma venta web. |
| D17 | Tope de tres pagos. |
| D18 | Factura con número único. |
| D18a | Pago total con AddOrderPayment (Invoice=true): factura una vez, no se re-factura; pago parcial no factura. |
| D18b | InvoiceOrder sobre comanda sin facturar: número único. |
| D18c | Antes de facturar, la web consulta la comanda. Si GetOrder devuelve Status=3, no llama a InvoiceOrder y no se emite una segunda factura. |
| D18d | Completa con NIF: usa BillingDetails frente a datos del cliente. |
| D19 | Respuesta perdida al facturar: sin segunda factura. |
| D19t | Sin respuesta al facturar: se consulta antes de repetir, sin duplicar. |
| D20 | Aviso y documento enlazados en una sola venta. |
| D21 | Llegan los siete avisos configurados. |
| D21a | Aviso Traspasar de comanda completa: identificadores, origen y destino en el aviso; GetOrder da líneas y estado final correctos. |
| D22 | Aviso repetido o desordenado: sin duplicar ni reabrir. |
| D22a | Aviso TraspasarLinea repetido o desordenado: la línea se mueve una vez, sin duplicar ni perderse. |
| D23a | Trabajador caído con receptor operativo: el aviso espera en el buzón. |
| D23b | Receptor inaccesible: el aviso puede perderse; solo se recupera lo localizable. |
| D23-m | Comanda manual con aviso perdido: aparece por listado o se declara límite. |
| D24a | Alta en caja: el artículo o departamento nuevo llega solo. |
| D24b | Modificación en caja: el cambio de precio o nombre llega solo. |
| D24c | Desactivación en caja: llega como inactivo, no desaparece. |
| D24d | Borrado en caja: llega el borrado, o D32 muestra procedimiento y responsable. |
| D24-p | Plano leído sin mover posiciones. |
| D24-m | Formas de pago y empleados leídos sin crear registros. |
| D25 | Cliente: cambio solo en BDP, solo en web (solo con sobrescritura aprobada) o cambios contradictorios: conflicto visible, nunca sobrescritura silenciosa. |
| D26 | Ajuste web: un solo movimiento, stock coincide. |
| D26a | Consulta unitaria: stock exacto; error con almacén inexistente. |
| D26b | Consulta en lista: pares exactos; artículo inexistente da error solo en su línea; almacén inexistente en otra llamada (error por confirmar). |
| D26c | Regularización: delta correcto; rechaza almacén o motivo inválido. |
| D26d | Traspaso entre almacenes: resta y suma; rechaza no inventariable. |
| D26e | Ajuste delta: suma o resta, con escandallo automático si aplica. |
| D26f | Inventario absoluto: nunca Units null; misma cantidad = diferencia cero sin movimiento. |
| D26g | Sin respuesta en stock: S+V no reintenta; S, un reintento controlado; otro valor, revisión manual. Serializado por artículo y almacén. |
| D27 | Venta de caja converge sin doble descuento. |
| D28 | Releer no duplica facturas, ventas ni albaranes. |
| D28-auto | Exportación programada sin botones: sin huecos ni duplicados. |
| D28-auto-R | Reinicio a mitad de exportación: reanuda por cursor, sin huecos ni duplicados. |
| D28a | Albaranes: importación automática y completa, sin botón. |
| D29 | Reinicio en tres pruebas separadas (proceso de sincronización, WebLinkRestAPI o servicio, BDP.NET): alerta, cola y cursor conservados, sin pérdidas ni duplicados. |
| D30 | Piloto en varios turnos consecutivos: cero duplicados; comandas, cobros, facturas, caja y stock cuadrados; rendimiento dentro del umbral; rollback probado. |
| D31 | Llamada avisa en caja sin dejar estado. |
| D32 | Cada operación fuera de la web (traspasos desde la web, borrados, rectificativas, devoluciones, apertura/cierre, arqueo, efectivo, PDF, reimpresión en cocina, controles avanzados, series, cambios tras pago) muestra procedimiento y responsable. |

Correspondencia con la guía operativa (sus códigos A01–A20 tienen otro significado):

| Guía | Fase D |
| ---- | ------ |
| A01 crear | D03+D03a |
| A02 respuesta perdida al crear | D04 |
| A03 modificar | D05–D08 |
| A04 anular | D09+D10+D09a |
| A05 comanda manual con aviso perdido | D23-m |
| A06 avisos | D21+D22+D21a+D22a |
| A07 trabajador caído | D23a |
| A08 receptor inaccesible | D23b |
| A09+A10 pagos | D13–D17+D13a+D14s+D15t |
| A11 propina | D11a–D11c |
| A12 puntos | D12a–D12c |
| A13 factura | D18–D20+D18a–D18d+D19t |
| A14 traspaso (origen, destino, líneas, estado) | D21a+D22a |
| A15 maestros | D24a–D24d+D24-p+D24-m+D25 |
| A16 exportación programada | D28-auto+D28-auto-R |
| A17 reinicio | D29 |
| A18 stock | D26+D26a–D26g |
| A19 excepciones | D32 |
| A20 piloto | D30 |
| — (detalle, albaranes) | D27+D28+D28a |
| — (llamar) | D31 |

Nota: D21a (comanda completa) y D22a (línea) cubren A06 y A14: el aviso señala el traspaso y `GetOrder` confirma líneas y estado, porque el aviso no trae líneas. Iniciar traspasos desde la web sigue pendiente de BDP. D32 cubre A19 y D24d (borrado sin ruta en la API: mensaje con procedimiento y responsable).

## Qué se logrará

Al terminar la fase D, el día a día quedará así:

- **Lo de caja llega solo.** Son 7 avisos (estado cambiado, crear, modificar, borrar, línea traspasada, traspasar, finalizar); probados 4 (crear al aparcar, modificar al retocar la aparcada, borrar, finalizar al cobrar). Pendientes: estado cambiado y los 2 traspasos, que además se ensayarán duplicados y fuera de orden. El receptor y el trabajador de sincronización funcionarán de forma continua (24 h), con reinicio automático, monitorización y alerta si dejan de responder; como puede caerse, trae buzón: lo que llegue a la web se guarda antes de procesar y se atiende aunque la web se caiga después. Lo que se envíe con el receptor caído se pierde (ya comprobado en esa prueba): el buzón no recupera lo que nunca recibió, y la conciliación recuperará los cambios que puedan localizarse mediante las consultas y exportaciones disponibles. No garantiza descubrir una comanda manual abierta cuyo aviso nunca llegó si BDP 36.2 no ofrece una lista completa de comandas abiertas.
- **Casi sin botones de repaso.** Artículos, clientes, departamentos, empleados, formas de pago y plano se actualizan solos con lecturas programadas (cada 5–15 min, con cursor y control del último éxito); se prueban alta, cambio, desactivación y borrado, no solo la primera carga. Las líneas y pagos de cada ticket se leen después de cada aviso y con la copia de exportación. La exportación automática desde código está pendiente (duda 8); hoy la copia se saca manual. El plano, las formas de pago y los empleados se gestionan en el BDP; las coordenadas gráficas del plano las conserva la web (no demostrado que la API las entregue) y cambiar el nombre a una mesa no mueve su posición.
- **Sin huecos silenciosos (cobertura parcial).** Además del aviso al momento, habrá conciliación por niveles (objetivos iniciales, a ajustar tras medir el equipo real): abiertas conocidas cada 30–60 segundos, cerrados recientes cada 1–5 minutos, maestros cada 5–15 minutos y barrido nocturno amplio, vigilando que no degrade el TPV (se miden CPU, memoria, tiempos y errores). Si falta una relación esperada, se abre incidencia automática. Además, conciliación obligatoria al arrancar, otra antes de abrir el restaurante y otra después del cierre. Si solo pudiera operar en horario de servicio, se declarará como limitación, no como solución completa.
- **Abiertas bajo control parcial.** La web llevará su propia lista de comandas abiertas conocidas (creadas vía web + avisos recibidos) y la comprobará aviso a aviso (modificaciones, traspasos, cierres); los tickets cerrados se cruzan con sus avisos y documentos. Límite formal: una comanda manual de caja con aviso perdido es indetectable hasta su finalización, salvo que BDP confirme la lista de comandas de la 36.2 (pendiente de BDP).
- **Pagos y factura atados.** Pagos con identificador local estable y sin reintento ciego (ante timeout, se consulta antes de repetir; doble clic o pago simultáneo: un solo cobro, prueba D14s). Propina (`Tip`, de la comanda) y puntos (`Loyalty`, del cliente) en flujos separados; cada movimiento de puntos lleva ID local único con saldo anterior y posterior (abono, cargo y pérdida de respuesta: pruebas D12a–D12c). Antes de facturar, la web consulta la comanda y si `GetOrder` devuelve `Status=3` no llama a `InvoiceOrder` (prueba D18c): la protección contra la segunda factura la hace la integración, no el BDP. Cada factura quedará vinculada a su aviso de finalizada guardando el `InvoiceNumber` (`Status 3` con `Order` vacío = ya facturada); el enlace aviso ↔ exportación se cruza por serie, número y fecha. Modificar, cobrar, facturar y anular (con efecto en cocina) lo ya enviado funcionará con prueba completa previa en local.
- **Stock cuadrado.** El BDP es el maestro y la web lleva su propio libro de movimientos (identificador único, artículo, almacén, unidades, motivo, estado y verificación): se registra antes de enviar y se comprueba después, serializando por artículo y almacén. Sin reintento ciego y sin aplicar dos veces el descuento de una venta que el BDP ya procesó (el manual indica que las ventas de caja descuentan stock; falta confirmar con una venta web). Errores de almacén, motivo, no inventariable y escandallo se prueban aparte. Los albaranes de compra entrarán por el perfil de exportación. Queda pendiente que el cliente indique cuál de los 6 almacenes vistos en local es el real, con motivos de movimiento válidos.
- **Compras, propina y camarero.** La propina viajará con el cobro (sumar con `AddTip=true`, sustituir con `AddTip=false` y dejar a cero: pruebas D11a–D11c); llamar al camarero saldrá en caja; los puntos de fidelización van en flujo propio, pendientes de soporte en el terminal.
- **Movimientos registrados.** Quedará constancia de cada aviso (qué llegó, cuándo y qué cambió en cada venta) para revisar descuadres con el cliente delante.
- **En producción, nada sin probar.** Solo se instalará lo ya comprobado en local con la serie de pruebas; primero modo observación (recibir y comparar sin enviar operaciones económicas), luego activación por etapas (maestros, lectura, escritura, pagos/factura, stock al final), con turnos de prueba, parada ante duplicado económico, descuadre fiscal, pérdida de comanda o degradación del TPV, y vuelta atrás probada; el BDP no se actualiza sin el distribuidor y repitiendo pruebas. La verificación final incluirá fallo y recuperación, no solo el camino feliz: aviso duplicado y fuera de orden, caída del trabajador, endpoint inaccesible, timeout con doble clic y reinicio a mitad de proceso. Si BDP publica documentación nueva que contradiga este alcance, prevalece la nueva; se actualiza la matriz y se repiten las pruebas afectadas antes de desplegar.

Seguirán fuera de la web hasta respuesta escrita o procedimiento asignado: borrar artículos, rectificativas y devoluciones, iniciar traspasos desde la web, la operativa de caja (apertura/cierre, arqueo, efectivo, PDF), la reimpresión manual en cocina y los controles avanzados de cocina no reproducibles, las series de facturación, pendientes de clasificar, y los cambios posteriores al pago. El envío a cocina al crear y la anulación con efecto en cocina se probarán en local (pendientes de prueba; `OrderEndType`/`CancelOrder` documentados en el manual, pendientes de prueba local). No es que el BDP "no tenga" esas operaciones: es que la API entregada no las expone (pendiente de BDP).

## Fuera del alcance

- **Borrar artículos, rectificativas y devoluciones**: sin ruta en la API entregada (pendiente de BDP); se sigue haciendo a mano.
- **Administrar plano, formas de pago y empleados**: lectura automática desde la web, gestión en el BDP.
- **Reservas, gastos, marketing**: son solo web, ya funcionan, no necesitan al BDP.

## Operativa diaria (quién hace qué hasta respuesta de BDP)

| Operación                                    | Estado                                                                      |
| -------------------------------------------- | --------------------------------------------------------------------------- |
| Apertura/cierre turno-caja, arqueo, efectivo | Excepción obligatoria en BDP/TPV, realizada por [responsable real: pendiente confirmar con Guillermo y el propietario] |
| Factura simplificada al cobrar               | En caja sale facturada, no se re-factura; pago web con `Invoice = true` pendiente de prueba   |
| Facturación separada de una comanda existente y todavía no facturada mediante InvoiceOrder | Documentado mediante InvoiceOrder; pendiente de prueba local con comanda pendiente de facturar (no aplicar sobre ya facturada: riesgo de doble factura); confirmar si necesita suscripción (roadmap: pendiente de suscripción) |
| Factura completa con NIF                     | Documentado en el manual (`InvoiceParameters`, campo `BillingDetails`); pendiente de prueba local con la configuración real del terminal     |
| Pago parcial (no factura todavía; el manual solo admite Invoice=true con pago total)            | Pendiente de prueba                                                                          |
| Timeout / respuesta incierta en cobro o factura | Pendiente de prueba en pago web, InvoiceOrder y timeouts (sin duplicar cobro ni factura: consultar antes de repetir)            |
| Rectificación, devolución                    | Excepción: en BDP ([responsable real: pendiente confirmar con Guillermo y el propietario])   |
| PDF de factura                               | Excepción: desde BDP ([responsable real: pendiente confirmar con Guillermo y el propietario]) |
| Envío a cocina al crear                      | Documentado en el manual (`OrderEndType = 0`: crea, aparca y envía a cocina); pendiente de prueba local con terminal e impresoras reales |
| Anulación con efecto en cocina               | Documentado en el manual (`CancelOrder`); pendiente de prueba local (comprobar qué se imprime). Contradicción: el roadmap (l.118) lo da por verificado el 18/09; decidir cuál prevalece  |
| Reimpresión manual en cocina                 | Excepción: en TPV (personal de sala); incidencias a administración                           |
| Cambios tras cobro                           | Excepción por diseño (factura cerrada; vía rectificativa)                                    |
| Series de facturación                        | Excepción: en BDP ([responsable real: pendiente confirmar con Guillermo y el propietario])   |
| Mesa/barra/para llevar, llamar camarero      | Parcial (pendiente prueba de traspaso e inicio desde web, pendiente de BDP)                  |

Las excepciones las validan Guillermo y el propietario por escrito antes del piloto, indicando quién realiza cada una; cuando la web no soporte una acción, mostrará un mensaje claro con el procedimiento. Preguntar a BDP si existe procedimiento oficial para apertura/cierre, arqueo y entradas o salidas de efectivo antes de darlo por fuera de la web. Decisión pendiente antes del piloto (D30, objetivo: sin excepciones sin dueño): Guillermo y el propietario indicarán por escrito quién realiza cada excepción —apertura/cierre, arqueo, efectivo, rectificativas, devoluciones, PDF y series—; sin esa asignación no se abre el piloto. No bloquea T2. Cierre de Guillermo (5ª tanda): cada ficha se corrige antes de ejecutarla y solo entonces se da por aprobada; D30 y D32 completas antes del piloto.

## Dudas que quedan

Todas se prueban en un entorno local con la misma versión de BDP.NET (distinta instalación y configuración). Hasta ahora no se hicieron: parte por falta de tiempo y parte porque faltan respuestas de BDP, datos del distribuidor y la base T2. Se harán al aprobarse la nueva fase. En producción no se experimenta: hay piloto controlado (observación, etapas, parada y vuelta atrás). Antes del piloto se comparará y documentará la configuración real (series, terminales, perfiles de artículos, empleados, formas de pago, almacenes, motivos de stock, impresoras, configuración de cocina, avisos, suscripciones y perfiles de exportación); las pruebas que dependan de esa configuración se repetirán en el piloto.

1. **Traspasar**: falta crear un plano de prueba en local para provocarlo. Los traspasos de mesa no aplican si el cliente solo trabaja en barra; los de línea y para llevar sí se prueban. Iniciar traspasos desde la web, pendiente de BDP.
2. **Reenviar una comanda corregida**: se pide el ejemplo oficial a BDP y se prueba solo mirando, en local (¿sustituye o mezcla?, campo `Lin`, cocina).
3. **Lista de comandas abiertas**: la anuncia el portal para 36.2 pero sin ruta ni contrato en el manual. Pregunta escrita a BDP; si no existe o es parcial, queda como límite formal (prueba D23-m).
4. **Carga y pago doble**: en local (con serie de pruebas: en pruebas no importa que no se pueda deshacer). Con identificador de pago estable y consulta antes de repetir (pruebas D15t, D19t); doble clic o dos pagos simultáneos con un solo cobro (prueba D14s).
5. **Cobrar, facturar, llamar y anular**: probar en local cada operación, con la suscripción activa. Factura completa con NIF e `Invoice` aislado, pendientes de prueba (subpruebas D18a–D18d, D13a, D03a, D09a); anular con efecto en cocina.
6. **Propina y puntos**: propina sumar (`AddTip=true`), sustituir (`AddTip=false`) y dejar a cero (pruebas D11a–D11c); puntos positivos, negativos y con pérdida de respuesta, sin duplicar (pruebas D12a–D12c); puntos en flujo propio, pendientes de soporte en el terminal. `AddPoints` reintentado puede duplicar: prohibido el reintento ciego.
7. **Datos reales del distribuidor**: almacén, motivos, series, perfiles, avisos y suscripción. Sin ellos no se instala en producción.
8. **Exportación automática**: invocar `ExportProfiles/Documents` desde código, probar `OnlyLastMovements` true/false (ambiguo en el manual) y ventanas solapadas con cursor, sin pulsar botones (prueba D28-auto); reinicio a mitad con reanudación por cursor (prueba D28-auto-R).
9. **Enlace factura ↔ exportación**: cruzar aviso de finalizada + `Status 3` + `InvoiceNumber` + documento exportado; incidencia automática si falta la relación.
10. **Base previa (T2)**: las 6 tablas (inbox, outbox, mapeo de IDs, cursor, pendientes agotados, libro de stock) no existen en código; sin ellas no empiezan las pruebas de aceptación.
11. **Duplicado y fuera de orden**: cada uno de los 7 avisos se ensayará normal, duplicado y fuera de orden (matriz D22 y D22a); sin ese ensayo ningún evento se da por cerrado.
12. **Stock (respuestas de BDP, 5ª tanda)**: artículo inexistente dentro de una llamada válida; almacén inexistente; efecto de `Units` null; si `GetStock` es fiable tras escribir. Todo se confirma en BDP real antes de dar por aprobadas D26b, D26f y D26g.

## Lo que descubrimos con el TPV de pruebas (01–02/10)

Entorno propio: portátil con la misma versión BDP.NET 36.2 licenciada (distinta instalación y configuración), serie de pruebas `PRUEBA`, sin tocar ventas reales.

- **La caja ya avisa sola** (4 de 7 avisos en real): crear al aparcar, finalizar al cobrar, borrar y modificar al retocar la aparcada. Pendientes: estado cambiado y los 2 traspasos. La comanda se registra al aparcar, no al añadir líneas.
- **Los avisos son solo la señal** (en lo probado hasta ahora: no traen líneas ni pagos); traen el identificador (crear/modificar/borrar), el estado (estado cambiado) y Tipo+Serie+Número al finalizar. De los traspasos se espera origen y destino (pendiente de ver en real). La web lee el detalle después. `Status 3` con `Order` vacío = ya facturada.
- **Lo que no llega, se pierde**: con el receptor apagado, el aviso caído no se reintentó en esa prueba. El receptor tiene que funcionar de forma continua, con reinicio automático.
- **La copia del ticket trae todo**: con el perfil de exportación configurado (campos todos marcados), salen cabeceras, líneas y pagos en texto separado por `|`. Comprobado con los tickets de prueba.
- **El ticket de factura simplificada YA es la factura**: en caja sale facturada al cobrar; allí no hay que facturarla después. El paso aislado `InvoiceOrder` (facturar desde la web una comanda existente y todavía no facturada) está pendiente de prueba (duda 5).
- **La vista Salón del TPV necesita plano**: sin diseño de salones, no se abre.
- **Serie de pruebas `PRUEBA`**: el TPV la conserva al abrir turno; para cambiarla hay que reabrir turno.
- **La exportación instantánea queda en cola sin generar fichero**: la copia se saca con exportación manual por fecha. La automática desde código está pendiente.
