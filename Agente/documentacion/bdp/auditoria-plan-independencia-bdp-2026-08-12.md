# Auditoría del plan «Independencia total del BDP» (128A-1) — 2026-08-12

> Archivo vivo de hallazgos. Se actualiza durante la auditoría para no perder contexto al compactar.
> Plan auditado: `Agente/planes/completados/plan-independencia-bdp-2026-08-12.md`
> Repo: `RESTAURANTE` — rama `glory-rs-rest`

## Método y evidencia

- Lectura completa del plan (557 líneas), roadmap.md y documentación BDP relacionada.
- Verificación en código: `src/`, `migrations/`, `frontend/src/` (grep + lectura de handlers/servicios/modelos).
- Veredicto según skill `supervisor-thinking` (dimensiones SOLID, eficiencia, escala, seguridad, UI, gate).
- Restricciones: no se implementa nada; no se toca producción; no SSH.

## Estado del repositorio (verificado)

| Ítem | Estado | Nota |
| --- | --- | --- |
| Rama | `glory-rs-rest` | Correcta |
| Árbol | Sucio (ajeno/preexistente) | `M roadmap.md` (entrada 128A-1), `M tools/sentinel` (0.7.1 sin commit), `?? plan-independencia-bdp...` |
| Sentinel local | 0.7.1 (submódulo movido, sin commit) | `quality-tools.json` pinea 0.7.0 (a804c0d); origin en 0.7.4 (12 commits detrás) → **F0 debe verificar identidad del gate** |
| Gate | `npm run task:check -- <task-id>` existe; `sentinel.config.json` exige taskId | ID: 128A-1 |
| Deploy 1e | Pendiente según roadmap (no ejecutado) | Premisa de F0 correcta |

## HALLAZGOS CRÍTICOS (correcciones al plan antes de ejecutar F1+)

### C1. `ventas` NO tiene campo `estado` → la máquina de estados de F4 se crea de cero
- Afirmación del plan §4.7/M10: transición «pendiente/pagada → anulada» con estado guardado.
- Realidad: migración `20260325100000_restaurant.up.sql` crea `ventas` con fecha/turno/canal/`metodo_pago` (NOT NULL, CHECK 'efectivo'|'tarjeta'|'transferencia')/importes. **No existe `estado` ni `anulada`**.
- Impacto: F4 no es «añadir columna de estado», es definir por primera vez el estado de la venta. Afecta:
  - `repositories/venta.rs:354 total_periodo` (resumen diario) → debe excluir anuladas.
  - Query de listado de ventas y filtros.
  - `metodo_pago` NOT NULL: una venta «pendiente» sin pago no puede existir sin valor → hay que decidir semántica (¿la venta se crea pagada por defecto? ¿`pendiente` es derivado de `total_pagado < total`?).
- Corrección: F4 debe enumerar todos los agregados que leen `ventas` y definir el estado desde cero (o derivar `pagado` de `total_pagado vs total`).

### C2. `ventas` NO tiene `mesa_id` → M11 subespecificado
- Realidad: `reservas.mesa_id` (migración `20260326200000_plano_sala.up.sql:143`, FK SET NULL); `ventas.reserva_id` (FK → reservas, SET NULL); PlanoOcupacion deriva de **reservas**, no de ventas.
- El plan dice «liberar mesa solo si la venta es la ocupante actual» sin definir la cadena venta→reserva→mesa ni qué es «ocupante actual».
- Corrección: definir en F4 la resolución de la mesa desde la venta (via reserva) y la regla de ocupación (qué reserva/venta es «la actual»). Verificar PlanoOcupacion para no liberar mesas de reservas futuras.

### C3. Contradicción CancelOrder ↔ arming/allowlists
- Plan §4.7: «Con BDP intenta `CancelOrder`; si la suscripción no lo permite → anulada local + reintento (poller M8)». Plan §6: «no se tocan allowlists ni arming»; §4.12 «sin otros cambios».
- Realidad: `cancel_order` existe en `BdpWeblinkClient` (`bdp_weblink.rs:270`) pero **no está en** `VALID_BDP_WRITE_SCOPES` (`bdp_write_guard.rs:10-11`: create_order, add_payment, invoice, create_customer) ni en el CHECK `bdp_write_arming_scopes_safe` (migración `20260718300000_bdp_write_safety_v2.up.sql:92-95`).
- Consecuencia: implementar el reintento `CancelOrder` exige **migración del CHECK + ampliar scopes + wiring arming/auto-arm/backup** → contradice §6. Alternativa: descartar el reintento BDP y dejar «pendiente de anular en BDP» como estado resuelto manualmente por el TPV (coherente con U6, que bloquea el flag «Cancelar comandas» con tooltip).
- Corrección: el plan debe elegir una de las dos vías explícitamente. Recomendado: M8 con `anulada_local_pendiente_bdp` **sin llamada API** mientras BDP no habilite el módulo; CancelOrder solo como mejora futura con su propia migración de scopes.

### C4. F2 duplicaría columnas ya existentes en `bdp_article_map`
- Plan §4.2 propone añadir `precio, iva_pct, familia, codigo_barras, activo`.
- Realidad: migración `20260715200000` ya añadió `descripcion, precio_tarifa1, iva_pct, departamento, familia, subfamilia, activo, barcode, ultima_sync_at`; `20260723000000` añadió `stock_actual`.
- Faltan solo: `origen`, `local_dirty` (y decidir `precio` = `precio_tarifa1` o nueva columna). F2 se reduce.
- El plan ya lo reconoce en A2/A3 y checklist («A2/A3 reducen F2/F3») → la sección §4.2 concreta quedó desactualizada. Corregir el listado de migración en §4.2.

### C5. `bdp_pagos` ON DELETE CASCADE sobre `ventas`
- Al desbloquear delete de ventas no sincronizadas (`venta::delete`), se borran sus pagos locales del ledger.
- Aceptable si se documenta; las anuladas nunca se borran (D5) → consistente. Añadir nota en F4.

### C6. Sentinel: gate 0.7.0 vs submodule local 0.7.1 / origin 0.7.4
- El plan dice gate 0.7.0 (consistente con `quality-tools.json`), pero el submodule local está en 0.7.1 sucio y origin 12 commits detrás.
- F0 debe: verificar `sentinel doctor --json` con readyForGate, confirmar la identidad del binario que ejecutará `task:check`, y no prometer gate reproducible hasta alinear submodule/lock.

## Verificaciones por funcionalidad (A1–A14 del plan — §15 contrastado)

El plan ya contiene su propia auditoría §15 (A1–A14, «estado verificado en código — 2026-08-12»). Esta
auditoría independiente confirmó cada entrada contra el código:

| ID | Afirmación del plan | Verificado | Evidencia | Hallazgo |
| --- | --- | --- | --- | --- |
| A1 | Config por request; PATCH /api/configuracion existe; M3 invalida modo en PATCH | ✅ | `handlers/configuracion.rs` PATCH; config cargada por request | Correcto; M3 se implementa en el PATCH (nuevo) |
| A2 | CRUD `/api/bdp/article-maps` + import/sync catálogo/precios existen | ✅ | `handlers/bdp_article_map.rs` rutas 47-69; upsert por `articulo_glory_codigo` | Correcto; hoy «glory code = BDP code» |
| A3 | `bdp_article_stock` existe, almacén «General», UNIQUE(user_id, articulo, warehouse) | ✅ | migraciones 20260715200000/20260723000000 | Correcto |
| A4 | `/api/bdp/customers/import` + `/clientes/:id/bdp-sync` existen | ✅ | `handlers/bdp_customer_sync.rs:35,311` | Correcto |
| A5 | `/bdp/sync-tables` con confirmación en PlanoSala | ✅ | `handlers/bdp_article_map.rs:64`; frontend PlanoSala | Correcto |
| A6 | No existe pago local como operación separada | ✅ | `bdp_payment` exige `bdp_order_id` (`bdp_sync.rs:1330-1332`) y flags | Correcto |
| A7 | `bdp-invoice` exige `bdp_order_id`; no existe factura local | ✅ | `handlers/ventas.rs:433`; `bdp_invoiced` bool (`20260715100000`) | Correcto; D9 pendiente |
| A8 | GET bdp-payments calcula total/pagado/pendiente sin BDP | ✅ | `handlers/ventas.rs:597-627` | Correcto |
| A9 | delete devuelve 409 si haddock o bdp sync enabled | ✅ | `services/venta.rs:205-229`; mapeo 409 en `errors/mod.rs:55` | Correcto |
| A10 | Flags compras gatean read/draft/reconcile; rango fechas ≤31 días solo en sync; reconcile escribe importe_iva=0 | ✅ | handlers purchase_note + services | Correcto |
| A11 | `bdp_audit_log.direccion` solo glory_to_bdp/bdp_to_glory | ✅ | migración + modelo | Correcto; ops locales puras no encajan → `origen_operacion` (F6 lo contempla) |
| A12 | Explorador solo GET /bdp/menus/:id, /fastfoods/:id, /packs/:id | ✅ | `handlers/bdp_article_map.rs:65-67` | Correcto |
| A13 | `require_role` sin uso | ✅ | `middleware/auth.rs:28` definido; sin call-sites en src | Correcto; F8 wiring completo |
| A14 | Polling/preflight/arming sin cambios salvo M8 | ⚠️ | poller/preflight/write_guard existen | **Contradice C3**: si se implementa CancelOrder, arming/allowlists SÍ cambian |

### Detalle adicional verificado en esta pasada
- `resolve_article` (`bdp_sync.rs:711-787`): hoy NO resuelve desde `bdp_article_map` para el artículo
  por defecto (usa `bdp_default_article_code` → `GetArticle` → primer artículo del perfil → fallback
  genérico con `iva_por_defecto`). El plan §4.2/F2 («se extiende para resolver desde el catálogo local
  antes del fallback») es trabajo real pendiente, como afirma. Nota: `resolve_line_articles`
  (`bdp_sync.rs:792+`) YA consulta `bdp_article_map` para las líneas → el plan debe aclarar que solo
  `resolve_article` (default) se extiende.
- Anexo B del plan: su propio veredicto es «VIABLE CON RESERVAS → reservas incorporadas» y declara
  «autorizado para ejecutar el ciclo local». **Desacuerdo razonado:** C1–C6 no están incorporadas en el
  texto del plan (especialmente C1/C2 no detectadas y C3/C4 contradicciones vivas). Recomendación:
  corregir el plan antes de ejecutar F1+, no solo «aplicar en la fase».

## Verificaciones frontend (patrón U1–U8, plan 08-08)

| ID | Patrón | Verificado | Evidencia |
| --- | --- | --- | --- |
| U1 | Guía en Ventas + botones visibles por venta | ✅ | `ListaVentas.tsx:116-129` aviso + enlace Configuración; `venta-row-actions.tsx` botones si `bdpSyncEnabled && bdp_synced && bdp_order_id` |
| U2 | Aviso integración desactivada + indicador | ✅ | `site-header.tsx:63-100` dropdown «Integración BDP desactivada» |
| U3 | Demo visible + «Salir del modo demo» | ✅ | `BdpDemoToggle.tsx:15-30` textos exactos; usado en BdpStock/Explorador/Historial/Compras |
| U4 | Snapshots = respaldos, no lectura de documentos | ✅ | `BdpHistorial.tsx:277-282` texto aclaratorio; empty state en :245 |
| U5 | «Perfil de exportación BDP» visible + aviso plantilla | ✅ | `BdpPurchaseSyncControls.tsx:90-120` label y `BdpRequiredSetting` |
| U6 | Tooltip CancelOrder «Subscripción no activada» | ✅ | `ConfigBdp.tsx:432-443` flag bloqueado con tooltip |
| U7 | «Correspondencias Glory ↔ BDP» | ✅ | `ConfigBdp.tsx:268` |
| U8 | Aviso envío desactivado + Reintentar solo si falla | ✅ | `ListaVentas.tsx:116-129`; `venta-row-actions.tsx:196-202`; `ListaClientes.tsx:330` (confirmación CREAR CLIENTE) |

## Inventario N1–N14 (Anexo A del plan)

| # | Plan | Verificado | Evidencia |
| --- | --- | --- | --- |
| N1 | Payment/Add → «Subscripción no activada» | ✅ | roadmap.md:94 (2026-08-05 prueba 2.3) y :238 |
| N2 | Factura BDP en espera de 2.3 | ✅ | roadmap.md:96 (:238, prueba 2.4 pendiente) |
| N3 | CancelOrder método existe, no expuesto, BDP rechaza | ✅ | `bdp_weblink.rs:270`; no en scopes (C3); roadmap :109/:246 |
| N4 | Compras F2 solo borradores locales | ✅ | roadmap.md:156 (fases 1-3 locales) |
| N5 | Compras F3 recepción BDP no implementada | ✅ | roadmap.md:156 |
| N6 | GetStock/GetListStock NO existen | ✅ | grep `bdp_weblink.rs` sin `get_stock`/`get_list_stock` → F3 trabajo real |
| N7 | Explorador visible sin verificar | ✅ | roadmap.md:243 (1b) |
| N8 | Pruebas lectura listas, no ejecutadas | ✅ | roadmap.md:252 + `plan-pruebas-lectura-bdp-2026-07-26.md` |
| N9 | Deploy 1e pendiente | ✅ | roadmap.md:241 |
| N10 | Flags false en producción | ⚠️ | No verificable localmente; sin deploy previo es razonable. F0 debe confirmarlo en producción |
| N11 | Tarifa/plantilla Compras del cliente | ✅ | roadmap.md:243 (1b) |
| N12 | Suscripción WebLink sin confirmar | ✅ | roadmap.md:239 (1c) |
| N13 | Limpieza datos prueba TPV | ✅ | roadmap.md:240 (1d) |
| N14 | Bidireccional rechazado (D3) | ✅ | Plan §13 D3 (decisión de diseño, no código) |

## Transversal (pendiente de cierre)

### Histéresis M2
- No existe `evento_fallo_bdp` ni estado de modo en `AppState` (`lib.rs:23-29`: pool, jwt_secret, config, notif_tx). Es diseño nuevo.
- Plan §5: «modo auto: reevaluación con TTL e histéresis, sin polling agresivo»; §14: N=3 éxitos/fallos, nunca a mitad de operación; TTL 30-60s.
- **Pendiente definir:** dónde vive el estado (¿en memoria por proceso?), qué pasa con 2 TPV/multi-instancia (plan lo descarta como riesgo abierto §5/§14 — aceptable pero documentar), y si el conteo N se persiste o se pierde al reiniciar.

### Preflight «OK» en modo auto
- `bdp_configurado()` (`bdp_sync_preflight.rs:694`) solo valida credenciales.
- El dry-run completo (`BdpSyncPreflightService::execute`, líneas 54-164) hace ~8+ llamadas remotas (health, version, POS, employee, tenders, departamentos, artículos, create-order dry-run) y termina con `listo_para_sincronizar`.
- **Pendiente definir:** el switch de modo auto debe usar preflight LIGERO (credenciales + health) en TTL, no el dry-run completo (costoso y con create_order de prueba); el dry-run completo queda on-demand. El plan no distingue ambos.

### Idempotencia pagos parciales locales
- Existe hoy y es robusta: `SYNC_LOCKS` por venta (`bdp_sync.rs:1340-1351`), `idempotency_key` UNIQUE en `bdp_pagos`, reuso verificado por venta/amount/tender (`1357-1374`), `ON CONFLICT` con update condicional (`1591-1604`), `check_idempotency` en write_guard (`20-34`).
- Para F6 (endpoint local): reutilizar este mecanismo; falta el path de escritura sin `bdp_order_id` (hoy exige `bdp_sync_enabled && bdp_configurado && bdp_order_id`).

### Numeración factura local (D9)
- No existe tabla contador ni numeración local; solo `bdp_invoiced` bool y `bdp_invoice_number` (respuesta BDP). D9 ⏳ pendiente (default: sí, mínima).
- **Riesgo de alcance:** la DoD/F6 exigen factura local pero D9 no está resuelta. No bloquea F0-F5, pero F6 depende de esa decisión. Marcarlo en el plan como decisión requerida antes de F6, no «no bloquea» en general.

### Permisos M17
- `require_role` sin call-sites. Todos los endpoints BDP solo exigen `AuthUser` (JWT). M17/F8 correcto.
- Recomendación: enumerar en F8 los endpoints de escritura a proteger (article-maps CRUD, sync-prices, stock ajuste, purchase-notes draft/reconcile, bdp-payment, bdp-invoice, anular, customers/import, clientes/:id/bdp-sync, sync-tables, bdp-poll) y añadir tests 403.

### Anuladas ↔ poller (M8)
- Hoy `list_bdp_pending` = `bdp_synced=true` y `bdp_order_status` no final (`bdp_order_poller.rs:1-17,137-166`). Sin estado `anulada` no hay conflicto aún.
- M8 (excluir `anulada_local_pendiente_bdp`) es diseño correcto; implementar junto con C1/C3 (el campo de estado y la decisión de cancelación).

### Call-sites de `bdp_sync_enabled` (dimensionar M1)
- 29 referencias en 13 archivos de src (`rg bdp_sync_enabled`). Puntos de decisión de comportamiento:
  - `bdp_sync.rs:95,1314,1653` (no-op si !enabled||!configurado)
  - `bdp_order_poller.rs:39,80` (guard + query)
  - `bdp_write_guard.rs:59` (auto-arm)
  - `services/venta.rs:217,294` (delete/retry)
  - `handlers/ventas.rs:302` (bdp-status refresh)
  - `handlers/configuracion.rs:203,479-537` (respuestas)
  - models/repositories/seed/bootstrap
- Recomendación: F1 debe incluir inventario de call-sites y test de matriz (auto/standalone/bdp × flags × sync_mode) para que M1 sea verificable.

## Cobertura de §1–§14 del plan (pasada final)

Verificaciones adicionales de afirmaciones de los capítulos 1–14 (todas confirmadas excepto las dos
notas marcadas):

| Afirmación del plan | Evidencia | Resultado |
| --- | --- | --- |
| §2: `bdp_sync.rs` no-op si `!bdp_sync_enabled || !bdp_configurado` (líneas 95, 1314, 1653) | `src/services/bdp_sync.rs:95,1314,1653` | ✅ exacto |
| §2: `venta::delete` 409 con BDP/Haddock | `services/venta.rs:205-229` | ✅ (Haddock fuera de alcance, M14) |
| §2: sin anulación local; reservas sí tienen `cancelada` | `20260325100000:79` CHECK estado reservas | ✅ |
| §2: `AuthUser` con `role`/`effective_role`/impersonación/`trabajador_id`; `require_role` | `middleware/auth.rs:20-29,74-76` | ✅ (base real de D8/M17) |
| §2: `venta_lineas.articulo_codigo TEXT` libre (sin FK) | `20260714200000_venta_lineas.up.sql:9` | ✅ |
| §2: `bdp_purchase_notes` serie/numero/fecha/proveedor/total/`datos_bdp` JSONB + UNIQUE(serie,numero) | `20260725170000_bdp_purchase_notes.up.sql` | ✅ (valida M18: series locales `L-…`) |
| §2: 6 flags BDP `false` por defecto | `20260724120000_bdp_feature_flags.up.sql` | ✅ (valida M12) |
| §5: `/api/bdp/diagnostics` (existe) | ⚠️ el endpoint real es `/api/configuracion/bdp/diagnostico` (`handlers/configuracion.rs:448`) + dry-run `:451` | **Imprecisión menor**: corregir nombre/ruta en el plan |
| §0/§4.3: `GetStock`/`GetListStock` no implementados | sin hits en `src/` (solo `cancel_order` en `bdp_weblink.rs:270`) | ✅ coincide; trabajo real N6 |
| §4.6/M4: venta local se crea primero; sync fallido → `sync_error` + reconcile | `20260607000000_bdp_sync_fields.up.sql:15` (`bdp_sync_error`) | ✅ |
| M13 vs §4.6: redacción «generalizar ledger `bdp_pagos` → `pagos_parciales`» | §15/A8 aclara «no se renombra el ledger» | ⚠️ redacción ambigua; fijar «extender, no renombrar» en §14 |

## Veredicto preliminar
**VIABLE CON RESERVAS** — el plan es sólido (arquitectura DIP/OCP, mitigaciones M1-M18, decisiones D1-D8, gate por fases), pero **no debe ejecutarse F1+ tal cual** hasta incorporar C1-C6 (estado de venta inexistente, cadena mesa, contradicción CancelOrder/arming, F2 duplicado, cascade, gate 0.7.x) y cerrar las definiciones transversales (histéresis en memoria, preflight ligero vs completo, D9 antes de F6).

## Checklist de correcciones al plan
- [ ] §4.2: corregir lista de columnas nuevas (solo `origen`, `local_dirty`; precio→precio_tarifa1)
- [ ] §5: corregir ruta/nombre del endpoint de diagnóstico (`/api/configuracion/bdp/diagnostico`)
- [ ] §14/M13: fijar redacción «extender el ledger `bdp_pagos` con origen, sin renombrar» (coherente con §4.6)
- [ ] §4.3/F3: decidir fuente de verdad de `stock_local` (columna en `bdp_article_map.stock_actual` vs `bdp_article_stock` por almacén) antes de implementar
- [ ] §4.7/F4: definir estado de `ventas` desde cero y enumerar agregados afectados (total_periodo, listados, filtros)
- [ ] §4.7/F4: definir cadena venta→reserva→mesa para M11, incluido el fallback `num_mesa` de reservas sin `mesa_id` (`services/plano_sala.rs:351-353`)
- [ ] §4.7/M8: elegir vía CancelOrder (con migración de scopes/arming) o estado sin llamada API; alinear §6 y U6
- [ ] §5/M2: definir almacenamiento de histéresis y comportamiento multi-proceso
- [ ] §3.1/F1: distinguir preflight ligero (TTL) vs dry-run completo (on-demand)
- [ ] F8: enumerar endpoints protegidos y tests 403
- [ ] F6: marcar D9 como decisión requerida antes de F6
- [ ] F0: verificación Sentinel (doctor readyForGate, identidad 0.7.x, submodule/lock alineados) antes de prometer task:check
- [ ] F4: documentar ON DELETE CASCADE de bdp_pagos al desbloquear delete

## Pendiente de verificación
- [x] Pase de supervisor_reviewer ejecutado → veredicto APROBADO CON RESERVAS; correcciones I1 (citas de línea), I2 (fuente de `stock_local` F3, fallback `num_mesa` M11) y M1 (U8/ListaClientes) incorporadas a este documento
- [ ] N10 / estado deploy 1e en producción: solo verificable tras autorización de acceso (F0 con coolify-manager-rs)
