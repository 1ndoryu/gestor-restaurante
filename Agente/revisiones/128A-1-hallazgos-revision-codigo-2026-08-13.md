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
