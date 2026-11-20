# 18AB-7 y 18AB-8 en el ticket — 20/11/2026

Origen: pruebas 1 y 2 de la respuesta de Guillermo del 18/11. El doc
`pruebas-comandas-guillermo-2026-11-18.md` las verificó en la **comanda abierta**
(`Orders/Get`); aquí se comprueba lo que llega al **documento de venta** (el ticket
impreso), que es lo que pedían las dos tareas.

BDP demo del portátil (aplicación 01 Hostelería, 36.2, servicio 84), salón 1, mesa 6.
Comandas 139 a 143 y tickets `00001TB00000051`–`53` y `54`. Sondas en
`C:/tmp/probe-bdp-18ab6` (no versionadas; solo escriben en el demo). La lectura del
ticket es directa sobre `Bdp-Net_NetHos_36` (`Documentos_Cabecera`/`Documentos_Lineas`),
que es lo que se imprime. `ExportProfiles/Documents` con el perfil 1 no devolvió los
tickets 51–54 en ninguna de las llamadas probadas (rangos de un día, de un mes, de un
año, `OnlyLastMovements` en ambos valores), aunque las filas están en
`Documentos_Cabecera`, así que no vale como testigo. El turno abierto del TPV sigue
fechado el 18/11 y los documentos nuevos heredan esa fecha.

## 18AB-7 — Producto retirado en el ticket

Comanda 139: 1 COCA-COLA (1001) + 1 SCHWEPPES (1002), se retira la Coca-Cola con
`Items: [{Lin:1, Id:1001, Units:-1}]` y `Order.Total` final, se añade otra Coca-Cola,
se cobra y se factura (`00001TB00000051`).

| Línea del ticket | PLU | Descripción | Unidades | Total |
|---|---|---|---|---|
| 1 | 1001 | COCA-COLA | 1 | 1,05 |
| 2 | 1002 | SCHWEPPES LIMÓN | 1 | 1,20 |
| 3 | 1001 | COCA-COLA | **−1** | **−1,05** |
| 4 | 1001 | COCA-COLA | 1 | 1,05 |

**La línea retirada sí aparece en el ticket**, como contrapartida de −1 ud con su
importe en negativo; no se borra. Total del documento 2,25 = 1,20 + 1,05. Es el mismo
comportamiento que ya mostraba `Orders/Get` el 18/11, confirmado ahora en el documento
de venta.

## 18AB-8 — Cambio de comentario de línea

Comanda 138: la línea se envía con `Comments: [{Id:0, Name:"SIN HIELO"}]` y después se
reenvía la **misma `Lin`** con `Units: 0` y `Comments: [{Id:0, Name:"CON HIELO"}]`.
BDP acepta el reenvío sin error y deja **dos líneas**, no sustituye:

| Línea de la comanda | Unidades | Comentario |
|---|---|---|
| 5 | 1 | `SIN HIELO` |
| 6 | **0** | `CON HIELO` |
| 7 | 1 | `CON HIELO` |

No hay operación para reescribir el comentario de una línea ya enviada: reenviar con
el mismo `Lin` **añade** una línea nueva con el comentario nuevo, y la vieja se queda.

### La forma segura: contrapartida + alta

Comanda 142, facturada como `00001TB00000053`:

1. Alta de 1 Coca-Cola con `SIN HIELO`.
2. Contrapartida `Units: -1` **con el mismo comentario** `SIN HIELO`.
3. Alta de 1 Coca-Cola con `CON HIELO`.

Estado de la comanda (y del ticket):

| Línea | Unidades | Comentario |
|---|---|---|
| 1 | 1 | `SIN HIELO` |
| 2 | **−1** | `SIN HIELO` |
| 3 | 1 | `CON HIELO` |

Total 1,05, correcto. La línea vieja queda anulada por su propia contrapartida con el
comentario original, y la nueva lleva el comentario nuevo. **Esta es la única forma
segura de cambiar un comentario ya enviado.** El comentario no se guarda en
`Documentos_Lineas` ni en `Documentos_Suplementos`/`Documentos_Tratamientos` (los tres
vacíos para estos tickets); vive en `Comandas_Comentarios`, de la comanda abierta, y
solo la contrapartida con el mismo texto deja el ticket coherente.

## Hallazgo nuevo: una línea de 0 uds bloquea la facturación

`Orders/Create` **acepta** una línea con `Units: 0` (y `Total: 0`), también sobre una
comanda que ya tiene líneas válidas. El total no cambia y `Orders/Get` la devuelve.
Pero al cobrar, `Orders/Payment/Add` con `Invoice: true` responde
**«Intento de dividir por cero.»** y **no factura**.

| Comanda | Líneas | Factura |
|---|---|---|
| 141 (control) | 1 Coca-Cola de 1,05 | `00001TB00000054` |
| 138 / 140 | normal + línea de 0 uds | «Intento de dividir por cero.» |
| 142 | 1 + contrapartida −1 + 1 | `00001TB00000053` |

La comanda queda cobrada pero sin facturar, y el error no nombra la línea: el operador
no sabe qué corregir. **Nuestro lado no debe enviar nunca `Units: 0`** (ver más abajo).

## Qué implica en el código

- Las líneas de una venta salen de `venta_lineas.cantidad`, que el repositorio rellena
  con `cantidad.unwrap_or(ONE)` (`src/repositories/venta_linea.rs:33` y `:62`). No hay
  validación de `cantidad > 0` en la entrada: `CrearVentaLineaRequest.cantidad` es
  `Option<Decimal>` y un `0` explícito llega tal cual a BDP
  (`src/services/bdp_sync_venta/payload.rs:141`, con solo un `warn!` en `:121`).
  Con ese `0` la venta se cobra y **no se puede facturar**. Queda como tarea abierta.
- Quitar una línea: nuestra `update_order` debe emitir la contrapartida de −1, no
  omitir la línea de `Items`. Es el comportamiento que ya documenta el 18/11.
- Cambiar un comentario: nunca reenviar `Lin` con el texto nuevo; contrapartida con el
  texto viejo + alta con el nuevo. Hoy el payload manda `Comments: []` siempre
  (`payload.rs:145`), así que el cambio de comentario **no existe** en nuestra
  integración: solo se cubre el caso cuando se implemente.
