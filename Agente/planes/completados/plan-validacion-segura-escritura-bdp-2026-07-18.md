# Plan de validación segura — Escritura BDP

> **Fecha:** 2026-07-18
> **Objetivo:** alcanzar la máxima confianza posible localmente sin que nuestro equipo contacte ni pruebe el BDP real.
> **Estado al 2026-07-18:** completado localmente. Las comprobaciones reales quedan transferidas al cliente mediante una guía no técnica y excluyen toda escritura.
> **Prohibición vigente:** nuestro equipo no ejecutará llamadas, cambios de modo, snapshots ni escrituras contra el BDP del restaurante.

## 0. Avance ejecutado

- [x] Inventario contractual de endpoints usados: `Agente/documentacion/api/bdp-contrato-seguro-local-2026-07-18.md`.
- [x] Simulador local aislado: `tools/bdp-weblink-simulator/`.
- [x] Fixtures falsos, reset, historial redactado y fallos programables.
- [x] 8 tests del simulador pasaron en loopback.
- [x] Allowlist externa deny-by-default para escrituras.
- [x] Eliminado retry ciego de `CreateOrder`; reconciliación o estado ambiguo.
- [x] Lock distribuido por venta.
- [x] Pago y factura separados, con preflight de estado/saldo.
- [x] Checklist operativo futuro: `Agente/usuario/checklist-operativo-prueba-real-bdp-2026-07-18.md`.
- [x] 84 tests unitarios Rust, 65 tests SQLx locales y 8 tests del simulador pasan; frontend compila con TypeScript/Vite.
- [x] Armado temporal persistente con destino, alcance, expiración y máximo de operaciones.
- [x] Estado persistente equivalente a outbox mediante auditoría `pendiente/exito/error/ambiguo`; bloquea nuevas escrituras sobre la entidad si quedó pendiente o ambigua.
- [x] Eliminada asignación de cliente `max + 1`/hash; código explícito, preflight e índice único.
- [x] Ventas y líneas transaccionales; edición de ventas ya sincronizadas bloqueada.
- [x] Polling periódico multiinstancia, opt-in y con estados terminales.
- [x] UI segura para clientes, pagos, facturas y preview de mesas.
- [x] Pagos parciales bloqueados; solo se permite una intención estable por el saldo completo.
- [x] Modo `bidirectional` bloqueado hasta disponer de un contrato real.
- [x] Snapshot y armado ligados a URL y huella de conexión exactas; evidencia legacy no autoriza.
- [x] Preparación de pago/factura fail-closed y autorización/auditoría atómicas.
- [x] Retorno transaccional a `read_only` antes del HTTP de escritura.
- [x] Confirmaciones críticas verificadas también por backend.
- [x] `OnlyCheck` externo bloqueado por defecto y botón limitado al simulador loopback.
- [x] Guía del cliente: `Agente/usuario/guia-cliente-pruebas-integracion-bdp-2026-07-18.md`.
- [x] Validación final del bloque endurecido: formato, check, Clippy estricto, pruebas Rust/SQLx, simulador y build frontend aprobados.

## 1. Principios obligatorios

1. No copiar, modificar ni emular ejecutables, licencias o mecanismos de activación de BDP.
2. El simulador local reproducirá solamente el contrato HTTP/JSON necesario para interoperabilidad.
3. Producción permanecerá en `read_only` durante diseño, implementación y validación local.
4. “Pasa localmente” no significa “confirmado en BDP real”. Cada conclusión se marcará como:
   - **Verificado localmente:** demostrado contra el simulador y tests propios.
   - **Inferido del contrato:** coincide con documentación/código, pero no se observó en BDP.
   - **Verificado en BDP:** únicamente después de una prueba real autorizada.
5. Un snapshot BDP sirve para comparación y auditoría; no es rollback.
6. Ninguna autorización general habilitará todas las escrituras. Cliente, orden, pago y factura requieren autorizaciones separadas.
7. Ante respuesta ambigua, timeout o desconexión, se reconcilia antes de reintentar.

## 2. Alcance del simulador WebLink

Se creará un servicio local, desechable y sin código propietario, con estado controlado para:

- autenticación y expiración de token;
- `ExportArticles`, `ExportCustomers`, `ExportDepartments`;
- `GetRoomsTables`, `GetEmployees`, `GetTenderList`, `GetPOSes`;
- `CreateCustomer` con colisiones y `Overwrite`;
- `CreateOrder`/`CheckOrder` e idempotencia por `MarketplaceOrderId`;
- `GetOrder` y transiciones de estado;
- `AddOrderPayment` con saldo pendiente;
- `InvoiceOrder` con número de factura;
- errores funcionales en `ErrorMessage`;
- errores HTTP, JSON inválido, latencia, timeout y pérdida de respuesta.

El simulador deberá poder reiniciarse a un fixture conocido y exponer un historial de llamadas que permita demostrar exactamente qué recibió.

## 3. Matriz mínima de escenarios locales

Cada endpoint se evaluará con:

- caso válido;
- datos obligatorios ausentes;
- identificadores inexistentes;
- códigos duplicados;
- credenciales/token inválidos o expirados;
- HTTP 4xx/5xx;
- `ErrorMessage` con HTTP 200;
- respuesta JSON incompleta o corrupta;
- timeout antes de aplicar la operación;
- operación aplicada y respuesta perdida;
- repetición idéntica;
- repetición con payload diferente;
- dos solicitudes concurrentes;
- fallo local después del éxito remoto;
- datos y mensajes con secretos o información personal, verificando redacción.

## 4. Refuerzos por punto crítico

### 4.1 Configuración y armado de escritura

- Mantener `read_only` como default y kill switch.
- Requerir auto-backup activo y audit disponible.
- Requerir snapshot completo de menos de 24 horas, vigente y sin secciones nulas.
- Mostrar claramente host, empresa, POS y modo antes de confirmar.
- Añadir una allowlist explícita de destinos BDP autorizados.
- Usar un armado temporal: un alcance, una entidad UUID, usuario, motivo, expiración y exactamente una operación.
- Desarmar automáticamente tras la operación o al vencer el tiempo.
- Bloquear cualquier escritura que no coincida con el alcance autorizado.

### 4.2 `CreateOrder`

- `MarketplaceOrderId` determinista y estable por venta.
- Lock distribuido en PostgreSQL, no solo mutex del proceso.
- Estado local tipo outbox: `preparada → enviando → confirmada/ambigua/error`.
- En estado `ambigua`, prohibir retry hasta consultar/reconciliar.
- Validar artículos, cantidades, descuentos, IVA, total, tender, canal, POS, empleado y serie.
- Limitar la primera prueba real a una venta exacta autorizada.

### 4.3 `CreateCustomer`

- Consultar y confirmar que el código no pertenece a otro cliente.
- Exigir un código reservado explícito; nunca calcularlo con `max + 1` ni hash.
- Mantener `Overwrite=true` fuera de alcance; el flujo implementado siempre usa `false`.
- No enviar datos personales reales en fixtures.
- Reconciliar si BDP pudo crear el cliente pero Glory perdió la respuesta.

### 4.4 Pagos

- Separar pago de facturación en contratos/handlers distintos.
- Validar importe positivo, precisión decimal, tender permitido y saldo pendiente.
- Usar un identificador de pago idempotente y persistente.
- Registrar estado `preparado/enviado/confirmado/ambiguo/error`.
- No repetir pagos ambiguos hasta reconciliar la orden.
- Bloquear pagos sobre órdenes canceladas o ya facturadas.

### 4.5 Facturación

- No combinarla implícitamente con una prueba de pago.
- Exigir autorización propia para una orden exacta.
- Verificar estado, saldo, POS, empleado y serie antes de enviar.
- Solo marcar éxito local con `InvoiceNumber` válido.
- Ante timeout/respuesta perdida, consultar primero; nunca facturar de nuevo a ciegas.

### 4.6 Auditoría y datos sensibles

- Fallo cerrado: sin audit no hay escritura.
- Cerrar cada registro como `exito`, `error` o `ambiguo`.
- Correlacionar autorización, payload, respuesta, entidad local e identificador remoto.
- Redactar password, token, integrator code y datos personales innecesarios.
- Hacer el historial append-only para operaciones críticas.

## 5. Fases de trabajo y criterios de salida

### Fase A — Inventario contractual

Entregables:

- matriz endpoint → request → response → errores → efectos;
- diferencias entre documentación, código y suposiciones;
- lista de campos fiscales/operativos que no se pueden inferir.

Criterio de salida: ninguna escritura depende de un campo cuyo significado sea desconocido.

### Fase B — Simulador local

Entregables:

- servidor WebLink simulado;
- fixtures sin datos reales;
- reset determinista;
- registro inspeccionable de llamadas;
- escenarios de fallo configurables.

Criterio de salida: todos los endpoints usados por Glory pueden reproducirse sin BDP.

### Fase C — Pruebas locales y robustez — completada

Entregables:

- tests de contrato y estado;
- tests de duplicación, concurrencia y respuesta perdida;
- tests de guards, armado temporal y audit fail-closed;
- evidencia de que ninguna prueba local usa la URL real.

Criterio de salida: cero escrituras duplicadas en escenarios controlables y ningún retry ciego en estado ambiguo.

### Fase D — Revisión estática final — completada

Entregables:

- auditoría línea por línea de cada camino write;
- revisión de rutas indirectas y tareas automáticas;
- matriz de riesgos residuales;
- checklist operativo y procedimiento de aborto.

Criterio de salida: ningún riesgo crítico abierto que pueda corregirse localmente.

### Fase E — Comprobaciones reales de solo lectura — responsabilidad del cliente

Nuestro equipo no ejecutará esta fase. El cliente puede seguir únicamente las lecturas y cambios locales descritos en la guía no técnica. `OnlyCheck` queda excluido porque comparte el endpoint de creación.

Criterio de salida: respuestas reales compatibles con el contrato local o diferencias documentadas y corregidas.

### Fase F — Escrituras reales — fuera del alcance vigente

No forma parte de la aceptación actual. Cliente, comanda, pago y factura permanecerán como **no verificados en el BDP real**. Si el restaurante decidiera probarlos por su cuenta en el futuro, necesitaría un procedimiento separado que contemple:

- endpoint y entidad exactos;
- payload revisado antes del envío;
- responsable del restaurante informado;
- una sola llamada;
- observación/reconciliación inmediata;
- criterio de aborto;
- remediación manual definida;
- retorno inmediato a `read_only`.

No se incluye un orden de ejecución para evitar que este documento se interprete como autorización.

## 6. Puerta de autorización

Antes de cualquier comprobación que el cliente decida realizar en su instalación se debe conocer:

- qué comando o llamada se ejecutará;
- destino exacto;
- si escribe en Glory, BDP o ambos;
- datos exactos que enviará;
- efecto esperado;
- riesgo residual;
- posibilidad real de rollback;
- cómo se verificará el resultado;
- cómo se detendrá o remediará.

El presente plan **no constituye autorización para nuestro equipo ni para ejecutar escrituras reales**. La única guía operativa entregable al cliente es `Agente/usuario/guia-cliente-pruebas-integracion-bdp-2026-07-18.md`.
