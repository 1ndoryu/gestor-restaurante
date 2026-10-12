# Integración web–BDP: estado actual y propuesta de fase D (02/10/2026)

Cuarta revisión: incorpora los resultados de las pruebas con TPV real (01–02/10/2026). Copia de seguridad de la versión anterior (21/09): `borrador-revision-guia-bidireccional-2026-09-19.backup-2026-10-02.md`.

> **Novedad 02/10: ya tenemos entorno de pruebas propio.** Su portátil ejecuta el TPV real (v36.2) con licencia (`122417` + `751080-84`). Todo lo que antes estaba "pendiente de solicitar a BDP" se comprobó allí, sin tocar ventas reales: serie de pruebas `PRUEBA` creada, 6 tickets de prueba cobrados (serie `00001TB` nº 1–3 y serie `PRUEBA` nº 1–3) y el receptor de la web leyendo la caja probado con éxito.

> **Novedad 02/10 (tarde): la caja ya avisa sola a la web.** Con la configuración de demostración de BDP y su programa de ejemplo en el propio portátil, se comprobó que al trabajar en caja llegan solos los avisos de crear (al aparcar), finalizar (al cobrar), borrar y modificar (al añadir línea a la aparcada), con las comandas 7, 8, 9 y 10 de la serie `00001TB`. Detalle útil: la comanda se registra al aparcar el ticket, no al añadir la línea. Queda por ver el aviso de traspasar en real: el TPV de pruebas no tiene mesas ni plano (el botón Salón no abre; se probó crear el rango 91–92 por aplicación y se revirtió). Su formato ya se conoce por el ejemplo de BDP (comanda origen y destino, salón y mesa origen y destino; 0 = barra) y usa el mismo mecanismo que los otros cuatro, ya comprobados. Se probará con el distribuidor en el terminal real, que sí tiene plano; si el cliente solo trabaja en barra, este aviso no aplica.
> **Novedad 02/10 (noche): perfil de exportación probado en local.** Se creó el perfil 1 (`Fra. Simpl. Barra`, terminal 1–1, texto a carpeta) y se configuró la exportación instantánea (terminal 1, perfil 1, a fichero). Al cobrar, la caja apunta el trabajo en la cola (`ExportacionInstantaneaTickets` con 1 fila) pero no escribe el fichero ni reiniciando el programa. Pendiente: ver si sale al cierre o con la exportación manual. Turno 1 cerrado el 02/10 de noche y repetida la exportación manual: sigue sin escribir fichero (solo se actualiza el registro de sesiones). Falta configurar el detalle de campos por línea (`Config_Export_Lineas` está vacío): eso lo enseña el distribuidor en 5 minutos o se mira otro día.
> **Novedad 02/10 (noche): exportación conseguida sin distribuidor.** La ayuda del programa explica que al marcar cada documento se abre una pestaña para elegir campos (botón T = todos). Con todos marcados, la exportación manual genera por terminal y fecha: cabeceras (`_CTB_...`), líneas (`_LTB_...`) y pagos (`_PTB_...`) en texto separado por `|`. Comprobado con los tickets de prueba (serie `00001TB` nº 1–10 y `PRUEBA` nº 1–3, con totales, bases de IVA y pagos). La copia del ticket trae todo lo que la web necesita.

Este documento resume en qué punto está la integración entre la web y el BDP: qué funciona hoy con botón, por qué algunas cosas aún no llegan solas (con los propios documentos de BDP como referencia), qué habría que confirmar con pruebas en el terminal y qué incluiría la siguiente fase para que lo de caja llegue solo a la web. Al final encontrará las decisiones que necesito de su parte.

> **Cómo leer esta tabla.** Importar = traer datos del BDP a la web (lo hace usted con un botón). Exportar = enviar datos de la web al BDP (lo hace usted con un botón). Dos direcciones = lo que se toque en la caja llega solo a la web, sin tocar nada.
> Sí = comprobado en real. Pendiente = documentado o probado en parte; falta un paso (pago, dato o prueba). No = el BDP no lo permite. — = no aplica.

| Funcionalidad                         | Importar  | Exportar  |  Dos direcciones  | Estado                                                    |
| ------------------------------------- | :-------: | :-------: | :---------------: | --------------------------------------------------------- |
| Catálogo (artículos, precio, familia) |    Sí     |    Sí     | Automático fase D | Comprobado el 18/09; altas nuevas en pausa                |
| Clientes                              |    Sí     |    Sí     | Automático fase D | Con vista previa antes de aplicar                         |
| Plano de sala                         |    Sí     |     —     |        No         | 7 salones, 87 mesas traídos a la web                      |
| Departamentos                         |    Sí     |    Sí     | Automático fase D | Datos de referencia, repaso eventual                      |
| Formas de pago (lectura)              |    Sí     |     —     |        No         | Se consultan, se administran en el BDP                    |
| Empleados (lectura)                   |    Sí     |     —     |        No         | 3 empleados                                               |
| Albaranes de compra                   | Pendiente |     —     |        No         | Se pueden traer; falta indicar el perfil                  |
| Detalle de comanda / stock            | Pendiente |     —     |        No         | Faltaba suscripción / almacén en el terminal; el 02/10 se vio que el TPV trae 6 almacenes (ALMACÉN GENERAL, BAR, CAFETÍN, COCINA, LAVANDERÍA, BAJAS) |
| Crear comanda (cobrada o no)          |     —     |    Sí     | Pendiente fase D  | Comprobado el 02/10 en TPV real: 6 tickets de prueba cobrados (serie `00001TB` nº 1–3 y serie `PRUEBA` nº 1–3) |
| Propina / puntos                      |     —     |    Sí     | Automático fase D | Propina comprobada en real; puntos, pendiente de terminal |
| Anular comanda                        |     —     |    Sí     | Pendiente fase D  | Anulada por la aplicación y registrada                    |
| Modificar comanda enviada             |     —     | Pendiente | Pendiente fase D  | Posible con pago; semántica por confirmar                 |
| Traspasar (mesa / líneas)             |     —     |     —     | Pendiente fase D  | Aviso documentado, pendiente fase D                       |
| Cobrar comanda existente              |     —     | Pendiente |  Parcial fase D   | Solo con suscripción de pago                              |
| Facturar                              |     —     | Pendiente |  Parcial fase D   | Aclarado el 02/10 en TPV real: el ticket de factura simplificada (series `00001TB` y `PRUEBA`) YA es la factura; no hay paso de "facturar" posterior. Crear y facturar a la vez es posible en gratis |
| Mover stock                           |     —     | Pendiente | Automático fase D | El TPV trae 6 almacenes; falta decidir cuál es el real con el distribuidor |
| Llamar al camarero                    |     —     | Pendiente |        No         | Solo con suscripción de pago                              |
| Borrar artículos                      |    No     |    No     |        No         | El BDP no lo permite, solo desactivar                     |
| Reservas, gastos                      |     —     |     —     |         —         | Solo web, funcionan siempre                               |
| Menús / packs                         |    Sí     |     —     |        No         | Solo consulta de la definición del BDP                    |
| Marketing, empleados web, turnos/caja |     —     |     —     |         —         | Solo web, funcionan siempre                               |

## Por qué algunas cosas no funcionan en dos direcciones

Todo lo siguiente sale de los propios documentos de BDP, no es una limitación de la web:

- **Catálogo, clientes, plano:** el documento _Avisos de Acciones_ de BDP solo contempla avisos de comandas (crear, modificar, cambiar estado, borrar, finalizar y traspasar). No existe aviso para artículos, clientes, salones o stock, así que cuando algo cambia allí la web tiene que ir a traerlo. Hoy funciona con el botón Importar de cada pantalla y el botón Exportar para enviar; cada operación la confirma usted. En la fase D ese repaso pasaría a ser automático, sin botón. La versión gratuita ya permite estos comandos, así que esta parte se puede adelantar sin esperar a la de pago.
- **Stock:** el propio documento _Suscripción extendida para WordPress_ de BDP dice que el stock fuera del BDP "no está conectado en tiempo real" (esa frase habla del módulo de WordPress, no del almacén del BDP). El manual sí trae funciones propias de almacén (regularizaciones, traspasos y movimientos), y la gratuita no las bloquea; falta probarlo contra el terminal real, donde hoy no hay un almacén válido creado.
- **Pagos cobrados en caja:** entre los datos que un aviso puede enviar (identificador de comanda, hora, estado, documento, salón y mesa en traspasos) no hay ninguno de pago. Por eso la web tiene que ir a preguntar. Hoy funciona con el botón Cobrar de cada venta, que verifica el estado antes de cobrar. Añadir pagos a una comanda existente sí exige la suscripción de pago. El 02/10 se comprobó además que el BDP por sí solo no avisa a nadie: la web tiene que leer la caja cada pocos segundos (probado con éxito en el portátil: lee cada 3 segundos y detecta el ticket nuevo al momento). Regla de trabajo: lo que la API no deje leer se lee directo de la base de datos, que es del propio terminal.
- **Facturas:** el aviso de comanda finalizada trae el tipo, la serie y el número del documento, pero no el documento en sí; el contenido se recupera con el perfil de documentos. La factura la emite el BDP y la web guarda el número. Crear y facturar a la vez desde la web es posible incluso en gratuita si los pagos cuadran; facturar una comanda existente exige la suscripción de pago.
- **Borrar y modificar lo enviado:** el manual _WebLink REST API_ no incluye operación de borrado de artículos (solo crear y modificar). Para comandas la capacidad de modificar con pago está documentada, pero no su semántica exacta: no se programará el reenvío como contrato hasta que BDP dé el ejemplo oficial y las pruebas lo demuestren. Mientras tanto, lo enviado se anula y se crea de nuevo, no se edita.
- **Avisos y camarero:** los avisos se configuran con una dirección web (ruta base, ruta por aviso y método), no con una IP de mensajes. Llamar al camarero es una operación aparte que exige la suscripción de pago.

## Lo que falta por confirmar (con pruebas en el terminal)

Ordenado por importancia. Las pruebas del 01–02/10 se hicieron en su portátil con el TPV real licenciado, sin tocar ventas reales (6 tickets de prueba, serie `PRUEBA` creada para ello). Nada toca ventas reales ni la caja del día. Lo ya comprobado se marca con ✅:

1. **¿El aviso de "modificada" incluye pagos?** Comprobado el 02/10: no, ningún aviso trae líneas ni pagos. El de finalizar de la comanda 10 (café + capuchino, ticket `00001TB` nº 7) trae solo el identificador y la referencia del ticket (`TipoDocumento`, `Serie`, `Numero`). Los avisos son solo la señal; la web lee después la comanda completa y repasará cada cierto tiempo lo que cambie sin aviso.
2. **¿Cómo responde el BDP al reenviar una comanda corregida?** La capacidad existe con pago, pero el manual no dice si sustituye las líneas o las suma, ni si acepta cambios en comandas ya pagadas. Se pedirá el ejemplo oficial a BDP y se probará por separado añadir línea, cambiar unidades y retirar línea, solo mirando. Riesgo nulo. Solo se programará lo demostrado; si el resultado no es limpio, se mantiene lo actual: anular y crear de nuevo.
3. **Si un aviso se pierde, ¿llega después?** Comprobado el 02/10: no. Con el receptor apagado se aparcó la comanda 11 y al cobrarla solo llegó `Finalizar`, nunca `Crear`. Lo que no llega, se pierde. El receptor tiene que estar siempre encendido, con rearranque automático, y la web repasará cada cierto tiempo para recuperar lo que la API permita descubrir.
4. **¿Qué datos trae el perfil de exportación?** El manual no describe sus campos. Prueba: configurarlo con el distribuidor y revisar lo que llega. Riesgo nulo. Si viene incompleto, lo avisaremos antes de usarlo.
5. **¿Se puede pedir la lista de comandas abiertas?** No hay operación documentada. Prueba: intentar pedirla y ver qué responde. Riesgo nulo. Si no se puede, una comanda creada en caja quedará invisible hasta finalizarse.
6. **¿Cuántas consultas aguanta el terminal sin ir lento?** Ningún documento da límites. Se fijarán límites prudentes en desarrollo y, en el terminal real, una prueba mínima de noche con umbral de parada (si se nota lento, se para). Nunca se forzará hasta que vaya lento. Mientras tanto se usará frecuencia baja para no afectar al cobro.
7. **Pagos, facturas y series.** ✅ Parcialmente comprobado el 02/10: la serie de pruebas `PRUEBA` se crea y se asigna al terminal sin problema (los tickets salen con esa serie y su numeración propia); el ticket de factura simplificada YA es la factura, sin paso posterior. Queda por probar con BDP: qué pasa si un pago se envía dos veces y cómo enlazar el aviso de finalizada con la factura real. Prueba en el portátil primero; en real, una sola vez, juntos y con procedimiento aprobado, cuando usted decida. Un pago de prueba, aunque sea de céntimos, no se puede borrar ni devolver, y la factura real no se puede deshacer. Riesgo alto.
8. **Rectificativas y devoluciones:** no existen como operación en ningún documento; se hacen a mano en el BDP. Quedan fuera de esta fase.

## Respuesta a su guía 2: aceptamos sus correcciones

He comprobado los 15 puntos de su guía en los manuales y lleva razón en todos:

- **Licencia:** la gratuita permite artículos, clientes, departamentos, stock, perfiles, creación y anulación; la de pago (84) se necesita para detalle de comanda, modificar existente, pagos y propinas en existente, facturar existente y camarero. Esa parte se adelantará sin esperar al pago.
- **Pruebas seguras:** pagos y facturas primero en desarrollo; sin cortes de red ni saturación en el terminal abierto.
- **Códigos reales:** se usará el almacén, los motivos, las series y los perfiles reales que indique el distribuidor, nunca los de los ejemplos.
- **Canal técnico:** las preguntas de funcionamiento van a BDP por su canal; el distribuidor configura licencias, almacenes, perfiles, series y avisos.
- **Matices:** departamentos, propina y puntos con vuelta eventual; albaranes importables; reservas y gastos solo web; camarero separado; avisos por dirección web.

## Fase D por hitos

El plazo empezará cuando estén los accesos y las respuestas imprescindibles de BDP; el tiempo esperando a terceros contará aparte. La licencia y el entorno de pruebas ✅ ya están (portátil con TPV 36.2 licenciado `122417` + `751080-84`, serie `PRUEBA`, receptor probado). Cada hito se acepta con su prueba superada, sin duplicados y con evidencia. Estaré atenta a las respuestas que le den a sus preguntas.

**Hito 1 — Alcance, secretos y base.** Matriz de operaciones firmada; cambio de contraseñas y secretos fuera del código; comprobación de versión, servicio 84 y acceso (✅ TPV 36.2 licenciado verificado el 01/10); códigos reales y perfiles con el distribuidor (✅ serie de pruebas `PRUEBA` creada y asignada al terminal el 02/10; queda elegir almacén real entre los 6 existentes). Pruebas D01–D02.

**Hito 2 — Comandas y avisos.** Receptor siempre encendido y buzón; crear comandas con identidad estable; actualización solo de lo demostrado; anulación; traspasos reflejados. Pruebas D03–D10, D03a, D09a, D21–D22, D21a, D22a.

**Hito 3 — Pagos, propina, puntos y factura.** Pagos sin reintento ciego (máximo 3); propina y puntos con lectura posterior; factura única con clave fiscal y documento por perfil. Pruebas D11–D20, D13a, D15t, D18a–D18d, D19t.

**Hito 4 — Maestros, stock y documentos.** Catálogo, clientes y departamentos sin botón; stock con el BDP como referencia y sin doble descuento; documentos y albaranes por perfil. Pruebas D24–D28, D24-p, D24-m, D26a–D26g, D28-auto, D28a.

**Hito 5 — Fallos, piloto y producción.** Receptor caído simulado, reinicios y tiempos de espera sin duplicados; reconciliación de comandas conocidas; turno piloto con caja cuadrada; vuelta a solo lectura si hay riesgo. Pruebas D23a, D23b, D23-m, D29–D30, D32.

## Las pruebas, una por una (resumen corto)

Cada hito se acepta con sus pruebas superadas, sin duplicados y con evidencia. Este es el resumen de cada una:

- **D01.** Salud, versión, licencia 84 y acceso: todo correcto y registrado.
- **D02.** La contraseña vieja ya no entra; la nueva sí, sin secretos legibles pero con registro de intentos.
- **D03.** Una venta en la web crea una sola comanda en caja, vinculadas.
- **D04.** Si se pierde la respuesta al crear, se recupera la misma comanda, no se crea otra.
- **D05.** Añadir una línea mantiene la misma comanda y el contenido exacto.
- **D06.** Cambiar unidades o comentario mantiene la comanda, con el total correcto.
- **D07.** Retirar una línea da el resultado exacto, o la función se desactiva si no está soportada.
- **D08.** Si se toca a la vez en caja y web, el conflicto se ve; nada se pisa en silencio.
- **D09.** Anular desde la web anula en caja de forma coherente.
- **D10.** Anular en caja deja la misma venta anulada en la web.
- **D11.** La propina queda exacta, sin duplicarse.
- **D12.** El saldo de puntos coincide y queda el motivo.
- **D13.** El pago parcial queda registrado una vez, con saldo y caja correctos.
- **D14.** El pago total liquida la comanda una sola vez.
- **D15.** Si se pierde la respuesta al pagar, se comprueba antes de repetir; nunca hay segundo pago.
- **D16.** Un pago cobrado en caja se refleja en la misma venta de la web.
- **D17.** La web impide pasar de tres pagos.
- **D18.** Facturar desde la web da un número de factura único.
- **D19.** Si se pierde la respuesta al facturar, no se emite segunda factura.
- **D20.** Al finalizar en caja, el aviso y el documento quedan enlazados en una sola venta.
- **D21.** Llegan los siete tipos de aviso configurados.
- **D22.** Un aviso repetido o desordenado no duplica ni reabre ventas cerradas.
- **D23a.** Con el trabajador caído y el receptor operativo, el aviso espera en el buzón.
- **D23b.** Con el receptor inaccesible, el aviso puede perderse; solo se recupera lo localizable.
- **D23-m.** Una comanda manual cuyo aviso se perdió aparece por listado o se declara límite.
- **D24.** Un cambio de artículo o departamento en caja llega a la web sin botón.
- **D24-p.** El plano se lee sin mover posiciones.
- **D24-m.** Las formas de pago y los empleados se leen sin crear registros.
- **D25.** Un cliente cambiado en los dos lados: se actualiza o marca conflicto, nunca se duplica.
- **D26.** Un ajuste desde la web genera un solo movimiento y el stock coincide.
- **D27.** Una venta de caja converge en la web sin descontar dos veces.
- **D28.** Releer documentos no duplica facturas, ventas ni albaranes.
- **D29.** Tras un reinicio: alerta, cola conservada y recuperación en orden.
- **D30.** Turno piloto: cero duplicados; caja, facturas y stock cuadrados.
- **D31.** Una llamada avisa en caja sin dejar estado.

**Pruebas nuevas de la 3ª tanda (resumen corto):**

- **D03a.** Crear con envío a cocina aparca la venta y sale por impresora.
- **D09a.** Anular con CancelOrder sale por impresoras auxiliares.
- **D13a.** Un parcial sin factura deja la venta abierta con el saldo.
- **D15t.** Sin respuesta al pagar, se consulta antes de repetir, sin duplicar.
- **D18a.** La simplificada sale facturada al cobrar, no se re-factura.
- **D18b.** InvoiceOrder sobre comanda sin facturar da número único.
- **D18c.** InvoiceOrder sobre ya facturada no emite segunda factura.
- **D18d.** La completa con NIF usa BillingDetails frente a datos del cliente.
- **D19t.** Sin respuesta al facturar, se consulta antes de repetir, sin duplicar.
- **D21a.** El aviso de traspaso trae origen, destino, líneas y estado final correctos.
- **D22a.** Un traspaso duplicado o desordenado no duplica ni reabre.
- **D26a.** La consulta unitaria da el stock exacto; error con almacén inexistente.
- **D26b.** La consulta en lista da pares exactos y error por línea.
- **D26c.** La regularización aplica el delta; rechaza almacén o motivo inválido.
- **D26d.** El traspaso entre almacenes resta y suma; rechaza no inventariable.
- **D26e.** El ajuste delta suma o resta, con escandallo automático si aplica.
- **D26f.** El inventario absoluto deja el stock declarado; nulo no mueve nada.
- **D26g.** Sin respuesta en stock, un solo movimiento tras consultar.
- **D28-auto.** La exportación programada no deja huecos ni duplicados.
- **D28a.** Los albaranes se importan automáticos y completos, sin botón.
- **D32.** Cada excepción muestra procedimiento y responsable en la web.

**Lo que no entrará (para que no haya malentendidos):**

1. Las rectificativas, devoluciones y reembolsos seguirán haciéndose a mano en el BDP: no existe operación para ellos.
2. Las reservas y gastos seguirán siendo solo de la web, sin sincronizar; el plano, los empleados, las formas de pago y los menús se consultan desde la web pero se administran en el BDP; los traspasos se reflejan pero no se inician desde la web (cubierto por D32); y la factura no se descarga en PDF en la web (se obtiene por correo o impresión desde el BDP).
3. Una comanda creada en caja cuyo aviso se pierda quedará invisible en la web hasta que se finalice: los avisos no se reenvían solos.
4. Los pagos cobrados en caja pueden verse con retraso, porque el aviso no trae datos de pago: la web los confirmará en su repaso periódico.

## Cómo hemos llegado hasta aquí

**Primera fase (julio): conectar la página con el BDP.** Empezamos integrando lo esencial: enviar comandas y ventas al BDP, y traer a la web el catálogo, los clientes, el plano y el histórico, dejando funcionando el ciclo principal de la venta. Desde el inicio vimos juntos que la comunicación era de un solo sentido: la página envía al BDP, y lo que hay en el BDP la web va a buscarlo, descubrimos que era necesario que funcionara en ambos sentidos; la solución planteada en la siguiente fase fue lograr una integración completa semiautomática.

**Segunda fase (agosto-septiembre): trabajar sin depender del BDP.** Acordamos añadir la pestaña propia de stock y albaranes, la lectura de facturas, el funcionamiento independiente del BDP y cerrar los pendientes de la anterior fase (cobro, factura y cancelación de comandas). Todo eso ya está funcionando y verificado en la guía del 18/09. En esas mismas fechas comentamos que la llegada automática de los cambios de caja sería un desarrollo aparte y aclaramos que no todo podía funcionar en 2 direcciones.

**Siguiente paso (fase D): que la web sea la pantalla diaria y el BDP el motor.** Sería un trabajo por hitos: receptor siempre disponible para los avisos, buzón y reconciliación sin duplicados, pagos y factura vinculados, maestros y stock automáticos. La documentación del BDP deja puntos sin definir y cada uno se confirma con sus respuestas y con pruebas, primero en desarrollo y luego en el terminal real fuera de horario, con cuidado porque algunas operaciones no se deshacen. El alcance final de cada hito y su precio se fijan antes de empezar, con su prueba de aceptación.

## Decisiones que necesito

- **Fase D por hitos sí o no:** es un trabajo nuevo, fuera del alcance ya entregado; sin ella todo sigue funcionando como hoy. El precio de cada hito se define antes de empezar, con su prueba de aceptación.
- **Entorno de pruebas:** ✅ resuelto — su portátil con TPV real licenciado; ya no hay que pedirlo a BDP.
- **Receptor (decisión propuesta):** la web leerá la caja cada pocos segundos (probado el 02/10: detecta cada ticket al momento). No requiere configurar nada en el BDP.
- **Sus respuestas de BDP:** estaré atenta a lo que le contesten a sus 16 preguntas; sin lo imprescindible no se programan pagos, actualización ni factura.
- **Contraseñas:** coordinamos el cambio cuando me diga, y se sacan de códigos, documentos y registros.
- **En el terminal (con el distribuidor):** almacén y motivos reales, series, perfiles, avisos (ruta base, ruta por aviso y método) y suscripción de pago 84.
