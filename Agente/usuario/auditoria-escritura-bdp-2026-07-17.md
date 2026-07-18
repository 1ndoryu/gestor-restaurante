# Reauditoría de riesgos — Sección 3 ESCRITURA BDP

> Reauditoría y correcciones: 2026-07-18
> Referencia: `Agente/usuario/checklist-bdp-integracion-2026-07-16.md`
> Plan: `Agente/planes/plan-validacion-segura-escritura-bdp-2026-07-18.md`
> Alcance ejecutado: código, migraciones, PostgreSQL local, frontend y simulador loopback.
> No ejecutado: conexión, preflight o escritura contra el BDP del restaurante.

## Veredicto

La preparación local queda endurecida para impedir escrituras accidentales, pero nuestro equipo **no realizará pruebas contra el BDP del restaurante**. El cliente dispone de una guía limitada a lecturas y cambios locales. `CreateOrder`, `CreateCustomer`, `AddOrderPayment` e `InvoiceOrder` alteran BDP y permanecen fuera de la aceptación vigente.

No hace falta copiar el programa BDP ni realizar ingeniería inversa de licencia. El simulador local reproduce únicamente el contrato HTTP/JSON observado por Glory y no contiene binarios, código ni mecanismos de activación de BDP.

La protección alcanzada es defensa en profundidad:

1. destino externo denegado por defecto;
2. modo de escritura exacto y temporal;
3. snapshot completo reciente como evidencia, no como rollback;
4. alcance, tiempo y cupo de operaciones;
5. preflight y auditoría antes de consumir la autorización;
6. una sola escritura y reconciliación ante respuesta ambigua;
7. bloqueo de nuevas escrituras mientras exista ambigüedad;
8. confirmación local solo después de persistencia coherente.

## Evidencia local

| Verificación | Resultado |
| --- | --- |
| Pruebas unitarias Rust | 84 aprobadas, 0 fallos |
| Integración SQLx en PostgreSQL `localhost` | 65 aprobadas, 0 fallos |
| Simulador WebLink en loopback | 8 aprobadas, 0 fallos |
| Compilación frontend TypeScript/Vite | aprobada |
| Compilación backend SQLx offline | aprobada |
| Clippy estricto (`-D warnings`) | aprobado |
| Pruebas `bdp_readonly.rs` contra BDP real | no ejecutadas |
| Escrituras contra BDP real | ninguna |

Las suites SQLx aplicaron las migraciones desde cero en bases temporales locales. El archivo que puede usar `BDP_BASE_URL` real fue excluido expresamente.

## Hallazgos corregidos

| ID | Riesgo previo | Corrección verificada localmente | Estado |
| --- | --- | --- | --- |
| W01 | Un destino externo podía recibir escrituras por configuración | `BDP_WRITE_ALLOWED_ORIGINS` es allowlist exacta; loopback es la única excepción automática | Cerrado localmente |
| W02 | Habilitación genérica y persistente | Solo `unidirectional`; armado 1–15 min, un scope, una operación, destino, motivo y UUID objetivo | Cerrado localmente |
| W03 | `bidirectional` prometía una capacidad inexistente | Bloqueado en API/UI y también por los gates de escritura legados | Cerrado localmente |
| W04 | Retry ciego de `CreateOrder` | Una sola llamada; ante transporte ambiguo consulta por `MarketplaceOrderId` estable | Cerrado localmente |
| W05 | Concurrencia entre procesos | Mutex local y advisory lock transaccional por venta | Cerrado localmente |
| W06 | Éxito remoto con fallo de persistencia local podía parecer éxito | Audit pasa a `ambiguo` y bloquea nuevas escrituras | Cerrado localmente |
| W07 | Venta y líneas podían quedar inconsistentes | Alta/edición transaccional; edición carga y reemplaza líneas | Cerrado localmente |
| W08 | Editar una venta sincronizada podía crear otra comanda | Edición bloqueada para ventas `bdp_synced` hasta existir update BDP confirmado | Cerrado localmente |
| W09 | Cliente automático `max+1`/hash y `Overwrite=true` | Código explícito, preflight de identidad, índice único y siempre `Overwrite=false` | Cerrado localmente |
| W10 | Importación de clientes aplicaba cambios directamente | Preview obligatorio, frase exacta, conflictos explícitos y alta/vínculo atómico local | Cerrado localmente |
| W11 | Pago parcial reutilizaba un único PaymentId | Solo pago completo; valida estado, saldo, tender e importe contra `GetOrder` | Cerrado por restricción deliberada |
| W12 | Factura podía duplicarse o quedar incoherente | Relee orden/saldo; reconcilia factura existente; exige `InvoiceNumber`; persiste antes de cerrar audit | Cerrado localmente |
| W13 | Catálogo interpretaba mal PascalCase/códigos y precios | Parser tipado número/string, código BDP correcto y upsert de todos los campos | Cerrado localmente |
| W14 | Mesas se creaban directamente y superpuestas | Preview por defecto, confirmación exacta, solo aditivo y cuadrícula local | Cerrado localmente |
| W15 | Polling no programado o infinito para canceladas | Scheduler multiinstancia opt-in; excluye estados terminales; límite de lote | Cerrado localmente |
| W16 | Errores BDP invisibles en ventas | Badge y retry usan `bdp_sync_error` aunque `bdp_synced=false` | Cerrado localmente |
| W17 | Un snapshot del mismo usuario podía autorizar otra conexión | Snapshot y armado guardan URL exacta y huella de credenciales/POS/empleado/perfil; legacy queda inelegible | Cerrado localmente |
| W18 | Pago/factura podían continuar si fallaba el snapshot selectivo | La captura `GetOrder` es obligatoria y fail-closed; no consume permiso ni crea intención al fallar | Cerrado localmente |
| W19 | Auditoría pendiente antes de consumir un armado inválido | Consumo, intención y kill switch se confirman en una sola transacción | Cerrado localmente |
| W20 | La allowlist se comprobaba después de gastar el cupo | Se valida antes de la transacción y nuevamente dentro del cliente HTTP | Cerrado localmente |
| W21 | El modo podía permanecer en escritura tras consumir el cupo | La misma transacción vuelve a `read_only` y elimina el armado antes del HTTP | Cerrado localmente |
| W22 | Confirmaciones críticas existían solo en frontend | Cliente, pago y factura validan frase, UUID e importe/código también en backend | Cerrado localmente |
| W23 | `OnlyCheck` se presentaba como prueba inocua | Allowlist separada deny-by-default y UI habilitada únicamente para simulador loopback | Cerrado localmente |

## Auditoría por operación

### `CreateOrder`

Antes de escribir exige: integración habilitada, modo exactamente `unidirectional`, backup automático, ausencia de audit pendiente/ambiguo, destino permitido y armado `create_order` vigente. La venta y sus líneas ya fueron persistidas atómicamente. Las ventas sincronizadas no se editan ni reenvían.

El ID de marketplace es determinista. Se envía una vez. Un timeout/HTTP/JSON ambiguo provoca una consulta de reconciliación; si no aparece un `OrderId` inequívoco, queda `ambiguo` y el sistema impide reintentar.

Riesgo no eliminable: la primera llamada real crea una comanda verdadera y el comportamiento exacto de deduplicación debe confirmarse contra la versión BDP instalada.

### `CreateCustomer`

No existe creación automática durante una venta. El operador debe introducir un código positivo y confirmar `VINCULAR <código>`. Antes de escribir se exportan clientes y se comprueba la identidad. Si el código ya corresponde al mismo teléfono/email, solo se vincula localmente; si corresponde a otro cliente, se bloquea. Una alta nueva siempre usa `Overwrite=false`.

Riesgo no eliminable: crear el cliente real no tiene rollback WebLink. En una futura prueba se usarán datos no personales y un código reservado por el responsable.

### `AddOrderPayment`

Pago y factura son endpoints separados. El pago relee orden, estado, total y pagos. Solo acepta exactamente el saldo completo; los pagos parciales están deshabilitados hasta disponer de un ledger independiente de intenciones. Usa un `PaymentId` estable por venta, audit, armado `add_payment` y bloqueo ante ambigüedad.

Riesgo no eliminable: un pago confirmado es irreversible por la API disponible. Debe ser una autorización propia, posterior a reconciliar la orden.

### `InvoiceOrder`

Relee la orden y exige saldo cero y estado no cancelado. Si ya está facturada, reconcilia el `InvoiceNumber` sin otra escritura. Una respuesta nueva solo se acepta con número no vacío; un fallo de persistencia local queda `ambiguo`.

Riesgo no eliminable: la factura real es fiscal/operativamente irreversible por WebLink y será la última prueba, con autorización independiente.

## Lecturas que modifican solo Glory

- Catálogo y precios: leen BDP y hacen upsert local.
- Clientes: preview no cambia nada; aplicar solo cambia Glory.
- Mesas: preview no cambia nada; aplicar crea únicamente faltantes en Glory.
- Polling: opt-in y actualiza estados locales.

Estas operaciones no corrompen BDP, pero no deben ejecutarse todavía porque el usuario no autorizó contacto real ni cambios locales derivados de respuestas reales.

## Riesgos residuales aceptables antes del preflight real

- El contrato exacto de la versión BDP del restaurante solo puede confirmarse con lecturas autorizadas.
- No existe rollback BDP; los snapshots son evidencia y apoyo de conciliación.
- La importación masiva de decenas de miles de clientes es reanudable e idempotente, pero no se usará como primera prueba por su carga y duración.
- Menús, fastfoods y packs permanecen como lectura API informativa; no se presentan como funciones completas de producto.
- Los pagos parciales permanecen fuera de alcance, de forma explícita y segura.
- El warning de tamaño de bundle frontend no afecta seguridad BDP.

Ninguno de estos puntos justifica escribir. Nuestro equipo no pasará a un preflight real; las comprobaciones no destructivas que el cliente decida realizar están descritas en `guia-cliente-pruebas-integracion-bdp-2026-07-18.md` y excluyen `OnlyCheck`.

## Acciones reales excluidas

- [x] Nuestro equipo no contactará BDP ni cargará sus credenciales en pruebas.
- [x] `OnlyCheck` externo permanece bloqueado.
- [x] El cliente no debe activar escritura temporal durante la guía de aceptación.
- [x] Reintento de comanda, alta BDP de cliente, pago y factura quedan sin ejecutar.
- [x] Los resultados se rotulan como verificados localmente o no verificados en BDP real.

## Decisión vigente

**Código local endurecido; nuestro equipo no realizará pruebas reales.** El cliente validará únicamente el alcance no destructivo de la guía. Toda escritura queda fuera de esta etapa y no se considera demostrada en el BDP real.
