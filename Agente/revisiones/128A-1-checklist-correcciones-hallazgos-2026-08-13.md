# Checklist de correcciones — 40 hallazgos de la 2a revision (bloque 128A-1)

> Fecha: 2026-08-13 · Rama: `glory-rs-rest` · Objetivo: verificar cada hallazgo de
> `Agente/revisiones/128A-1-hallazgos-revision-codigo-2026-08-13.md`, probarlo como haga falta y
> corregirlo. Se marca `[x]` al verificar + corregir + dejar evidencia (test/gate).
> Severidad: **[ALTA] [MEDIA] [BAJA]**.

## F0/F1 — Modo operativo (5)
* [x] F0/F1-1 [ALTA] M1 gates en `bdp_sync` (sync_venta, add_order_payment, invoice_order) y
  poller (`poll_due` SQL + loop, `poll_pending`): modo efectivo != Bdp corta escrituras y polling
  aunque `bdp_sync_enabled` siga activo. Evidencia: clippy verde + tests de `modo_operacion`.
* [x] F0/F1-2 [MEDIA] M2 histeresis minima real: `registrar_fallo_bdp`/`registrar_exito_bdp`
  (N=3 en memoria) + `modo_efectivo_sin_red`; el poller alimenta el contador y degrada a
  standalone. Evidencia: 3 tests unitarios nuevos (degradacion, reset, no-op en standalone).
* [x] F0/F1-3 [MEDIA] M3 cache usada de verdad: `obtener_modo_operacion`, `cambiar_modo_operacion`,
  `diagnosticar_bdp` y los 4 handlers de purchase notes pasan por `state.modo_operacion.modo_efectivo()`.
* [x] F0/F1-4 [BAJA] `guardar_cache` poda entradas expiradas al insertar. Evidencia: test
  `cache_purga_entradas_expiradas_al_insertar`.
* [x] F0/F1-5 [BAJA] Badge derivado con la logica del backend (`modoEfectivoBdp`): modo forzado
  `bdp` muestra BDP aunque `bdp_sync_enabled=false`. Evidencia: type-check frontend verde.

## F2 — Catalogo local (4)
* [x] F2-1 [MEDIA] `crear()` (POST/upsert) pisa campos locales con defaults
  * DO UPDATE con `COALESCE($n, bdp_article_map.x)` (params Option) en vez de `EXCLUDED.x`;
    un mapeo puro ya no vacia descripcion/precio/iva ni reactiva un articulo desactivado (M7).
    Evidencia: clippy verde + `test_crear_mapeo_puro_no_pisa_campos_locales`.
* [x] F2-2 [MEDIA] Alta local no queda protegida del import (M6)
  * `local_dirty = $15 = tiene_campos_locales`; DO UPDATE marca dirty solo si la fila era `origen='bdp'`.
    Evidencia: `test_crear_local_marca_origen_local` (dirty=true) + clippy verde.
* [x] F2-3 [MEDIA] Doble escritura de stock en `sync_catalog` + filas Omitido*
  * `aplicar_upsert` ya no llama a `upsert_stock` extra (lo hace `upsert_from_bdp` solo para no omitidas).
    Evidencia: `test_upsert_omite_dirty_sin_escritura_stock` + clippy verde.
* [x] F2-4 [BAJA] `resolve_article_local` solo con codigo numerico
  * Busca por el string configurado (ignora vacio/GLORY); id BDP del `articulo_bdp_codigo` del mapeo
    cuando el codigo configurado no es numerico. Evidencia: `test_resolve_article_local_sin_codigo_numerico`
    (lib `services::bdp_sync`, 34/34) + clippy verde.

## F3 — Stock local + N6 (4)
* [x] F3-1 [MEDIA] Sync pisa stock local ajustado
  * Migracion `20260814000001`: `ajustado_local BOOLEAN NOT NULL DEFAULT false`;
    `upsert_stock` con `WHERE NOT ajustado_local AND stock IS DISTINCT FROM EXCLUDED.stock`;
    `ajustar_stock` escribe `ajustado_local = true`; modelo `BdpArticleStock.ajustado_local`.
    Evidencia: `test_sync_no_pisa_stock_ajustado_local` + clippy verde (35/35 en `bdp_article_map`).
* [ ] F3-2 [MEDIA] N6 (get_stock/get_list_stock) sin handler ni uso operativo
  * Pendiente: resolver en bloque F9/F10 (docs) declarando la deuda (queda como transporte sin exponer).
* [x] F3-3 [BAJA] Sin guard de stock negativo
  * `AjusteStockError` (StockNegativo/Db) en `crear/ajustar_stock`: valida `stock < 0` antes del
    commit con rollback; mapeo 422 en `errors/mod.rs`. Evidencia:
    `test_ajustar_stock_rechaza_negativo` + clippy verde.
* [x] F3-4 [BAJA] `warehouse_name` siempre 'General'
  * Derivado: `"General"` solo para id `"0"`, si no usa el id. Evidencia:
    `test_ajustar_stock_warehouse_name_derivado` + clippy verde.

## F4 — Anulacion local + delete D5 (5)
* [x] F4-1 [ALTA] `venta::delete` guard de config antes de checks por venta
  * Eliminado el guard `config.bdp_sync_enabled` global: los checks per-venta (`anulada`,
    `bdp_synced`/`bdp_order_id`) son el unico bloqueo BDP (D5=A); guard Haddock M14 permanece.
    Evidencia: `tests/bdp_f4_anulacion_delete.rs` (`delete_venta_local_sin_sync_con_bdp_activo_ok`,
    `delete_venta_sincronizada_bdp_bloqueada`) + clippy verde.
* [x] F4-2 [MEDIA] M11 liberacion de mesa no implementado
  * Deuda declarada (accion aceptada: "implementar o declarar la deuda"): la ocupacion de mesas se
    deriva de reservas (venta->reserva_id->mesa, fallback `num_mesa`) y no hay vinculo de ocupacion
    que una anulacion deba liberar; no se toca el plano en F4. Nota ampliada en
    `Agente/completados/128A-1-F4-anulacion-local-ventas.md`.
* [x] F4-3 [MEDIA] `anulacion_usuario` client-provided (spoofeable)
  * Campo eliminado de `AnularVentaRequest` (backend y `frontend/src/api/bdp.ts`); el servicio pasa
    siempre `Some(user_id)` del autenticado. Evidencia:
    `tests/bdp_f4_anulacion_delete.rs::anular_registra_usuario_autenticado` + type-check frontend verde.
* [x] F4-4 [MEDIA] `total_periodo` excluye anuladas siempre (modalidad)
  * Firma con `excluir_anuladas: bool` (SQL dinamico); `DashboardService::resumen_mes` lee
    `config.anulacion_modalidad` y excluye solo en `credito_completo`. Evidencia:
    `tests/bdp_f4_anulacion_delete.rs::resumen_mes_respeta_modalidad_anulacion` + clippy verde.
* [x] F4-5 [BAJA] Idempotency key no scoped por venta (anulacion)
  * `VentaRepository::anular` verifica `target_entity_id != id` en el camino de conflicto ->
    `sqlx::Error::Protocol("idempotency_key_otra_venta")` -> `AppError::Conflict` con rollback.
    Evidencia: `tests/bdp_f4_anulacion_delete.rs::idempotency_key_reutilizada_en_otra_venta_conflicto`.

## F5 — Compras locales (5)
* [x] F5-1 [MEDIA] Serie local no forzada al prefijo reservado (M18)
  * `crear_local` exige el prefijo reservado `L` (constante `SERIE_LOCAL_PREFIJO`): serie fuera del
    prefijo -> `sqlx::Error::Protocol("serie_local_prefijo_invalido")` -> 422 en handler; el default
    sigue siendo `L`. El `ON CONFLICT` de `upsert_from_bdp` ahora lleva `WHERE
    bdp_purchase_notes.origen = 'bdp'`: un sync NUNCA pisa total/fecha/datos de un albaran local.
    Evidencia: `tests/bdp_f5_compras_locales.rs` (`serie_local_fuera_del_prefijo_reservado_rechazada`,
    `upsert_bdp_nunca_pisa_albaran_local`) + clippy verde.
* [x] F5-2 [MEDIA] Numeracion `COUNT(*)` no segura por (user_id, serie)
  * Secuencial por `(user_id, serie)` con `MAX(numero::integer) + 1` (solo filas `origen='local'` y
    numeros numericos) y reintento ante 23505 (carrera, hasta 3 intentos). Numero explicito se
    respeta; duplicado explicito -> 409 (no reintento). Evidencia:
    `tests/bdp_f5_compras_locales.rs::numeracion_secuencial_por_serie_y_usuario` + clippy verde.
* [x] F5-3 [MEDIA] Total explicito puede discrepar del desglose de lineas
  * El total SIEMPRE se calcula de las lineas; si el cliente manda un total explicito distinto al
    desglose (base+IVA), `calcular_desglose` devuelve error -> `Protocol` -> 422 con mensaje
    accionable ("no coincide"). Aplica a crear y actualizar. Evidencia:
    `tests/bdp_f5_compras_locales.rs` (`total_discrepante_con_lineas_rechazado`,
    `total_guardado_desde_lineas_con_total_coincidente`) + unit test del repo
    `construir_total_y_datos_rechaza_total_discrepante`.
* [x] F5-4 [BAJA] `desglose_desde_datos` fallback silencioso a (total, 0)
  * El fallback en `conciliar_purchase_note` ahora loguea `tracing::warn!` con id/origen/total
    (importados BDP sin desglose o locales sin lineas son legitimos; ya no es silencioso). El alta
    exige los 4 campos por tipo (`BdpPurchaseNoteLineaLocal` sin opcionales). Evidencia:
    `tests/bdp_f5_compras_locales.rs::conciliacion_sin_desglose_usa_total_con_iva_cero`.
* [x] F5-5 [BAJA] Fecha invalida y numero duplicado sin mapeo de errores
  * `validar_fecha_local` en crear/actualizar: fecha malformada -> 422 "YYYY-MM-DD" (ya no se ignora
    silenciosamente). `map_repo_error`: 23505 (UNIQUE user_id/serie/numero) -> 409 "Ya existe un
    albaran con esa serie y numero (duplicado)". Evidencia:
    `tests/bdp_f5_compras_locales.rs` (`fecha_invalida_rechazada_en_crear_y_actualizar`,
    `numero_duplicado_mapeado_a_conflicto_409`).

## F6 — Auditoria/pagos/factura (6)
* [x] F6-1 [MEDIA] `venta::delete` no bloquea `facturada_local`
  * `VentaService::delete` ahora trata `facturada_local` como estado final (D5): Conflict con
    mensaje accionable; además rechaza ventas con filas en `bdp_pagos` (el DELETE no puede
    cascadear el ledger). Evidencia: `tests/bdp_f6_correcciones.rs`
    (`delete_venta_facturada_local_bloqueada`, `delete_venta_con_filas_bdp_pagos_bloqueada`).
* [x] F6-2 [MEDIA] Idempotencia factura local inalcanzable (guard M9 antes de clave)
  * `VentaRepository::facturar_local` resuelve la clave ANTES de los guards M9: reintento con la
    misma clave sobre la venta ya facturada → Ok idempotente (mismo numero), nunca 409. Evidencia:
    `tests/bdp_f6_correcciones.rs::factura_local_reintento_misma_clave_es_exito_idempotente`.
* [x] F6-3 [MEDIA] Filas legacy `error`/`ambiguo` bloquean factura local
  * El guard de pagos solo mira filas con `resultado IN ('exito','ambiguo')` (EXISTS y SUM): una
    fila legacy `error` de un flujo BDP previo ya no deja la venta bloqueada para siempre. Evidencia:
    `tests/bdp_f6_correcciones.rs::factura_local_con_fila_legacy_error_ok`.
* [x] F6-4 [BAJA] Numeracion `F-{anio}-{n}` con COUNT global mezcla anos
  * Numeración por `(user_id, año)`: `MAX((regexp_match(...))[1]::integer) + 1` sobre
    `factura_numero LIKE 'F-{anio}-%'` (retry 23505 del servicio cubre la carrera). Evidencia:
    `tests/bdp_f6_correcciones.rs::numeracion_por_anio_no_mezcla_numeros_previos`.
* [x] F6-5 [BAJA] Idempotency key cross-venta -> exito falso (factura)
  * La clave se valida contra `target_entity_id` en ambos caminos (previo y carrera): si apunta a
    OTRA venta → `Protocol("idempotency_key_otra_venta")` → `AppError::Conflict`, y la venta queda
    sin facturar. Evidencia: `tests/bdp_f6_correcciones.rs`
    (`factura_local_clave_reutilizada_otra_venta_conflicto`).
* [x] F6-6 [BAJA] `tender_id` sin validar contra formas de pago
  * No existe tabla local de tenders: el mapeo método Glory → tender BDP vive en
    `configuracion_restaurante.bdp_tender_map` (JSONB) y `bdp_pagos` no tiene FK. El contrato queda
    documentado en `src/handlers/ventas.rs::pago_parcial_local` (validación `tender_id > 0` ya
    existente en el handler). Evidencia: comentario de contrato + guard existente.

## F7 — Menus/packs locales (4)
* [x] F7-1 [MEDIA] Filtro `tipo` no validado + `From<String>` default silencioso
  * `BdpMenuLocalTipo` pasa de `From<String>` (default silencioso a `Menu`) a `TryFrom<String>`/
    `TryFrom<&str>` con `Error = &'static str`; `crear`/`actualizar` convierten con `.try_into()` →
    `sqlx::Error::Protocol("tipo_invalido")` → `AppError::Validation`; `listar_menus_locales`
    valida `params.tipo` antes de consultar (400). `map_error_unique` renombrado a `map_repo_error`
    y mapea `tipo_invalido` y `articulo_no_en_catalogo:...`. Evidencia:
    `tests/bdp_f7_menus_locales.rs::handler_listar_filtro_tipo_invalido_rechaza` +
    `tipo_desconocido_falla_al_convertir` (unit).
* [x] F7-2 [BAJA] `articulo_codigo` sin validar contra catalogo local
  * `validar_articulos_en_catalogo(pool, user_id, lineas)` en el repo: códigos no vacíos deben
    existir en `bdp_article_map.articulo_glory_codigo` del usuario (`= ANY($1)`); falta →
    `Protocol("articulo_no_en_catalogo:...")` → `AppError::Validation` con mensaje accionable.
    Se llama en `crear` y en `actualizar` (solo si llegan líneas). Evidencia:
    `tests/bdp_f7_menus_locales.rs::crear_menu_con_articulo_fuera_del_catalogo_rechazado`.
* [x] F7-3 [BAJA] CRUD menus sin auditoria local (A11)
  * `auditar(conn, user_id, operacion, menu_id, payload)` inserta en `bdp_audit_log` con
    `direccion='internal'`, `resultado='exito'`, `origen_operacion='local'`,
    `target_entity_type='menu_local'`, `target_entity_id`, `authorization_reason` y SIN
    `idempotency_key`; se llama dentro de la tx en `crear`, `actualizar` (payload con `tipo_audit`
    calculado antes del move de `tipo`) y `eliminar` (ahora `eliminar(pool, id, user_id)` con tx
    interna). Evidencia: `tests/bdp_f7_menus_locales.rs::crud_menus_registra_auditoria_local`.
* [x] F7-4 [BAJA] ILIKE sin escape de wildcards
  * `listar_menus` escapa `\`, `%` y `_` del término (`replace` + `ESCAPE '\'` en ambos ILIKE):
    buscar `100%` o `Combo_` ya no es comodín. Evidencia:
    `tests/bdp_f7_menus_locales.rs::busqueda_escapa_wildcards_iliike`.

## F8 — Permisos operativos (4)
* [x] F8-1 [MEDIA] `pago_parcial_local`/`factura_local` sin permiso operativo
  * 2 variantes nuevas `AccionPermiso::PagosLocales`/`FacturacionLocal` + migración aditiva
    `20260820000000_bdp_permisos_operativos_locales` (columnas `permisos_pagos_locales`/
    `permisos_facturacion_local`, `VARCHAR(20) NOT NULL DEFAULT 'admin'` con CHECK). Guards en
    `ventas.rs` al inicio del handler (antes de validaciones). Evidencia:
    `tests/bdp_f8_permisos.rs::trabajador_recibe_403_pago_parcial_local_con_default_admin` y
    `..._factura_local_con_default_admin`; UI: 2 selects nuevos en `ConfigBdp.tsx`.
* [x] F8-2 [BAJA] `eliminar_venta` sin permiso por accion
  * Decisión: reusar `AccionPermiso::AnulacionVentas` (misma clase de escritura destructiva,
    default `admin`); documentado en comentario del handler. Evidencia:
    `tests/bdp_f8_permisos.rs::trabajador_recibe_403_eliminar_venta_con_default_admin` y
    `..._admin_no_recibe_403_al_eliminar_venta_inexistente`.
* [x] F8-3 [BAJA] Tests 403 parciales por endpoint (albaranes/delete)
  * Añadidos 403 para `actualizar/eliminar/marcar_borrador/conciliar_purchase_note` y
    `eliminar_venta`; total `tests/bdp_f8_permisos.rs` pasa de 14 a 24. Evidencia:
    `trabajador_recibe_403_{actualizar,eliminar,marcar_borrador,conciliar}_purchase_note_con_default_admin`.
* [x] F8-4 [BAJA] `verificar_permiso` usa `obtener_o_crear` (escritura en lectura)
  * Nuevo `ConfiguracionRepository::obtener` (SELECT puro, devuelve `Option`); `verificar_permiso`
    lo usa y falla cerrado a `NivelPermiso::Admin` si no hay fila (no crea configuración al
    comprobar un permiso). Evidencia:
    `tests/bdp_f8_permisos.rs::verificar_permiso_sin_config_no_crea_fila_y_falla_cerrado`.

## F9/F10 — Docs (3)
* [ ] F9-1 [BAJA] Reporte "reproducible" en ruta mutable (latest.md sobrescrito)
* [ ] F10-1 [MEDIA] Docs sobrevenden M1/M2/M3 (declarar deuda o implementar)
* [ ] F10-2 [BAJA] Referencia con wildcard en roadmap.md:151
