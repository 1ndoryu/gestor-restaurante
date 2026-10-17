# Receptor de Avisos de Acciones del TPV — contrato y configuración (199A-1/F1, 2026-10-17)

Fuente única de lo que hay que configurar en el TPV y de lo que hace nuestro receptor.
Complementa `guia-bidireccional-avisos-2026-09-19.md` (la guía del distribuidor) y sustituye
a la especificación del plan en lo que respecta a nombres de campo y respuestas.

Contrato tomado del paquete del distribuidor `AvisosDeAcciones_Documentacion`
(`DataContracts/*.cs` y `Controllers/ComandasController.cs`), extraído el 2026-10-17 a
`C:\tmp\avisos-bdp\` (fuera del repo).

## 1. Ruta

```
GET|POST  /api/bdp/avisos/{accion}      ← el TPV, sin sesión, con el secreto
GET       /api/bdp/avisos               ← el restaurante, con sesión (Admin)
POST      /api/bdp/avisos/{id}/reintentar
```

`{accion}` admite las siete del distribuidor y sus variantes escritas de cualquier manera
(mayúsculas, guiones, guiones bajos, espacios, acentos, y los nombres largos
`CambiarEstado`, `TraspasarLinea`, `Comanda creada`…). Los slugs canónicos:

| Acción del TPV | Slug canónico |
|---|---|
| Crear | `crear` |
| Modificar | `modificar` |
| CambiarEstado | `cambiar-estado` |
| Borrar | `borrar` |
| Finalizar | `finalizar` |
| Traspasar | `traspasar` |
| TraspasarLinea | `traspasar-linea` |

En la configuración del TPV, `Ruta Base` = `https://<host>/api/bdp/avisos/` y
`Ruta Acción` = el slug. El TPV admite GET (parámetros en la query) o POST (cuerpo JSON);
el receptor acepta las dos y **el cuerpo manda sobre la query** cuando repiten un campo.

## 2. Secreto compartido

El TPV no puede firmar con JWT, así que cada aviso lleva el secreto compartido en un
parámetro fijo (`Secreto` en la configuración de la acción) o en la cabecera
`X-BDP-Secreto` (o `Authorization: Bearer`). El valor esperado está en la variable de
entorno `BDP_AVISOS_SECRETO`, que **no** viaja por el repo: se pone en el `.env` local y en
`envRemoto` de la ficha.

- Sin `BDP_AVISOS_SECRETO` el receptor contesta **503** y no admite nada (apagado, no abierto).
- Con secreto distinto o ausente, **401**. La comparación es en tiempo constante.
- El secreto se borra del payload antes de guardarlo (queda como `***`), así que no aparece
  ni en el buzón ni en los logs.

## 3. Qué se guarda y qué se contesta

Todo aviso admitido va a `bdp_avisos_inbox` (base T2, migración `20261017120000`). Nada de
este receptor escribe en el TPV.

| Situación | Respuesta | ¿Se guarda? |
|---|---|---|
| Aviso correcto | `202` `{"estado":"recibido"}` | Sí |
| Aviso identificable con algún valor raro (§4) | `202` `{"estado":"requiere_revision", "nota":…}` | Sí, marcado |
| Reenvío del mismo aviso (misma clave) | `200` `{"estado":"duplicado"}` | No (ya estaba) |
| Misma clave, otro contenido | `409` | No: se conserva el original |
| Acción desconocida | `404` | No |
| Falta `IdentificadorComanda` | `422` | No |
| Secreto inválido o ausente | `401` | No |
| Sin `BDP_AVISOS_SECRETO` o sin una única cuenta | `503` | No |
| Cuerpo > 16 KiB | `413` | No |
| Más de 120 avisos por minuto | `429` | No |

**Clave de idempotencia:** `{slug}|{comanda}|{timestamp canónico}`, única por cuenta en la
tabla. El timestamp se canoniza a UTC con microsegundos, así que `…T14:00:00Z`,
`…T14:00:00.000Z`, `…T16:00:00+02:00` y `…T14:00:00` son el mismo aviso y no duplican nada.
Esa misma forma canónica se escribe en el payload guardado, para que un reenvío reformateado
se reconozca como duplicado y no como conflicto.

**Regla de admisión (decisión L4.4):** solo se rechaza lo que no se puede ni identificar
(falta el identificador de comanda). Cualquier otro valor ausente o inesperado se **guarda**
con estado `requiere_revision` y su nota, y no se actúa sobre él. Un aviso nunca se pierde
por un campo raro; el worker de F2 decidirá si lo puede aplicar.

## 4. Campos por acción

Nombres del contrato del distribuidor. El receptor los busca por alias y sin distinguir
mayúsculas, acentos ni separadores (`IdentificadorComanda`, `IdComanda`, `OrderId`…), porque
los nombres finales los fija la configuración del TPV.

Comunes a las siete: `IdentificadorComanda` (obligatorio), `IdentificadorEstablecimiento`,
`TimeStamp` (UTC).

| Acción | Campos propios | Se marca para revisión si… |
|---|---|---|
| `crear`, `modificar`, `borrar` | — | falta `TimeStamp` |
| `cambiar-estado` | `Estado`: 0 esperando validación, 1 aceptada, 2 cancelada, 3 facturada | falta `Estado` o está fuera de 0–3 |
| `finalizar` | `TipoDocumento` (1 ticket barra, 2 factura mesa, 3 albarán, 4 factura de albarán, 5 rectificativa, 6 ticket mesa), `Serie`, `Numero` | falta `TipoDocumento`/`Serie`/`Numero`, o el tipo está fuera de 1–6 |
| `traspasar`, `traspasar-linea` | `IdentificadorComandaOrigen`, `IdentificadorLineaComanda` (destino), `IdentificadorLineaComandaOrigen`, `SalonOrigen`, `SalonDestino`, `MesaOrigen`, `MesaDestino` (0 = barra) | falta la comanda de origen, o salón/mesa no son numéricos |

Las líneas de comanda **no** viajan por la API: el distribuidor solo las exporta a fichero.
Para completar un aviso hay que pedir la comanda por API (eso es F2).

## 5. Configuración en el TPV

`Utilidades → Configuración de servicios Web → Configuración de Avisos de Acciones`:

1. Interruptor global **Avisos de Acciones Activos**.
2. Punto de exportación: `Activo`, `Descripción`, `Ruta Base`
   (`https://<host>/api/bdp/avisos/`), `Tipo Exportación` = `Api`.
3. Por cada acción: `Activo`, `Acción`, `Ruta Acción` (el slug de §1), `Método` (`Get` o
   `Post`), y los parámetros `Clave`/`Valor` (incluido el secreto).

`<host>` es el host y puerto del backend que se levanta con `Arranque up "RESTAURANTE"`
(3105 en dev). Como el TPV corre en el portátil y el backend en la 89, hace falta el túnel
que abre `Arranque up` (`127.0.0.1:3105` en el portátil) o el host público de dev. El
secreto que se ponga aquí es el mismo `BDP_AVISOS_SECRETO` del `.env`; en la 89 llega por
la copia del `.env` que hace `up` (comprobado en la sonda de §7).

## 6. Lo que el distribuidor NO documenta

El PDF no dice nada de reintentos, tiempos de espera ni respuesta esperada. Por eso:

- La idempotencia la sostenemos nosotros (clave única), no el TPV.
- La respuesta se contesta corta y en JSON, pero no se depende de que el TPV la mire.
- El orden de llegada no se presume: un aviso que llega tarde o desordenado se guarda igual,
  con su `TimeStamp`, y el worker decide (F2).
- Si el TPV reintenta con otro formato de hora, no genera duplicados ni conflictos (§3).

## 7. Evidencia

- Pruebas: `tests/bdp_avisos.rs` (11) y `tests/bdp_avisos_limite.rs` (1), verdes el
  2026-10-17 en la VPS 89.
- Unidad en `src/services/bdp_avisos/` (7): alias, canonización, censura, secreto,
  admisión.
- Sonda HTTP real contra el servidor de la 89 (`127.0.0.1:3105` por el túnel), con el
  secreto puesto en el `.env`: acción desconocida **404**, sin secreto **401**, secreto
  erróneo **401**, `POST /api/bdp/avisos/crear` **202** `recibido` (con `id`), el mismo
  aviso repetido **200** `duplicado`, sin `IdentificadorComanda` **422**,
  `GET /api/bdp/avisos/cambiar-estado` por query **202**, y el buzón sin sesión **401**.
  En la base de la 89, las dos filas quedaron con el `Secreto` censurado (`"***"`) y el
  `TimeStamp` canonizado a microsegundos. Filas de sonda borradas después.
- Sonda con los siete eventos reales del TPV: pendiente de L4.2 (la usuaria configura el
  TPV y los provoca); la matriz resultante irá a `avisos-local-matriz-<fecha>.md` (L4.3).
