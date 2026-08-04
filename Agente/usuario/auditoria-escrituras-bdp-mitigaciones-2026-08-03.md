# Auditoría profunda de las cuatro escrituras BDP y mitigaciones

> **Fecha:** 3 de agosto de 2026
> **Rama:** `glory-rs-rest`
> **Alcance:** únicamente las cuatro operaciones que escriben datos reales en BDP.
> **Regla:** no se probó contra producción ni se modificaron los cambios locales preexistentes.

## 1. Alcance exacto

| Operación | Endpoint | Efecto remoto |
| --- | --- | --- |
| Crear cliente | `POST /api/clientes/:id/bdp-sync` | `CreateCustomer` |
| Crear comanda | `POST /api/ventas/:id/bdp-sync` | `CreateOrder` |
| Registrar pago | `POST /api/ventas/:id/bdp-payment` | `AddOrderPayment` |
| Facturar | `POST /api/ventas/:id/bdp-invoice` | `InvoiceOrder` |

No se consideran escrituras remotas los snapshots, polling, importaciones ni sincronizaciones de catálogo: pueden escribir en Glory, pero no modifican BDP.

## 2. Controles comunes verificados

Las cuatro acciones pasan por controles equivalentes:

- autenticación y consulta de la entidad con `user_id`;
- confirmación textual específica en el handler;
- allowlist del destino BDP;
- fingerprint de conexión;
- arming con alcance y entidad exactos;
- máximo de una operación y caducidad;
- snapshot pre-write donde aplica;
- auditoría con estado `pendiente`, `exito`, `error` o `ambiguo`;
- retorno a `read_only` durante la autorización;
- bloqueo de nuevas escrituras mientras exista una operación `pendiente` o `ambiguo`;
- idempotencia para pagos y para requests que aportan `idempotency_key`;
- transacción local posterior a la respuesta positiva de BDP.

Una transacción local no convierte la operación remota en una transacción distribuida. Si BDP acepta y el proceso muere antes del commit local, el resultado correcto es **ambiguo** y nunca debe reintentarse a ciegas.

## 3. Hallazgos descartados como falsos positivos

### 3.1 Lock distribuido liberado antes del HTTP — descartado

`sync_venta()` obtiene un advisory lock inicial y lo libera antes del HTTP para no mantener una conexión de PostgreSQL durante una llamada externa larga. A primera vista parece permitir dos `CreateOrder` concurrentes.

La revisión de `BdpWriteGuard::authorize()` demuestra que no es un bypass:

1. la autorización vuelve a serializar por usuario, entidad y scope;
2. consume atómicamente `remaining_operations`;
3. inserta una única intención auditada;
4. fuerza `read_only`;
5. elimina el arming;
6. la segunda instancia no puede obtener un arming válido.

Además, `MarketplaceOrderId` es estable por venta y la reconciliación consulta ese identificador. Por tanto, se clasifica como **falso positivo de duplicación**, aunque el comentario del código se mantiene documentando que el advisory inicial no sustituye al guard.

### 3.2 Polling de una comanda con auditoría ambigua — falso positivo parcial

`list_bdp_pending()` exige `bdp_synced = true`; las comandas huérfanas de `CreateOrder` se buscan por una ruta separada. Las dos consultas son mutuamente excluyentes por estado local.

Para pagos y facturas ambiguos, actualizar el estado de la orden desde BDP no ejecuta una escritura remota: es reconciliación de lectura. El guard sigue bloqueando una nueva escritura hasta que la auditoría se resuelva.

### 3.3 Carrera entre `GetOrder` y `AddOrderPayment`/`InvoiceOrder` — riesgo residual aceptable

Existe una ventana entre la lectura de estado y la escritura remota. No puede eliminarse con una transacción PostgreSQL porque BDP es externo. La mitigación real es:

- `GetOrder` obligatorio inmediatamente antes;
- estado cancelado/facturado bloqueado;
- saldo remoto verificado;
- una sola autorización;
- BDP como autoridad final;
- resultado ambiguo si la respuesta se pierde.

No se presenta como ausencia de riesgo, pero tampoco como vulnerabilidad explotable por Glory sin un segundo escritor remoto concurrente.

## 4. Hallazgos verdaderos y solución aplicada

### 4.1 Pago calculaba el saldo solo con el ledger local — corregido

**Problema:** `AddOrderPayment` consultaba `Order.Total`, pero calculaba lo pagado solo desde `bdp_pagos`. Un pago hecho directamente desde TPV/BDP podía no estar todavía en Glory.

**Mitigación:** ahora se exige `Order.Payments` y se suma con parseo estricto. El saldo pagado usado para autorizar es la unión conservadora de ambos libros:

- los pagos remotos siempre se suman;
- un pago local con `bdp_payment_id` que coincide y tiene el mismo importe que un pago remoto no se duplica;
- `PaymentId` se normaliza tanto si BDP lo devuelve como string como si lo devuelve como número;
- un mismo `PaymentId` remoto repetido con el mismo importe no se cuenta dos veces;
- un mismo `PaymentId` con importes contradictorios bloquea;
- un pago remoto sin `PaymentId` no se deduplica y se suma como cobro independiente;
- un pago local sin identidad remota verificable se suma aparte;
- un `PaymentId` local/remoto con importes distintos bloquea;
- se bloquea si la unión supera el total remoto.

Así, una discrepancia de identidad o importe bloquea de forma conservadora en lugar de arriesgar un sobrepago.

**Límite:** no existe una garantía distribuida entre la lectura y el endpoint remoto; una segunda terminal puede cambiar BDP después de `GetOrder`. La operación continúa siendo de resultado ambiguo ante pérdida de respuesta.

### 4.2 Importes remotos inválidos podían convertirse a cero — corregido

**Problema:** al reconciliar pagos de factura, un `Amount` ausente, `null` o inválido podía terminar como `Decimal::ZERO`.

**Mitigación:** se añadió `parse_remote_money()`:

- acepta números JSON y strings decimales;
- rechaza `null`, booleanos y objetos;
- rechaza strings no numéricos;
- rechaza valores negativos;
- nunca usa cero como fallback de un dato corrupto.

Se aplica a `Total` y a todos los elementos de `Payments` en pagos y facturas.

### 4.3 Líneas de comanda inválidas se enviaban tras un warning — corregido

**Problema:** una cantidad cero/negativa, precio negativo o descuento negativo solo generaba un warning y el payload seguía hacia BDP.

**Mitigación:** validación server-side justo antes de construir/enviar el pedido:

- cantidad estrictamente mayor que cero;
- precio no negativo;
- descuento no negativo;
- descuento no superior al importe bruto de la línea;
- rechazo sin realizar HTTP si falla.

El precio cero sigue permitido para cortesías o servicios gratuitos.

## 5. Revisión por operación

### Crear cliente

**Estado:** protegido con riesgo operativo medio bajo.

- Preflight de código e identidad.
- `Overwrite=false`.
- Vinculación segura si el código ya existe y la identidad coincide.
- Conflicto si pertenece a otra identidad.
- Resultado ambiguo bloquea reintentos ciegos.
- El polling puede cerrar auditorías huérfanas cuando el cliente ya quedó vinculado.

No se requiere una mitigación adicional en este tramo porque el flujo ya dispone de persistencia atómica y reconciliación de clientes en la rama actual.

### Crear comanda

**Estado:** mitigado y sujeto a validación final con simulador.

- Identificador de marketplace estable.
- Reconciliación de timeout.
- Guard de un solo uso.
- Ahora rechaza líneas financieramente imposibles antes del HTTP.

Una venta que consulta correctamente sus líneas pero no tiene ninguna conserva el fallback legacy a un artículo genérico. Si la lectura de líneas falla, la comanda se bloquea antes del HTTP para no enviar una representación incompleta de la venta.

### Registrar pago

**Estado:** mitigado; no habilitar pruebas reales hasta validar integración.

- Saldo remoto y local unidos conservadoramente por `bdp_payment_id`.
- `Payments` ausente o malformado bloquea.
- Pagos remotos externos ya no se ignoran.
- Idempotencia y ledger siguen siendo transaccionales.
- Resultado ambiguo requiere reconciliación, nunca doble clic.

### Facturar

**Estado:** mitigado; requiere validación con una orden de prueba aceptable fiscalmente.

- `Total` y `Payments.Amount` usan parseo estricto.
- Saldo local y remoto deben estar cubiertos.
- Orden ya facturada se reconcilia sin llamar otra vez a `InvoiceOrder`.
- `InvoiceNumber` vacío se considera ambiguo.
- Actualización local y auditoría se confirman en transacción.

## 6. Pruebas añadidas

Se añadieron regresiones unitarias puras para las reglas de dinero y líneas. Los
casos que dependen de PostgreSQL, del repositorio de líneas o de un servidor BDP
siguen requiriendo integración/simulador aislado; no se presentan como cubiertos
por estas pruebas unitarias.

Se añadieron regresiones unitarias para:

- rechazar cantidad cero;
- rechazar precio negativo;
- rechazar descuento negativo;
- aceptar importes remotos como número o string;
- rechazar `null`, `NaN` y negativos;
- rechazar descuentos superiores al importe bruto;
- conservar cero explícito como valor monetario válido;
- aceptar `PaymentId` remoto numérico y textual;
- deduplicar el mismo `PaymentId` con el mismo importe;
- rechazar el mismo `PaymentId` con importes distintos;
- tratar `PaymentId` vacío o nulo como ausencia de identidad, no como una identidad deduplicable;
- bloquear si un `PaymentId` local y remoto tiene importes distintos;
- sumar pagos locales sin identidad remota o con identidad desconocida de forma conservadora;
- no sumar dos veces un pago local con el mismo `PaymentId` e importe remoto.

## 7. Validación y límites del entorno

Validación prevista:

```bash
SQLX_OFFLINE=true cargo fmt --check
SQLX_OFFLINE=true cargo test --lib bdp -- --nocapture
SQLX_OFFLINE=true cargo check --tests
```

La validación también debe comprobar que una falla de lectura de líneas no alcanza
`CreateOrder`; ese caso se cubre en la integración/simulador porque requiere
inyectar el error del repositorio.

La ejecución anterior de la suite de integración fue limitada por disponibilidad de recursos del entorno Cygwin y por la necesidad de PostgreSQL/simulador aislados. No se debe interpretar como prueba contra BDP real.

El árbol ya contenía cambios ajenos a esta auditoría:

```text
M .gitignore
M frontend/package-lock.json
M glory-rs
```

No se deben incluir en el commit de estas mitigaciones.

## 8. Criterio de habilitación real

No ejecutar en el restaurante hasta que:

1. pasen formato, compilación y tests unitarios;
2. pasen los tests con PostgreSQL aislado;
3. pasen los flujos completos contra el simulador BDP;
4. se verifique saldo remoto y local en casos con pago externo;
5. se prueben respuestas `Payments` como números, strings, `null` y campos ausentes;
6. se confirme una comanda con línea inválida bloqueada sin llamada remota;
7. el responsable acepte la reversión manual de comanda, pago y factura.

**Conclusión:** los controles de autorización son sólidos. Se corrigieron los problemas de unión de pagos, parseo financiero y validación de líneas detectados durante la revisión. Persisten los límites inevitables de toda integración con un sistema remoto: no hay 2PC, y una caída después de la aceptación remota requiere reconciliación, no reintento automático.
