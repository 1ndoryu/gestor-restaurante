# Segunda lista de Guillermo: pruebas contra el TPV demo (23/11/2026)

Todo lo de este documento se ejecutó contra el **TPV demo** del portátil
(`TpvNet.exe`, WebLink REST 36.2, puerto 8070, alcanzado desde la VPS 89 por el
túnel inverso 8071) y la aplicación de la 89 en modo `build` (puerto 3105).
**No hay ninguna escritura automática conectada al restaurante.**

## 1. Saldo de los pagos parciales (punto 1)

Antes de cobrar, la web refresca la comanda con `GetOrder` y el pendiente sale de
`Order.Total` menos los `Payments` de BDP; el ledger local solo aporta los pagos
sin identidad remota. La respuesta de saldo expone `origen: "bdp"` cuando manda
BDP y `origen: "local"` cuando no hay comanda.

| Prueba | Comanda | Resultado |
|---|---|---|
| Un pago parcial (3,50 de 10,50) | 181 | `pendiente` 10,50 → 7,00; BDP con 1 pago |
| Tres pagos de 3,50 (formas 1, 2, 1) | 189 | `pendiente` 10,50 → 7,00 → 3,50 → **0,00**; BDP con los 3 pagos |
| Cuarto pago (0,10) | 189 | **422** «la venta ya tiene 3 pagos registrados en BDP»; BDP sin cambios |
| Factura tras los tres pagos | 189 | `00001TB00000068`; saldo final `pendiente: "0"` |

La comanda 181 se facturó igual con su ticket `00001TB00000067`.

## 2. Invitaciones (punto 2) — bloqueadas, como se pidió

`INVITACIONES_HABILITADAS = false` en `src/services/invitacion.rs`. El servidor
rechaza cualquier línea con `invitacion: true` antes de tocar el TPV, y la
validación nombra las líneas culpables. El fallo de origen está localizado: el
mapeo del demo solo tiene activos `1001`, `1002` y `D26G-1`, así que un artículo
como `1003` caía al artículo por defecto **en silencio**. Pendiente `21AB-5b`.

**Comprobado el 24/11 con el interruptor encendido** (y devuelto a `false`
después, con el binario recompilado): la invitación llega al TPV como el
artículo realmente pedido —`GetOrder` devuelve `1002 SCHWEPPES LIMÓN` con
`Invitation: true`— y el ticket `00001TB00000069` imprime la línea `1002` a 0,00
con `Invitacion = 1` junto a la línea normal `1001`. El artículo sin
equivalencia activa (`1003`) se rechaza **antes de escribir** nombrando la línea.
El artículo por defecto ya solo queda para cuando no hay equivalencia, y entonces
avisa. Detalle en § 10.

## 3. Identificador de pago (punto 3)

El `PaymentId` identifica el **pago**, no el intento: sale de la clave de
idempotencia (`P<venta>-<clave>`, SHA-256 si es larga) y todos los reintentos de
ese mismo pago reutilizan exactamente el mismo.

| Prueba (venta 796ce0d4, comanda 188) | Resultado |
|---|---|
| Pago de 0,50 con clave `K` | `P796ce0d4571145-94b7…` |
| Reintento con la misma clave `K` | **mismo** `PaymentId`; BDP sigue con 1 pago |
| Pago con otra clave | `P796ce0d4571145-4bda…`; BDP con 2 pagos |

BDP 36.2 **no deduplica** `AddOrderPayment` por `PaymentId`: dos envíos con el
mismo id quedarían como dos cobros. La protección es nuestra y es la lectura
previa (`GetOrder`) antes de reenviar.

## 4. Tres pagos y cuarto rechazado (punto 4)

Ver la tabla del punto 1: comanda 189, tres pagos de 3,50 verificados uno a uno
en BDP (`GetOrder`), total completo, un solo ticket, y el cuarto rechazado con
422 **sin escribir nada** (BDP idéntico antes y después del intento).

**Repetido el 24/11 sobre el binario actual** (venta `db915a7a`, comanda 214,
10,50 en tres partes de 3,50, formas 1, 2 y 1): los tres pagos aparecen en BDP
(`P…e7a26f`, `P…ee4b68`, `P…5424c7`), `pendiente` 10,50 → 7,00 → 3,50 → **0,00**,
factura **`00001TB00000070`** con una sola línea `1001 COCA-COLA` × 10 a 10,50, y
**un solo** documento para esa comanda. El cuarto (0,10) devuelve 422 «la venta
ya tiene 3 pagos registrados en BDP» y BDP queda idéntico.

## 5. Reinicio real de BDP (punto 5)

1. `Stop-Process` sobre `TpvNet.exe` (PID 2672); puerto 8070 sin listener.
2. Alta de venta `feb84906-6264-46e3-8052-987a2e4ad89d` con el TPV apagado →
   `bdp_sync_error` de transporte y auditoría `create_order` abierta.
3. Arranque de `C:\Bdp-Net\NetHos\Prog\TpvNet.exe` (PID 5848).
4. El poller reautentica y crea la comanda: **182**, total 2,10, una sola línea.

Auditoría: un `create_order` en `error` (transporte) y **un solo** `create_order`
en `exito` con `{"order_id": 182}`. Una sola comanda para esa venta.

El 24/11 se repitió parando **los dos** procesos de BDP (`TpvNet.exe` —el que
sirve la WebLink REST en 8070— y `SvrWCF.exe`) con una venta a medias y un cobro
lanzado; el detalle de los tres escenarios y sus PIDs está en § 10.

## 6. Modificaciones rápidas / throttled (punto 6)

Venta `796ce0d4`, comanda de mesa 188 (salón 1, mesa 2).

| Prueba | Resultado |
|---|---|
| Dos ediciones en paralelo (2 y 3 unidades) | ambas 200; BDP aplica las dos como deltas |
| Serie 4, 5, 6 unidades | 200; BDP 4,2 → 5,25 → 6,30 |
| Estado final | **web 6 unidades / 6,30 = BDP 6 unidades / 6,30** |

Las modificaciones de una misma venta se serializan con un lock por venta
(espera acotada a 30 s), el throttled se espera en vez de rechazarse y la
modificación se reconcilia con `GetOrder`.

## 6-bis. Modificaciones rápidas, repetido el 24/11 (punto 6)

Venta `201b8b4f` (comanda 213, mesa 5): dos ediciones en paralelo y después la
serie 4 → 5 → 6 unidades; todas 200, BDP 4,20 → 5,25 → 6,30 y la web en 6
unidades / 6,30 (`/lineas` y los totales de la venta). Mismo `PaymentId` al
reintentar con la misma clave (`P201b8b4f24b745-e76eef84…`) y otro distinto con
clave nueva (`P201b8b4f24b745-e82e25ff…`), que era el punto 3.

## 7. Licencia del demo (punto 7)

El TPV sigue avisando «sin conexión con el servidor de subscripción» y sigue
operativo: todas las pruebas de este documento se hicieron con ese aviso en
pantalla. La pregunta para BDP está en
`Agente/documentacion/bdp/consulta-licencia-demo-2026-11-23.txt`.

## 8. Límites observados del TPV demo (no del producto)

- **Comandas de barra (`Type=0`) rechazan toda modificación** con
  `300033`/`300008`; las de mesa (`Type=1`) aceptan el contrato del 02/11.
- El manual del demo (pág. 40) limita a 6 líneas y a las mesas 1–3, pero
  **este ejecutable no aplica esos límites**: aceptó una comanda de 7 líneas y
  mesas fuera del rango. Solo aplica una comanda abierta por mesa.
- Tras facturar, `GetOrder` por `OrderId` devuelve `{"Order":null,"Status":3}`:
  el ticket ya no se puede releer. El número se guarda en la web al facturar.

## 8-bis. Corte DENTRO de un pago (24/11/2026) — defecto encontrado y corregido

El 23/11 el corte del punto 5 se hizo con el TPV ya caído (prueba de red). El
24/11 se hizo el corte **en mitad de la escritura**: matar `TpvNet.exe` con el
`AddOrderPayment` en vuelo. Barrido de retardos con la petición ya lanzada:

| Retardo del corte | Petición alcanzada | Efecto |
|---|---|---|
| 1000–1380 ms | `GetOrder` de auditoría previa | pago no enviado |
| **1400 ms** | **`/API/Orders/Payment/Add`** | `connection closed before message completed` |
| 1412 ms o más | — | ya respondido 200 |

Con el corte en 1400 ms, BDP **no** tenía el pago (`pagos: []`), pero la web
dejó una auditoría `add_payment` en `error` y el reintento con la misma clave
murió para siempre:

```text
422 {"error":"validation_error",
     "message":"idempotencia_duplicada:b7aa95b9-…:error"}
```

Causa: el índice único `(user_id, idempotency_key)` de `bdp_audit_log` admite un
solo asiento por clave, y la reconciliación escribía «reintento permitido» en el
mensaje sin liberar ese asiento. Además cada intento muerto filtraba su snapshot
pre-write.

Corregido en `21AB-5i` (`liberar_intento_sin_escribir`): antes de registrar la
intención, se lee el intento previo de esa clave, se comprueba con `GetOrder`
que el pago **no** está y solo entonces se borran auditoría y snapshot. Sin BDP
delante no se libera nada (nunca se reenvía a ciegas). Test de regresión con
`disconnect_before_apply`:
`corte_sin_aplicar_permite_reintentar_y_cobra_una_vez`.

## 9. Hallazgo para el roadmap (heredado, no de este cambio)

Siete auditorías abiertas de días anteriores (19/10 a 16/11) siguen bloqueando
escrituras de sus ventas, entre ellas un `inventory` que bloquea el volcado de
stock. `reconcile_invoice` e `reconcile_add_payment` solo cierran cuando la
comanda existe; si no existe, la dejan abierta para siempre. Detalle y forma de
arreglo: `21AB-5g` en el roadmap.

## 10. Parada real de BDP.NET y WebLinkRestAPI (24/11/2026)

### 10.1 Procesos que forman el TPV demo

Comprobado con `netsh http show servicestate view=session` y `Get-NetTCPConnection`:

| Proceso | Puerto | Papel |
|---|---|---|
| `TpvNet.exe` | 8070 (HTTP.sys, `HTTP://+:8070/`) | **WebLinkRestAPI**: aquí van `Auth/Login`, `AddOrder`, `AddOrderPayment`, `GetOrder`, facturación |
| `SvrWCF.exe` | 9011, 9021 | bloqueos y broadcast (`ConfigPuertosBloqueos.bdp`); **no está en el camino de escritura** |

Ninguno es servicio de Windows (`Get-Service` solo lista `MSSQL$BDPSOFTWARE`,
`MSSQL$BDPSOFTWARECF` y los `SQLTELEMETRY$*`). El autostart del TPV es
`%APPDATA%\...\Startup\bdp-tpv-arranque.vbs` (espera 90 s y lanza `TpvNet.exe`);
`SvrWCF.exe` no tiene lanzador propio.

Prueba de que `SvrWCF` no escribe: con `SvrWCF.exe` muerto y `TpvNet.exe` vivo,
el cobro entra sin problema. El corte que sí corta el cobro es matar `TpvNet.exe`.

### 10.2 Escenario completo: los dos procesos parados, operación pendiente y rearranque

Escenario del 24/11, venta `a0fe6d96-a6e7-405c-81d0-d99aa585e161` (4,40).

1. `Stop-Process -Id 31176` (`SvrWCF`) y `-Id 30724` (`TpvNet`): sin procesos y
   sin listener en 8070, 9011 ni 9021.
2. Alta de la venta con BDP parado → 201; el alta responde igual y el envío
   queda pendiente: `bdp_sync_error` «error de transporte BDP:
   `http://127.0.0.1:8071/Auth/Login` … Connection reset by peer», saldo
   `origen: "local"`.
3. Cobro intentado con BDP parado → **422 «no tiene bdp_order_id»**: sin comanda
   en BDP no se escribe ningún pago.
4. Arranque de los dos procesos con los PID nuevos `SvrWCF` 28548 y `TpvNet`
   26052.
5. El poller **se reautentica solo** y crea la comanda **215**: auditoría
   `create_order` en `error` (consulta previa sin BDP) y una sola en `exito`
   (`16:42:55`).
6. Cobro de 1,00 con clave `p5-real-reintento`, **enviado dos veces seguidas**:
   dos 200 al cliente, **una sola** auditoría `add_payment` en `exito`, y BDP
   con un único pago `Pa0fe6d96a6e740-p5-real-reintento`.
7. Resto del saldo (3,40, clave `p5-real-resto`) → BDP con dos pagos, y el saldo
   queda `pagado 4,40 / pendiente 0,0 / origen: "bdp"`.

Ninguna operación se duplicó y nada quedó pendiente tras el rearranque.
