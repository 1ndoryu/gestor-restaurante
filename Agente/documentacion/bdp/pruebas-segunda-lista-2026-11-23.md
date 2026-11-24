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

## 5. Reinicio real de BDP (punto 5)

1. `Stop-Process` sobre `TpvNet.exe` (PID 2672); puerto 8070 sin listener.
2. Alta de venta `feb84906-6264-46e3-8052-987a2e4ad89d` con el TPV apagado →
   `bdp_sync_error` de transporte y auditoría `create_order` abierta.
3. Arranque de `C:\Bdp-Net\NetHos\Prog\TpvNet.exe` (PID 5848).
4. El poller reautentica y crea la comanda: **182**, total 2,10, una sola línea.

Auditoría: un `create_order` en `error` (transporte) y **un solo** `create_order`
en `exito` con `{"order_id": 182}`. Una sola comanda para esa venta.

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
