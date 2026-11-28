# Tabla de evidencias del piloto — §5 del plan de Guillermo

Tarea `28AA-4`. El §5 del plan (`Agente/usuario/plan-practico-avance-bdp-guillermo-2026-10-28.docx`)
pide «una tabla breve compartida entre Guillermo y Wan con una fila por prueba crítica y un enlace
a la evidencia», para que las pruebas críticas se puedan revisar **sin acceso al BDP de Wan**.

Esta es esa tabla, con lo que **ya está probado** y lo que falta. La columna «contra qué» es la
que importa: casi todo está probado contra el **TPV demo del portátil** o contra el **simulador**,
no contra el TPV del restaurante. Una prueba del demo no vale como prueba del restaurante: el
demo tiene otros artículos, otra licencia y ningún dato real.

## Estado por prueba crítica

| # | Prueba | Contra qué | Evidencia | Estado |
|---|---|---|---|---|
| 4.1 | Diagnóstico de identidad y licencia | TPV demo | 19/19 checks en verde, versión 36.2, aplicación 84 — `diagnostico-tpv-demo-2026-11-21.md:3-16` | **Superada en demo.** En el restaurante, solo lectura parcial el 03/11 (`licencia_ok`, versión y aplicación 84: `tareas-2026-11-03.md:25-26`); el 27/11 el diagnóstico completo falló por timeout al 8068 (`diagnostico-produccion-2026-11-27.md:8-16`) |
| 4.2 | Comandas: crear, añadir línea, cambiar unidades y comentario, retirar línea, anular | TPV demo | Comanda 88 (`S18A3940896`): crear 2,55 → añadir 3,60 → quitar, comentario «SIN HIELO» en la línea 7 (`pruebas-comandas-guillermo-2026-11-18.md:22-40`). Totales web = ticket en comandas 155/156/157, ticket `00001TB00000065` (`regresion-21ab-3-2026-11-21.md:19-28`). Retirar línea en ticket: comanda 139, `00001TB00000051` (`ticket-lineas-comentarios-2026-11-20.md:20-34`). Anular: `Orders/Cancel` (`pruebas-comandas-guillermo-2026-11-18.md:89-97`) | **Superada en demo.** Pendiente en el restaurante |
| 4.3 | Conflicto web ↔ TPV con un cambio real hecho en el TPV | — | **Ninguna.** Solo base de datos con aviso y pedido simulados (`tareas-2026-11-26.md:25-27`), declarado como límite por escrito (`borrador-respuesta-guillermo-2026-11-27.txt:21`) | **No probada.** El paso 5 del guion es el que la produce |
| 4.4 | Propina, pago parcial, doble clic, pérdida de respuesta, factura con datos fiscales | TPV demo | Propina: comanda 58, `Tip` 0→2,00 (`tareas-2026-10-21.md:874-887`). Parcial: comanda 181, 3,50 de 10,50; comanda 214, tres pagos y factura `00001TB00000070` (`pruebas-segunda-lista-2026-11-23.md:15-22`, `:63-68`). Doble petición simultánea: orden 77 → un solo pago en BDP (`tareas-2026-10-28.md:36`). Pérdida de respuesta: corte real a 1400 ms (`tareas-2026-11-24.md:3-6`). Factura con NIF: comanda 86, `00001TB00000042` (`tareas-2026-11-03.md:6`) | **Superada en demo, salvo lo posterior al 27/11.** El tope de tres pagos solo se retiró el 27/11 con la respuesta de BDP: **el cuarto pago no se ha probado contra ningún TPV**. Pendiente en el restaurante |
| 4.5 | Reinicio de la sincronización, de WebLinkRestAPI y de BDP.NET, sin duplicados | TPV demo | Sincronización: PID 2938965→3542747 con la cola idéntica y sin duplicados (`tareas-2026-10-29.md:17`). `TpvNet` PID 2672→5848 → una sola comanda 182 (`pruebas-segunda-lista-2026-11-23.md:70-79`). Los dos procesos parados → comanda 215 y un único pago (`:186-207`) | **Superada en demo.** Pendiente en el restaurante |
| 4.6 | Activar una categoría no habilita otra | Test automático contra base de datos | `cada_etapa_se_enciende_por_separado` (`src/services/bdp_write_guard/permisos_automaticos.rs:345-362`) y `modificar_una_comanda_necesita_su_propia_etapa`, que deja `modify_order` bloqueado con solo `comandas` encendida (`:367-377`) | **Superada por test**, no contra un TPV. Suficiente: es lógica de permisos nuestra, no del TPV |
| 4.7 | Stock: inicial, operación, final | TPV demo | Ajuste ±1 del artículo 1002: 4 → 5 → 4, un movimiento por ajuste (`tareas-2026-10-26.md:39`). En simulador, ajuste neto 0 queda `descartado` (`tareas-2026-11-20.md:52`) | **Parcial.** El cierre de turno no movió stock y ese BDP no descuenta por venta (`tareas-2026-10-26.md:35`). Fuera de los nueve pasos hasta que el restaurante conteste el §3 |

## Lo que hay que rellenar en la franja del piloto

Cada fila que hoy dice «pendiente en el restaurante» se convierte en una fila nueva, con fecha,
entorno, licencia, acción, resultado en web y en BDP, y captura o registro de auditoría sin
credenciales. El formato, por fila:

```
Fecha | Entorno | Versión y licencia | Acción | Resultado esperado | Resultado web | Resultado BDP | Evidencia | Estado
```

Estados admitidos (los del §5): **superada**, **pendiente de BDP**, **pendiente de restaurante**,
**desactivada porque no se utilizará**.

## Dos huecos de evidencia que no son del piloto sino nuestros

- **La prueba de conflicto 4.3 no existe de verdad.** Es la única prueba crítica sin ninguna
  evidencia, ni siquiera del demo, y el mensaje a Guillermo del 27/11 dice que está probada con
  datos simulados. Está escrito así, sin adornarlo, y el paso 5 del guion es el que la produce.
- **El cuarto pago nunca se ha enviado.** BDP contestó el 27/11 que el tope de tres está
  desactualizado y el código ya no lo aplica, pero la prueba de los cuatro pagos sigue siendo la
  del paso 7 del guion, sin ejecutar.
