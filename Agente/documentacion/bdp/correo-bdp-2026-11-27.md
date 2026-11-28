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

De la modificación de barra, esto es lo que nuestra aplicación envía.

*Endpoint.* `POST /API/Orders/Create`. En el manual no encontramos ninguna ruta pensada
para modificar una comanda existente, así que para modificar usamos `Create` con el
`OrderId` de la comanda viva. Si hay otra ruta para modificarla, decidnos cuál, porque es
lo que nos falta.

*Ejemplo real.* Comanda de barra (`Type = 0`), creada por nosotros desde la web y
modificada después. Este es el cuerpo que nuestra aplicación genera al modificarla, campo
por campo:

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
    "Total": 2.1,
    "ExecutionTime": "2026-11-26T06:49:29Z",
    "Status": 0,
    "AlreadyInvoiced": false,
    "Comments": "Glory venta a47278fb-e089-4d5e-a3a6-7256f94b3850"
  }
}
```

Dos cosas que conviene saber al leerlo, porque son la parte del contrato que estamos
usando y quizá sea donde nos equivocamos:

- `Items` **no** lleva la comanda entera, sino solo las líneas del cambio. Aquí es una
  unidad más del mismo artículo; quitar producto va como línea con `Units` negativa.
- `Order.Total` es el total **final** de la comanda después del cambio (2,10), no la suma
  de `Items` (1,05).

`OrderId` es el de la comanda que ya existe; el `MarketplaceOrderId` es el mismo que se
envió al crearla. Los números de comanda son de nuestro TPV de pruebas, así que no os
dirán nada: sirven solo para fijar la forma del JSON.

*Lo que devuelve.* En comandas de barra hemos visto las dos cosas. Unas veces, HTTP 200 con
el mismo `OrderId` y `ErrorMessage: ""`, y al releer con `Orders/Get` la comanda igual:
mismo `Total` y mismas líneas. Otras veces, `[300033]`; es lo que dio esta comanda con el
cuerpo de arriba, y ese error lo entiendo como nuestro y no lo discuto.

*Dónde está nuestra duda.* No sabemos si ese 200 significa que el TPV acepta la
modificación y no la aplica, o que algo de nuestra petición no le vale y no llega a
procesarla. Es justo lo que os pedimos que nos aclaréis.

*Para que lo podáis revisar rápido:* en la de mesa (`Type = 1`, con salón y mesa) este
mismo cuerpo sí se aplica, y lo único que cambia es `Type`, `RoomNumber` y `TableNumber`.
Y en el manual, `Order.OrderId` viene como campo de entrada («si no se especifica ninguno,
o es cero, BDP asignará un número»), pero no encontramos ninguna frase que diga que
enviarlo con el identificador de una comanda viva la modifique, ni una ruta aparte para
modificarla; `OrderOperationType` solo tiene `0` (CheckAndCreateOrder) y `1` (OnlyCheck).
Si el camino correcto para modificar una comanda existente no es `Create` con `OrderId`,
¿cuál es?

Gracias.

Un saludo,
Asley Crespo Navarro
Autorizada por el distribuidor Guillermo García Sáez (n.º 4652)
