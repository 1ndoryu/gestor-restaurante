# Integración web–BDP: estado actual y propuesta de fase D (21/09/2026)

Tercera revisión: incorpora sus correcciones del 21/09 (guía 2).

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
| Detalle de comanda / stock            | Pendiente |     —     |        No         | Falta suscripción / almacén en el terminal                |
| Crear comanda (cobrada o no)          |     —     |    Sí     | Pendiente fase D  | Comprobado con comandas de prueba                         |
| Propina / puntos                      |     —     |    Sí     | Automático fase D | Propina comprobada en real; puntos, pendiente de terminal |
| Anular comanda                        |     —     |    Sí     | Pendiente fase D  | Anulada por la aplicación y registrada                    |
| Modificar comanda enviada             |     —     | Pendiente | Pendiente fase D  | Posible con pago; semántica por confirmar                 |
| Traspasar (mesa / líneas)             |     —     |     —     | Pendiente fase D  | Aviso documentado, pendiente fase D                       |
| Cobrar comanda existente              |     —     | Pendiente |  Parcial fase D   | Solo con suscripción de pago                              |
| Facturar                              |     —     | Pendiente |  Parcial fase D   | Crear y facturar a la vez es posible en gratis            |
| Mover stock                           |     —     | Pendiente | Automático fase D | Falta almacén real en el terminal                         |
| Llamar al camarero                    |     —     | Pendiente |        No         | Solo con suscripción de pago                              |
| Borrar artículos                      |    No     |    No     |        No         | El BDP no lo permite, solo desactivar                     |
| Reservas, gastos                      |     —     |     —     |         —         | Solo web, funcionan siempre                               |
| Menús / packs                         |    Sí     |     —     |        No         | Solo consulta de la definición del BDP                    |
| Marketing, empleados web, turnos/caja |     —     |     —     |         —         | Solo web, funcionan siempre                               |

## Por qué algunas cosas no funcionan en dos direcciones

Todo lo siguiente sale de los propios documentos de BDP, no es una limitación de la web:

- **Catálogo, clientes, plano:** el documento _Avisos de Acciones_ de BDP solo contempla avisos de comandas (crear, modificar, cambiar estado, borrar, finalizar y traspasar). No existe aviso para artículos, clientes, salones o stock, así que cuando algo cambia allí la web tiene que ir a traerlo. Hoy funciona con el botón Importar de cada pantalla y el botón Exportar para enviar; cada operación la confirma usted. En la fase D ese repaso pasaría a ser automático, sin botón. La versión gratuita ya permite estos comandos, así que esta parte se puede adelantar sin esperar a la de pago.
- **Stock:** el propio documento _Suscripción extendida para WordPress_ de BDP dice que el stock fuera del BDP "no está conectado en tiempo real" (esa frase habla del módulo de WordPress, no del almacén del BDP). El manual sí trae funciones propias de almacén (regularizaciones, traspasos y movimientos), y la gratuita no las bloquea; falta probarlo contra el terminal real, donde hoy no hay un almacén válido creado.
- **Pagos cobrados en caja:** entre los datos que un aviso puede enviar (identificador de comanda, hora, estado, documento, salón y mesa en traspasos) no hay ninguno de pago. Por eso la web tiene que ir a preguntar. Hoy funciona con el botón Cobrar de cada venta, que verifica el estado antes de cobrar. Añadir pagos a una comanda existente sí exige la suscripción de pago.
- **Facturas:** el aviso de comanda finalizada trae el tipo, la serie y el número del documento, pero no el documento en sí; el contenido se recupera con el perfil de documentos. La factura la emite el BDP y la web guarda el número. Crear y facturar a la vez desde la web es posible incluso en gratuita si los pagos cuadran; facturar una comanda existente exige la suscripción de pago.
- **Borrar y modificar lo enviado:** el manual _WebLink REST API_ no incluye operación de borrado de artículos (solo crear y modificar). Para comandas la capacidad de modificar con pago está documentada, pero no su semántica exacta: no se programará el reenvío como contrato hasta que BDP dé el ejemplo oficial y las pruebas lo demuestren. Mientras tanto, lo enviado se anula y se crea de nuevo, no se edita.
- **Avisos y camarero:** los avisos se configuran con una dirección web (ruta base, ruta por aviso y método), no con una IP de mensajes. Llamar al camarero es una operación aparte que exige la suscripción de pago.

## Lo que falta por confirmar (con pruebas en el terminal)

Ordenado por importancia. Las pruebas que tocan pagos, facturas o fallos se hacen primero en el entorno de desarrollo (pendiente de solicitar a BDP); en el terminal real solo se mira, de noche y con comandas de prueba que al terminar se anulan. Nada toca ventas reales ni la caja del día:

1. **¿El aviso de "modificada" incluye pagos?** El documento de avisos dice que el aviso trae el identificador, pero no detalla si incluye pagos. Prueba: crear una comanda de prueba, cambiarla y cobrarla en caja, y ver qué datos llegan. Riesgo bajo. Si el aviso sirve de señal, la web leerá la comanda completa; si no trae pagos, los cobros se verán con retraso y la web los repasará cada cierto tiempo.
2. **¿Cómo responde el BDP al reenviar una comanda corregida?** La capacidad existe con pago, pero el manual no dice si sustituye las líneas o las suma, ni si acepta cambios en comandas ya pagadas. Se pedirá el ejemplo oficial a BDP y se probará por separado añadir línea, cambiar unidades y retirar línea, solo mirando. Riesgo nulo. Solo se programará lo demostrado; si el resultado no es limpio, se mantiene lo actual: anular y crear de nuevo.
3. **Si un aviso se pierde, ¿llega después?** No está documentado. Prueba: simular que el receptor de la web falla (respuesta de error), hacer un cambio en caja y ver qué pasa. Nunca se cortará la red del terminal. Riesgo bajo. Si no llega, la web repasará cada cierto tiempo para recuperar lo que la API permita descubrir.
4. **¿Qué datos trae el perfil de exportación?** El manual no describe sus campos. Prueba: configurarlo con el distribuidor y revisar lo que llega. Riesgo nulo. Si viene incompleto, lo avisaremos antes de usarlo.
5. **¿Se puede pedir la lista de comandas abiertas?** No hay operación documentada. Prueba: intentar pedirla y ver qué responde. Riesgo nulo. Si no se puede, una comanda creada en caja quedará invisible hasta finalizarse.
6. **¿Cuántas consultas aguanta el terminal sin ir lento?** Ningún documento da límites. Se fijarán límites prudentes en desarrollo y, en el terminal real, una prueba mínima de noche con umbral de parada (si se nota lento, se para). Nunca se forzará hasta que vaya lento. Mientras tanto se usará frecuencia baja para no afectar al cobro.
7. **Pagos, facturas y series.** El manual permite pagar una comanda a plazos y consultar sus pagos, y devuelve el número de factura al facturar; lo que no explica es qué pasa si un pago se envía dos veces ni cómo enlazar el aviso de finalizada con la factura real. Prueba en desarrollo primero; en real, una sola vez, juntos y con procedimiento aprobado, cuando usted decida. Un pago de prueba, aunque sea de céntimos, no se puede borrar ni devolver, y la factura real no se puede deshacer. Riesgo alto.
8. **Rectificativas y devoluciones:** no existen como operación en ningún documento; se hacen a mano en el BDP. Quedan fuera de esta fase.

## Respuesta a su guía 2: aceptamos sus correcciones

He comprobado los 15 puntos de su guía en los manuales y lleva razón en todos:

- **Licencia:** la gratuita permite artículos, clientes, departamentos, stock, perfiles, creación y anulación; la de pago (84) se necesita para detalle de comanda, modificar existente, pagos y propinas en existente, facturar existente y camarero. Esa parte se adelantará sin esperar al pago.
- **Pruebas seguras:** pagos y facturas primero en desarrollo; sin cortes de red ni saturación en el terminal abierto.
- **Códigos reales:** se usará el almacén, los motivos, las series y los perfiles reales que indique el distribuidor, nunca los de los ejemplos.
- **Canal técnico:** las preguntas de funcionamiento van a BDP por su canal; el distribuidor configura licencias, almacenes, perfiles, series y avisos.
- **Matices:** departamentos, propina y puntos con vuelta eventual; albaranes importables; reservas y gastos solo web; camarero separado; avisos por dirección web.

## Fase D por hitos

El plazo empezará cuando estén la licencia, los accesos, el entorno de pruebas y las respuestas imprescindibles de BDP; el tiempo esperando a terceros contará aparte. Cada hito se acepta con su prueba superada, sin duplicados y con evidencia. Queda pendiente el entorno de pruebas (a solicitar a BDP) y estaré atenta a las respuestas que le den a sus preguntas.

**Hito 1 — Alcance, secretos y base.** Matriz de operaciones firmada; cambio de contraseñas y secretos fuera del código; comprobación de versión, servicio 84 y acceso; códigos reales y perfiles con el distribuidor. Pruebas A01–A02.

**Hito 2 — Comandas y avisos.** Receptor siempre encendido y buzón; crear comandas con identidad estable; actualización solo de lo demostrado; anulación; traspasos reflejados. Pruebas A03–A10, A21–A22.

**Hito 3 — Pagos, propina, puntos y factura.** Pagos sin reintento ciego (máximo 3); propina y puntos con lectura posterior; factura única con clave fiscal y documento por perfil. Pruebas A11–A20.

**Hito 4 — Maestros, stock y documentos.** Catálogo, clientes y departamentos sin botón; stock con el BDP como referencia y sin doble descuento; documentos y albaranes por perfil. Pruebas A24–A28.

**Hito 5 — Fallos, piloto y producción.** Receptor caído simulado, reinicios y tiempos de espera sin duplicados; reconciliación de comandas conocidas; turno piloto con caja cuadrada; vuelta a solo lectura si hay riesgo. Pruebas A23, A29–A30.

## Las 30 pruebas, una por una (resumen corto)

Cada hito se acepta con sus pruebas superadas, sin duplicados y con evidencia. Este es el resumen de cada una:

- **A01.** Salud, versión, licencia 84 y acceso: todo correcto y registrado.
- **A02.** La contraseña vieja ya no entra; la nueva sí, y no aparece en registros.
- **A03.** Una venta en la web crea una sola comanda en caja, vinculadas.
- **A04.** Si se pierde la respuesta al crear, se recupera la misma comanda, no se crea otra.
- **A05.** Añadir una línea mantiene la misma comanda y el contenido exacto.
- **A06.** Cambiar unidades o comentario mantiene la comanda, con el total correcto.
- **A07.** Retirar una línea da el resultado exacto, o la función se desactiva si no está soportada.
- **A08.** Si se toca a la vez en caja y web, el conflicto se ve; nada se pisa en silencio.
- **A09.** Anular desde la web anula en caja de forma coherente.
- **A10.** Anular en caja deja la misma venta anulada en la web.
- **A11.** La propina queda exacta, sin duplicarse.
- **A12.** El saldo de puntos coincide y queda el motivo.
- **A13.** El pago parcial queda registrado una vez, con saldo y caja correctos.
- **A14.** El pago total liquida la comanda una sola vez.
- **A15.** Si se pierde la respuesta al pagar, se comprueba antes de repetir; nunca hay segundo pago.
- **A16.** Un pago cobrado en caja se refleja en la misma venta de la web.
- **A17.** La web impide pasar de tres pagos.
- **A18.** Facturar desde la web da un número de factura único.
- **A19.** Si se pierde la respuesta al facturar, no se emite segunda factura.
- **A20.** Al finalizar en caja, el aviso y el documento quedan enlazados en una sola venta.
- **A21.** Llegan los siete tipos de aviso configurados.
- **A22.** Un aviso repetido o desordenado no duplica ni reabre ventas cerradas.
- **A23.** Con el receptor caído, se documenta qué se recupera y qué no, sin cortar la red de caja.
- **A24.** Un cambio de artículo o departamento en caja llega a la web sin botón.
- **A25.** Un cliente cambiado en los dos lados: se actualiza o marca conflicto, nunca se duplica.
- **A26.** Un ajuste desde la web genera un solo movimiento y el stock coincide.
- **A27.** Una venta de caja converge en la web sin descontar dos veces.
- **A28.** Releer documentos no duplica facturas, ventas ni albaranes.
- **A29.** Tras un reinicio: alerta, cola conservada y recuperación en orden.
- **A30.** Turno piloto: cero duplicados; caja, facturas y stock cuadrados.

**Lo que no entrará (para que no haya malentendidos):**

1. Las rectificativas, devoluciones y reembolsos seguirán haciéndose a mano en el BDP: no existe operación para ellos.
2. Las reservas y gastos seguirán siendo solo de la web, sin sincronizar; el plano, los empleados, las formas de pago y los menús se consultan desde la web pero se administran en el BDP; los traspasos se reflejan pero no se inician desde la web; y la factura no se descarga en PDF en la web (se obtiene por correo o impresión desde el BDP).
3. Una comanda creada en caja cuyo aviso se pierda quedará invisible en la web hasta que se finalice: los avisos no se reenvían solos.
4. Los pagos cobrados en caja pueden verse con retraso, porque el aviso no trae datos de pago: la web los confirmará en su repaso periódico.

## Cómo hemos llegado hasta aquí

**Primera fase (julio): conectar la página con el BDP.** Empezamos integrando lo esencial: enviar comandas y ventas al BDP, y traer a la web el catálogo, los clientes, el plano y el histórico, dejando funcionando el ciclo principal de la venta. Desde el inicio vimos juntos que la comunicación era de un solo sentido: la página envía al BDP, y lo que hay en el BDP la web va a buscarlo, descubrimos que era necesario que funcionara en ambos sentidos; la solución planteada en la siguiente fase fue lograr una integración completa semiautomática.

**Segunda fase (agosto-septiembre): trabajar sin depender del BDP.** Acordamos añadir la pestaña propia de stock y albaranes, la lectura de facturas, el funcionamiento independiente del BDP y cerrar los pendientes de la anterior fase (cobro, factura y cancelación de comandas). Todo eso ya está funcionando y verificado en la guía del 18/09. En esas mismas fechas comentamos que la llegada automática de los cambios de caja sería un desarrollo aparte y aclaramos que no todo podía funcionar en 2 direcciones.

**Siguiente paso (fase D): que la web sea la pantalla diaria y el BDP el motor.** Sería un trabajo por hitos: receptor siempre disponible para los avisos, buzón y reconciliación sin duplicados, pagos y factura vinculados, maestros y stock automáticos. La documentación del BDP deja puntos sin definir y cada uno se confirma con sus respuestas y con pruebas, primero en desarrollo y luego en el terminal real fuera de horario, con cuidado porque algunas operaciones no se deshacen. El alcance final de cada hito y su precio se fijan antes de empezar, con su prueba de aceptación.

## Decisiones que necesito

- **Fase D por hitos sí o no:** es un trabajo nuevo, fuera del alcance ya entregado; sin ella todo sigue funcionando como hoy. El precio de cada hito se define antes de empezar, con su prueba de aceptación.
- **Entorno de pruebas:** pendiente de solicitar a BDP; sin él no hay pruebas de pagos, facturas ni fallos.
- **Sus respuestas de BDP:** estaré atenta a lo que le contesten a sus 16 preguntas; sin lo imprescindible no se programan pagos, actualización ni factura.
- **Contraseñas:** coordinamos el cambio cuando me diga, y se sacan de códigos, documentos y registros.
- **En el terminal (con el distribuidor):** almacén y motivos reales, series, perfiles, avisos (ruta base, ruta por aviso y método) y suscripción de pago 84.
