# Guion del piloto en el restaurante — nueve pasos

Tarea `28AA-4`. El plan de Guillermo del 28/10 (`Agente/usuario/plan-practico-avance-bdp-guillermo-2026-10-28.docx`, §7)
fija seis etapas; el mensaje del 26/11 las baja a un piloto y la respuesta del 27/11 dice «los
nueve pasos los tenemos preparados». **Hasta hoy no existían en ningún documento**: solo se
citaban de pasada (`borrador-respuesta-guillermo-2026-11-27.txt:23`). Este es el guion.

Reglas que valen para todos los pasos:

- **Fuera del horario de servicio**, en la franja que acuerde Guillermo, con la web y el TPV a
  la vista y una copia reciente del restaurante hecha (§6 del plan).
- **Ninguna escritura automática.** Todo lo que escribe se hace con una categoría encendida a
  mano y se apaga al terminar el paso. La vuelta rápida a solo lectura
  (`POST /api/bdp/permisos-automaticos/solo-lectura`) apaga las ocho categorías de golpe, en una
  transacción y dejando rastro en el historial.
- **Cada paso deja una fila en la tabla de evidencias** (`evidencias-piloto-2026-11-28.md`).
  Sin evidencia, el paso no cuenta como superado.
- **Se para el piloto** ante comanda duplicada, cambio perdido, cobro o factura dudosa, o
  descuadre de stock: se apaga todo y se vuelve a solo lectura antes de seguir.

## Los nueve pasos

| # | Paso | Qué se enciende | Qué se hace | Se supera si |
|---|---|---|---|---|
| 1 | Diagnóstico del restaurante | nada (solo lectura) | `GET /api/configuracion/bdp/diagnostico` contra el TPV del restaurante | Los seis checks en verde y la salida archivada — sabiendo que el de tenders sale verde sin mapeo (ver «antes del paso 1») |
| 2 | Lectura completa de un servicio | solo `lectura` | Comparar artículos, clientes, comandas conocidas y documentos durante un servicio entero | Web y TPV dan el mismo dato en todo lo comparado |
| 3 | Comanda autorizada manualmente | `comandas` | Crear una comanda de mesa desde la web y anularla | Aparece en el TPV con el mismo identificador, líneas y total; la anulación también llega |
| 4 | Modificación de esa comanda | `comandas` + `modificaciones` | Añadir línea, cambiar unidades, cambiar comentario, retirar línea | Cada cambio se ve en el TPV y la web relee lo que el TPV tiene, no lo que ella creía |
| 5 | Conflicto web ↔ TPV | las mismas dos | Cambiar la misma comanda primero desde la web y luego desde el TPV | La web detecta el cambio del TPV, marca la venta para revisión y no la reescribe sola; el botón «aceptar estado del TPV» (`POST /api/bdp/avisos/{id}/aceptar-tpv`) adopta sus líneas solo tras escribir la confirmación con el número de comanda |
| 6 | Comandas nacidas en el TPV | las mismas dos | Crear una comanda en el TPV sin tocar la web | Entra sola en el ciclo siguiente, con líneas e importe, aunque la web no conocía su identificador |
| 7 | Cobros | `pagos` (`facturas` solo si se prueba la factura) | Pago parcial, segundo pago, tercero y un cuarto; doble clic; después uno con datos fiscales | El TPV admite más de tres pagos sucesivos; un doble clic no cobra dos veces; ninguna factura sale duplicada |
| 8 | Caída y recuperación | las del paso en curso | Cortar brevemente el proceso de sincronización, luego WebLinkRestAPI, luego BDP.NET; y una pérdida de respuesta a mitad de cobro | La cola y los vínculos se recuperan sin duplicar comandas, cobros ni facturas; la web consulta antes de reintentar |
| 9 | Invitación y barra, si BDP ya contestó | `comandas` | Una invitación en un ticket real con el interruptor encendido un momento; y modificar una comanda de barra | La invitación sale a 0,00 en el ticket; la modificación de barra se aplica o, si no, se documenta cómo se comporta el TPV del restaurante frente al demo |

Los pasos 1 y 2 son la **etapa 0** del plan. Los pasos 3 a 6 son las etapas **1 y 2**. El 7 es
la etapa **4**. El stock es la etapa **3** y **no está en este guion** porque el cuestionario del
restaurante sigue sin respuesta válida (ver `roadmap.md`, `28AA-4`): si el restaurante confirma
que no usa escandallos ni inventarios, esa etapa se anota como desactivada y no bloquea. La
etapa **5** (automático estable) no es un paso del piloto: es lo que queda encendido cuando los
anteriores están superados.

## Lo que hay que tener resuelto antes del paso 1

- Autorización escrita de Guillermo para cada etapa (§1 del plan: él autoriza, la web no se
  autoautoriza).
- El cuestionario del §3 contestado por el restaurante. El 05/11 Guillermo dijo que sí a stock,
  cocina, camarero e inventarios; el propietario el 10/11 contestó «no entendemos esas
  funciones» (`estado-respuesta-guillermo-2026-11-15.md:34`). **Sin eso, los pasos 7 y 9 no
  se pueden dar por superados**: no se sabe si la factura, la propina o la invitación se usan.
- Formas de pago configuradas en el TPV. Hoy los cobros salen sin `TenderId`
  (`tareas-2026-11-15.md:24-27`), así que el paso 7 no puede empezar hasta que eso esté.
  Medido el 28/11 en la 89: `BDP_TENDER_MAP_JSON` vale `{}` y `BDP_ORDER_TYPE_MAP_JSON` también.
  **Ojo:** el diagnóstico del paso 1 no lo detecta como fallo, y está bien que no lo haga: una
  comanda abierta sin cobro es configuración válida. Desde el 28/11 su mensaje sí nombra la
  consecuencia —sin `TenderId` la facturación automática se salta en silencio y las ventas cobradas
  quedan pendientes del botón de facturar— y dice que se rellene en Configuración → BDP antes del
  piloto. Leer ese aviso: el verde no significa que el paso 7 esté listo.

## Cómo se enciende y se apaga

Los interruptores son por categoría y **activar una no habilita otra** (`CATEGORIAS` en
`src/services/bdp_write_guard/permisos_automaticos.rs:28`). Son diez: `lectura`, `comandas`,
`modificaciones`, `descuentos`, `articulos_clientes`, `propinas_puntos`, `pagos`, `facturas`,
`stock` y `camarero`. Por encima de todas está el freno general: con `bdp_sync_mode` en
`solo_lectura` ninguna escribe aunque su interruptor esté encendido
(`permisos_automaticos.rs:4-5`).

- Encender o apagar una: `PUT /api/bdp/permisos-automaticos/{categoria}` con `{"activo": true}`.
- Volver a solo lectura de golpe: `POST /api/bdp/permisos-automaticos/solo-lectura`.
- Cada cambio queda en el historial y se ve en `GET /api/bdp/permisos-automaticos/actividad`.

**Las invitaciones no se encienden con un interruptor de permisos**: están apagadas en el código
(`INVITACIONES_HABILITADAS = false`, `src/services/invitacion.rs:21`) y probarlas exige
recompilar y desplegar con eso en `true`, y volver a `false` al terminar. Ese es un despliegue a
producción y lo autoriza la usuaria, no este guion.
