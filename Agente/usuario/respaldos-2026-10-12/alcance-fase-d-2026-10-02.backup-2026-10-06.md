# Fase D: que lo de caja llegue solo a la web — alcance y estado (02/10/2026, rev. 05/10/2026 tras la guía de Guillermo)

## Objetivo

Hoy casi todo va con botón: traer datos a la web, enviar comandas al BDP, y lo que se hace en caja hay que ir a buscarlo. La fase D quita los botones: **casi todo viaja solo en ambos sentidos** — lo de la web entra al BDP y lo de caja llega a la web al momento, salvo las excepciones que se listan más abajo.

Se prueba todo en el TPV de pruebas (mismo programa) y en producción solo se instala lo ya comprobado.

## Antes de la fase D (hoy, con botones)

Lectura: Se trae = del BDP a la web; Se envía = de la web al BDP; Automático = viaja solo; Sí = incluido; Pendiente = falta un paso; Pendiente de BDP = falta respuesta o contrato de BDP; — = no aplica.

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

| Funcionalidad                                                    |     Se trae      |     Se envía     |         Automático          |
| ---------------------------------------------------------------- | :--------------: | :--------------: | :-------------------------: |
| Catálogo (artículos, precio, familia)                            |        Sí        |        Sí        |             Sí              |
| Clientes                                                         |        Sí        |        Sí        |             Sí              |
| Plano de sala                                                    |        Sí        |        —         |         Lectura sí          |
| Departamentos                                                    |        Sí        |        Sí        |             Sí              |
| Formas de pago, empleados                                        |        Sí        |        —         |         Lectura sí          |
| Albaranes de compra | Sí | — | Por exportación |
| Detalle de comanda / stock                                       |        Sí        |        —         |             Sí              |
| Crear comanda                                                    |        Sí        |        Sí        |             Sí              |
| Propina (en comanda) | — | Sí | Sí |
| Puntos fidelización (cliente) | — | Sí | Sí |
| Anular comanda | Sí | Sí | Sí |
| Modificar comanda enviada | Sí | Sí | Sí |
| Traspasar (mesa / líneas) | Sí | — | Sí |
| Cobrar comanda existente | Sí | Sí | Sí |
| Facturar | Sí | Sí | Sí |
| Mover stock (v1: consulta, regularización, traspaso, inventario) | — | Sí | Sí |
| Llamar al camarero | — | Sí | Sí |
| Borrar artículos                                                 | Pendiente de BDP | Pendiente de BDP |             No              |
| Reservas, gastos, marketing                                      |        —         |        —         |              —              |

## Qué se logrará

Al terminar la fase D, el día a día quedará así:

- **Lo de caja llega solo.** Son 7 avisos (estado cambiado, crear, modificar, borrar, línea traspasada, traspasar, finalizar); probados 4 (crear al aparcar, modificar al retocar la aparcada, borrar, finalizar al cobrar). Pendientes: estado cambiado y los 2 traspasos, que además se ensayarán duplicados y fuera de orden. El receptor estará encendido en horario de servicio, con reinicio automático; como puede caerse, trae buzón: lo que llegue a la web se guarda antes de procesar y se atiende aunque la web se caiga después. Lo que se envíe con el receptor caído se pierde (ya comprobado en esa prueba): el buzón no recupera lo que nunca recibió y lo perdido lo descubre la conciliación.
- **Casi sin botones de repaso.** Artículos, clientes, departamentos, empleados, formas de pago y plano se actualizan solos con lecturas programadas (cada 5–15 min, con cursor y control del último éxito); se prueban alta, cambio, desactivación y borrado, no solo la primera carga. Las líneas y pagos de cada ticket se leen después de cada aviso y con la copia de exportación. La exportación automática desde código está pendiente (duda 8); hoy la copia se saca manual. El plano, las formas de pago y los empleados se gestionan en el BDP; las coordenadas gráficas del plano las conserva la web (no demostrado que la API las entregue) y cambiar el nombre a una mesa no mueve su posición.
- **Sin huecos silenciosos (cobertura parcial).** Además del aviso al momento, habrá conciliación por niveles (objetivos iniciales, a ajustar tras medir el equipo real): abiertas conocidas cada 30–60 segundos, cerrados recientes cada 1–5 minutos, maestros cada 5–15 minutos y barrido nocturno amplio, vigilando que no degrade el TPV (se miden CPU, memoria, tiempos y errores). Si falta una relación esperada, se abre incidencia automática.
- **Abiertas bajo control parcial.** La web llevará su propia lista de comandas abiertas conocidas (creadas vía web + avisos recibidos) y la comprobará aviso a aviso (modificaciones, traspasos, cierres); los tickets cerrados se cruzan con sus avisos y documentos. Límite formal: una comanda manual de caja con aviso perdido es indetectable hasta su finalización, salvo que BDP confirme la lista de comandas de la 36.2 (pendiente de BDP).
- **Pagos y factura atados.** Pagos con identificador local estable y sin reintento ciego (ante timeout, se consulta antes de repetir). Propina (`Tip`, de la comanda) y puntos (`Loyalty`, del cliente) en flujos separados; cada movimiento de puntos lleva ID local único con saldo anterior y posterior. Cada factura quedará vinculada a su aviso de finalizada guardando el `InvoiceNumber` (`Status 3` con `Order` vacío = ya facturada); el enlace aviso ↔ exportación se cruza por serie, número y fecha. Modificar, cobrar, facturar y anular (con efecto en cocina) lo ya enviado funcionará con prueba completa previa en local.
- **Stock cuadrado.** El BDP es el maestro y la web lleva su propio libro de movimientos (identificador único, artículo, almacén, unidades, motivo, estado y verificación): se registra antes de enviar y se comprueba después, serializando por artículo y almacén. Sin reintento ciego y sin aplicar dos veces el descuento de una venta que el BDP ya procesó (pendiente confirmar con BDP si la venta descuenta). Errores de almacén, motivo, no inventariable y escandallo se prueban aparte. Los albaranes de compra entrarán por el perfil de exportación. Queda pendiente que el cliente indique cuál de los 6 almacenes vistos en local es el real, con motivos de movimiento válidos.
- **Compras, propina y camarero.** La propina viajará con el cobro (pendiente probar sumar frente a sustituir); llamar al camarero saldrá en caja; los puntos de fidelización van en flujo propio, pendientes de soporte en el terminal.
- **Movimientos registrados.** Quedará constancia de cada aviso (qué llegó, cuándo y qué cambió en cada venta) para revisar descuadres con el cliente delante.
- **En producción, nada sin probar.** Solo se instalará lo ya comprobado en local con la serie de pruebas; primero modo observación (recibir y comparar sin enviar operaciones económicas), luego activación por etapas (maestros, lectura, escritura, pagos/factura, stock al final), con turnos de prueba, parada ante duplicado económico, descuadre fiscal, pérdida de comanda o degradación del TPV, y vuelta atrás probada; el BDP no se actualiza sin el distribuidor y repitiendo pruebas. La verificación final incluirá fallo y recuperación, no solo el camino feliz: aviso duplicado y fuera de orden, caída del trabajador, endpoint inaccesible, timeout con doble clic y reinicio a mitad de proceso. Si BDP publica documentación nueva que contradiga este alcance, prevalece la nueva; se actualiza la matriz y se repiten las pruebas afectadas antes de desplegar.

Seguirán fuera de la web hasta respuesta escrita o procedimiento asignado: borrar artículos, rectificativas y devoluciones, iniciar traspasos desde la web, la operativa de caja (apertura/cierre, arqueo, efectivo, reimpresión, PDF), la cocina (envío/anulación) y las series de facturación, pendientes de clasificar. No es que el BDP "no tenga" esas operaciones: es que la API entregada no las expone (pendiente de BDP).

## Fuera del alcance

- **Borrar artículos, rectificativas y devoluciones**: sin ruta en la API entregada (pendiente de BDP); se sigue haciendo a mano.
- **Administrar plano, formas de pago y empleados**: lectura automática desde la web, gestión en el BDP.
- **Reservas, gastos, marketing**: son solo web, ya funcionan, no necesitan al BDP.

## Operativa diaria (quién hace qué hasta respuesta de BDP)

| Operación                                    | Estado                                                                      |
| -------------------------------------------- | --------------------------------------------------------------------------- |
| Apertura/cierre turno-caja, arqueo, efectivo | Excepción: en TPV (administración/distribuidor)                             |
| Factura simplificada al cobrar               | Pendiente de prueba (`Invoice` aislado)                                     |
| Factura completa con NIF                     | Pendiente de prueba                                                         |
| Rectificación, devolución                    | Excepción: en BDP (administración)                                          |
| PDF de factura                               | Excepción: desde BDP (administración)                                       |
| Cocina (envío/anulación), reimpresión        | Excepción: en TPV (personal de sala); incidencias a administración          |
| Cambios tras cobro                           | Excepción por diseño (factura cerrada; vía rectificativa)                   |
| Series de facturación                        | Excepción: en BDP (administración)                                          |
| Mesa/barra/para llevar, llamar camarero      | Parcial (pendiente prueba de traspaso e inicio desde web, pendiente de BDP) |

Las excepciones las validan Guillermo y el propietario por escrito antes del piloto; cuando la web no soporte una acción, mostrará un mensaje claro con el procedimiento.

## Dudas que quedan

Todas se prueban en el TPV de pruebas, que es el mismo programa. Hasta ahora no se hicieron: parte por falta de tiempo y parte porque faltan respuestas de BDP, datos del distribuidor y la base T2. Se harán al aprobarse la nueva fase. En producción no se experimenta: hay piloto controlado (observación, etapas, parada y vuelta atrás).

1. **Traspasar**: falta crear un plano de prueba en local para provocarlo. Los traspasos de mesa no aplican si el cliente solo trabaja en barra; los de línea y para llevar sí se prueban. Iniciar traspasos desde la web, pendiente de BDP.
2. **Reenviar una comanda corregida**: se pide el ejemplo oficial a BDP y se prueba solo mirando, en local (¿sustituye o mezcla?, campo `Lin`, cocina).
3. **Lista de comandas abiertas**: la anuncia el portal para 36.2 pero sin ruta ni contrato en el manual. Pregunta escrita a BDP; si no existe o es parcial, queda como límite formal.
4. **Carga y pago doble**: en local (con serie de pruebas: en pruebas no importa que no se pueda deshacer). Con identificador de pago estable y consulta antes de repetir.
5. **Cobrar, facturar, llamar y anular**: probar en local cada operación, con la suscripción activa. Factura completa con NIF e `Invoice` aislado, pendientes de prueba; anular con efecto en cocina.
6. **Propina y puntos**: propina pendiente de probar sumar frente a sustituir; puntos en flujo propio, pendientes de soporte en el terminal. `AddPoints` reintentado puede duplicar: prohibido el reintento ciego.
7. **Datos reales del distribuidor**: almacén, motivos, series, perfiles, avisos y suscripción. Sin ellos no se instala en producción.
8. **Exportación automática**: invocar `ExportProfiles/Documents` desde código, probar `OnlyLastMovements` true/false (ambiguo en el manual) y ventanas solapadas con cursor, sin pulsar botones.
9. **Enlace factura ↔ exportación**: cruzar aviso de finalizada + `Status 3` + `InvoiceNumber` + documento exportado; incidencia automática si falta la relación.
10. **Base previa (T2)**: las 6 tablas (inbox, outbox, mapeo de IDs, cursor, pendientes agotados, libro de stock) no existen en código; sin ellas no empiezan las pruebas de aceptación.
11. **Duplicado y fuera de orden**: cada uno de los 7 avisos se ensayará normal, duplicado y fuera de orden (matriz A06); sin ese ensayo ningún evento se da por cerrado.

## Lo que descubrimos con el TPV de pruebas (01–02/10)

Entorno propio: portátil con TPV real v36.2 licenciado, serie de pruebas `PRUEBA`, sin tocar ventas reales.

- **La caja ya avisa sola** (4 de 7 avisos en real): crear al aparcar, finalizar al cobrar, borrar y modificar al retocar la aparcada. Pendientes: estado cambiado y los 2 traspasos. La comanda se registra al aparcar, no al añadir líneas.
- **Los avisos son solo la señal** (en lo probado hasta ahora: no traen líneas ni pagos); traen el identificador (crear/modificar/borrar), el estado (estado cambiado) y Tipo+Serie+Número al finalizar. De los traspasos se espera origen y destino (pendiente de ver en real). La web lee el detalle después. `Status 3` con `Order` vacío = ya facturada.
- **Lo que no llega, se pierde**: con el receptor apagado, el aviso caído no se reintentó en esa prueba. El receptor tiene que estar encendido en horario de servicio, con reinicio automático.
- **La copia del ticket trae todo**: con el perfil de exportación configurado (campos todos marcados), salen cabeceras, líneas y pagos en texto separado por `|`. Comprobado con los tickets de prueba.
- **El ticket de factura simplificada YA es la factura**: en caja sale facturada al cobrar; allí no hay que facturarla después. El paso aislado `Invoice` (facturar desde la web una comanda ya cerrada) está pendiente de prueba (duda 5).
- **La vista Salón del TPV necesita plano**: sin diseño de salones, no se abre.
- **Serie de pruebas `PRUEBA`**: el TPV la conserva al abrir turno; para cambiarla hay que reabrir turno.
- **La exportación instantánea queda en cola sin generar fichero**: la copia se saca con exportación manual por fecha. La automática desde código está pendiente.
