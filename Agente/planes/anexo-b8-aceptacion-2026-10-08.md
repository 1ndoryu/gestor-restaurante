# Anexo B8: fichas de aceptación D (08/10/2026, rev. 10/10/2026 con 5ª tanda)

Fichas de cada prueba de aceptación de la fase D. Cada ficha define previos, datos utilizados, pasos exactos, resultado esperado, qué error la detiene y evidencia en T2. Se entrega como archivo separado del PDF de alcance. Matriz resumen en el alcance; correspondencia con la guía operativa (A01–A20) en el alcance.

## Regla para declarar una ficha cerrada (2026-10-26)

Una función programada no equivale a una función demostrada. Una ficha solo figura como **cerrada** si el entorno cumple sus requisitos (licencia, configuración), se ejecuta la operación real, el resultado se ve en la web y en BDP y se conserva la evidencia. Si falta algo, el estado es **pendiente de licencia**, **de configuración**, **de prueba local** o **de piloto**; **límite confirmado** solo si BDP o su documentación lo confirman. Una prueba hecha con la edición DEMO o gratuita demuestra el código, no el flujo de producción. Estado vigente por ficha: `Agente/completados/informe-fase-d-2026-10-22.md` §1.

## Checklist de ejecución

Estado: fichas de aceptación pendientes de ejecución; el avance real de cada prueba se registra en `plan-fase-d-bdp-local-2026-10-10.md` (§8 y §10).

- [ ] Hito 1 — Base y secretos (D01–D02)
- [ ] Hito 2 — Comandas y cocina (D03–D10, D21–D23)
- [ ] Hito 3 — Pagos, propina, puntos y factura (D11–D20)
- [ ] Hito 4 — Maestros, stock y documentos (D24–D28)
- [ ] Hito 5 — Llamar, excepciones y piloto (D29–D32)

## Base y secretos (Hito 1)

### D01 — Salud, versión, licencia 84 y acceso
- Previos: servicio 84 activo, serie `PRUEBA` asignada al terminal.
- Datos: terminal local 36.2, serie `PRUEBA`, licencia esperada 84.
- Pasos: consultar salud, versión, licencia 84 y acceso; registrar valores.
- Esperado: los cuatro responden y quedan registrados.
- Fallo si: algún valor falta o no coincide con el terminal.
- Evidencia: registro en `bdp_sync_cursor` + captura.

### D02 — Contraseñas y secretos
- Previos: acceso de administración a secretos.
- Datos: contraseña vieja y nueva del terminal local (no se escriben en el informe).
- Pasos: intentar entrar con la contraseña vieja; entrar con la nueva; revisar registros.
- Esperado: la vieja no entra; la nueva sí; sin secretos legibles pero con registro de intentos.
- Fallo si: la vieja entra o aparece un secreto legible en un registro.
- Evidencia: log de intentos + revisión de registros.

## Comandas y cocina (Hito 2)

### D03 — Crear comanda vinculada
- Previos: receptor operativo, serie `PRUEBA`.
- Datos: venta web de prueba (artículo 1001, 1 unidad, serie `PRUEBA`).
- Pasos: crear una venta desde la web; localizar la comanda en caja.
- Esperado: una sola comanda en caja, vinculada a la venta web.
- Fallo si: cero o más de una comanda, o sin vínculo.
- Evidencia: IDs en `bdp_id_map`.

### D03a — Crear con envío a cocina
- Previos: receptor operativo, impresora de cocina configurada.
- Datos: venta web con `OrderEndType=0`, serie `PRUEBA`.
- Pasos: crear venta con `OrderEndType=0`; comprobar estado y salida por cocina.
- Esperado: la venta queda aparcada y sale por cocina.
- Fallo si: no aparca o no sale por cocina.
- Evidencia: estado de la venta + testigo de cocina.

### D04 — Respuesta perdida al crear
- Previos: receptor operativo, serie `PRUEBA`.
- Datos: venta web de prueba con identificador local estable.
- Pasos: crear venta provocando pérdida de respuesta; consultar antes de repetir.
- Esperado: se recupera la misma comanda, no se crea otra.
- Fallo si: aparece una comanda duplicada.
- Evidencia: consulta previa + IDs en `bdp_id_map`.

### D05 — Añadir línea
- Previos: comanda existente con vínculo web.
- Datos: comanda vinculada, línea nueva (artículo 1001, 1 unidad).
- Pasos: añadir una línea desde la web; releer comanda; comprobar salida por cocina.
- Esperado: misma comanda con el contenido exacto añadido; la corrección sale por cocina.
- Fallo si: cambia el ID de comanda, el contenido no coincide o cocina no recibe la corrección.
- Evidencia: diff de contenido + `bdp_id_map` + testigo de cocina.

### D06 — Cambiar unidades o comentario
- Previos: comanda con líneas.
- Datos: comanda con 1 línea (2 unidades); cambio a 3 unidades o comentario nuevo.
- Pasos: cambiar unidades o comentario de una línea; releer total; comprobar salida por cocina.
- Esperado: la comanda se mantiene, total correcto; la corrección sale por cocina.
- Fallo si: total incorrecto, se duplica la línea o cocina no recibe la corrección.
- Evidencia: total antes/después + log + testigo de cocina.

### D07 — Retirar línea
- Previos: comanda con líneas.
- Datos: comanda con 2 líneas; retirada de una línea exacta.
- Pasos: retirar una línea exacta; releer comanda; comprobar salida por cocina.
- Esperado: resultado exacto y la corrección sale por cocina, o función desactivada si no está soportada.
- Fallo si: la línea persiste cobrada, el total no cuadra o cocina no recibe la corrección.
- Evidencia: contenido final + testigo de cocina + decisión documentada si se desactiva.

### D08 — Toque simultáneo caja y web
- Previos: misma venta abierta en caja y web.
- Datos: venta vinculada abierta en ambos lados.
- Pasos: modificar a la vez en ambos lados; observar resolución.
- Esperado: conflicto visible, nada se pisa en silencio.
- Fallo si: un cambio desaparece sin aviso.
- Evidencia: log de conflicto + estado final.

### D09 — Anular desde la web
- Previos: comanda cobrable o abierta.
- Datos: comanda vinculada abierta, serie `PRUEBA`.
- Pasos: anular desde la web; comprobar caja.
- Esperado: anulación coherente en caja.
- Fallo si: la comanda sigue activa en caja.
- Evidencia: estado en ambos lados + log.

### D09a — Anular con efecto en cocina
- Previos: venta con líneas enviadas a cocina.
- Datos: venta con líneas en cocina; `CancelOrder` con `PosId` y `OrderIdentifier` (sin `EmployeeId`, que el manual no lista para esta llamada).
- Pasos: anular con `CancelOrder`; comprobar impresoras auxiliares.
- Esperado: la anulación sale por impresoras auxiliares.
- Fallo si: cocina no recibe la anulación.
- Evidencia: testigo de impresora auxiliar + estado.

### D10 — Anular en caja
- Previos: venta vinculada web-caja.
- Datos: venta vinculada abierta.
- Pasos: anular en caja; comprobar web.
- Esperado: la misma venta queda anulada en la web.
- Fallo si: la web la sigue mostrando activa.
- Evidencia: estado en ambos lados + aviso si aplica.

## Pagos, propina, puntos y factura (Hito 3)

### D11a — Propina sumada (AddTip=true)
- Previos: venta abierta con propina configurable.
- Datos: base 10,00 €, propina 2,00 €, `AddTip=true`.
- Pasos: añadir propina 2,00 € con `AddTip=true`; releer comanda.
- Esperado: total 12,00 €; la propina se suma a la base.
- Fallo si: sustituye la base o se duplica.
- Evidencia: importes antes/después + log.

### D11b — Propina sustituida (AddTip=false)
- Previos: venta abierta con propina configurable.
- Datos: propina inicial 3,00 €, nueva 5,00 €, `AddTip=false`.
- Pasos: fijar propina 3,00 € con `AddTip=false`; refijar 5,00 €; releer.
- Esperado: queda 5,00 €; la propina sustituye, no suma.
- Fallo si: suma (8,00 €) o se duplica.
- Evidencia: importes antes/después + log.

### D11c — Propina a cero (AddTip=false)
- Previos: venta con propina 5,00 €.
- Datos: propina 5,00 € a fijar a 0,00 € con `AddTip=false`.
- Pasos: fijar propina 0,00 € con `AddTip=false`; releer comanda.
- Esperado: propina 0,00 € sin residuo.
- Fallo si: queda resto o reaparece al releer.
- Evidencia: importes antes/después + log.

### D12a — Puntos positivos
- Previos: cliente con saldo de puntos conocido.
- Datos: saldo S, abono +N con ID local único, motivo de prueba.
- Pasos: abonar +N en flujo propio con ID único; releer saldo y motivo.
- Esperado: saldo S+N con el motivo registrado; sin reintento ciego de `AddPoints`.
- Fallo si: saldo descuadra o hay doble abono.
- Evidencia: saldo antes/después + motivo + ID.

### D12b — Puntos negativos
- Previos: cliente con saldo de puntos conocido.
- Datos: saldo S, cargo −M con ID local único, motivo de prueba.
- Pasos: cargar −M en flujo propio con ID único; releer saldo y motivo.
- Esperado: saldo S−M con el motivo registrado.
- Fallo si: saldo descuadra o hay doble cargo.
- Evidencia: saldo antes/después + motivo + ID.

### D12c — Pérdida de respuesta en puntos
- Previos: cliente con saldo conocido.
- Datos: movimiento de +N con ID local único, pérdida de respuesta provocada.
- Pasos: mover puntos provocando pérdida de respuesta; consultar antes de repetir.
- Esperado: un solo movimiento tras consultar.
- Fallo si: doble movimiento o repetición a ciegas.
- Evidencia: consulta + un solo movimiento + ID.

### D13 — Pago parcial único
- Previos: venta abierta.
- Datos: venta abierta, parcial de prueba con ID local estable.
- Pasos: registrar un pago parcial; releer saldo y caja.
- Esperado: registrado una vez, saldo y caja correctos.
- Fallo si: doble registro o saldo incorrecto.
- Evidencia: movimientos + saldo.

### D13a — Parcial sin factura
- Previos: venta abierta.
- Datos: venta abierta, parcial sin `Invoice`.
- Pasos: cobrar parcial sin emitir factura; releer venta.
- Esperado: la venta sigue abierta con el saldo pendiente.
- Fallo si: se cierra sola o se emite factura.
- Evidencia: estado de la venta + saldo.

### D14 — Pago total
- Previos: venta abierta con saldo conocido.
- Datos: venta abierta, saldo X, pago total con ID local estable.
- Pasos: pagar el total; releer venta.
- Esperado: liquida una sola vez.
- Fallo si: admite un segundo pago total.
- Evidencia: estado cerrada + movimientos.

### D14s — Doble clic o pagos simultáneos
- Previos: venta abierta con saldo conocido.
- Datos: venta abierta, saldo X, ID de pago local estable.
- Pasos: disparar dos pagos totales en paralelo (o doble clic); releer movimientos.
- Esperado: solo se realiza un cobro.
- Fallo si: dos cobros o doble movimiento.
- Evidencia: movimientos + consulta + log de idempotencia.

### D15 — Respuesta perdida al pagar
- Previos: venta abierta.
- Datos: pago con ID local estable, pérdida de respuesta provocada.
- Pasos: pagar provocando pérdida de respuesta; verificar antes de repetir.
- Esperado: un solo pago tras verificar.
- Fallo si: segundo pago sin verificación.
- Evidencia: verificación + movimientos.

### D15t — Timeout al pagar
- Previos: venta abierta.
- Datos: pago con ID local estable, timeout simulado.
- Pasos: pagar con timeout simulado; consultar estado; solo entonces decidir.
- Esperado: se consulta antes de repetir, sin duplicar.
- Fallo si: se repite a ciegas.
- Evidencia: consulta + decisión + movimientos.

### D16 — Pago de caja reflejado
- Previos: venta vinculada.
- Datos: venta vinculada abierta.
- Pasos: cobrar en caja; comprobar web (repaso periódico, el aviso no trae pagos).
- Esperado: el pago aparece en la misma venta web.
- Fallo si: queda invisible tras el repaso.
- Evidencia: repaso + estado.

### D17 — Tope de tres pagos
- Previos: venta abierta.
- Datos: venta con tres pagos registrados.
- Pasos: intentar un cuarto pago.
- Esperado: la web lo impide.
- Fallo si: acepta el cuarto pago.
- Evidencia: rechazo + log.

### D18 — Factura con número único
- Previos: venta cobrada sin facturar.
- Datos: venta cobrada sin facturar, serie `PRUEBA`.
- Pasos: facturar desde la web; releer número.
- Esperado: número de factura único.
- Fallo si: número duplicado o ausente.
- Evidencia: `InvoiceNumber` + documento.

### D18a — Simplificada al cobrar
- Previos: perfil de factura simplificada; `PosId` y `EmployeeId` válidos en BDP.
- Datos: venta con saldo pendiente y perfil de simplificada.
- Pasos: pagar el saldo completo con `AddOrderPayment` (`Invoice=true`); repetir con un pago parcial en otra venta.
- Esperado: pago total: sale facturada y no se re-factura; pago parcial: no genera factura.
- Fallo si: pago total queda sin facturar o admite re-factura; pago parcial genera factura.
- Evidencia: estado fiscal + documento; prueba del pago parcial sin factura.

### D18b — InvoiceOrder sobre no facturada
- Previos: comanda existente todavía no facturada.
- Datos: OrderId de comanda sin facturar, PosId/EmployeeId reales.
- Pasos: llamar a `InvoiceOrder`; releer.
- Esperado: número único emitido.
- Fallo si: error o número duplicado.
- Evidencia: `InvoiceNumber` + documento.

### D18c — Guarda Status=3 antes de facturar
- Previos: comanda ya facturada (`Status=3`).
- Datos: OrderId de comanda facturada.
- Pasos: consultar la comanda con `GetOrder`; comprobar `Status=3`; no llamar a `InvoiceOrder`.
- Esperado: la web no llama a `InvoiceOrder` y no se emite una segunda factura (la protección la hace la integración, no el BDP).
- Fallo si: se llama a `InvoiceOrder` sobre una comanda con `Status=3`.
- Evidencia: consulta con `Status=3` + ausencia de llamada + una sola factura.

### D18d — Completa con NIF
- Previos: cliente con NIF.
- Datos: cliente con NIF real de prueba, `BillingDetails` completo.
- Pasos: facturar con `BillingDetails`; comparar con datos del cliente.
- Esperado: usa `BillingDetails` frente a datos del cliente.
- Fallo si: mezcla datos o falta el NIF.
- Evidencia: documento + datos usados.

### D19 — Respuesta perdida al facturar
- Previos: venta cobrada sin facturar.
- Datos: venta cobrada sin facturar, pérdida de respuesta provocada.
- Pasos: facturar provocando pérdida de respuesta; verificar antes de repetir.
- Esperado: sin segunda factura.
- Fallo si: duplicado fiscal.
- Evidencia: verificación + una sola factura.

### D19t — Timeout al facturar
- Previos: venta cobrada sin facturar.
- Datos: venta cobrada sin facturar, timeout simulado.
- Pasos: facturar con timeout simulado; consultar; solo entonces decidir.
- Esperado: se consulta antes de repetir, sin duplicar.
- Fallo si: se repite a ciegas.
- Evidencia: consulta + decisión + una sola factura.

### D20 — Aviso y documento enlazados
- Previos: venta finalizada en caja.
- Datos: venta finalizada con `InvoiceNumber` y documento exportado.
- Pasos: cruzar aviso de finalizada + `Status 3` + `InvoiceNumber` + documento.
- Esperado: aviso y documento enlazados en una sola venta.
- Fallo si: falta la relación (incidencia automática).
- Evidencia: enlace OrderId-aviso-exportación.

## Avisos y caídas (Hitos 2 y 5)

### D21 — Siete avisos
- Previos: los siete avisos configurados en el distribuidor.
- Datos: plano de prueba y ventas de prueba por tipo de aviso.
- Pasos: provocar cada tipo; comprobar llegada al receptor.
- Esperado: llegan los siete configurados.
- Fallo si: falta alguno sin causa documentada.
- Evidencia: log del receptor por tipo.

### D21a — Aviso de traspaso de comanda completa
- Previos: plano de prueba, receptor operativo.
- Datos: traspaso en caja de una comanda completa entre dos mesas del plano de prueba.
- Pasos: traspasar en caja; inspeccionar el aviso `Traspasar`; consultar la comanda con `GetOrder`.
- Esperado: el aviso trae identificadores, origen y destino; `GetOrder` da las líneas y el estado final correctos (el aviso no trae líneas).
- Fallo si: falta un identificador, origen o destino en el aviso, o el estado final no coincide con caja.
- Evidencia: aviso completo + respuesta de `GetOrder` + estado en caja.

### D22 — Aviso repetido o desordenado
- Previos: receptor operativo con buzón.
- Datos: avisos de prueba por cada uno de los 7 tipos.
- Pasos: ensayar cada aviso normal, duplicado y fuera de orden.
- Esperado: sin duplicar ni reabrir ventas cerradas.
- Fallo si: duplicado económico o reapertura.
- Evidencia: log de idempotencia por aviso.

### D22a — Traspaso de línea, repetido o desordenado
- Previos: línea traspasada en caja y aplicada en el receptor.
- Datos: aviso `TraspasarLinea` de una línea entre dos mesas del plano de prueba.
- Pasos: traspasar una línea; reenviar el mismo aviso y luego en orden inverso; consultar origen y destino con `GetOrder`.
- Esperado: la línea aparece una sola vez en destino y ya no figura en origen.
- Fallo si: línea duplicada, perdida o doble movimiento.
- Evidencia: `GetOrder` de origen y destino + log de idempotencia.

### D23a — Trabajador caído, receptor operativo
- Previos: receptor encendido, trabajador detenido.
- Datos: avisos de prueba generados con el trabajador caído.
- Pasos: generar avisos con el trabajador caído; rearrancar.
- Esperado: los avisos esperan en el buzón y se procesan al rearrancar.
- Fallo si: se pierden con el buzón sano.
- Evidencia: buzón antes/después + procesado.

### D23b — Receptor inaccesible
- Previos: receptor detenido o sin red.
- Datos: avisos generados sin receptor.
- Pasos: generar avisos; restaurar; reconciliar.
- Esperado: el aviso puede perderse; solo se recupera lo localizable.
- Fallo si: se da por recuperado lo no localizable.
- Evidencia: lista de recuperados + límite declarado.

### D23-m — Comanda manual con aviso perdido
- Previos: sin lista de abiertas confirmada (pregunta a BDP).
- Datos: comanda manual creada en caja.
- Pasos: crear comanda manual en caja; buscarla por listado o declarar límite.
- Esperado: aparece por listado o se declara límite formal.
- Fallo si: queda invisible sin límite declarado.
- Evidencia: listado o límite firmado.

### D29 — Reinicio (tres pruebas separadas)
- Previos: cola con avisos pendientes y cursor guardado.
- Datos: avisos pendientes en cola.
- Pasos: en pruebas separadas, reiniciar (1) el proceso de sincronización, (2) WebLinkRestAPI o el servicio que lo aloja y (3) BDP.NET. En cada una comprobar alerta, cola, cursor, reanudación y conciliación.
- Esperado: alerta visible, cola conservada, cursor recuperado, reanudación sin pérdidas ni duplicados y conciliación sin diferencias.
- Fallo si: pérdida silenciosa o desorden con efecto económico.
- Evidencia: alerta + cola y cursor antes/después + conciliación por componente.

## Maestros, stock y documentos (Hito 4)

### D24a — Alta en caja
- Previos: caja con catálogo gestionable.
- Datos: artículo o departamento nuevo en caja.
- Pasos: dar de alta en caja; leer desde la web sin botón.
- Esperado: el alta llega sola.
- Fallo si: requiere acción manual o no llega.
- Evidencia: diff + cursor.

### D24b — Modificación en caja
- Previos: artículo o departamento existente.
- Datos: cambio de precio o nombre en caja.
- Pasos: modificar en caja; leer desde la web sin botón.
- Esperado: el cambio llega solo.
- Fallo si: requiere acción manual o no llega.
- Evidencia: diff + cursor.

### D24c — Desactivación en caja
- Previos: artículo o departamento activo.
- Datos: desactivación en caja.
- Pasos: desactivar en caja; leer desde la web sin botón.
- Esperado: llega como inactivo, no desaparece.
- Fallo si: desaparece o sigue activo.
- Evidencia: diff + cursor.

### D24d — Borrado en caja
- Previos: artículo o departamento existente.
- Datos: borrado en caja.
- Pasos: borrar en caja; leer desde la web sin botón.
- Esperado: llega el borrado, o D32 muestra procedimiento y responsable (si la API no expone el borrado).
- Fallo si: queda fantasma sin mensaje ni responsable.
- Evidencia: diff + cursor, o mensaje D32.

### D24-p — Plano
- Previos: plano con mesas y posiciones.
- Datos: plano con mesas y coordenadas web conservadas.
- Pasos: leer `Rooms/GetTables`; conservar coordenadas.
- Esperado: plano leído sin mover posiciones.
- Fallo si: pérdida de posición.
- Evidencia: diff + cursor.

### D24-m — Formas de pago y empleados
- Previos: formas de pago y empleados en caja.
- Datos: listas de `Tenders/GetList` + `Employees/Get`.
- Pasos: leer `Tenders/GetList` + `Employees/Get`.
- Esperado: leídos sin crear registros.
- Fallo si: alta fantasma en la web.
- Evidencia: diff de listas.

### D25 — Clientes: tres casos
- Previos: cliente vinculado a un código BDP.
- Datos: (a) cambio solo en BDP; (b) cambio solo en la web; (c) cambios contradictorios en ambos lados.
- Pasos: (a) cambiar en BDP y comprobar la actualización en la web con la lectura `ExportCustomers`; (b) cambiar en la web y enviar con `CreateCustomer`: el manual solo lo admite con `Overwrite=true`, que sobrescribe el cliente, así que requiere aprobación; (c) provocar cambios contradictorios y sincronizar.
- Esperado: (a) la web actualiza el cliente; (b) BDP lo recibe si se aprueba la sobrescritura, y si no se aprueba el caso queda declarado no soportado; (c) conflicto visible, nunca sobrescritura silenciosa.
- Fallo si: cliente duplicado, sobrescritura silenciosa o conflicto no visible.
- Evidencia: diff del cliente en ambos lados + IDs + registro del conflicto.

### D26 — Ajuste web con un solo movimiento
- Previos: stock conocido en un almacén.
- Datos: artículo y almacén con stock conocido, delta de prueba.
- Pasos: ajustar desde la web; releer stock.
- Esperado: un solo movimiento, stock coincide.
- Fallo si: doble movimiento o descuadre.
- Evidencia: `bdp_stock_ledger` + stock.

### D26a — Consulta unitaria
- Previos: artículo y almacén conocidos.
- Datos: par artículo-almacén válido y almacén inexistente.
- Pasos: consultar `GetStock`; probar con almacén inexistente.
- Esperado: stock exacto; error con almacén inexistente.
- Fallo si: valor erróneo o éxito silencioso con almacén falso.
- Evidencia: respuesta + error.

### D26b — Consulta en lista
- Previos: varios pares artículo-almacén.
- Datos: una llamada por almacén con lista de `Article` o `Altern` válidos, un artículo inexistente dentro de una llamada válida, y un almacén inexistente en otra llamada.
- Pasos: consultar `GetListStock` una vez por almacén; repetir con el almacén inexistente.
- Esperado: pares exactos; el artículo inexistente da error solo en su línea. Ante almacén inexistente, el error que devuelva BDP (el manual solo documenta el error por línea: por confirmar en la prueba).
- Fallo si: línea inexistente con éxito, pares cruzados o un artículo inexistente que tumba toda la llamada.
- Evidencia: respuesta por línea + error de almacén.

### D26c — Regularización
- Previos: motivo válido e inválido.
- Datos: motivo válido e inválido del distribuidor, delta de prueba.
- Pasos: regularizar con cada uno.
- Esperado: delta correcto; rechaza almacén o motivo inválido.
- Fallo si: acepta motivo inválido.
- Evidencia: movimiento + rechazo.

### D26d — Traspaso entre almacenes
- Previos: dos almacenes con stock conocido.
- Datos: dos almacenes, artículo inventariable y uno no inventariable.
- Pasos: traspasar con `Transfers`; probar artículo no inventariable.
- Esperado: resta y suma; rechaza no inventariable.
- Fallo si: descuadre entre almacenes.
- Evidencia: stocks antes/después + rechazo.

### D26e — Ajuste delta
- Previos: stock conocido, artículo con y sin escandallo.
- Datos: artículo con escandallo y otro sin él, delta de prueba.
- Pasos: ajustar con `UpdateStock` en ambos casos.
- Esperado: suma o resta correcta, con escandallo automático si aplica.
- Fallo si: ignora el escandallo.
- Evidencia: movimiento + stock.

### D26f — Inventario absoluto
- Previos: stock conocido leído con `GetStock`.
- Datos: `Units` es decimal y nunca se envía `null`.
- Pasos: leer el stock y enviar con `UpdateMassiveInventory` la misma cantidad que ya hay; después repetir con un valor distinto para comprobar que sí hay movimiento.
- Esperado: con la misma cantidad, diferencia cero y ningún movimiento.
- Fallo si: se envía `null` o la diferencia cero genera movimiento.
- Evidencia: stock antes/después + ausencia de movimiento en el caso de diferencia cero.

### D26g — Respuesta perdida en stock
- Previos: stock inicial S registrado antes de escribir; bloqueo por artículo y almacén durante la operación. Estado real (2026-10-19): el esquema existe en `migrations/20261017120000_bdp_base_t2.up.sql` (`bdp_stock_ledger`, `id_local` único por usuario, índice de bloqueo `uq_bdp_stock_ledger_bloqueo` sobre `pendiente` por artículo y almacén) y está probado en `tests/bdp_base_t2.rs`. Lo que falta es el flujo de escritura que lo usa: ningún handler ni servicio de `src/` escribe todavía en el libro ni toma el bloqueo. BDP no devuelve identificador de operación. Fiabilidad de `GetStock` tras escribir: por confirmar.
- Datos: artículo y almacén con S conocido; variación V de prueba; ID local único (no lo devuelve BDP); pérdida de respuesta provocada. En el TPV las ventas también mueven stock: un valor ajeno puede venir de una venta, no de un error.
- Pasos: guardar S y V; enviar; provocar la pérdida; consultar `GetStock`: si ve S+V, no reintentar; si ve S, un único reintento controlado; si ve otro valor, parar y pasar a revisión manual.
- Esperado: como máximo un movimiento; cualquier otro valor queda en revisión manual.
- Fallo si: doble movimiento; reintento sin consultar; otro valor sin parar.
- Evidencia: S, V y lectura tras la pérdida + un solo movimiento.

### D27 — Venta de caja sin doble descuento
- Previos: venta en caja con movimientos de stock.
- Datos: venta en caja con líneas que mueven stock.
- Pasos: converger la venta en la web; releer stock.
- Esperado: converge sin doble descuento.
- Fallo si: doble descuento.
- Evidencia: stock antes/después + `bdp_stock_ledger`.

### D28 — Releer sin duplicar
- Previos: facturas, ventas y albaranes existentes.
- Datos: documentos existentes con conteos iniciales.
- Pasos: releer documentos; comprobar conteos.
- Esperado: no duplica facturas, ventas ni albaranes.
- Fallo si: duplicado en cualquier tipo.
- Evidencia: conteos antes/después.

### D28-auto — Exportación programada
- Previos: perfil de exportación con programación.
- Datos: perfil de exportación, ventanas con `OnlyLastMovements` true/false.
- Pasos: invocar `ExportProfiles/Documents` desde código; probar `OnlyLastMovements` true/false y ventanas solapadas con cursor, sin pulsar botones.
- Esperado: sin huecos ni duplicados.
- Fallo si: hueco o duplicado en cualquier ventana.
- Evidencia: series por ventana + cursor.

### D28-auto-R — Reinicio a mitad de exportación
- Previos: exportación en curso.
- Datos: ventana en curso con cursor guardado.
- Pasos: reiniciar a mitad; reanudar por cursor guardado; releer ventanas solapadas.
- Esperado: reanuda sin hueco ni duplicado.
- Fallo si: cursor perdido, hueco o duplicado.
- Evidencia: series por ventana + cursor antes/después + log.

### D28a — Albaranes automáticos y completos
- Previos: albaranes pendientes en el perfil.
- Datos: albaranes pendientes en el perfil de exportación.
- Pasos: importar con `ExportPurchaseNotes` de forma automática; comprobar integridad.
- Esperado: importación automática y completa, sin botón.
- Fallo si: requiere botón o llega incompleto.
- Evidencia: albaranes importados + completitud.

## Llamar, excepciones y piloto (Hito 5)

### D31 — Llamar sin estado
- Previos: receptor operativo.
- Datos: llamada de prueba desde una mesa vinculada.
- Pasos: llamar desde la web; comprobar caja y estado posterior.
- Esperado: avisa en caja sin dejar estado.
- Fallo si: silencio o estado residual.
- Evidencia: log + hora + estado limpio.

### D32 — Mensaje ante cada excepción
- Previos: lista de operaciones que la API no permite desde la web, con procedimiento y responsable asignados a cada una. Verificado en el manual 2026-10-10: series solo en lectura (`GetPOSSeriesList`); `InvoiceOrder` envía la factura en PDF por email, sin descarga; controles de cocina solo en lectura de dos campos de menú; el resto sin endpoint.
- Datos: una operación de la lista por intento: iniciar traspaso desde la web, eliminar artículos, rectificaciones, devoluciones, apertura/cierre, arqueo, entradas y salidas de efectivo, PDF de factura, reimpresión en cocina, controles avanzados de cocina, series, cambios posteriores al pago.
- Pasos: intentar cada una desde la web.
- Esperado: la web no la ejecuta y muestra el procedimiento y el responsable.
- Fallo si: error críptico, sin responsable, o la operación se ejecuta sin soporte.
- Evidencia: mensajes + responsables + lista verificada.

### D30 — Piloto
- Previos: todas las pruebas anteriores superadas; responsables asignados; umbral de rendimiento y procedimiento de rollback fijados por escrito.
- Datos: varios turnos consecutivos con caja y stock iniciales cuadrados.
- Pasos: operar los turnos por etapas; al cierre de cada uno cuadrar comandas, cobros, facturas, caja y stock; probar el rollback.
- Esperado: cero duplicados; comandas, cobros, facturas, caja y stock cuadrados; sin diferencias críticas; rendimiento dentro del umbral; rollback probado.
- Fallo si: duplicado económico, descuadre fiscal o de caja, pérdida de comanda, TPV degradado o rollback no probado.
- Evidencia: hoja de resultados por turno + rollback probado + aceptación escrita del responsable.
