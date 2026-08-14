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
* [ ] F6-1 [MEDIA] `venta::delete` no bloquea `facturada_local`
* [ ] F6-2 [MEDIA] Idempotencia factura local inalcanzable (guard M9 antes de clave)
* [ ] F6-3 [MEDIA] Filas legacy `error`/`ambiguo` bloquean factura local
* [ ] F6-4 [BAJA] Numeracion `F-{anio}-{n}` con COUNT global mezcla anos
* [ ] F6-5 [BAJA] Idempotency key cross-venta -> exito falso (factura)
* [ ] F6-6 [BAJA] `tender_id` sin validar contra formas de pago

## F7 — Menus/packs locales (4)
* [ ] F7-1 [MEDIA] Filtro `tipo` no validado + `From<String>` default silencioso
* [ ] F7-2 [BAJA] `articulo_codigo` sin validar contra catalogo local
* [ ] F7-3 [BAJA] CRUD menus sin auditoria local (A11)
* [ ] F7-4 [BAJA] ILIKE sin escape de wildcards

## F8 — Permisos operativos (4)
* [ ] F8-1 [MEDIA] `pago_parcial_local`/`factura_local` sin permiso operativo
* [ ] F8-2 [BAJA] `eliminar_venta` sin permiso por accion
* [ ] F8-3 [BAJA] Tests 403 parciales por endpoint (albaranes/delete)
* [ ] F8-4 [BAJA] `verificar_permiso` usa `obtener_o_crear` (escritura en lectura)

## F9/F10 — Docs (3)
* [ ] F9-1 [BAJA] Reporte "reproducible" en ruta mutable (latest.md sobrescrito)
* [ ] F10-1 [MEDIA] Docs sobrevenden M1/M2/M3 (declarar deuda o implementar)
* [ ] F10-2 [BAJA] Referencia con wildcard en roadmap.md:151
