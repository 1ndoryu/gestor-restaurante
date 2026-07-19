# Guía del cliente — Revisión de la integración con BDP

> **Fecha:** 19 de julio de 2026
> **Objetivo:** comprobar únicamente las funciones que nuestro equipo no puede verificar sin crear o modificar información real en BDP.
> **Responsable:** personal autorizado del restaurante.

## Qué queda por comprobar

Solo quedan pendientes cuatro operaciones porque producen cambios reales en BDP:

| Prueba         | Cambio que producirá en BDP                                              |
| -------------- | ------------------------------------------------------------------------ |
| Crear cliente  | Dará de alta un cliente con un código nuevo                              |
| Crear comanda  | Creará una comanda que puede aparecer en TPV, cocina e informes          |
| Registrar pago | Marcará el saldo completo de la comanda como pagado y puede afectar caja |
| Facturar       | Emitirá una factura y puede afectar numeración e información fiscal      |

Estas funciones permanecen **no verificadas en la instalación real del restaurante** hasta que el cliente decida probarlas.

## Condiciones antes de probar

No comience hasta que el responsable del restaurante haya acordado:

- una hora de poca actividad;
- quién observará BDP, TPV y cocina;
- un cliente de prueba y un código confirmado como libre;
- una venta pequeña, claramente identificada como prueba;
- cómo anular o corregir manualmente cada registro si fuera necesario;
- que cualquier pago o factura de prueba sea aceptable para caja y administración.

Si el restaurante no dispone de un procedimiento para corregir o anular una comanda, un pago o una factura, esa prueba no debe realizarse.

## Reglas durante las pruebas

1. Realizar una sola acción cada vez.
2. Comprobar el resultado directamente en BDP antes de continuar.
3. No repetir una acción si la aplicación queda esperando, pierde la conexión o muestra un resultado dudoso.
4. Guardar una captura y anotar la hora, el importe y los identificadores mostrados.
5. Detener toda la revisión ante el primer resultado inesperado.

## 1. Crear un cliente

Usar un nombre que indique claramente que es una prueba y el código previamente confirmado como libre.

- [ ] Confirmar en BDP que el código todavía no existe.
- [ ] Autorizar únicamente la creación de ese cliente.
- [ ] Ejecutar la acción una sola vez desde Glory.
- [ ] Confirmar que BDP creó exactamente un cliente con el código y los datos esperados.
- [ ] Confirmar que Glory quedó vinculado al mismo código.
- [ ] Confirmar que ningún cliente existente fue reemplazado o modificado.

**Detenerse si:** aparece más de un cliente, se usa otro código, se modifica un cliente existente o el resultado no es claro.

## 2. Crear una comanda

Usar una venta pequeña con artículos reales conocidos y una descripción que permita reconocerla como prueba.

- [ ] Revisar previamente artículos, cantidades, precios, impuestos, descuentos, cliente, canal y total.
- [ ] Autorizar únicamente esa venta.
- [ ] Enviarla una sola vez desde Glory.
- [ ] Confirmar que BDP creó exactamente una comanda.
- [ ] Confirmar dónde apareció: TPV, cocina e informes.
- [ ] Comparar artículos, cantidades, total, mesa/canal, cliente y forma de pago.
- [ ] Confirmar que Glory muestra el mismo número y estado de comanda.

**Detenerse si:** se duplica la comanda, falta un artículo, cambia el total, se asigna otro cliente o Glory no puede identificar con certeza la comanda creada.

## 3. Registrar el pago

Esta prueba debe utilizar la comanda anterior y únicamente su saldo completo pendiente. No se probarán pagos parciales.

- [ ] Confirmar en BDP que la comanda sigue abierta, no está facturada y conserva el saldo esperado.
- [ ] Confirmar con caja la forma de pago que se utilizará.
- [ ] Autorizar únicamente el pago de esa comanda y por ese importe exacto.
- [ ] Ejecutar el pago una sola vez.
- [ ] Confirmar en BDP que existe un solo pago por el importe y la forma correctos.
- [ ] Confirmar que el saldo quedó en cero y que caja refleja el efecto esperado.

**Detenerse si:** no hay respuesta clara, aparece más de un pago, el importe o la forma de pago son incorrectos, o el saldo no coincide. No vuelva a pulsar el botón.

## 4. Facturar

Esta prueba puede tener consecuencias fiscales. Debe realizarse solamente con aprobación expresa de administración y con un procedimiento conocido para anular o corregir la factura.

- [ ] Confirmar que la comanda correcta está pagada y aún no está facturada.
- [ ] Autorizar únicamente la factura de esa comanda.
- [ ] Ejecutar la acción una sola vez.
- [ ] Confirmar en BDP que se emitió exactamente una factura.
- [ ] Confirmar número, serie, cliente, impuestos y total.
- [ ] Confirmar que Glory muestra el mismo número y estado facturado.

**Detenerse si:** no aparece un número claro, se emite más de una factura, cambia la serie o el total, o Glory y BDP no coinciden. No intente facturar nuevamente.

## Información que debe enviarnos el cliente

Por cada prueba, informar:

- fecha y hora;
- operación realizada;
- identificador del cliente, comanda, pago o factura;
- resultado mostrado en Glory;
- resultado observado en BDP, TPV, cocina o caja;
- captura sin contraseñas, tokens ni datos personales completos;
- cualquier diferencia encontrada.

## Criterio de aceptación

La integración real se considerará confirmada solamente si cada operación autorizada:

- crea exactamente un registro;
- usa los datos, importes y relaciones esperados;
- deja el mismo identificador y estado en Glory y BDP;
- no modifica otros registros;
- no requiere repetir una acción dudosa.

Las pruebas no realizadas continuarán registradas como **no verificadas en BDP real**, no como fallidas.
