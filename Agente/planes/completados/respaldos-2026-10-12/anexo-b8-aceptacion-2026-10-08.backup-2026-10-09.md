# Anexo B8: fichas de aceptación D01–D32 (08/10/2026)

Fichas de cada prueba de aceptación de la fase D. Cada ficha define previos, pasos, esperado, qué error la detiene y evidencia en T2. Matriz resumen en el alcance; correspondencia con la guía operativa (A01–A20) en el alcance.

## Base y secretos (Hito 1)

### D01 — Salud, versión, licencia 84 y acceso
- Previos: servicio 84 activo, serie `PRUEBA` asignada al terminal.
- Pasos: consultar salud, versión, licencia 84 y acceso; registrar valores.
- Esperado: los cuatro responden y quedan registrados.
- Fallo si: algún valor falta o no coincide con el terminal.
- Evidencia: registro en `bdp_sync_cursor` + captura.

### D02 — Contraseñas y secretos
- Previos: acceso de administración a secretos.
- Pasos: intentar entrar con la contraseña vieja; entrar con la nueva; revisar registros.
- Esperado: la vieja no entra; la nueva sí; sin secretos legibles pero con registro de intentos.
- Fallo si: la vieja entra o aparece un secreto legible en un registro.
- Evidencia: log de intentos + revisión de registros.

## Comandas y cocina (Hito 2)

### D03 — Crear comanda vinculada
- Previos: receptor operativo, serie `PRUEBA`.
- Pasos: crear una venta desde la web; localizar la comanda en caja.
- Esperado: una sola comanda en caja, vinculada a la venta web.
- Fallo si: cero o más de una comanda, o sin vínculo.
- Evidencia: IDs en `bdp_id_map`.

### D03a — Crear con envío a cocina
- Previos: receptor operativo, impresora de cocina configurada.
- Pasos: crear venta con `OrderEndType=0`; comprobar estado y salida por cocina.
- Esperado: la venta queda aparcada y sale por cocina.
- Fallo si: no aparca o no sale por cocina.
- Evidencia: estado de la venta + testigo de cocina.

### D04 — Respuesta perdida al crear
- Previos: receptor operativo, serie `PRUEBA`.
- Pasos: crear venta provocando pérdida de respuesta; consultar antes de repetir.
- Esperado: se recupera la misma comanda, no se crea otra.
- Fallo si: aparece una comanda duplicada.
- Evidencia: consulta previa + IDs en `bdp_id_map`.

### D05 — Añadir línea
- Previos: comanda existente con vínculo web.
- Pasos: añadir una línea desde la web; releer comanda.
- Esperado: misma comanda con el contenido exacto añadido.
- Fallo si: cambia el ID de comanda o el contenido no coincide.
- Evidencia: diff de contenido + `bdp_id_map`.

### D06 — Cambiar unidades o comentario
- Previos: comanda con líneas.
- Pasos: cambiar unidades o comentario de una línea; releer total.
- Esperado: la comanda se mantiene, total correcto.
- Fallo si: total incorrecto o se duplica la línea.
- Evidencia: total antes/después + log.

### D07 — Retirar línea
- Previos: comanda con líneas.
- Pasos: retirar una línea exacta; releer comanda.
- Esperado: resultado exacto, o función desactivada si no está soportada.
- Fallo si: la línea persiste cobrada o el total no cuadra.
- Evidencia: contenido final + decisión documentada si se desactiva.

### D08 — Toque simultáneo caja y web
- Previos: misma venta abierta en caja y web.
- Pasos: modificar a la vez en ambos lados; observar resolución.
- Esperado: conflicto visible, nada se pisa en silencio.
- Fallo si: un cambio desaparece sin aviso.
- Evidencia: log de conflicto + estado final.

### D09 — Anular desde la web
- Previos: comanda cobrable o abierta.
- Pasos: anular desde la web; comprobar caja.
- Esperado: anulación coherente en caja.
- Fallo si: la comanda sigue activa en caja.
- Evidencia: estado en ambos lados + log.

### D09a — Anular con efecto en cocina
- Previos: venta con líneas enviadas a cocina.
- Pasos: anular con `CancelOrder`; comprobar impresoras auxiliares.
- Esperado: la anulación sale por impresoras auxiliares.
- Fallo si: cocina no recibe la anulación.
- Evidencia: testigo de impresora auxiliar + estado.

### D10 — Anular en caja
- Previos: venta vinculada web-caja.
- Pasos: anular en caja; comprobar web.
- Esperado: la misma venta queda anulada en la web.
- Fallo si: la web la sigue mostrando activa.
- Evidencia: estado en ambos lados + aviso si aplica.

## Pagos, propina, puntos y factura (Hito 3)

### D11 — Propina exacta
- Previos: venta abierta con propina configurable.
- Pasos: añadir propina (sumar frente a sustituir según distribuidor); releer.
- Esperado: propina exacta, sin duplicar.
- Fallo si: se duplica o se pierde al releer.
- Evidencia: importes antes/después + log.

### D12 — Puntos con motivo
- Previos: cliente con saldo de puntos.
- Pasos: mover puntos en flujo propio; releer saldo y motivo.
- Esperado: saldo coincide y queda el motivo; sin reintento ciego de `AddPoints`.
- Fallo si: saldo descuadra o hay doble abono.
- Evidencia: saldo antes/después + motivo.

### D13 — Pago parcial único
- Previos: venta abierta.
- Pasos: registrar un pago parcial; releer saldo y caja.
- Esperado: registrado una vez, saldo y caja correctos.
- Fallo si: doble registro o saldo incorrecto.
- Evidencia: movimientos + saldo.

### D13a — Parcial sin factura
- Previos: venta abierta.
- Pasos: cobrar parcial sin emitir factura; releer venta.
- Esperado: la venta sigue abierta con el saldo pendiente.
- Fallo si: se cierra sola o se emite factura.
- Evidencia: estado de la venta + saldo.

### D14 — Pago total
- Previos: venta abierta con saldo conocido.
- Pasos: pagar el total; releer venta.
- Esperado: liquida una sola vez.
- Fallo si: admite un segundo pago total.
- Evidencia: estado cerrada + movimientos.

### D15 — Respuesta perdida al pagar
- Previos: venta abierta.
- Pasos: pagar provocando pérdida de respuesta; verificar antes de repetir.
- Esperado: un solo pago tras verificar.
- Fallo si: segundo pago sin verificación.
- Evidencia: verificación + movimientos.

### D15t — Timeout al pagar
- Previos: venta abierta.
- Pasos: pagar con timeout simulado; consultar estado; solo entonces decidir.
- Esperado: se consulta antes de repetir, sin duplicar.
- Fallo si: se repite a ciegas.
- Evidencia: consulta + decisión + movimientos.

### D16 — Pago de caja reflejado
- Previos: venta vinculada.
- Pasos: cobrar en caja; comprobar web (repaso periódico, el aviso no trae pagos).
- Esperado: el pago aparece en la misma venta web.
- Fallo si: queda invisible tras el repaso.
- Evidencia: repaso + estado.

### D17 — Tope de tres pagos
- Previos: venta abierta.
- Pasos: intentar un cuarto pago.
- Esperado: la web lo impide.
- Fallo si: acepta el cuarto pago.
- Evidencia: rechazo + log.

### D18 — Factura con número único
- Previos: venta cobrada sin facturar.
- Pasos: facturar desde la web; releer número.
- Esperado: número de factura único.
- Fallo si: número duplicado o ausente.
- Evidencia: `InvoiceNumber` + documento.

### D18a — Simplificada al cobrar
- Previos: perfil de factura simplificada.
- Pasos: cobrar; comprobar estado fiscal.
- Esperado: sale facturada, no se re-factura.
- Fallo si: queda sin facturar o admite re-factura.
- Evidencia: estado fiscal + documento.

### D18b — InvoiceOrder sobre no facturada
- Previos: comanda existente todavía no facturada.
- Pasos: llamar a `InvoiceOrder`; releer.
- Esperado: número único emitido.
- Fallo si: error o número duplicado.
- Evidencia: `InvoiceNumber` + documento.

### D18c — InvoiceOrder sobre ya facturada
- Previos: comanda ya facturada.
- Pasos: llamar a `InvoiceOrder` de nuevo.
- Esperado: no emite segunda factura.
- Fallo si: emite duplicado.
- Evidencia: rechazo + una sola factura.

### D18d — Completa con NIF
- Previos: cliente con NIF.
- Pasos: facturar con `BillingDetails`; comparar con datos del cliente.
- Esperado: usa `BillingDetails` frente a datos del cliente.
- Fallo si: mezcla datos o falta el NIF.
- Evidencia: documento + datos usados.

### D19 — Respuesta perdida al facturar
- Previos: venta cobrada sin facturar.
- Pasos: facturar provocando pérdida de respuesta; verificar antes de repetir.
- Esperado: sin segunda factura.
- Fallo si: duplicado fiscal.
- Evidencia: verificación + una sola factura.

### D19t — Timeout al facturar
- Previos: venta cobrada sin facturar.
- Pasos: facturar con timeout simulado; consultar; solo entonces decidir.
- Esperado: se consulta antes de repetir, sin duplicar.
- Fallo si: se repite a ciegas.
- Evidencia: consulta + decisión + una sola factura.

### D20 — Aviso y documento enlazados
- Previos: venta finalizada en caja.
- Pasos: cruzar aviso de finalizada + `Status 3` + `InvoiceNumber` + documento.
- Esperado: aviso y documento enlazados en una sola venta.
- Fallo si: falta la relación (incidencia automática).
- Evidencia: enlace OrderId-aviso-exportación.

## Avisos y caídas (Hitos 2 y 5)

### D21 — Siete avisos
- Previos: los siete avisos configurados en el distribuidor.
- Pasos: provocar cada tipo; comprobar llegada al receptor.
- Esperado: llegan los siete configurados.
- Fallo si: falta alguno sin causa documentada.
- Evidencia: log del receptor por tipo.

### D21a — Aviso de traspaso
- Previos: plano de prueba, receptor operativo.
- Pasos: traspasar en caja; inspeccionar el aviso recibido.
- Esperado: origen, destino, líneas y estado final correctos.
- Fallo si: falta algún campo o el estado final no coincide.
- Evidencia: aviso completo + estado en caja.

### D22 — Aviso repetido o desordenado
- Previos: receptor operativo con buzón.
- Pasos: ensayar cada aviso normal, duplicado y fuera de orden.
- Esperado: sin duplicar ni reabrir ventas cerradas.
- Fallo si: duplicado económico o reapertura.
- Evidencia: log de idempotencia por aviso.

### D22a — Traspaso duplicado o desordenado
- Previos: traspaso ya aplicado.
- Pasos: reenviar el mismo aviso y en orden inverso.
- Esperado: sin duplicar ni reabrir.
- Fallo si: doble movimiento o reapertura.
- Evidencia: log + estado final único.

### D23a — Trabajador caído, receptor operativo
- Previos: receptor encendido, trabajador detenido.
- Pasos: generar avisos con el trabajador caído; rearrancar.
- Esperado: los avisos esperan en el buzón y se procesan al rearrancar.
- Fallo si: se pierden con el buzón sano.
- Evidencia: buzón antes/después + procesado.

### D23b — Receptor inaccesible
- Previos: receptor detenido o sin red.
- Pasos: generar avisos; restaurar; reconciliar.
- Esperado: el aviso puede perderse; solo se recupera lo localizable.
- Fallo si: se da por recuperado lo no localizable.
- Evidencia: lista de recuperados + límite declarado.

### D23-m — Comanda manual con aviso perdido
- Previos: sin lista de abiertas confirmada (pregunta a BDP).
- Pasos: crear comanda manual en caja; buscarla por listado o declarar límite.
- Esperado: aparece por listado o se declara límite formal.
- Fallo si: queda invisible sin límite declarado.
- Evidencia: listado o límite firmado.

### D29 — Reinicio
- Previos: cola con avisos pendientes.
- Pasos: reiniciar; comprobar alerta, cola y orden.
- Esperado: alerta, cola conservada, orden recuperado.
- Fallo si: pérdida silenciosa o desorden con efecto económico.
- Evidencia: alerta + cola antes/después.

## Maestros, stock y documentos (Hito 4)

### D24 — Catálogo y departamentos
- Previos: cambio en caja (artículo o departamento).
- Pasos: cambiar en caja; leer desde la web sin botón.
- Esperado: el cambio llega solo.
- Fallo si: requiere acción manual o no llega.
- Evidencia: diff + cursor.

### D24-p — Plano
- Previos: plano con mesas y posiciones.
- Pasos: leer `Rooms/GetTables`; conservar coordenadas.
- Esperado: plano leído sin mover posiciones.
- Fallo si: pérdida de posición.
- Evidencia: diff + cursor.

### D24-m — Formas de pago y empleados
- Previos: formas de pago y empleados en caja.
- Pasos: leer `Tenders/GetList` + `Employees/Get`.
- Esperado: leídos sin crear registros.
- Fallo si: alta fantasma en la web.
- Evidencia: diff de listas.

### D25 — Clientes
- Previos: cliente cambiado en ambos lados.
- Pasos: sincronizar; resolver.
- Esperado: actualiza o marca conflicto, sin duplicar.
- Fallo si: cliente duplicado.
- Evidencia: resolución + IDs.

### D26 — Ajuste web con un solo movimiento
- Previos: stock conocido en un almacén.
- Pasos: ajustar desde la web; releer stock.
- Esperado: un solo movimiento, stock coincide.
- Fallo si: doble movimiento o descuadre.
- Evidencia: `bdp_stock_ledger` + stock.

### D26a — Consulta unitaria
- Previos: artículo y almacén conocidos.
- Pasos: consultar `GetStock`; probar con almacén inexistente.
- Esperado: stock exacto; error con almacén inexistente.
- Fallo si: valor erróneo o éxito silencioso con almacén falso.
- Evidencia: respuesta + error.

### D26b — Consulta en lista
- Previos: varios pares artículo-almacén.
- Pasos: consultar `GetListStock`; incluir un par inexistente.
- Esperado: pares exactos y error por línea.
- Fallo si: línea inexistente con éxito o pares cruzados.
- Evidencia: respuesta por línea.

### D26c — Regularización
- Previos: motivo válido e inválido.
- Pasos: regularizar con cada uno.
- Esperado: delta correcto; rechaza almacén o motivo inválido.
- Fallo si: acepta motivo inválido.
- Evidencia: movimiento + rechazo.

### D26d — Traspaso entre almacenes
- Previos: dos almacenes con stock conocido.
- Pasos: traspasar con `Transfers`; probar artículo no inventariable.
- Esperado: resta y suma; rechaza no inventariable.
- Fallo si: descuadre entre almacenes.
- Evidencia: stocks antes/después + rechazo.

### D26e — Ajuste delta
- Previos: stock conocido, artículo con y sin escandallo.
- Pasos: ajustar con `UpdateStock` en ambos casos.
- Esperado: suma o resta correcta, con escandallo automático si aplica.
- Fallo si: ignora el escandallo.
- Evidencia: movimiento + stock.

### D26f — Inventario absoluto
- Previos: stock conocido.
- Pasos: fijar con `UpdateMassiveInventory`; probar valor nulo.
- Esperado: deja el stock declarado; nulo no mueve nada.
- Fallo si: nulo genera movimiento.
- Evidencia: stock final + caso nulo.

### D26g — Respuesta perdida en stock
- Previos: operación de stock pendiente.
- Pasos: operar provocando pérdida de respuesta; consultar antes de repetir.
- Esperado: un solo movimiento tras consultar.
- Fallo si: doble movimiento.
- Evidencia: consulta + un solo movimiento.

### D27 — Venta de caja sin doble descuento
- Previos: venta en caja con movimientos de stock.
- Pasos: converger la venta en la web; releer stock.
- Esperado: converge sin doble descuento.
- Fallo si: doble descuento.
- Evidencia: stock antes/después + `bdp_stock_ledger`.

### D28 — Releer sin duplicar
- Previos: facturas, ventas y albaranes existentes.
- Pasos: releer documentos; comprobar conteos.
- Esperado: no duplica facturas, ventas ni albaranes.
- Fallo si: duplicado en cualquier tipo.
- Evidencia: conteos antes/después.

### D28-auto — Exportación programada
- Previos: perfil de exportación con programación.
- Pasos: invocar `ExportProfiles/Documents` desde código; probar `OnlyLastMovements` true/false y ventanas solapadas con cursor, sin pulsar botones.
- Esperado: sin huecos ni duplicados.
- Fallo si: hueco o duplicado en cualquier ventana.
- Evidencia: series por ventana + cursor.

### D28a — Albaranes automáticos y completos
- Previos: albaranes pendientes en el perfil.
- Pasos: importar con `ExportPurchaseNotes` de forma automática; comprobar integridad.
- Esperado: importación automática y completa, sin botón.
- Fallo si: requiere botón o llega incompleto.
- Evidencia: albaranes importados + completitud.

## Llamar, excepciones y piloto (Hito 5)

### D31 — Llamar sin estado
- Previos: receptor operativo.
- Pasos: llamar desde la web; comprobar caja y estado posterior.
- Esperado: avisa en caja sin dejar estado.
- Fallo si: silencio o estado residual.
- Evidencia: log + hora + estado limpio.

### D32 — Mensaje ante cada excepción
- Previos: lista de operaciones no soportadas (iniciar traspaso, borrar, rectificativas, devoluciones).
- Pasos: intentar cada una desde la web.
- Esperado: cada excepción muestra procedimiento y responsable en la web.
- Fallo si: error críptico o sin responsable.
- Evidencia: mensajes + responsables.

### D30 — Piloto
- Previos: todas las pruebas anteriores superadas, responsables asignados.
- Pasos: turno piloto observado por etapas con caja cuadrada; rollback probado.
- Esperado: cero duplicados; caja, facturas y stock cuadrados.
- Fallo si: duplicado económico, descuadre fiscal, pérdida de comanda o TPV degradado.
- Evidencia: hoja de resultados + aceptaciones por escrito.
