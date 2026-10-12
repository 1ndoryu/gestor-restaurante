# Fase D: que lo de caja llegue solo a la web — alcance y estado (02/10/2026)

## Objetivo

Hoy casi todo va con botón: traer datos a la web, enviar comandas al BDP, y lo que se hace en caja hay que ir a buscarlo. La fase D quita los botones: **todo viaja solo en ambos sentidos** — lo de la web entra al BDP y lo de caja llega a la web al momento.

Se prueba todo en el TPV de pruebas (mismo programa) y en producción solo se instala lo ya comprobado.

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
| Propina / puntos                      |     —     |    Sí     | No (botón) |
| Anular comanda                        |     —     |    Sí     | No (botón) |
| Modificar comanda enviada             |     —     | Pendiente |     No     |
| Traspasar (mesa / líneas)             |     —     |     —     |     No     |
| Cobrar comanda existente              |     —     | Pendiente |     No     |
| Facturar                              |     —     | Pendiente |     No     |
| Mover stock                           |     —     | Pendiente |     No     |
| Llamar al camarero                    |     —     | Pendiente |     No     |
| Borrar artículos                      |    No     |    No     |     No     |
| Reservas, gastos, marketing, turnos   |     —     |     —     |     —      |

## Después de la fase D (todo solo)

| Funcionalidad                         |    Se trae    | Se envía |   Automático    |
| ------------------------------------- | :-----------: | :------: | :-------------: |
| Catálogo (artículos, precio, familia) |      Sí       |    Sí    |       Sí        |
| Clientes                              |      Sí       |    Sí    |       Sí        |
| Plano de sala                         |      Sí       |    —     |       No        |
| Departamentos                         |      Sí       |    Sí    |       Sí        |
| Formas de pago, empleados             |      Sí       |    —     |       No        |
| Albaranes de compra                   |      Sí       |    —     | Por exportación |
| Detalle de comanda / stock            |      Sí       |    —     |       Sí        |
| Crear comanda                         |      Sí       |    Sí    |       Sí        |
| Propina / puntos                      |       —       |    Sí    |       Sí        |
| Anular comanda                        |      Sí       |    Sí    |       Sí        |
| Modificar comanda enviada             |      Sí       |    Sí    |       Sí        |
| Traspasar (mesa / líneas)             |      Sí       |    —     |       Sí        |
| Cobrar comanda existente              |      Sí       |    Sí    |       Sí        |
| Facturar                              |      Sí       |    Sí    |       Sí        |
| Mover stock                           |       —       |    Sí    |       Sí        |
| Llamar al camarero                    |       —       |    Sí    |       Sí        |
| Borrar artículos                      |      No       |    No    |       No        |
| Reservas, gastos, marketing, turnos   |       —       |    —     |        —        |

## Qué se logrará

Al terminar la fase D, el día a día quedará así:

- **Lo de caja llega solo.** Cada aviso (crear al aparcar, modificar al retocar la aparcada, borrar, finalizar al cobrar y traspasar) entra en la web al momento, sin que nadie tenga que ir a buscarlo. El receptor estará siempre encendido, con rearranque automático si se cae, y con buzón: si la web está apagada o sin conexión en ese momento, el aviso se guarda y se procesa al volver.
- **Sin botones de repaso.** Artículos, clientes, departamentos y demás datos se actualizan solos; las líneas y pagos de cada ticket se leen después de cada aviso y con la copia de exportación (cabeceras, líneas y pagos). El plano, las formas de pago y los empleados se seguirán trayendo como hoy, pero se gestionan en el BDP.
- **Nada se pierde en silencio.** Si un aviso no llega (ya comprobamos que el caído no se reintenta), el repaso diario compara caja y web ticket por ticket, por número y turno, y avisa de lo que falte. Ningún hueco quedará sin señalar.
- **Abiertas bajo control.** La web llevará su propia lista de comandas abiertas y la cuadrará contra los tickets cerrados, incluidos los traspasos de mesa. Siempre se sabrá qué sigue abierto en caja.
- **Pagos y factura atados.** Pagos sin reintento ciego; cada factura quedará vinculada a su aviso de finalizada (el ticket de factura simplificada ya es la factura, no hay paso posterior). Modificar, cobrar y facturar lo ya enviado funcionará con prueba completa previa en local.
- **Stock cuadrado.** El BDP manda y no se descuenta dos veces: cada movimiento se aplica una sola vez aunque el aviso llegue repetido. Los albaranes de compra entrarán por el perfil de exportación. Queda pendiente que el cliente indique cuál de los 6 almacenes es el real.
- **Compras, propina y camarero.** La propina viajará con el cobro; llamar al camarero saldrá en caja; los puntos de fidelización se activarán cuando el terminal los soporte (la propina ya está comprobada).
- **Todo registrado.** Quedará constancia de qué aviso llegó, cuándo y qué cambió en cada venta, para poder revisar cualquier descuadre con el cliente delante.
- **En producción, cero experimentos.** Solo se instalará lo ya comprobado en local con la serie de pruebas; si hay riesgo, vuelta a solo lectura. La verificación final se hará conjunta: tocar en caja y verlo reflejado en la web.

Lo único que seguirá a mano en el BDP: borrar artículos y rectificativas o devoluciones, porque el BDP no tiene esa operación.

## Fuera del alcance

- **Borrar artículos y rectificativas**: el BDP no tiene esa operación; se sigue haciendo a mano.
- **Administrar plano, formas de pago y empleados**: se consulta desde la web, pero se gestiona en el BDP.
- **Reservas, gastos, marketing y turnos**: son solo web, ya funcionan, no necesitan al BDP.

## Dudas que quedan (pocas)

Todas se prueban en el TPV de pruebas, que es el mismo programa. En producción no se prueba nada. No se hicieron hasta ahora por falta de tiempo, no porque falte nada técnico. Se harán al aprobarse la nueva fase.

1. **Traspasar**: falta crear un plano de prueba en local para provocarlo. Si el cliente solo trabaja en barra, no aplica.
2. **Reenviar una comanda corregida**: se pide el ejemplo oficial a BDP y se prueba solo mirando, en local.
3. **Lista de comandas abiertas**: no hay operación documentada. Se prueba si se puede pedir; si no, la web cuadra su lista contra los tickets cerrados.
4. **Carga y pago doble**: en local; el pago doble con serie de pruebas (no se deshace ni en pruebas, pero allí no importa).
5. **Cobrar, facturar y llamar**: probar en local cada operación, con la suscripción activa.
6. **Puntos de fidelización**: la propina está comprobada; los puntos están pendientes de terminal.
7. **Datos reales del distribuidor**: almacén, motivos, series, perfiles, avisos y suscripción. Sin ellos no se instala en producción.

## Lo que descubrimos con el TPV de pruebas (01–02/10)

Entorno propio: portátil con TPV real v36.2 licenciado (`122417` + `751080-84`), serie de pruebas `PRUEBA`, sin tocar ventas reales.

- **La caja ya avisa sola** (4 de 5 avisos en real): crear al aparcar, finalizar al cobrar, borrar y modificar al retocar la aparcada. La comanda se registra al aparcar, no al añadir líneas.
- **Los avisos son solo la señal**: no traen líneas ni pagos; traen el identificador y, al finalizar, la referencia del ticket. La web lee el detalle después.
- **Lo que no llega, se pierde**: con el receptor apagado, el aviso caído no se reintenta. El receptor tiene que estar siempre encendido.
- **La copia del ticket trae todo**: con el perfil de exportación configurado (campos todos marcados), salen cabeceras, líneas y pagos en texto separado por `|`. Comprobado con los tickets de prueba.
- **El ticket de factura simplificada YA es la factura**: no hay paso posterior de "facturar".
- **El botón Salón necesita plano**: sin diseño de salones, el táctil no lo abre.
- **Serie de pruebas `PRUEBA`**: el TPV la memoriza al abrir turno; para cambiarla hay que reabrir turno.
sss
- **La exportación instantánea encola pero no escribe**: la copia se saca con exportación manual por fecha.
