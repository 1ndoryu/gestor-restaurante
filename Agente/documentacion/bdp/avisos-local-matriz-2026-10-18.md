# Matriz de avisos TPV → receptor (local, 2026-10-18)

Entorno de prueba: TPV local de la laptop, receptor `glory-backend` en la 89 (`127.0.0.1:3105`, túnel), inbox `bdp_avisos_inbox` de `glory_backend`. Datos sin importancia.

Fuentes: log del receptor (`~/taller/_logs/RESTAURANTE/dev-up-RESTAURANTE-3105.log` en la 89) e inbox. El log **no guarda el cuerpo** de los avisos; solo acción, clave de idempotencia y estado.

## Filas observadas

| Acción | Origen (disparo en el TPV) | Resultado HTTP | Inbox | Evidencia |
|---|---|---|---|---|
| crear | Aparcar con comanda en barra | 202 (`estado recibido`) | sí | 2026-10-18 16:32:23 |
| crear | (sintético, L4.1) | 202 `requiere_revision` | sí | 2026-10-17 22:27 |
| crear | (primer intento, sin admin) | 401 con secreto inválido | no | 2026-10-18 14:55; 16:23 idem en modificar |
| finalizar | cierre de comanda (sesión de admin) | 202 | sí | 2026-10-18 16:28:35 y 16:29:02 |
| modificar | añadir COCA y volver a aparcar un ticket recuperado | 202 | sí | 2026-10-18 16:34:47 |
| borrar | `borrar-comanda` + `confirmar` sobre ticket aparcado recuperado | 202 | sí | 2026-10-18 16:36:20 |
| cambiar-estado | no provocado | — | no | Requiere Monitor de Cocina (ver abajo) |
| traspasar | toda la comanda de mesa 1 a mesa 3 (salón 1), manual y luego `tpv.mjs traspasar` | 202 | sí | 19:57:30 y 20:00:56 (2026-10-18) |
| traspasar-linea | no probada | — | — | — |

Clave de idempotencia: `<accion>|<id_comanda>|<marca de tiempo>` (p. ej. `crear|19|2026-10-18T16:28:09Z`).

## Hallazgos

1. **Secreto y elevación.** Los dos intentos sin elevar (14:55 y 16:23) llegaron con secreto inválido (401). Tras reabrir el TPV como administrador, los avisos entraron. Hipótesis: el secreto viaja como `$env:BDP_AVISOS_SECRETO` y el proceso sin elevar no lo ve. Sin confirmar con el log del TPV.
2. **Cocina no emite aviso.** La tecla Cocina con comanda vacía solo conmuta impresoras auxiliares.
3. **Aparcar** emite `crear`; recuperar un ticket aparcado y volver a aparcarlo emite `modificar`.
4. **cambiar-estado.** Según `Estado_en_Cocina.htm` y `Configuracion_Impresoras_Auxiliares.htm`, los estados Nueva/Pasada/Avisada los gestiona el **Monitor de Cocina**, una aplicación aparte instalada en otro ordenador de la red local. No hay ningún ejecutable de monitor en `C:\Bdp-Net\NetHos\Prog`, así que este equipo no puede generar `cambiar-estado`. Para probarlo hace falta instalar el monitor (decisión de la usuaria; ver plan §8).

## Decisión L4.4 (valor seguro por defecto)

Lo no documentado se registra y no se actúa. Un aviso con acción o `estado` desconocidos se guarda en el inbox como `recibido` y no dispara ninguna operación. Así no hay efectos en la base del TPV hasta que haya una evidencia que lo justifique. Coincide con el criterio del receptor actual, que responde 202 y solo guarda.

## Pendiente

- `traspasar-linea`: sin medir. Marcar la línea 1 de la mesa 3 (fila en 150,36) sí funciona. Después, «Trasp. Mesas» (982,102) no responde a clic en dos intentos desde el reinicio, así que no hay forma automatizada de iniciar el traspaso de línea.
- Acción `salon` del PANEL (854,102): abre «SELECCIÓN DE SALÓN» (1 - SALON 1, 2 - TERRAZA, 91, 92, Salir) y «Salir» (222,322 en su captura) la cierra. Verificado. No se seleccionó ningún salón distinto de 1.
- Mesa 3 verificada tras la apertura manual de la usuaria: dos líneas de COCA-COLA (1,05 cada una), subtotal 2,10.
- `cambiar-estado`: bloqueado por el Monitor de Cocina (hallazgo 4).
- **Limitación del plano de mesas (pendiente de arreglar):** `pc.mjs capturar` solo ve la ventana del Tpv Táctil; el plano a pantalla completa no aparece en ella, así que la captura no lo muestra. `pc.mjs pulsar-local` no puede apuntar al plano. Para verlo hay que capturar la pantalla entera (PowerShell, `CopyFromScreen`, 1805×1203). El clic de ratón por PowerShell (`SetCursorPos` + `mouse_event`) no cae donde se pide: en la prueba pedida a (95,650) el cursor quedó en (1310,233), así que no es fiable. Mientras tanto, abrir la mesa la hace la usuaria y el agente verifica después.

## D22 / D22a en el receptor (sonda sintética, 2026-10-18)

Comanda de prueba `990001` en la 89 (receptor `127.0.0.1:3105`). Son cargas sintéticas con el contrato de `avisos-receptor-199A-1-2026-10-17.md`, no avisos del TPV.

| Caso | Esperado | Obtenido |
|---|---|---|
| `crear` T1 | 202 | 202 `recibido` |
| `crear` T1 repetido (mismo timestamp, otro formato) | 200 duplicado | 200 `duplicado` |
| `crear` T1 con otro contenido | 409 | 409 `conflict` (se conserva el primero) |
| `traspasar-linea` T3 y luego T2 (desordenados) | 202 los dos | 202 y 202 |
| `traspasar-linea` T2 repetido | 200 duplicado | 200 `duplicado` |

Las tres filas de la comanda se borraron del inbox de la 89 después.

**Limitación:** esto prueba el receptor, no el TPV. Que el TPV repita o desordene avisos en real queda sin testigo; la D22 completa exige provocar un traspaso en el TPV local.

## Traspasar en el TPV local

- `scripts/pc/tpv.mjs traspasar <mesa> [salón]` (Trasp. Mesas, salón si lo pide, Sí). Ejecutado dos veces con éxito: el primer traspaso fue manual; el segundo, `traspasar 3 1`, emitió `traspasar` en el inbox a las 20:00:56.
- **Sin verificar:** el contenido de la mesa 3 tras el traspaso. Los intentos posteriores de abrir la mesa 3 dejaron el TPV con «30c3» en el visor y sin respuesta a los clics (ver `Nada cambió`); requiere reinicio con UAC (`scripts/pc/elevar-reinicio-tpv-driver.ps1`).
- `traspasar-linea`: sin medir. Hace falta marcar una línea en el ticket con dos líneas y medir la fila.
