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
* [ ] F2-1 [MEDIA] `crear()` (POST/upsert) pisa campos locales con defaults
* [ ] F2-2 [MEDIA] Alta local no queda protegida del import (M6)
* [ ] F2-3 [MEDIA] Doble escritura de stock en `sync_catalog` + filas Omitido*
* [ ] F2-4 [BAJA] `resolve_article_local` solo con codigo numerico

## F3 — Stock local + N6 (4)
* [ ] F3-1 [MEDIA] Sync pisa stock local ajustado
* [ ] F3-2 [MEDIA] N6 (get_stock/get_list_stock) sin handler ni uso operativo
* [ ] F3-3 [BAJA] Sin guard de stock negativo
* [ ] F3-4 [BAJA] `warehouse_name` siempre 'General'

## F4 — Anulacion local + delete D5 (5)
* [ ] F4-1 [ALTA] `venta::delete` guard de config antes de checks por venta
* [ ] F4-2 [MEDIA] M11 liberacion de mesa no implementado
* [ ] F4-3 [MEDIA] `anulacion_usuario` client-provided (spoofeable)
* [ ] F4-4 [MEDIA] `total_periodo` excluye anuladas siempre (modalidad)
* [ ] F4-5 [BAJA] Idempotency key no scoped por venta (anulacion)

## F5 — Compras locales (5)
* [ ] F5-1 [MEDIA] Serie local no forzada al prefijo reservado (M18)
* [ ] F5-2 [MEDIA] Numeracion `COUNT(*)` no segura por (user_id, serie)
* [ ] F5-3 [MEDIA] Total explicito puede discrepar del desglose de lineas
* [ ] F5-4 [BAJA] `desglose_desde_datos` fallback silencioso a (total, 0)
* [ ] F5-5 [BAJA] Fecha invalida y numero duplicado sin mapeo de errores

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
