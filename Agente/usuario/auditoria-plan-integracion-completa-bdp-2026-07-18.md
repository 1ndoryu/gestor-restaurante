# Auditoría del plan de integración completa BDP

> Fecha: 2026-07-18
> Alcance: trazabilidad estática plan → backend → base de datos → frontend → tests.
> Seguridad: no se llamó al BDP real, no se cambió su configuración y no se ejecutó ninguna escritura externa.

## Veredicto

La primera auditoría detectó defectos P0/P1 reales. Tras corregirlos, el alcance prioritario de la integración queda **completo y validado localmente**, pero no confirmado extremo a extremo contra el BDP del restaurante. Por decisión del usuario, nuestro equipo no realizará esas pruebas; el cliente usará una guía no destructiva y las escrituras quedarán como no verificadas en real.

“Completo” aquí significa el alcance elegido del producto —configuración, catálogo, clientes, comandas multi-item, estados, pago completo, factura, mesas y respaldos—, no todos los endpoints que ofrece WebLink. Stock, compras, transferencias, tallas/colores, fidelización y modelos propios de menús/packs quedan formalmente fuera de alcance. Menús/fastfoods/packs permanecen como consultas API informativas.

Las tablas de deficiencias que aparecen más adelante conservan el estado **antes de las correcciones** como trazabilidad histórica; no describen el estado vigente.

## Resolución de los hallazgos originales

| Hallazgo original | Resolución |
| --- | --- |
| Edición podía recrear comanda | Ventas sincronizadas no se editan; no se simula un update BDP inexistente |
| Venta/líneas no transaccionales | Alta y edición atómicas; endpoint de líneas y carga real en formulario |
| Código cliente `max+1`/hash | Eliminado; código explícito, identidad verificada, índice único, `Overwrite=false` |
| Catálogo PascalCase/códigos | Parser tipado número/string y upsert completo corregidos |
| Precio consultaba código Glory | Usa `articulo_bdp_codigo` |
| Polling manual/infinito | Scheduler opt-in, claim multiinstancia, terminales y lotes limitados |
| Error/retry invisible | Badge y acción corregidos para ventas no sincronizadas con error |
| Mapeos UI incompatibles | JSON validado server-side y campos fallback visibles |
| Clientes sin interfaz segura | Estado visible, vínculo con código/frase y preview de importación |
| Pago/factura sin interfaz | Diálogos separados, frases explícitas y preflight server-side |
| Pago parcial no idempotente | Bloqueado; se admite solo saldo completo con ID estable |
| Mesas directas/superpuestas | Preview obligatorio, confirmación y cuadrícula aditiva |
| `bidirectional` engañoso | Bloqueado hasta que exista un contrato implementado |

La auditoría detallada de escritura vigente es `Agente/usuario/auditoria-escritura-bdp-2026-07-17.md`.

## Evidencia final local

- 84 pruebas unitarias Rust aprobadas.
- 65 pruebas SQLx locales aprobadas en bases temporales.
- 8 pruebas del simulador loopback aprobadas.
- `cargo check`, Clippy estricto y build TypeScript/Vite aprobados.
- Ninguna prueba real, llamada a la URL del restaurante ni escritura BDP ejecutada.

## Hallazgos críticos originales (histórico; todos corregidos localmente)

### P0 — No autorizar todavía pruebas reales de escritura

1. **Editar una venta sincronizada no actualiza una comanda de forma demostrada.** `VentaService::update()` llama otra vez a `sync_venta(..., true)`, pero `build_order()` siempre usa `OrderOperationType=0` y el cliente vuelve a invocar `CreateOrder`. `is_update` solo cambia el nombre de auditoría. La deduplicación/actualización por `MarketplaceOrderId` no está confirmada contra BDP. El frontend sí permite editar y solo muestra advertencia para Haddock, no para BDP.

2. **Las líneas de una venta no forman un agregado transaccional.** Primero se crea `ventas`; después se insertan `venta_lineas`. Si falla la inserción, se conserva la venta y el sync continúa silenciosamente con el artículo genérico. Esto puede enviar a BDP una comanda diferente de la que el usuario capturó.

3. **La edición multi-item no existe realmente.** `ActualizarVentaRequest` no acepta líneas, el backend no las actualiza, el listado de ventas no devuelve líneas y el formulario de edición inicia el editor vacío. Se pueden cambiar totales mientras el reenvío BDP reutiliza las líneas antiguas guardadas.

4. **La asignación automática de códigos de cliente no es segura.** El flujo calcula `max + 1` leyendo `Code`, mientras el importador interpreta el identificador como `Customer`. Si la respuesta real usa este último campo, el máximo puede resultar cero y proponer código 1. Además, dos procesos pueden calcular el mismo siguiente código y el fallback por hash no garantiza ausencia de colisión.

5. **No hay semántica confirmada de reintento/idempotencia para cliente, pago y factura.** El guard temporal reduce el riesgo, pero no sustituye una clave de idempotencia confirmada por BDP ni una reconciliación completa de respuestas ambiguas.

### P1 — Funciones marcadas como completas que están rotas o incompletas

1. **El sync enriquecido de catálogo puede convertir una respuesta válida en catálogo vacío.** `BdpExportArticlesResponse` espera el campo Rust `articles`, pero no declara `rename_all = "PascalCase"` ni `rename = "Articles"`; el campo tiene `default`, por lo que `{"Articles": [...]}` puede deserializar como vector vacío sin error. Tampoco está demostrado que `Code` acepte tanto número como string pese al comentario.

2. **El refresh de precios consulta el código equivocado.** `sync_prices()` parsea `articulo_glory_codigo` para llamar `GetPricesArticles`; en un mapeo real Glory → BDP debe usar `articulo_bdp_codigo`. Solo funciona accidentalmente cuando ambos códigos son iguales.

3. **El polling no es periódico.** `bdp_poll_interval_secs` se guarda y aparece en la UI, pero no existe scheduler/background loop que lo consuma. Solo hay polling manual y el GET de estado individual termina consultando todas las ventas pendientes del usuario.

4. **Las ventas canceladas se siguen consultando indefinidamente.** `list_bdp_pending()` solo excluye `invoiced` y `error`, no `cancelled`.

5. **`GetOrder` puede aplicar un estado aunque exista `ErrorMessage`.** El poller registra el error remoto como warning, pero devuelve y persiste el `Status` igualmente.

6. **Los errores de sincronización quedan invisibles en frontend.** `BdpSyncBadge` retorna `none` antes de evaluar `syncError` cuando `bdp_synced=false`, que es precisamente el estado habitual de un fallo. El botón de retry también exige simultáneamente `bdp_synced=true` y `bdp_sync_error`, por lo que normalmente no aparece.

7. **Los formatos sugeridos por la UI no coinciden con el parser.** El placeholder de tender propone valores como `"EF"`/`"TC"`, pero el backend solo acepta strings numéricos parseables. El placeholder de canales propone números JSON (`1`, `2`), pero el backend solo lee strings (`"1"`, `"2"`) y ante cualquier discrepancia cae silenciosamente a `Type=0`.

8. **El cliente por defecto y el artículo fallback no están completamente configurables desde la UI.** El backend usa `bdp_default_article_code` y `bdp_default_article_name`, pero el formulario no los expone. `bdp_auto_sync_customers` existe en backend/OpenAPI, pero falta en `EstadoConfiguracion`, defaults, guardado y controles del frontend.

9. **La importación de clientes no cumple lo prometido para volumen.** El comentario promete batch/progreso para ~43k clientes, pero el handler procesa secuencialmente toda la respuesta, con búsquedas y escrituras por cliente, sin paginación, job, progreso, transacción por lote ni reporte recuperable.

10. **El sync de mesas es solo aditivo y crea mesas superpuestas.** Crea zonas por coincidencia exacta de nombre y nuevas mesas en `(0,0)`; no conserva un identificador BDP dedicado, no actualiza renombres/atributos ni retira elementos obsoletos. Es una importación inicial parcial, no sincronización completa.

## Huecos de frontend

| Capacidad backend | Manifestación real en frontend | Estado |
| --- | --- | --- |
| Estado de comanda y polling manual | Columna, badge, filtro y botón global | Parcial; error/retry rotos |
| Crear/reintentar comanda | Retry generado y acción por fila | Parcial; condición incorrecta y edición insegura |
| Multi-item | Editor en alta | Parcial; sin carga/edición posterior ni transacción backend |
| Mapeo de artículos | Tabla CRUD + catálogo/precios | Parcial; formatos y sync de precios/catálogo con defectos |
| Importar clientes BDP → Glory | Solo hook generado | Sin UI |
| Crear cliente Glory → BDP | Solo hook generado | Sin UI ni estado visible en clientes |
| Auto-sync de clientes | Campo backend/OpenAPI | Sin configuración UI |
| Pago BDP | Endpoint backend | Sin hook actualizado visible ni UI operativa |
| Factura BDP | Hook generado | Sin botón/confirmación/estado detallado en UI |
| Sync de mesas | Botón en plano de sala | Parcial; import aditivo y superposición inicial |
| Menús, fastfoods y packs | Endpoints + hooks generados | Sin pantalla; “lectura informativa” no equivale a integración |
| Armado temporal de escritura | Panel con prompts del navegador | Existe, pero no guía una operación granular por entidad |

## Cobertura BDP real frente al plan

La documentación inventaría unos 55 endpoints WebLink y reconoce alrededor de 30 no implementados. Es razonable no integrar stock, compras, transferencias, tallas/colores o fidelización si no aportan al producto; el problema es que el alcance funcional elegido tampoco quedó cerrado.

Para declarar completa la integración prioritaria deben quedar consistentes estos dominios:

- Configuración y descubrimiento: POS, empleado, perfiles, tender, tipo, serie y fallbacks visibles y validados.
- Catálogo: identidad Glory/BDP separada, import incremental correcto, precio/IVA y manejo de bajas.
- Clientes: matching normalizado, conflicto explícito, asignación coordinada de código, UI y progreso de import.
- Ventas/comandas: venta + líneas transaccionales, alta idempotente, política explícita de edición y reconciliación.
- Lifecycle: polling automático real, estados terminales, errores visibles, pago y factura separados con UI segura.
- Operación: auditoría, armado temporal, observabilidad, runbook y pruebas locales contra simulador + PostgreSQL desechable.

## Estado real por fase del plan maestro

| Fase | Estado auditado |
| --- | --- |
| 1 Configuración y mapeos | Parcial: backend/tabla/UI existen; fallbacks ausentes y formatos inconsistentes |
| 2 Multi-item | Parcial: alta funciona en ruta feliz; sin atomicidad ni edición |
| 3 Cliente, tender y canal | Parcial: mapeo backend; cliente manual no seleccionable y formatos UI defectuosos |
| 4 Lifecycle/polling | Parcial: consulta manual; scheduler inexistente y estados/error incompletos |
| 5 Visibilidad frontend | Parcial: columna/filtro existen; error y retry normalmente invisibles |
| 6 Frontend multi-item | Parcial: alta sí; edición no |
| 7 Sync de clientes/artículos | Incompleta: backend sin UI, import masivo no es batch y código automático inseguro |
| 8 Pagos/facturación | Incompleta: backend separado y endurecido, pero sin UI ni tests de contrato suficientes |
| 9 Catálogo/mesas/menús | Parcial y fuera del plan maestro: catálogo/precios tienen defectos, mesas son import inicial, menús solo proxy JSON |

## Plan de corrección recomendado

### Bloque A — Consistencia local, sin BDP

1. Hacer atómica la creación/actualización de venta y líneas.
2. Añadir lectura y edición real de líneas; impedir reenvíos automáticos de ventas ya sincronizadas hasta definir semántica de actualización.
3. Corregir badge/retry y mostrar estado ambiguo/auditoría al usuario.
4. Corregir parser PascalCase, códigos Glory/BDP, formatos de mapeos y campos de configuración ausentes.
5. Implementar polling programado con límites, backoff, terminales y tratamiento fail-closed de `ErrorMessage`.

### Bloque B — Flujos maestros, sin BDP

1. Rediseñar import de clientes como job por lotes con dry-run local, conflictos y progreso.
2. Sustituir `max + 1`/hash por una asignación explícita o coordinada; mantener `Overwrite=false`.
3. Añadir UI segura para clientes, pagos y facturas; no basta con hooks generados.
4. Convertir mesas en import asistido con preview/diff y colocación, no botón directo.
5. Decidir si menús/packs tendrán modelo y pantalla o deben quedar formalmente fuera de alcance.

### Bloque C — Verificación local integral

1. Ejecutar migraciones en PostgreSQL desechable.
2. Probar backend completo contra el simulador WebLink: éxito, rechazo funcional, timeout, JSON inválido y respuesta aplicada sin conexión.
3. Añadir tests de rutas/UI para cada capacidad visible y tests de contrato para PascalCase y estados.
4. Congelar una matriz de trazabilidad con evidencia por requisito.

### Bloque D — Futuro, solo con autorización granular

Después de A-C, revisar los supuestos que solo BDP real puede confirmar: estructura exacta de `GetOrder`, deduplicación por `MarketplaceOrderId`, identidad de pago, esquema de cliente, series fiscales y comportamiento de factura. Una autorización deberá cubrir una sola operación, entidad y destino durante una ventana acotada.

## Criterio de salida

La integración podrá considerarse lista para pruebas reales cuando todos los P0/P1 estén corregidos, las migraciones y flujos completos pasen en una base desechable contra el simulador, cada función operable tenga UI o quede explícitamente declarada como API-only/fuera de alcance, y la matriz plan → código → test → pantalla no contenga casillas basadas solo en codegen o comentarios.
