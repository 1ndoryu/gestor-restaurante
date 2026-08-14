# Checklist — 2a revision de codigo completa · Bloque 128A-1 (independencia BDP)

> Fecha: 2026-08-13 · Rama: `glory-rs-rest` · Base de revision: `821954c0..2475cba0`
> Metodo: revision de codigo estatica, tarea por tarea (F0-F10), sin ejecutar pruebas ni
> sentinel check. Cada tarea tiene un check pendiente que se marca conforme se revisa.
> Hallazgos: registrados en `Agente/revisiones/128A-1-hallazgos-revision-codigo-2026-08-13.md`.

## Checklist por tarea

* [x] **F0/F1 — Conmutador de modo operativo + badge independiente** (`821954c0`): revisar
      migracion `modo_operacion`, `ServicioModoOperacion` (TTL/histeresis/invalidacion M1-M3),
      guards de coherencia, badge frontend y degradacion. Hallazgos: 5 (1 alta, 2 media, 2 baja).
* [x] **F2 — Catalogo local** (`92dd4cfe`): revisar migracion `bdp_article_map` (origen/local_dirty),
      modelo/repositorio CRUD, `resolve_article` (M5), import sin pisar ediciones (M6) ni reactivar
      desactivados (M7), UI catalogo. Hallazgos: 4 (3 media, 1 baja).
* [x] **F3 — Stock local + GetStock/GetListStock** (`978dd3f4`): revisar ajuste manual con auditoria,
      idempotencia, weblink N6 (structs/endpoints), fuente de verdad `bdp_article_stock`, UI origen.
      Hallazgos: 4 (2 media, 2 baja).
* [x] **F4 — Anulacion local + delete D5** (`624cc9f1`): revisar migracion `venta_anulacion`,
      `AnulacionVentaService` (modalidades D4, M8-M11), transicion de estado/idempotencia,
      desbloqueo `venta::delete` (Haddock M14), poller, UI. Hallazgos: 5 (1 alta, 3 media,
      1 baja).
* [x] **F5 — Compras locales** (`24a22b64`): revisar migracion `bdp_purchase_notes` (origen/series L-),
      CRUD albaranes locales, gates M12 (flags solo bdp), IVA por linea (A10), conciliacion M18, UI.
      Hallazgos: 5 (3 media, 2 baja).
* [x] **F6 — Auditoria local + pagos parciales + factura local minima** (`acf59e77`): revisar
      migracion `bdp_audit origen_local`, ledger `bdp_pagos` local (idempotencia/saldo), factura
      local `facturada_local` (numeracion), guards, UI. Hallazgos: 6 (3 media, 3 baja).
* [x] **F7 — Menus/packs locales** (`17cc1a03`): revisar migracion `bdp_menu_local` (CRUD con lineas),
      modelo/repositorio/handler, convivencia con BDP (origen), UI Explorador. Hallazgos: 4
      (1 media, 3 baja).
* [x] **F8 — Permisos operativos** (`3fc17534`): revisar migracion `bdp_permisos_operativos`,
      `src/services/permisos.rs` (fail-closed, `permiso_habilitado`, `verificar_permiso`),
      enforcement por endpoint (M17), UI ConfigBdp. Hallazgos: 4 (1 media, 3 baja).
* [x] **F9 — Pruebas con/sin BDP** (`e12b3968`): revisar documentacion de evidencia (suites,
      simulador, gate) y coherencia de lo declarado con el codigo revisado. Hallazgos: 1 (baja).
* [x] **F10 — Cierre documental** (`e9eef0dd`, `2475cba0`): revisar roadmap, completados,
      feature-flags/mapeo/guia y consistencia de rutas y referencias. Hallazgos: 2 (1 media,
      1 baja).

**Total de la 2a revision:** 40 hallazgos (2 alta, 19 media, 19 baja) — ver
`Agente/revisiones/128A-1-hallazgos-revision-codigo-2026-08-13.md` (resumen global al final).

## Veredicto de la 2a revision (pasada dura de cierre, estilo supervisor-review)

- **Revision completa:** F0–F10 revisados tarea por tarea contra el plan
  (`Agente/planes/completados/plan-independencia-bdp-2026-08-12.md`, §4 diseños, M1–M18,
  D1–D9, A7–A13, §11/§12) y el estado real del codigo (commits `821954c0..2475cba0`). Cada
  hallazgo con `archivo:linea`, severidad y accion sugerida.
- **Metodo:** revision estatica; NO se ejecutaron pruebas ni sentinel check (instruccion
  explicita del usuario). Los conteos de evidencia de F9 se validaron estaticamente
  (13/13, 15/15, 9/9, 2/2, 10/10, 24/24, simulador 92/92, reporte del gate presente).
- **Hallazgos criticos:** 2 ALTOS — F4-1 (`venta::delete` bloquea todo con sync activa por el
  guard de config ANTES de los checks por venta, `src/services/venta.rs:225`) y F0/F1-1 (M1 no
  aplicado en los caminos de escritura/polling de BDP). Ademas F10-1 (MEDIA): la documentacion
  de cierre da por implementadas M2 (histeresis) y M3 (cache), que el codigo difiere o no usa.
- **Estado del bloque:** la implementacion F0–F10 esta funcionalmente cubierta por tests, pero
  NO esta libre de hallazgos: se recomienda corregir los 2 ALTOS y declarar explicitamente la
  deuda M2/M3/M1-write antes de considerar el plan "100% operacional" cerrado. El resto son
  correcciones puntuales (media/baja) listadas por fase.
- **Sentinel/gate:** no ejecutado (fuera de alcance por instruccion); los cambios de esta
  revision son solo 2 MDs bajo `Agente/revisiones/`. Cuando se implementen correcciones, correr
  el gate normal del proyecto.
