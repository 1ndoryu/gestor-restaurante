# Respuesta de BDP al correo del 20/11 (`18AB-9`)

Recibida el 24/11/2026 07:04 de Gael (BDP Software), con `integraciones@bdpcenter.com`
en copia. Contesta a las seis dudas y al fallo de facturación de
`correo-bdp-2026-11-20.md`. Resumen de lo que dicen y de lo que nos obliga a hacer,
sin valorar todavía las alternativas comerciales.

## Lo que dicen, punto por punto

| # | Pregunta | Respuesta de BDP | Qué implica para nosotros |
|---|---|---|---|
| 1 | Dividir / unir cuentas | **No existe en la API.** Sugieren dividir en nuestra aplicación y enviar a BDP los pagos correspondientes hasta dar la comanda por pagada. | El camino que ya usamos (varios pagos parciales) es el que ellos proponen. Choca con nuestro tope de 3 pagos por venta si la mesa se parte entre más comensales. |
| 2 | Retirar un pago parcial / anular un cobro | **No existe endpoint.** Lo hace un empleado desde el TPV. Para pasarela: devolver por la pasarela y registrar en BDP un **pago negativo** por el mismo importe. | La web no debe ofrecer «retirar pago» ni «anular cobro»: no hay operación. El pago negativo es una alternativa a valorar, no una capacidad que tengamos. |
| 3 | `NotAllowedAsInvitation` | **El campo no se aplica en la infraestructura**: es la capa de negocio de la aplicación la que debe impedir invitar esos artículos. El terminal lo hace en su propio flujo de venta. | **Es nuestro trabajo**: leer la marca de la ficha del artículo y bloquear la invitación antes de enviarla. Hoy no se comprueba. |
| 4 | Cambio de mesa | **No existe endpoint.** Lo hace un empleado desde el TPV. | La web no debe ofrecer cambio de mesa en una comanda ya enviada; reenviarla la duplica en las dos mesas. |
| 5 | Comanda facturada | **Es lo previsto**: facturada deja de estar disponible como comanda, de ahí `Order: null` con `Status: 3` («Facturada»). Los estados de la API no cubren fases de preparación. | Confirma el diseño actual (guardar el número de ticket al facturar). No se puede usar `Status` para estados de cocina. |
| 6 | `Type=2` servicio a domicilio | La aplicación **01 Hostelería no lo admite**: hace falta **BDP Servicio a Domicilio**, que incluye Hostelería, y probablemente **sustituir la licencia** de Hostelería por una de Servicio a Domicilio. | No es un defecto nuestro ni del demo: es otra aplicación y otra licencia. Decisión de la usuaria con Guillermo/BDP, no un cambio de código. |
| 7 | Líneas con `Units: 0` | Lo dan por fallo: «la validación debería impedir que se llegara a crear la línea». Van a reproducirlo. | Aunque lo arreglen, nosotros debemos rechazar la línea de 0 uds antes de enviarla: no depende de su versión del TPV. |

Confirmaciones que nos ahorran dudas futuras:

- **`PaymentId` no es idempotente en BDP.** Ellos no lo contradicen; nuestro diseño ya
  protege con la consulta previa (`GetOrder`) antes de reenviar.
- **Quitar una línea exige `Units: -1`**, ya comprobado por nosotros.
- Ofrecen **`integraciones@bdpcenter.com`** como vía principal y teléfono
  **972 884 765**, lunes, miércoles y viernes de 9:00 a 11:00.

## Preguntas que nos devuelven

- ¿La aplicación la usan **camareros o los propios clientes**? De eso depende cómo
  deben tratar la restricción de invitación (punto 3) y el cambio de mesa (punto 4).
- Quieren **entender qué hace la aplicación y cómo se prevé que interactúe con BDP**,
  para proponer alternativas donde la API no llega.
- Para domicilio, piden **confirmar si vamos con BDP Net Hostelería** para decirnos
  cómo proceder con la licencia.

Ninguna de las tres se responde sin decisión de la usuaria; quedan anotadas en el
roadmap con el resto de lo que depende del piloto.

## Efecto sobre el roadmap

- `18AB-9` — cerrado: las respuestas están registradas aquí y las seis dudas quedan
  resueltas o convertidas en límite.
- `18AB-6` — cerrado como **límite**: `Type=2` exige otra aplicación y otra licencia;
  no hay nada que probar contra el demo.
- Trabajo de código resuelto en `18AB-11` (25/11): el catálogo cruza
  `NotAllowedAsInvitation` desde el perfil de items (el export no la manda), el envío
  exige cantidad mayor que 0 y `OPERACIONES_NO_API` recoge dividir/unir cuentas, cambio
  de mesa y retirar pago. Queda `18AB-12` (tope de 3 pagos frente a dividir cuentas).
