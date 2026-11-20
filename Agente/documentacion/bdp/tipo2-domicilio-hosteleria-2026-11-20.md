# `Type=2` (servicio a domicilio) en Hostelería 01 — 20/11/2026

Origen: punto 7 de la respuesta de Guillermo del 18/11. BDP demo del portátil (`127.0.0.1:8070`,
servicio 84), sesión con `BDP_LOGIN` del `.env`. Sondas en `C:/tmp/probe-bdp-18ab6` (no
versionadas; solo escriben en el demo). Nada de esto toca producción.

## Lo que dice el BDP

`/Service/GetVersion` → `{"Version":36,"Subversion":2,"Application":"01","ApplicationDescription":"Hostelería"}`.
`/Service/GetApplicationVersion` (aplicación 84) → WeblinkRestAPI 1.2, es decir, el servicio de
enlace está bien licenciado; lo que falta es la **aplicación de TPV**.

## Lo que hace el demo

| Prueba | Enviado | Respuesta |
|---|---|---|
| Control `Type=0` (barra) | 1 línea, salón 0, mesa 0 | OK, `OrderId` 134, sin `ErrorMessage` (cancelada después) |
| Control `Type=1` (mesa) | 1 línea, salón 1, mesa 2 | OK, `OrderId` 135, `BarCode` `KIN051UHKOUT` (cancelada después) |
| `Type=2` sin cliente | 1 línea, salón 0, mesa 0 | **`[300009]-LA APLICACIÓN NO ADMITE COMANDAS DE SERVICIO A DOMICILIO`**, `OrderId` 0 |
| `Type=2` con `Customer` completo (nombre, dirección, CP, ciudad, provincia, país, móvil, correo) | 1 línea, salón 0, mesa 0, `Customer` relleno | **El mismo 300009**: el rechazo no depende del cliente |
| `Type=2` en `OnlyCheck` (`OrderOperationType: 1`) | Mismo cuerpo | **El mismo 300009**: el tipo se rechaza antes de mirar nada más |

Los dos controles demuestran que la sonda y las credenciales funcionan: mismo cliente HTTP, mismo
artículo, misma forma de llamada. El rechazo no es de red ni de payload; es de la aplicación.

## La causa está en el manual

`# WEBLINK RESTAPI.md`, `CreateOrder` (línea 7176): «Las comandas de servicio a domicilio sólo se
podrán crear en la aplicación de **Servicio a Domicilio**». Y en la tabla de `Order.Type`
(línea 11311): `2 : Servicio a domicilio (delivery)`.

El demo corre la aplicación **01 Hostelería**, que admite `Type=0` y `Type=1` (y ese `Type=1` lo
exige con salón y mesa). No es un fallo de nuestro payload ni una carencia de WebLinkRestAPI:
`Type=2` está fuera del alcance de esta aplicación.

## Consecuencia

El hecho que pide `18AB-6` —comanda a domicilio completa (crear, modificar, cobrar, facturar,
recuperar tras reiniciar BDP)— **no se puede comprobar en el BDP demo**: para eso hace falta un
TPV con la suscripción de **Servicio a Domicilio**, que no tenemos.

Lo que sí queda cerrado en nuestro lado, con pruebas sin red:

- `resolve_order_type` (`src/services/bdp_sync.rs`) lee `bdp_order_type_map` del canal de la venta
  ignorando mayúsculas (`{"Comedor":1,"Delivery":2}`); un domicilio configurado viaja como `2`.
- `Type` solo se usa cuando la venta **no** tiene salón y mesa; con mesa manda `Type=1` y sus
  números (`bdp_sync_venta/payload.rs`).
- Un `300009` de BDP llega al operador como «BDP: delivery no soportado en este POS (300009)»
  (`bdp_sync_venta/utilidades.rs`).

Falta el extremo BDP. Se cierra cuando haya un TPV de Servicio a Domicilio donde probarlo, o
cuando BDP/Guillermo confirme qué aplicación hay que usar en el restaurante si va a vender a
domicilio.
