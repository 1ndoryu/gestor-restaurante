BORRADOR — respuesta al correo de BDP del 27/11/2026. Sin enviar; lo envía la usuaria.

**De:** Wan <andoryyu@gmail.com>
**Para:** `integraciones@bdpcenter.com`
**CC:** `guillermo.autoia@gmail.com`
**Estado:** borrador. **No enviado.**

Va como respuesta al hilo del 26/11. Solo contesta lo que piden: endpoint y JSON de una
comanda de barra. El resto de su correo (pagos, devoluciones, domicilio) se agradece y no
genera más preguntas.

---

Hola:

Gracias por la respuesta. Con eso quitamos el límite de tres pagos en nuestra aplicación y
dejamos las devoluciones en manos de caja, como decís. Sobre el domicilio, lo traslado al
restaurante.

De la modificación de barra, esto es exactamente lo que enviamos.

*Endpoint.* `POST /API/Orders/Create`. Hemos comprobado que no hay ruta de modificación en
el WebLinkRestAPI: `Orders/Get`, `Orders/Cancel` y `Orders/Create` son las tres que
publica el manual, y para modificar usamos `Create` con el `OrderId` de la comanda viva.
Si hay otra ruta para modificar, decídnoslo, porque es lo que nos falta.

*Ejemplo real.* Comanda de barra 228 (`Type = 0`), creada por nosotros desde la web. El
cuerpo de la modificación es este, literal, sin recortar:

```json
{
  "EmployeeId": 1,
  "ItemsProfileId": 1,
  "OrderEndType": 0,
  "OrderOperationType": 0,
  "Invoice": false,
  "Order": {
    "MarketplaceOrderId": "Ga47278fbe0894d",
    "MarketId": 9900,
    "MarketName": "Glory",
    "PreparationTime": "2026-11-26T06:49:29Z",
    "OrderId": 228,
    "PosId": 1,
    "Type": 0,
    "RoomNumber": 0,
    "TableNumber": 0,
    "Items": [
      {
        "Lin": 1,
        "Id": 1001,
        "Name": "COCA-COLA",
        "Units": 1.0,
        "Price": 1.05,
        "Supplement": 0.0,
        "Discount": 0.0,
        "DiscountPct": false,
        "Total": 1.05,
        "Invitation": false,
        "VatPct": 10.0,
        "Comments": [],
        "Supplements": [],
        "OrderItemType": 0,
        "OrderItemTypeMetaInfo": "",
        "TyC_D1": 0,
        "TyC_D2": 0,
        "TyC_D3": 0,
        "OnSale": false
      }
    ],
    "Discount": 0.0,
    "DiscountPct": false,
    "Total": 1.05,
    "ExecutionTime": "2026-11-26T06:49:29Z",
    "Status": 0,
    "AlreadyInvoiced": false,
    "Comments": "Glory venta a47278fb-e089-4d5e-a3a6-7256f94b3850"
  }
}
```

`OrderId` es el de la comanda que ya existe; el `MarketplaceOrderId` es el mismo que se
envió al crearla.

*Lo que devuelve.* HTTP 200, con el mismo `OrderId`, `ErrorMessage: ""`, y al releer con
`Orders/Get` la comanda está igual: mismo `Total` y mismas líneas. Lo hemos medido en las
comandas 222, 223, 225 y 227. No es un rechazo: contesta bien y no aplica.

*Una aclaración sobre el ejemplo.* El JSON de arriba es el de la comanda 228 porque es la
que tenemos documentada entera, pero en 228 el resultado fue otro: `[300033]`, «el importe
total indicado de la comanda no coincide con el teórico». Ese error lo entiendo como
nuestro y no lo discuto. Las cuatro en las que el resultado fue 200 sin aplicar son 222,
223, 225 y 227.

*Añadido para que lo podáis revisar rápido:* en la de mesa (`Type = 1`, con salón y mesa)
este mismo cuerpo sí se aplica; lo único que cambia es `Type`, `RoomNumber` y
`TableNumber`. Y en el manual, `OrderId` dentro de `Create` aparece descrito como salida
(«Número de comanda generada»), con el ejemplo a `0`, y `OrderOperationType` solo tiene
`0` (CheckAndCreateOrder) y `1` (OnlyCheck). Si el camino correcto para modificar una
comanda existente no es `Create` con `OrderId`, ¿cuál es?

Gracias.

Un saludo,
Asley Crespo Navarro
Autorizada por el distribuidor Guillermo García Sáez (n.º 4652)
