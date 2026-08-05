# Contrato seguro local — BDP WebLink

> Fecha: 2026-07-18
> Evidencia: documentación WebLink incluida en el repositorio, tipos Rust actuales y simulador local.
> Límite: **ninguna llamada fue contrastada hoy contra el BDP del restaurante**.

## Clasificación

- **Verificado localmente:** reproducido por tests del simulador.
- **Inferido del contrato:** aparece en el manual o cliente, pero no fue observado en BDP real.
- **Desconocido crítico:** debe bloquear una prueba real hasta aclararse por lectura o preflight autorizado.

## Matriz de endpoints usados

| Endpoint | Clase | Request mínimo usado | Respuesta consumida | Efecto | Estado |
|---|---|---|---|---|---|
| `/Service/Health` | lectura | `{}` | `IsAlive` | ninguno esperado | Inferido |
| `/Auth/Login` | sesión | `Login`, `Password`, `TiempoSession`, `CodigoIntegrador` | `AuthSession.Token`, expiración | crea sesión | Verificado localmente |
| `/Service/GetVersion` | lectura | `{}` + bearer | versión/aplicación | ninguno esperado | Inferido |
| `/API/Articles/Export` | lectura | rangos, tarifa, descuento | `Articles`, `ErrorMessage` | ninguno esperado en BDP | Verificado localmente |
| `/API/Customers/Export` | lectura | rango de clientes | `Customers`, `ErrorMessage` | ninguno esperado en BDP | Verificado localmente |
| `/API/Departments/Export` | lectura | rango/paginación | `Departments`, `ErrorMessage` | ninguno esperado en BDP | Verificado localmente |
| `/API/Rooms/GetTables` | lectura | `Ids` opcional | `Rooms`, `ErrorMessage` | ninguno esperado en BDP | Verificado localmente |
| `/API/Employees/Get` | lectura | `Ids`, filtro vendedor | `Employees`, `ErrorMessage` | ninguno esperado en BDP | Verificado localmente |
| `/API/Tenders/GetList` | lectura | `{}` | `Tenders`, `ErrorMessage` | ninguno esperado en BDP | Verificado localmente |
| `/API/POSes/Get` | lectura | `{}` | `POSes`, `ErrorMessage` | ninguno esperado en BDP | Verificado localmente |
| `/API/Customers/Create` | escritura | código, nombres, contacto, `Overwrite` | cliente/error | alta o reemplazo | Verificado solo en simulador |
| `/API/Orders/Create` con `OrderOperationType=1` | validación | empleado, perfil, tipo y `Order` | validación/error | el manual sugiere no persistente | **Desconocido crítico en BDP real** |
| `/API/Orders/Create` con `OrderOperationType=0` | escritura | configuración y `Order` completo | `OrderId`, error | crea comanda | Verificado solo en simulador |
| `/API/Orders/Get` | lectura | `OrderIdentifier` | `Order`, estado, pagos | ninguno esperado | Verificado localmente |
| `/API/Orders/Payment/Add` | escritura | orden, `TenderId`, `Amount`, `PaymentId` | orden/factura/error | registra pago; puede ser irreversible | Verificado solo en simulador |
| `/API/Orders/Invoice` | escritura | POS, empleado, orden | `InvoiceNumber` | factura; potencialmente irreversible | Verificado solo en simulador |

## Invariantes implementadas localmente

1. Toda escritura externa exige que la URL base coincida exactamente con `BDP_WRITE_ALLOWED_ORIGINS`; loopback se permite para el simulador.
2. `read_only`, auto-backup y auditoría son fail-closed.
3. `MarketplaceOrderId` es determinista, estable y de 15 caracteres.
4. `CreateOrder` se llama una sola vez. Un fallo de transporte obliga a `GetOrder`; si no aparece un `OrderId`, queda `ambiguo` y no se reintenta.
5. Dos instancias no pueden procesar simultáneamente la misma venta gracias a advisory lock PostgreSQL.
6. Pago y factura tienen endpoints Glory separados.
7. Antes de pagar o facturar, Glory exige `GetOrder`, estado compatible y saldo verificable.
8. `PaymentId` es estable por venta; el flujo actual admite una única intención de pago por venta.
9. La factura local solo se marca con `InvoiceNumber` no vacío.
10. El simulador rechaza bind no-loopback, redacta secretos y permite reproducir una operación aplicada cuya respuesta se perdió.

**Limitación de la API gratuita de WebLink (verificada en BDP real 2026-08-05):**
- `GET/POST /API/Orders/Get` devuelve `[301010]` en la API gratuita: **solo expone `Status`** (0=abierta, 2=cobrada, 3=anulada); no devuelve `Order`, `Total`, `Items` ni `Payments`. Suficiente para verificar estado de la comanda, insuficiente para reconciliar saldos.
- `POST /API/Orders/Payment/Add` responde HTTP 200 con `ErrorMessage: "Subscripción no activada"` → **los pagos requieren la WebLink RESTAPI de pago** (suscripción activa).
- Consecuencia en Glory: `add_order_payment` exige reconciliar (`Order.Total` + `Order.Payments`) antes de escribir → con la API gratuita falla siempre con 422. No es un bug de Glory; es el plan contratado de WebLink.
11. El modo write requiere un armado persistente con URL exacta, alcances, motivo, expiración máxima de 15 minutos y cupo atómico de operaciones.
12. Una auditoría `pendiente` o `ambiguo` bloquea nuevas escrituras para la misma entidad hasta reconciliación.
13. El snapshot habilitante guarda URL y huella de credenciales/configuración; evidencia legacy o de otra conexión nunca autoriza.
14. Consumir el permiso, registrar la intención y volver a `read_only` ocurre en una sola transacción antes del HTTP.
15. Pago y factura exigen un snapshot remoto previo; cualquier fallo bloquea y conserva la autorización sin enviar.
16. Cliente, pago y factura exigen confirmaciones exactas verificadas por backend antes de contactar BDP.
17. `OnlyCheck` usa una allowlist separada y la interfaz solo lo habilita contra loopback.
18. El PATCH general no puede modificar `bdp_sync_mode`; el endpoint dedicado solo admite `read_only` y `unidirectional`.

## Diferencias y supuestos aún abiertos

| Punto | Situación | Consecuencia segura |
|---|---|---|
| Header bearer | El manual no lo confirma inequívocamente | validar en preflight de lectura |
| Forma exacta de arrays (`Tenders`/`TenderList`, etc.) | hay variantes en código/manual | aceptar diferencias solo después de capturar respuesta real redactada |
| `OrderOperationType=1` | no se demostró que sea no persistente | no usarlo en real hasta confirmación de solo lectura |
| Dedupe real por `MarketplaceOrderId` | el simulador la implementa; BDP no está demostrado | reconciliar siempre; nunca confiar solo en dedupe |
| Dedupe real por `PaymentId` | no demostrado | ningún retry automático de pago |
| Estado/saldo devuelto por `GetOrder` | documentado, no observado | si falta cualquier campo, bloquear pago/factura |
| Serie fiscal, POS, empleado, perfil y tender | dependen de la instalación | obtenerlos por lecturas autorizadas y selección humana |
| IVA, redondeo y precisión | pueden variar por configuración | comparar total línea/orden y tolerancia antes de escribir |
| Reversión de pago/factura | no existe rollback implementado | aceptación expresa y remediación manual antes de prueba real |
| Alta de cliente y asignación de código | BDP no ofrece una reserva transaccional demostrada | usar código exacto reservado por una persona; `Overwrite=false`; nunca `max + 1` ni hash |

## Resultado de pruebas locales

El 2026-07-18 se aprobaron 157 pruebas locales: 84 unitarias Rust, 65 de integración SQLx en PostgreSQL temporal y 8 del simulador WebLink en loopback. También pasaron `cargo check`, Clippy con warnings denegados y la compilación TypeScript/Vite. El simulador cubre autenticación obligatoria, check no persistente simulado, idempotencia y conflicto de orden, respuesta perdida con reconciliación, pago idempotente, saldo antes de factura, redacción y fallos HTTP/funcionales/JSON.

La inspección visual automatizada no pudo ejecutarse porque el runtime del navegador integrado fue bloqueado por el sandbox de Windows. Esto no afectó la compilación ni las pruebas de contratos; la comprobación visual queda incluida en la guía del cliente.

La base configurada conserva un esquema antiguo de `notificaciones`; por ello las compilaciones de macros se realizan con el caché SQLx offline y las suites SQLx BDP usan bases temporales creadas por migraciones. Este problema local ajeno no autoriza modificar una base real.

## Decisión

Las fases de contrato, simulación, robustez y revisión estática quedan cubiertas localmente. Nuestro equipo no ejecutará fases reales contra el restaurante. La aceptación del cliente se limita a las comprobaciones sin escritura de `Agente/usuario/guia-cliente-pruebas-integracion-bdp-2026-07-18.md`; las escrituras quedan explícitamente como no verificadas en el BDP real.
