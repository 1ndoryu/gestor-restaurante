# Hallazgos — 2a revision de codigo completa · Bloque 128A-1 (independencia BDP)

> Fecha: 2026-08-13 · Rama: `glory-rs-rest` · Revision estatica sin ejecutar pruebas ni gate.
> Formato por fase: cada hallazgo con archivo:linea, severidad (Alta/Media/Baja) y accion sugerida.

## F0/F1 — Conmutador de modo operativo (`821954c0`)

1. **ALTA — M1 no se aplica en los caminos de escritura/polling.** `BdpSyncService::sync_venta`
   (`src/services/bdp_sync.rs`) y `bdp_order_poller` (`src/services/bdp_order_poller.rs`) siguen
   gateando solo por `bdp_sync_enabled` (+ `bdp_configurado`), sin consultar
   `ServicioModoOperacion`. Si un usuario fuerza `modo_operacion='standalone'` conservando
   `bdp_sync_enabled=true` (caso que el plan M1 permite: "se tratan como inactivos, sin borrarlos"),
   la sincronizacion y el polling seguiran llamando a BDP. Viola la invariante "standalone nunca
   llama a BDP" y la aceptacion de F1. Accion: gatear esos caminos con `modo_efectivo()==Bdp` (o
   normalizar/persistir sync=false al forzar standalone).
2. **MEDIA — M2 (histeresis/degradacion reactiva) no implementada.** `modo_efectivo_desde_config`
   deriva el modo solo de la configuracion (estatico, sin red). No existe `evento_fallo_bdp`, ni
   conteo de exito/fallo consecutivo, ni degradacion automatica cuando BDP cae en modo `auto`/`bdp`.
   El comentario en `src/services/modo_operacion.rs:82` lo difiere a "fase F1.2", pero el plan da
   F1 por completada y su aceptacion dice "BDP caido: degrada sin errores". Accion: implementar la
   histeresis o marcar la deuda explicitamente en el plan con evidencia.
3. **MEDIA — Cache TTL e invalidacion M3 sin uso real.** El metodo async `modo_efectivo()` (cache
   por usuario + TTL) no se invoca en ningun consumidor; todos usan `modo_efectivo_desde_config`.
   `invalidar()` se llama pero no afecta nada observable. Codigo muerto que da falsa sensacion de
   M3 implementado. Accion: usar la cache en un punto real (p. ej. badge/endpoint) o eliminarla.
4. **BAJA — Entradas expiradas nunca se purgan** en la cache (solo se reemplaza la del mismo
   usuario). Crecimiento monotono acotado por numero de usuarios; bajo riesgo, pero conviene podar
   al insertar.
5. **BAJA — Badge incoherente en modo forzado `bdp`.** En `site-header.tsx`, si
   `modo_operacion='bdp'` pero `bdp_sync_enabled=false`, el badge muestra "BDP: off" aunque el modo
   efectivo backend sea BDP. Accion: derivar el estado del badge con la misma logica que el backend
   (standalone / auto+sin sync / bdp).

## F2 — Catalogo local (`92dd4cfe`)

1. **MEDIA — `crear()` (POST/upsert) pisa campos locales con defaults al hacer conflicto de codigo.**
   En `src/repositories/bdp_article_map.rs`, el `ON CONFLICT ... DO UPDATE` usa
   `COALESCE(EXCLUDED.x, bdp_article_map.x)`, pero EXCLUDED nunca es NULL porque el INSERT liga
   defaults (`""`, `0`, `true`) para campos ausentes. Un POST de mapeo sobre un glory code existente
   sin campos locales vacia `descripcion`/`precio`/`iva` y pone `activo=true` (puede REACTIVAR un
   articulo desactivado localmente — M7 violado por este camino). Accion: ligar NULL para campos
   ausentes o separar el alta local del mapeo clasico.
2. **MEDIA — Articulo creado como `local` no queda protegido del import (M6 incompleto).** Un alta
   local inserta `origen='local'` con `local_dirty=false`; `upsert_from_bdp` solo respeta
   `local_dirty=true`. Si su codigo Glory coincide con un codigo BDP, el proximo import le pisa
   descripcion/precio/stock. Accion: marcar `local_dirty=true` tambien en el alta local (o
   considerar `origen='local'` como dirty).
3. **MEDIA — Doble escritura de stock en `sync_catalog`.** `aplicar_upsert` llama a `upsert_stock`
   despues de que `upsert_from_bdp` ya lo hizo internamente (2 escrituras por articulo). Ademas,
   para filas `Omitido*` (dirty o desactivadas) `aplicar_upsert` aun escribe el stock BDP en
   `bdp_article_stock`, pisando ajustes locales. Accion: escribir stock una sola vez y omitirla
   para filas omitidas (relacionado con hallazgo F3-1).
4. **BAJA — `resolve_article_local` solo aplica si `bdp_default_article_code` es numerico.**
   Catalogos con codigo alfanumerico nunca resuelven desde el catalogo local (caen al fallback
   generico). Accion: ampliar la resolucion al codigo Glory de la venta, no solo al default.

## F3 — Stock local + N6 (`978dd3f4`)

1. **MEDIA — El sync de catalogo pisa el stock local ajustado.** `upsert_stock` (usado por
   `sync_catalog`/`sync_prices` path) hace `stock = EXCLUDED.stock` sobre `bdp_article_stock`
   (almacen 'General'), la misma tabla que F3 define como fuente de verdad editable. Un ajuste
   local se pierde en el siguiente sync. El plan dice "nunca se pisa". Accion: no sobrescribir
   filas ajustadas localmente (p. ej. marca `ajustado_local` o respetar `local_dirty` del articulo).
2. **MEDIA — N6 sin handler ni uso operativo.** `get_stock`/`get_list_stock` existen en
   `bdp_weblink.rs` con structs y tests wiremock, pero no hay endpoint `GET /api/bdp/stock` ni
   llamada desde `BdpStock.tsx`; la UI solo usa `stock_actual` + `bdp_article_stock`. N6 queda
   como transporte sin exponer (el plan preveia handler + UI con origen por valor). Accion:
   cablear el endpoint o marcar la deuda.
3. **BAJA — Sin guard de stock negativo.** `ajustar_stock` suma el delta sin validar que el stock
   resultante no sea negativo; un error de tipeo puede dejar inventario negativo. Accion: validar
   `stock >= 0` o permitirlo explicitamente con aviso.
4. **BAJA — `warehouse_name` siempre 'General'** aunque `warehouse_id` sea distinto de "0" en el
   ajuste. Inconsistencia menor de etiqueta. Accion: derivar del id o aceptar solo el almacen por
   defecto.

## F4 — Anulacion local + delete D5 (`624cc9f1`)

1. **ALTA — D5 desbloqueo incompleto: el guard de config se mantiene ANTES de los checks por venta.**
   En `VentaService::delete` (`src/services/venta.rs:225`) el `if config.bdp_sync_enabled { 409 }`
   se evalua antes de los checks por venta (`anulada`, `bdp_synced`, `bdp_order_id`). El plan
   D5=A dice "se desbloquea el 409 solo para ventas no sincronizadas con BDP y no anuladas": con
   sync activa (modos bdp/auto con credenciales) NINGUNA venta se puede eliminar, aunque no este
   sincronizada ni anulada. Ademas el frontend (`venta-row-actions.tsx`) solo oculta el boton por
   `haddockSyncEnabled`, `v.anulada`, `bdp_synced` y `bdp_order_id`, no por `bdpSyncEnabled`:
   en modo sync activa el boton queda visible y responde 409 con mensaje que pide desactivar la
   sincronizacion (incomodo pero coherente). Accion: reordenar los checks (per-venta antes que
   config) o eliminar el guard de config reemplazandolo por los per-venta; alinear la visibilidad
   del boton con el backend.
2. **MEDIA — M11 (liberacion de mesa) no implementado.** El plan F4 incluye M11 ("liberacion de
   mesa solo si la venta es la ocupante actual; si no, avisar y no tocar el plano").
   `VentaRepository::anular` (`src/repositories/venta.rs:536`) no toca mesas ni reservas ni emite
   aviso en auditoria; solo hay un comentario en `src/services/venta.rs:269-273` documentando la
   intencion. La doc `Agente/completados/128A-1-F4-anulacion-local-ventas.md` admite "M11 no toca
   el plano en F4". Accion: implementar la derivacion venta->reserva->mesa y la liberacion/aviso,
   o declarar la deuda explicitamente en el plan (F4 esta dada por completada).
3. **MEDIA — `anulacion_usuario` es client-provided y ademas nadie lo envia.** El campo
   `AnularVentaRequest.anulacion_usuario` (`src/models/venta.rs:207`) viaja del cliente al handler
   (`src/handlers/ventas.rs:200`) y se audita tal cual, sin derivarlo de `auth.user_id`
   (spoofeable). El frontend no lo envia (`ListaVentas.tsx:300` solo manda motivo + key), asi que
   queda NULL siempre y la auditoria no registra quien anulo. Accion: derivarlo siempre de
   `auth.user_id` en el handler y eliminar el campo del request (o validar que coincida).
4. **MEDIA — `total_periodo` excluye anuladas de forma permanente, tambien en `estado_solo`.**
   Plan M10: en `credito_completo` el resumen diario excluye/revierte la venta; en `estado_solo`
   "solo marca estado (sin reversion contable)". El repo (`src/repositories/venta.rs:371`) filtra
   `anulada = false` siempre, sin consultar la modalidad; el comentario lo justifica "para no
   descuadrar caja nunca", pero difiere del plan. Accion: confirmar intencionalidad o
   parametrizar la exclusion por modalidad.
5. **BAJA — Idempotency key no esta scoped por venta.** En el camino de conflicto
   (`src/repositories/venta.rs:598-614`) la clave se busca solo por `(user_id, idempotency_key)`
   sin verificar `target_entity_id = venta_id`. Si el cliente reutiliza la misma clave para otra
   venta, el servicio devuelve exito idempotente (`resultado_previo='exito'`) aunque esa segunda
   venta NO quedo anulada (se devuelve su estado actual). Accion: incluir `venta_id` en la
   comprobacion de la clave o validar que la fila previa apunte a la misma venta.

Verificado en F4 (sin hallazgo): M9 bloquea `facturada_local` + `bdp_invoiced` + status invoiced
(`src/repositories/venta.rs:557`); transicion unica con guard + rollback de auditoria en carrera;
mapeo de errores `map_anular_error` (404/409/422) correcto; poller excluye anuladas
(`src/repositories/venta.rs:452`); `bdp_sync.rs`/`haddock.rs` saltan ventas anuladas; UI con
confirmacion `ANULAR {id}` + motivo + badge `anulada`/pendiente BDP; tests constructores
actualizados.

## F5 — Compras locales (`24a22b64`)

1. **MEDIA — La serie local no se fuerza al prefijo reservado (M18 incompleto).**
   `crear_local` (`src/repositories/bdp_purchase_note.rs:125-160`) usa `L` solo como default;
   `req.serie` es libre y no se valida. Si el cliente envia una serie que coincide con una de BDP:
   (a) el `UNIQUE(user_id, serie, numero)` puede chocar (23505 -> 500 sin mapeo); (b) peor,
   `upsert_from_bdp` (`src/repositories/bdp_purchase_note.rs:76-91`) hace `ON CONFLICT
   (user_id, serie, numero) DO UPDATE` sin guard de `origen='bdp'`, asi que un sync posterior
   sobreescribiria total/fecha/datos de un albaran local con datos BDP. El plan M18 pedia "series
   locales reservadas (L-... / prefijo configurable)". Accion: forzar/validar el prefijo en
   backend para origen local y excluir `origen='local'` del conflicto del import.
2. **MEDIA — Numeracion secuencial local via `COUNT(*)` no es segura ni estable.**
   `crear_local` calcula `numero = COUNT(*) + 1` sobre TODAS las notas locales del usuario,
   ignorando la serie, sin transaccion ni guard de carrera: dos altas simultaneas generan el mismo
   numero (colision UNIQUE -> 500), y tras un borrado se reutilizan numeros (cuestionable para un
   secuencial documental). Accion: secuencia por `(user_id, serie)` con reintento ante 23505 o
   `MAX(numero) + 1` bajo lock/reintento.
3. **MEDIA — Total explicito puede discrepar del desglose de lineas.**
   `calcular_desglose` (`src/repositories/bdp_purchase_note.rs`) usa `total_explicito` si viene,
   pero `datos_bdp.importe_base/importe_iva` se calculan de las lineas. La conciliacion
   (`desglose_desde_datos`) registra el gasto con base/IVA de lineas aunque el albaran muestre otro
   total -> descuadre albaran vs gasto. `crear_purchase_note_local`
   (`src/handlers/bdp_purchase_note.rs`) acepta total + lineas sin validar consistencia. Accion:
   validar `total == base + iva` cuando vienen lineas (o calcular siempre de las lineas).
4. **BAJA — `desglose_desde_datos` falla silenciosamente a `(total, 0)`.** El
   `unwrap_or((total, Decimal::ZERO))` en `conciliar_purchase_note` (`src/handlers/bdp_purchase_note.rs`)
   convierte lineas malformadas (sin cantidad/precio/iva) en gasto con IVA=0 sin log ni aviso.
   Accion: loggear el fallback y validar en el alta que cada linea tenga los 4 campos.
5. **BAJA — Fecha invalida y numero duplicado sin mapeo de errores.** Un `fecha` mal formado se
   ignora silenciosamente (None) en crear/actualizar; `actualizar_local` con un `numero` duplicado
   devuelve 23505 -> 500 generico (`AppError::Database`) sin mensaje accionable. Accion: validar
   formato YYYY-MM-DD y mapear 23505 -> 409 con mensaje de serie/numero en uso.

Verificado en F5 (sin hallazgo): gates M12 en los 4 handlers de compras
(`listar`/`sync`/`marcar_borrador`/`conciliar`) via `modo_efectivo_desde_config`; sync con BDP
rechazado en `standalone` (cero llamadas BDP); CRUD local disponible sin flags en `standalone`;
A10 IVA por linea implementado (desglose base/IVA por linea, 4 tests unitarios nuevos);
`origen` con CHECK y default 'bdp' (no altera importados); `eliminar_local` solo pendiente/borrador
(conciliados no se borran, D5); UI con badge origen + modal crear/editar con lineas e IVA.

## F6 — Auditoria local + pagos parciales + factura local (`acf59e77`)

1. **MEDIA — `venta::delete` no bloquea ventas con `facturada_local`.** Los guards de
   `VentaService::delete` (`src/services/venta.rs:236,242`) cubren `anulada` y
   `bdp_synced`/`bdp_order_id`, pero no `facturada_local`. Una venta facturada localmente
   (sin sync BDP) puede eliminarse: se pierde el historico fiscal y el numero de factura, y por
   `ON DELETE CASCADE` en `bdp_pagos` se borran sus pagos. El estado `facturada` es final igual
   que `anulada` (D5). Accion: bloquear el DELETE tambien para `facturada_local` (y, por robustez,
   para ventas con filas en `bdp_pagos`).
2. **MEDIA — Idempotencia de factura local inalcanzable en el reintento normal.** El guard M9
   (`venta.facturada_local` -> Protocol `venta_ya_facturada`, `src/repositories/venta.rs:705`)
   corre ANTES de la consulta de idempotency_key, asi que un reintento con la misma clave sobre una
   venta ya facturada devuelve 409 en vez de exito idempotente (C1). El camino de exito idempotente
   solo se alcanza si la clave existe y la venta NO esta facturada (caso cross-venta -> exito falso,
   ver F4-5). Accion: consultar la clave antes de los guards M9 (o tratar
   `venta_ya_facturada` + clave coincidente como exito idempotente).
3. **MEDIA — Filas legacy de pago fallido/ambiguo bloquean la facturacion local.**
   `facturar_local` (`src/repositories/venta.rs:709-718`) usa `EXISTS(SELECT 1 FROM bdp_pagos)` y
   suma solo `resultado='exito'`: una fila historica con `resultado='error'` o `'ambiguo'` (flujo
   BDP previo) deja `tiene_pagos=true` y `pagado=0` -> bloquea la factura local con
   "pagos pendientes" para siempre, aunque no haya saldo real pendiente. Accion: considerar solo
   filas `resultado='exito'`/`'ambiguo'` (o 'error' retentado) para el guard de pendiente.
4. **BAJA — Numeracion `F-{anio}-{n:04}` con `COUNT` global mezcla anos y reutiliza numeros.**
   El `COUNT(*)` (`src/repositories/venta.rs:725-726`) cuenta todas las facturas del usuario sin
   filtrar por anio (el numero embebido en `F-{anio}-...` puede saltarse o repetirse entre anos) y,
   combinado con el hallazgo 1 (delete permitido), un borrado decrementa el conteo y reutiliza
   numeros del mismo anio. Accion: numerar por `(user_id, anio)` con `MAX`+retry (el retry 23505
   del servicio ya existe) o secuencia dedicada.
5. **BAJA — Idempotency key explicita cross-venta -> exito falso en factura local.** Mismo patron
   que F4-5: la clave se busca solo por `(user_id, idempotency_key)` sin verificar
   `target_entity_id = venta`. Una clave fija reutilizada en otra venta devuelve exito idempotente
   sin facturar la segunda. El frontend genera `randomUUID()` por clic, asi que el riesgo es bajo,
   pero el contrato C1 queda abierto. Accion: incluir `venta_id` en la comprobacion.
6. **BAJA — `tender_id` sin validar contra una tabla de formas de pago.** `bdp_pagos.tender_id` es
   `INT NOT NULL` sin FK; el handler valida `> 0` pero no que el tender exista
   (`src/handlers/ventas.rs`, `PagoLocalRequest`). Un tender inexistente queda como referencia
   huerfana en el ledger. Accion: validar contra la tabla de tenders o documentar el contrato.

Verificado en F6 (sin hallazgo): A11 `origen_operacion` ('local') en las 4 auditorias (anular,
stock_ajuste, pago_parcial_local, factura_local) con default 'bdp' que no altera filas previas;
`bdp_invoice` rechaza ventas con `facturada_local` (M9 extendido); guard en `anular` ya cubre
`facturada_local`; normalizacion de claves vacias (evita colapso de pagos y exito falso entre
ventas distintas sin clave); `pago_parcial_local` con lock `FOR UPDATE`, saldo pendiente, guards
anulada/facturada y respuesta con balance; retry x3 ante 23505 en factura local; handlers con
confirmacion dinamica y mapeo de errores sin 500; UI (BdpHistorial con badge/filtro origen,
pagos/factura local en venta-row-actions, badge factura en tabla); tests 11/11 declarados.

## F7 — Menus/packs locales (`17cc1a03`)

1. **MEDIA — Filtro `tipo` no validado y `From<String>` default silencioso.** En
   `listar_menus` (`src/repositories/bdp_menu_local.rs`) el parametro `tipo` se bindea directo a
   la columna con `CHECK (tipo IN ('menu','pack'))`: un valor invalido desde la query string
   provoca un error de BD (23514) -> 500 en vez de 400. Ademas, `From<String> for
   BdpMenuLocalTipo` (`src/models/bdp_menu_local.rs:45-56`) defaultea a `Menu` con solo un warn
   para valores desconocidos: si el repo se reusa por otra via sin la validacion del handler,
   un tipo invalido se persistiria como 'menu' silenciosamente. Accion: validar el filtro en el
   handler (400) y hacer el `From` fallible o restringirlo a las conversiones ya validadas.
2. **BAJA — `articulo_codigo` de las lineas no se valida contra el catalogo local.**
   `bdp_menu_local_lineas.articulo_codigo` es texto libre sin FK a `bdp_article_map` ni
   comprobacion de existencia en el backend; el select del frontend mitiga el error de tipeo, pero
   un articulo borrado/desactivado (F2/M6/M7) deja referencias colgantes en menus activos. El plan
   §4.10 habla de "agrupaciones de articulos del catalogo local". Accion: validar existencia (o
   avisar) al crear/actualizar, o documentar el contrato de texto libre.
3. **BAJA — CRUD de menus sin auditoria local (A11 incompleto).** Las operaciones
   crear/actualizar/eliminar de `bdp_menus_locales` no registran entradas en `bdp_audit_log`
   (a diferencia de anular/stock/pagos/factura en F6). El Historial local no refleja cambios de
   menu/pack. Accion: auditar las mutaciones (con `origen_operacion='local'`) o declarar el
   alcance de A11.
4. **BAJA — Busqueda ILIKE sin escape de wildcards.** El filtro `busqueda` en `listar_menus`
   usa `format!("%{termino}%")` sin escapar `%`/`_`: un termino con wildcards matchea de mas.
   Riesgo bajo (solo busqueda), pero conviene escapar. Accion: `replace("%", "\\%")` +
   `ESCAPE '\'`.

Verificado en F7 (sin hallazgo): transacciones en crear/actualizar; reemplazo de lineas con
orden determinista (`orden`); `cargar_lineas` con `ANY($1)` (sin N+1); CASCADE de lineas en
eliminar; recalculado de precio desde lineas si no viene explicito; validaciones de tipo/nombre/
lineas/precios/cantidades en handlers; mapeo 23505 -> 409 (`map_error_unique`); UNIQUE
`(user_id, tipo, nombre)`; sin gates de flags (M12: capacidad standalone); UI en BdpExplorador +
BdpMenuLocalModal con select de articulos; tests 15/15 declarados.

## F8 — Permisos operativos configurables (`3fc17534`)

1. **MEDIA — `pago_parcial_local` y `factura_local` (F6, dinero) sin permiso operativo.**
   El plan §4.11/F8 enumeraba explicitamente entre los endpoints a proteger "bdp-payment" y
   "bdp-invoice" con "tests 403 por permiso". La nota de alcance documentada cubre las variantes
   BDP (guards `bdp_sync_enabled`/modo/flags/BdpWriteGuard), pero las variantes LOCALES de F6
   (`pago_parcial_local`, `src/handlers/ventas.rs:717`; `factura_local`,
   `src/handlers/ventas.rs:802`) NO son BDP-bound, funcionan en `standalone` y no tienen
   `verificar_permiso`: con el default 'admin' un Trabajador autenticado puede registrar pagos
   parciales y emitir facturas locales (operaciones monetarias) sin ningun permiso. La nota de
   alcance no cubre estos endpoints. Accion: anadir `AccionPermiso` (p. ej. `PagosLocales`/
   `FacturacionLocal`) + `verificar_permiso` en ambos handlers + tests 403 + selects en
   ConfigBdp.
2. **BAJA — `eliminar_venta` sin permiso por accion.** `src/handlers/ventas.rs:166` (DELETE
   `/ventas/:id`) tiene guards de estado (anulada/sync BDP/order BDP) pero ningun chequeo de rol
   ni permiso: con default 'admin' un Trabajador puede eliminar ventas no sincronizadas ni
   anuladas (historico fiscal local). No aparece en la lista explicita de F8 del plan, pero es
   escritura sensible dentro del espiritu de M17 ("ninguna accion sensible queda solo protegida
   por UI"). Accion: evaluar un permiso dedicado o reusar `AnulacionVentas` para el DELETE.
3. **BAJA — Cobertura de tests 403 parcial por endpoint.** `tests/bdp_f8_permisos.rs` (13/13)
   cubre 403 para stock, catalogo (crear/actualizar/eliminar), albaranes (solo `crear_local`) y
   anulacion; no cubre `actualizar_purchase_note_local`, `eliminar_purchase_note_local`,
   `marcar_borrador_purchase_note` ni `conciliar_purchase_note` (el guard existe en los 5
   handlers, mismo `AccionPermiso`). La promesa "tests 403 por permiso" se cumple por accion pero
   con huecos por endpoint. Accion: anadir los 4 casos restantes de albaranes (y 403 de
   `eliminar_venta` si se decide el hallazgo 2).
4. **BAJA — `verificar_permiso` lee config con `obtener_o_crear` en cada request.** Cada endpoint
   protegido ejecuta `ConfiguracionService::obtener` -> `obtener_o_crear` (`src/repositories/
   configuracion.rs:53`): (a) una query extra por request sin cache; (b) efecto colateral de
   escritura: si el user_id (o el impersonado) no tiene fila de configuracion, se le crea una con
   defaults. Aceptable con decenas de usuarios, pero conviene una lectura pura o cache.

Verificado en F8 (sin hallazgo): migracion aditiva M15 (`ADD COLUMN IF NOT EXISTS` con CHECK y
default 'admin', no altera filas previas); `desde_valor` fail-closed a Admin ante valor
desconocido; `permite` sobre `effective_role` consistente con `AuthUser::require_role` (los
unicos roles son Admin/Trabajador, asi que 'todos' y 'admin_trabajador' son equivalentes en la
practica — futuro-proofing, no defecto); validacion de valores en PATCH (`src/services/
configuracion.rs`) + CHECK en BD (defensa en profundidad); enforcement en los 5 handlers de
albaranes y los 4 de catalogo/stock; UI con 4 selects + defaults 'admin' + sync servidor->local
(ConfigBdp.tsx); exports para tests en `src/handlers/mod.rs`; 13 tests con 403 default admin,
admin sin 403, ampliacion `todos`/`admin_trabajador` habilita al trabajador, PATCH invalido ->
Validation y persistencia; decision documentada de no gatear sync-prices/sync-tables/
customers-import/bdp-poll por estar protegidos por guards BDP de backend (documentado en
`Agente/completados/128A-1-F8-permisos-operativos.md` y en el plan).
