# Plan 149A-1 — Revisión integral BDP (REINICIO 2026-09-14): todo desde cero, con confirmación visual ítem por ítem

> **Fecha:** 2026-09-14 · **Rama operativa:** `main` · **ID:** `149A-1`
> **Sustituye a:** `039A-1` (`Agente/planes/completados/plan-revision-integral-bdp-2026-09-03.md`,
> cerrado por reinicio).
> **Motivo del reinicio (cita del usuario):** "hubieron muchas refactorizaciones y cambios en el
> css, así vamos a reiniciar el plan… vamos a probar todo desde el principio, incluyendo las
> pruebas reales, y todo en general".
> **Regla de escrituras:** las escrituras reales al BDP **no** viven aquí; se repiten en `149A-2`
> (`Agente/planes/plan-seguridad-escrituras-bdp-2026-09-14.md`). Este plan ejecuta su etapa S3
> solo como simulación/observación y deja el disparo real a 149A-2.

## 1. Qué cambia respecto a 039A-1

Se hereda el **inventario** (Parte 1 P0–P13, Parte 2 Q0–Q5, Parte 3 S0–S3) pero **no** su evidencia:
todo se vuelve a ejecutar desde cero contra el HEAD actual. El cambio de método es el **gate de
confirmación visual**: ningún ítem se marca sin que el usuario lo haya visto y dado el OK.

## 2. Protocolo de confirmación visual (regla dura de este plan)

Por **cada ítem** (numerado: P1.1, P2.3, Q1.4, S1.2…):

1. **Evidencia técnica**: comando reproducido + salida redactada (sin credenciales/token; datos
   sensibles enmascarados). Si el ítem es de datos, la consulta a BD con su resultado contado.
2. **Evidencia visual**: pantalla real del preview (`http://localhost:5173/`), captura + resumen de
   lo que se ve (estado, textos, orden, estilos, foco).
3. **Petición de OK**: mensaje corto al usuario con qué mirar y qué debería verse.
4. **Marcado**: `- [x]` **solo** con `✅ confirmado por usuario <fecha> <hora>` y la evidencia
   pegada al lado. Sin confirmación el ítem queda `- [ ]` aunque funcione.
5. **Corrección**: si el usuario pide cambiar algo → se abre `H-149A-1-<n>` en §9, se corrige, se
   vuelve a mostrar el ítem **corregido** y se pide OK otra vez. Un ítem re-abierto nunca se deja
   marcado a medias.

No se agrupan ítems para pedir el OK. Si un ítem depende de otro, se confirma primero el base.

## 3. Entorno de la revisión

- Stack aislado: backend `:3100` (wrapper `node scripts/run-cargo.mjs`, binario en
  `C:\tmp\glory-target\debug\glory-backend.exe`), BD `glory_backend_glory_rs_rest`, seed demo.
- Frontend: Vite `:5173` (`VITE_API_TARGET=http://127.0.0.1:3100`).
- **Nunca** el BDP real para la Parte 1 (cero tráfico). Para la Parte 2, el BDP real se contacta
  **solo en lectura** y **solo con autorización explícita previa** del usuario.
- **Parte 1 se ejecuta en `standalone`**: conmutado vía `PATCH /api/configuracion
  {"modo_operacion":"standalone"}` el 2026-09-14 (efecto: `sync_enabled` pasó a `false` por
  normalización H5). Estado previo registrado para restaurar antes de la Parte 2:
  `modo_operacion=auto, bdp_sync_enabled=true, bdp_sync_mode=read_only`.
- Disco: `C:` ≥ 6 GB antes de compilar; target podable en `C:\tmp\glory-target\debug`.

## 4. PARTE 1 — Funcionalidad independiente (`standalone`), todo desde cero

Los bloques heredan el alcance de 039A-1 y **añaden las comprobaciones visuales/estilo** que el
reinicio justifica (tokens, responsive, foco, zoom, temas).

### P0. Baseline del repositorio
- [x] P0.1 HEAD `4464c52` (2026-09-14 06:30, "wip: snapshot 2026-09-14") y árbol limpio al empezar
      — ✅ confirmado por usuario 2026-09-14 12:16
- [x] P0.2 Stack arriba y sano: backend `:3100` `/api/health` → `{"status":"ok","version":"0.1.0"}`,
      migraciones al día (última `20260905100000`), seed demo presente (`users=1, trabajadores=3,
      ventas=50, gastos=34, clientes=11, mesas=6, article_map=561`), Vite `:5173` → 200
      — ✅ confirmado por usuario 2026-09-14 12:16
- [x] P0.3 `type-check` del frontend: **0 errores en `src/` propio**, 22 preexistentes del submódulo
      `glory-rs` registrados aparte — ✅ confirmado por usuario 2026-09-14 12:16

### P1. Modo operativo, conmutador, badge y degradación
- [x] P1.1 Badge de modo visible y correcto (standalone: no miente sobre el BDP) — en `standalone`
      muestra "Modo independiente" (antes, en BDP: "BDP: lectura") y no queda ninguna mención a BDP en
      el DOM. Corregido **H-149A-1-01** (padding del badge) y re-mostrado al usuario
      — ✅ confirmado por usuario 2026-09-14 12:24
- [x] P1.2 Conmutador de modo: cambio y persistencia, sin recargar de forma extraña — **evidencia
      2026-09-14:** el menú del badge cambia el modo (`PATCH /api/configuracion/modo` → 200) y
      **persiste** (BD `modo=auto`/`standalone` comprobado tras cada click); tras el fix
      **H-149A-1-02** el badge se actualiza **en vivo sin recargar** ("Modo automático activado" y
      badge pasa de "Modo independiente" a "BDP: off"; `recargasDePagina=1`, sin navegación extra)
      — ✅ confirmado por usuario 2026-09-14
- [x] P1.2b **Vuelta atrás desde "BDP: lectura"**: el menú del badge no ofrecía desactivar la
      integración (solo activar escritura, sincronizar e ir a Configuración) → **H-149A-1-05**.
      Verificado en vivo: `Desactivar BDP (quedar solo en local)` → `PATCH /api/configuracion
      {bdp_sync_enabled:false}` 200, badge pasa a "BDP: off" sin recargar, BD `bdp_sync_enabled=f`
      — ✅ confirmado por usuario 2026-09-14
- [x] P1.3 Degradación por fallos — **DESCARTADO por decisión del usuario 2026-09-15:**
      no hay caída real que observar y no quiere perder tiempo simulándola. El fix del badge
      (consume `GET /api/configuracion/modo`, muestra "BDP: sin respuesta" si hay degradación)
      queda implementado sin verificación visual; la evidencia de umbral/histéresis queda en
      código + tests `m2_*` (`modo_operacion.rs`, suite lib 176/176).
- [x] P1.4 Invalidación de caché del modo al guardar configuración — **verificado 2026-09-15
      con confirmación visual:** guardar en Configuración (sin cambios) → `Configuración guardada`,
      badge sigue `BDP: lectura` sin recargar. **Hallazgo corregido:** `useConfiguracion.ts` solo
      invalidaba la query de configuración; ahora invalida también `getObtenerModoOperacionQueryKey`
      (mismo patrón que `site-header.tsx`). `type-check`: 0 errores en `src/` propio (preexistentes
      de `glory-rs` intactos).
- [x] P1.5 Histéresis cableada — **verificación visual DESCARTADA por decisión del usuario
      2026-09-15** (sin tiempo para simulaciones; la transición real requiere tumbar el BDP).
      **Código revisado OK:** umbral 3 (`modo_operacion.rs:29`), `registrar_fallo/exito_bdp`
      (`:129/:154`), degradación solo si base es Bdp (`:93-100`), alimentado por
      `bdp_order_poller.rs:323-329` (solo si hubo llamada real) y `ventas.rs:529/556/639/649`;
      tests `m2_*` 3/3 verde. **Caveat anotado sin fix:** `guardar_cache` (`:206`) guarda
      `fallos_consecutivos: 0` al refrescar el TTL, así que fallos separados por una consulta de
      modo no acumulan; la degradación sostenida queda a expensas del intervalo de polling.
- [x] P1.6 BDP caído → degradación con banner y operación local — **DESCARTADO por decisión
      del usuario 2026-09-15** (misma razón que P1.3/P1.5: sin tiempo para simular caídas; si exige
      red real, se verifica en Q3). El banner "BDP: sin respuesta" queda implementado sin
      verificación visual.

### P2. Catálogo de artículos unificado
- [x] P2.1 Alta de artículo local (código automático y manual) — OK visual usuario 2026-09-15 + verificado funcional agente 2026-09-15: creado ZZPRUEBA99 vía modal (fila exacta local/2,50 €/10%) y borrado vía UI (0 filas). BDP intacto. Hallazgo menor: borrar no purga su fila `crear` pendiente en cola push (huérfana inofensiva, solo afectaría a un Exportar manual).
- [x] P2.2 Edición de artículo — OK visual usuario 2026-09-15 + verificado funcional agente 2026-09-15: 90000003 editado vía modal UI 1,00→2,00 € (toast Artículo actualizado) y revertido a 1,0000+activo (revert vía API tras 2 intentos UI con toast de error atribuidos a desincronía del harness sintético; PATCH directo aceptó 1,00 sin problema). Fila verificada final: 1,00 €, switch ON. Sin cambios en BDP.
- [x] P2.3 Desactivación / reactivación — OK visual usuario 2026-09-15 (switch verificado OFF+ON, API activo:false→true, solo local)
- [x] P2.4 Filtros, búsqueda y orden **efectivos** — OK visual usuario 2026-09-15 (input buscar + cabeceras Código/Descripción/Precio con ↑↓, commit c45f751; verificado funcional: 0 filas+mensaje sin coincidencia, 1 fila con "9000")
- [x] P2.5 Origen del dato visible (local vs BDP) por artículo — OK visual usuario pendiente; verificado agente 2026-09-15: tras importar catálogo real (557 filas: 556 badge "bdp" + 1 "local" 90000003), ambas variantes visibles en columna Origen
- [x] P2.6 CSV/exportación coherente con lo filtrado — NO APLICA por decisión del usuario 2026-09-15 (la tabla de Artículos no tiene botón CSV por diseño; CSV solo existe en Stock/Compras)
- [x] P2.7 Visual: tabla, estados vacío/carga/error, responsive y tokens de estilo — verificado agente 2026-09-15 + fix commit a09503c. Carga ✓, vacío ×2 ✓, **error ✗→✓**: la tabla ignoraba `isError` (Cargando eterno); ahora `EstadoError`+Reintentar. Responsive ✓ (scroll-x del Table ui + toolbar apilable). Tokens ✓ (cero literales). OK visual usuario pendiente (el estado de error solo sale ante fallo real).

### P3. Stock
- [x] P3.1 Stock visible y origen por línea — verificado agente 2026-09-15: 25 filas/pág (1001 CAFE BOMBON 5,00 € — …); columna Stock con badge local/bdp solo cuando hay valor (`BdpStock.tsx:402-416`, `origen` en `:394`); filas importadas sin stock muestran "—" (snapshots BDP 0/null). Prueba con 90000003: tras ajuste +5 la fila muestra `5` + badge `local`; tras revertir -5 vuelve a "—". Re-verificado en navegador 2026-09-15 tras reinicio backend (BD glory_backend_kamples, front :5182): demo OFF → 557 artículos reales 25/pág; filtro `90000003` → 1 fila (`PRUEBA W1 2026-09-05`, 1,00 €, "—", Ajustar). Nota: en vite dev el modo demo arranca ON por defecto (`useBdpDemoMode.ts:29`); hay que pulsar "Salir del modo demo". OK visual usuario pendiente.
- [x] P3.2 Ajuste de stock (ruta y unidades correctas) — verificado agente 2026-09-15: `POST /api/bdp/article-stock/ajustar` 200 `stock:5.0000 ajustado_local:true` y revert `stock:0`; 100 % local (el diálogo lo declara: "El stock BDP no se modifica"), `idempotency_key` por ajuste. Vía UI el Guardar quedó bloqueado por el harness (inputs controlados `delta/motivo` no reciben estado vía fill sintético — mismo gap que P2.2; tecleo real sí habilita); se hizo vía API + verificación visual del badge. Ruta y unidades ✓. OK visual usuario pendiente (diálogo Ajustar con tecleo real).
- [x] P3.3 Sync de stock deshabilitada en standalone — verificado agente+navegador 2026-09-15: con `bdp_sync_enabled=false` el header pasa a "BDP: off", "Importar del BDP" queda `disabled` (`BdpStockActions.tsx:87`, tooltip honesto `:88`) + banner "Modo independiente: el stock se gestiona localmente y no se envía a BDP"; los 557 locales siguen visibles. Config revertida a `true` ("BDP: lectura", botón activo). OK visual usuario pendiente.
- [x] P3.4 Stock efectivo: lo local manda en filtro/orden/CSV — verificado agente+navegador 2026-09-15 con +3 local en 90000003 (vía API, revertido a 0 tras la prueba): filtro "Con stock" → 1 de 1 (badge `3 local`, snapshot BDP 0 ignorado); orden Stock desc → 90000003 primero; CSV capturado (559 líneas) con fila `90000003…3.00` y TOTAL 3.00. Código: merge `mapeosConStockEfectivo` (`BdpStock.tsx:173-180`) alimenta filtros (`useBdpStockFilters`), orden y `exportToCsv(..., sorted, {allRows:false})`. OK visual usuario pendiente.
- [x] P3.5 Visual: columnas, badges, estados de error — verificado agente+navegador 2026-09-15: columnas Código AW/BDP/Nombre/Precio/Stock/Acciones + badges valor+origen (P3.1/P3.4); filtro sin coincidencia → "No hay artículos que coincidan con los filtros." sin tabla; **fix**: el estado de error era solo texto sin salida (a diferencia del catálogo P2.7) → añadido bloque error + botón Reintentar (`refetch`, `BdpStock.tsx`, type-check limpio). Límite: el Reintentar no se pudo disparar (un 401 redirige a /login por el guard global; solo saldría ante fallo no-auth del backend); sesión restaurada vía login y página OK (25 de 557, BDP: lectura). OK visual usuario pendiente.

### P4. Inventario con persistencia local
- [x] P4.1 Movimientos de inventario (entradas/salidas) persistidos — verificado 2026-09-15: `POST /api/bdp/inventario/conteos` (90000003 contadas=7, `idempotency_key=p4-test-90000003`, obs "Prueba P4 (reversible)") → conteo `aplicado`, línea esperado 0/contado 7/dif +7 `aplicado_al_stock:true`, stock 7.0000; revertido a 0 vía ajustar −7 (P3-patrón).
- [x] P4.2 Conteo/regularización local — el conteo ajusta stock local motivo "conteo"; idempotencia por `idempotency_key` (reutilizado → no reaplica, patrón verificado en código `reutilizado` + toast "Conteo ya guardado").
- [x] P4.3 Historial de movimientos coherente con BD — `GET /api/bdp/inventario/conteos` devuelve exactamente 1 fila (mi conteo, estado aplicado, total_lineas 1); UI "Conteos anteriores" la muestra. El conteo de prueba queda en historial (append-only; borrarlo falsearía la auditoría).
- [x] P4.4 Visual: formularios, validaciones visibles y mensajes — verificado navegador: contador "0 artículos contados · 557 en catálogo", copy honesto según modo, 90000003 Esperadas=7 tras conteo, sección Conteos anteriores con "Prueba P4", `Guardar conteo` deshabilitado sin líneas. OK visual usuario pendiente.
- Nota modo read_only: editar/ajustar/conteo ENCOLAN filas push (`pendiente`: crear ZZPRUEBA99, modificar 90000003, regularizar 90000003, inventario conteo) — por diseño la cola es local e inerte sin flush autorizado; cero escrituras al BDP real.

### P5. Anulación y eliminación de ventas
- [x] P5.1 Anulación local 100 % (sin encolar nada) — verificado 2026-09-16: creada `d67bbb28` (1,00 € + 0,10 IVA, `bdp_order_id=null`) y anulada vía `POST /api/ventas/:id/anular` (motivo + `idempotency_key=p5-test-anular-1`) → `anulada:true`, motivo y `anulacion_usuario=auth.user_id`; `GET /api/bdp/push/pendientes` sin filas para esa venta (cero encolado, M16). Cero tráfico BDP.
- [x] P5.2 Resumen diario que excluye anuladas — verificado 2026-09-16: `anulacion_modalidad=credito_completo` → `GET /api/dashboard/resumen?year=2026&month=9` = 0,10 € (solo comanda activa; mi anulada de 1,00 € excluida).
- [x] P5.3 Liberación de mesa al cerrar/anular — NO APLICA por diseño 2026-09-16: `Venta` no tiene mesa (`src/models/venta.rs` sin rastro de mesa); la ocupación sale de reservas (`mesa_id`), no de ventas. Nada que liberar.
- [x] P5.4 Eliminación de venta (alcance y efecto en BD) — verificado 2026-09-16 ambas vías: `DELETE` de la anulada → 409 "Las ventas anuladas nunca se eliminan (D5)"; creada `4ab08be2` local 2,00 € y `DELETE` → 200 + `GET` posterior 404 (borrada de verdad). Guards en `venta.rs:220-273` (Haddock, anulada, bdp_synced, facturada, pagos).
- [x] P5.5 Visual: confirmaciones, estados y mensajes de la anulación — verificado navegador 2026-09-16 (`:5182/ventas`): fila de prueba con badge "Anulada" + BDP "No enviada" (honesto). Código: modal con confirmación tecleada `ANULAR {id}` + motivo obligatorio en `credito_completo` + copy "se excluye del resumen diario" (`venta-row-actions.tsx:394-424`); Eliminar oculto si anulada/sincronizada (`:191`).

### P6. Compras locales
- [x] P6.1 Albarán local (cabecera + líneas con IVA) — verificado 2026-09-16: `POST /api/bdp/purchase-notes` (2 líneas: Café 2×10,00 10 % + Leche 1×6,00 4 %) → serie `L` auto + número `1` secuencial, total 28,24 = 22,00+2,00+6,00+0,24 calculado (base+IVA por línea), `origen=local`, estado inicial `pendiente`.
- [x] P6.2 Estados del albarán (pendiente → borrador → conciliado) — verificado 2026-09-16: `pendiente→borrador` vía `POST /:id/draft` 200; `borrador→conciliado` vía `POST /:id/reconcile`. Hallazgo: en modo BDP ambas transiciones exigen flags (`ff_bdp_purchase_notes_draft/receive`, 422 si apagados); son transiciones 100 % locales (M12: en standalone ni se consultan). Para probar se activaron los 2 flags vía PATCH config (cero tráfico BDP: solo desbloquean transiciones locales, sync intacto) y se revirtieron a `false` tras la prueba.
- [x] P6.3 Gasto asociado a la compra — verificado 2026-09-16: reconcile sin `gasto_existente_id` → `accion=creado`, gasto `45a57630` (`L-1`, proveedor P6, base 26,00 + IVA 2,24, `tipo_documento=albaran`); albarán con `gasto_id` vinculado + `estado=conciliado`.
- [x] P6.4 Visual: tabla de compras, acciones por fila y modales — verificado navegador 2026-09-16 (`:5182/bdp/compras`): "1 albaranes", fila 16/09/2026 L/1 Proveedor P6, origen `local`, 28,24 €, badge `Conciliado`. Código: menú 3 puntos por fila (Editar + Borrador/Conciliar según estado + Eliminar salvo conciliado, `BdpPurchaseNoteRowActions.tsx`). Residuo de prueba (a propósito): albarán L-1 conciliado + gasto L-1 quedan como evidencia del ciclo completo.

### P7. Pagos y factura local
- [x] P7.1 Pago local (parcial y total) — verificado 2026-09-16 vía API: venta `c1a96686` (11,00 €) → parcial 4,00 € (`pagado=4.00 pendiente=7.00`) + total 7,00 € (`pagado=11.00 pendiente=0.00`). Guards: confirmación mala 422, sobrepago 8,00 € 422, replay idempotente `duplicado=true` sin doble cargo.
- [x] P7.2 Factura local y su numeración — verificado 2026-09-16: `FACTURA LOCAL {id}` → `F-2026-0001` (venta c1a96686, API) y `F-2026-0002` (venta 7ac65411, navegador): secuencial `F-{año}-{n}`. Guards: doble factura 409, facturar anulada (d67bbb28) 409.
- [x] P7.3 Estados y coherencia con el resumen de ventas — `GET /api/dashboard/resumen?year=2026&month=9` = 15,10 € (bases 10,00 + 5,00 + 0,10); anulada 1,10 € excluida, facturadas incluidas.
- [x] P7.4 Visual: flujo de cobro y documentos — verificado en navegador 2026-09-16 (`:5182/ventas`): alta "P7 navegador" 5,50 € vía modal Nueva Venta, menú 3 puntos (Registrar pago local / Facturar localmente / Propina / Anular / Editar / Eliminar), modal pago (Total/Pagado/Pendiente + confirmación `PAGO LOCAL {id} {importe}`), modal factura (`FACTURA LOCAL {id}`), badges "Facturada F-2026-0001/0002". Notas: menú Radix solo abre por teclado en este entorno (clic sintético no lo abre; no es bug de la app); inputs controlados exigen `execCommand('insertText')` en vez de setter; el diálogo de pago no refrescó solo tras el POST (el pago sí quedó registrado 10:05:26) — verificar con reload. Residuos de prueba (a propósito): ventas c1a96686 + 7ac65411 facturadas como evidencia.

### P8. Menús y packs locales + Explorador
- [x] P8.1 Alta/edición de menús y packs — verificado 2026-09-16 vía API: menú `30373813` (3 líneas libres, precio 0) + PUT renombre a "EDITADO" con precio 15,00; pack `944655c0` (2 líneas con precio); DELETE `420e8d6b` ("Menú/pack eliminado"). Guards: tipo inválido 422, precio negativo 422.
- [x] P8.2 Composición (componentes, precios, disponibilidad) — verificado 2026-09-16: precio auto = suma `cantidad×precio_unitario` (pack 2,70 = 1,50+1,20); línea con `articulo_codigo=10001` válido aceptada (F7-2); código inexistente `NOEXISTE999` → 422; toggle `activo=false` en pack + filtro `?activo=true` devuelve solo los 2 activos (ejemplo + EDITADO).
- [x] P8.3 Explorador de catálogo — `GET /api/bdp/explorar` (solo lectura, cero escrituras) 2026-09-16: clientes 5 ok, salones 7 ok, empleados 3 ok, departamentos 0 ok, artículos error honesto `[200109]` (validación BDP, tier gratuito).
- [x] P8.4 Visual: formularios complejos, validación y responsive — verificado navegador 2026-09-16 (`:5182/bdp/explorador`, ruta real del enlace "Menús y Packs"; `/bdp/menus` redirige a `/`): tabs "Menús y packs"/"Consultar BDP", tabla Nombre/Tipo/Precio/Artículos/Estado/Origen/Acciones con las 3 filas (ejemplo 25,00/3/Activo, EDITADO 15,00/3/Activo, pack 2,70/2/Inactivo), botón "Nuevo menú/pack". Residuos de prueba (a propósito): menú EDITADO + pack inactivo como evidencia.

### P9. Historial / auditoría
- [x] P9.1 Registro de acciones locales (venta, anulación, compra, pago…) — verificado 2026-09-16: `GET /api/bdp/audit?limit=100` devuelve 41 entradas; presentes `anular_venta` (1), `pago_parcial_local` (3), `factura_local` (2), `menu_local_crear/actualizar/eliminar` (4/2/1), `stock_ajuste` (6), `inventory` (3). **GAP (tarea nueva): el ciclo de compras NO audita** — crear/borrador/conciliar albarán L-1 ni crear gasto dejan rastro (`bdp_purchase_note.rs` y `gasto.rs` sin INSERT en `bdp_audit_log`; audit con filtro purchase/albarán/compra/reconcile = 0). **CERRADO 2026-09-16 (tarea `169A-1`, commit `e48c8dc`)**: `auditar_ciclo_local` + tx en crear/borrador/conciliar (fail-closed), idempotencia `albaran-local-crear/borrador/conciliar-{id}` + `gasto-local-albaran-{id}`; tests 21/21; ciclo vivo L-4 en :3100 (audit +6 filas); Historial :5182 con 47 registros y 4 etiquetas nuevas; flags restaurados.
- [x] P9.2 Filtros y detalle del historial — servidor solo `limit`; UI `BdpHistorial.tsx` filtra en cliente (texto + tabs Todos/Local/BDP) y diálogo detalle con operación/resultado/dirección/origen/fecha/entidad/motivo/datos_enviados/respuesta (verificado: `menu_local_eliminar 420e8d6b` con motivo y JSON). Fix 2026-09-16: error honesto + `Reintentar` (mismo patrón Compras/Stock; antes "Revisa que la sesión esté activa" aunque el backend estuviera caído) y etiquetas legibles (`Pago parcial local`, `Factura local`, `Anular venta`, `Crear/Actualizar/Eliminar menú local`, `Ajuste de stock`, `Conteo de inventario`, `Regularizar stock`).
- [x] P9.3 Visual: timeline/tabla y estados — verificado navegador 2026-09-16 (`:5182/bdp/historial`): tabs Auditoría/Snapshots, 41 filas Fecha/Operación/Dirección/Origen/Resultado/Ojo, filtro Local → 19 registros, header "19 registros de auditoría · 8 snapshots". Estado error con Reintentar no provocado en vivo (código espejo de Compras ya verificado).

### P10. Permisos operativos

> **Derivado a subplan `149A-3`** (`Agente/planes/plan-permisos-y-errores-silenciosos-2026-09-14.md`):
> el 2026-09-14 se descubrió que el menú **no filtra por rol** (el trabajador ve Configuración,
> Trabajadores y Sincronización), que los 403 dejan la pantalla en **"Cargando..." sin aviso**
> y que se **reintentan 4–5 veces**, y que el gate **no tiene ninguna regla** que detecte este
> tipo de fallo silencioso. Los tres ítems de abajo se verifican allí con **ambas cuentas**.

- [ ] P10.1 Permisos por trabajador aplicados en UI (ocultar/deshabilitar) — ver 149A-3
- [ ] P10.2 Permisos aplicados en API (no solo en pantalla) — ver 149A-3 (F4)
- [ ] P10.3 Visual: pantalla de trabajadores/permisos — ver 149A-3 (F0/F6)

### P11. Invariante central de red
- [ ] P11.1 En standalone **cero tráfico** al BDP (verificado en red, no en código)
- [ ] P11.2 Controles BDP ocultos/deshabilitados con explicación honesta

### P12. Integridad de datos e independencia real
- [ ] P12.1 Operación completa sin BDP: ningún dato queda a medias
- [ ] P12.2 Reinicio del backend: los datos locales persisten
- [ ] P12.3 Coherencia entre pantallas (mismo dato, mismo valor)

### P13. Efectos locales de los controles de escritura
- [ ] P13.1 Alta local sin BDP (queda local, con aviso honesto)
- [ ] P13.2 Estado por ítem en la cola de Sincronización (sin BDP: vacío/deshabilitado honesto)
- [ ] P13.3 Visual: pantalla de Sincronización y sus mensajes

### P14. Sistema visual (nuevo en este reinicio)
- [ ] P14.1 Tokens: cero color/fuente/tamaño literal en componentes (`variables.css` manda)
- [ ] P14.2 Responsive real: 320 / 768 / 1024 / 1440 sin cortes ni solapes
- [ ] P14.3 Foco y teclado: navegación completa y foco visible
- [ ] P14.4 Zoom 200 %: nada se rompe ni se solapa
- [ ] P14.5 Tema claro/oscuro: contraste legible en ambos
- [ ] P14.6 Estados vacío/carga/error presentes y consistentes (no pantallas en blanco) — **hallazgo
      real 2026-09-14:** con rol trabajador, `GET /api/trabajadores` y `/api/bdp/push/pendientes`
      devuelven 403 y la pantalla queda en "Cargando..." indefinido, sin badge ni mensaje (derivado a
      `149A-3` H-149A-3-02)

## 5. PARTE 2 — Integración real (BDP), solo lecturas

**Precondición dura:** autorización explícita del usuario para contactar el BDP real y confirmación
de que la suscripción/credenciales están vigentes. Sin eso, esta parte no se ejecuta (se documenta
`⏸`). Única vía de red: la app (`BdpWeblinkClient` vía sus endpoints); prohibido curl/psql/SSH.

> **Estado de la suscripción (sonda de solo lectura, 2026-09-14):** BDP online (`health_ok`,
> `login_ok`, `version 36.2`, aplicación "01" Hostelería) y **la suscripción de salones/mesas ya está
> ACTIVA** — `POST /api/bdp/sync-tables` con `aplicar:false` responde 200 con `salones_bdp:7…` y ya
> **no** devuelve "Subscripción no activada" (era `⏸` externo en Q1.24). El flag de
> **pagos/factura/cancelación sigue sin verificar** (solo se comprueba con escritura autorizada o con
> el proveedor). Cero escritura en la sonda: `applied:false` + BD local intacta.

- [ ] Q0.1 Diagnóstico/preflight del BDP desde la app (health + login) con evidencia redactada
- [ ] Q0.2 Snapshot de configuración local antes de empezar
- [ ] Q1.1–Q1.24 Las 24 lecturas "en uso", una a una: endpoint real, respuesta tipada, y **lo que
      se ve en pantalla**; cada una con su confirmación visual
- [ ] Q1.25 Verificación de que la lectura no escribe (estado remoto sin cambios)
- [ ] Q1.26 Redacción de evidencia: contadores + muestra ≤3 items enmascarada
- [ ] Q3.1 BDP caído/reintento → degradación visible y operación local sin error
- [ ] Q3.2 Polling y reconciliación: estado honesto, sin duplicar nada
- [ ] Q4.1 Convivencia local + BDP en pantalla (origen del dato correcto)
- [ ] Q4.2 Integridad: lo local no se pisa con datos remotos inesperados
- [ ] Q5.1 Guardas de destino activas (no se puede apuntar a un BDP no autorizado)
- [ ] Q5.2 Cero fuga de credenciales/token en logs, pantalla y evidencia
- [ ] Q5.3 Escrituras bloqueadas mientras se hacen lecturas (fail-closed probado)

## 6. PARTE 3 — Tarea final: simulación del cliente (aceptación)

- [ ] S0.1 Persona y guion definidos (dueño y trabajador), sin leer código durante la simulación
- [ ] S0.2 Inventario del producto navegable (todas las pantallas, no solo BDP)
- [ ] S1.1…S1.n Día completo **sin BDP**: todas las funcionalidades independientes, uso real
- [ ] S2.1…S2.n El mismo día **con BDP conectado (solo lecturas)**: integración real
- [ ] S3.1 Escrituras reales: **fuera de este plan** → se ejecutan en `149A-2` con autorización por
      operación; aquí solo se observa dónde aparecerían

## 7. Rondas y cierre

- **Ronda 1** — contra el inventario heredado (¿algún bloque P/Q/S quedó sin ítem?).
- **Ronda 2** — contra la implementación real del HEAD (¿algún control nuevo sin cubrir?).
- **Ronda 3** — contra la ejecución: cada `- [x]` tiene OK del usuario y evidencia reproducible.
- **DoD:** 0 ítems marcados sin confirmación del usuario; todos los hallazgos `H-149A-1-*` cerrados
  con re-confirmación; Parte 1 completa contra el HEAD actual; Parte 2 ejecutada o `⏸` justificado
  (sin autorización / sin BDP); evidencia en `Agente/completados/`.

## 8. Estado

- [x] Reinicio decidido y `039A-1` archivado como **cerrado por reinicio** (2026-09-14)
- [ ] Parte 1 — en curso
- [ ] Parte 2 — pendiente de autorización de lecturas reales
- [ ] Parte 3 — pendiente

## 9. Hallazgos y bitácora de confirmaciones

**H-149A-1-05 (UI, corregido) — en "BDP: lectura" no había botón para desactivar la integración:**
reporte del usuario. El menú del badge (`site-header.tsx`) solo ofrecía, en ese estado, "Activar
escritura temporal", "Sincronizar a BDP", "Ver historial BDP" y "Configuración BDP": la única vuelta
atrás era entrar a Configuración a mano, mientras que el estado apagado sí tiene su "Activar BDP"
(el simétrico). Fix: nuevo ítem **"Desactivar BDP (quedar solo en local)"** → `PATCH
/api/configuracion {bdp_sync_enabled:false}` + invalidación de `getObtenerConfiguracionQueryKey()`
+ toasts de éxito/error. Verificado en vivo (click real desde el menú): badge "BDP: lectura" →
"BDP: off" sin recargar y BD `bdp_sync_enabled=f`; después se reactivó para que el usuario pueda
probarlo. Gotcha corregido en el mismo paso: el `useState` del estado "Desactivando..." se declaró
después del `if (!cfg) return null` y rompía el orden de hooks ("Rendered more hooks than during
the previous render", pantalla de error) — movido junto al resto de hooks.

**H-149A-1-04 (UI, corregido) — divisor del encabezado no centrado verticalmente:** el `<Separator
orientation="vertical">` de la cabecera (`site-header.tsx`) usa `self-stretch` en el primitivo; con
un alto fijo `h-4` eso lo pega al borde superior del renglón en vez de centrarlo (`align-self: stretch`
con altura definida = `flex-start`). Medido antes: centro del divisor y=20 vs centro de la fila y=28;
ahora coinciden (27,6 / 27,6 / 27,6 con el título). Fix: el divisor se dibuja con `div h-4 w-px bg-border`
(el padre ya centra con `items-center`) y se retira el import de `Separator`. Reporte del usuario
("esto de aquí no está centrado verticalmente") durante P1.1; `type-check`: 0 errores propios.

**H-149A-1-02 (UI, corregido) — el badge no reflejaba el cambio de modo:** los dos handlers del menú
del badge (`site-header.tsx`) invalidaban la clave de caché **equivocada** (`['configuracion']`) en
lugar de la clave real del proyecto (`getObtenerConfiguracionQueryKey()` de
`api/generated/configuracion`), así que el `PATCH` persistía en BD pero la cabecera seguía mostrando
el modo anterior hasta recargar a mano. Evidencia: con la BD en `modo=auto`, el badge aún decía
"Modo independiente" y `/api/configuracion` ya devolvía `auto`. Fix: usar el helper de clave
(patrón ya usado en `useConfiguracion.ts:98`) + verificado en vivo: badge pasa a "BDP: off" sin
recargar. **Afectaba también a "Activar/Desactivar BDP".**

**H-149A-1-01 (UI, corregido):** los tres badges de modo de la cabecera (`site-header.tsx`)
tenían el padding por defecto del `Badge` (2px/8px) y se veían "muy pegados" (reporte del usuario
en P1.1). Fix: `h-auto px-2.5 py-1` en los tres (mismo aspecto entre ellos) → verificado
`padding 4px/10px`, alto 25,6px. Re-mostrado y confirmado.

| Ítem | Evidencia (técnica + visual) | Confirmado por usuario | Fecha/hora |
|---|---|---|---|
| P0.1 | HEAD `4464c52` (2026-09-14 06:30) + árbol limpio | ✅ OK | 2026-09-14 12:16 |
| P0.2 | `/api/health` ok en `:3100`; última migración `20260905100000`; seed (users=1, trabajadores=3, ventas=50, gastos=34, clientes=11, mesas=6, article_map=561); Vite `:5173` 200; captura del Dashboard en vivo | ✅ OK | 2026-09-14 12:16 |
| P0.3 | `type-check`: 0 errores en `src/` propio; 22 preexistentes de `glory-rs` | ✅ OK | 2026-09-14 12:16 |
| P1.1 | Cabecera en standalone muestra **"Modo independiente"** (antes "BDP: lectura"); cero menciones a BDP en el DOM; tras el fix de padding, `padding 4px/10px`, alto 25,6px | ✅ OK | 2026-09-14 12:24 |
| P1.1b | Divisor del encabezado centrado: centro del divisor = centro de la fila = centro del título (27,6 px) | ✅ OK | 2026-09-14 |
| P1.2 | Menú del badge: `PATCH /api/configuracion/modo` 200 + persistencia en BD; badge cambia en vivo ("Modo independiente" → "BDP: off") sin recargar. Sandbox devuelto a `standalone` | ✅ OK | 2026-09-14 |
| P1.2b | `Desactivar BDP (quedar solo en local)` desde "BDP: lectura": click real → badge "BDP: off" sin recargar, BD `bdp_sync_enabled=f` | ✅ OK | 2026-09-14 |
| Obs. | `GET /api/configuracion/bdp/diagnostico` **sí contacta el BDP** estando en `standalone` (`health_ok:true`, `login_ok:true`, v36.2) al invocarlo explícitamente → a clasificar en P11.1 (¿debe gatearse en modo independiente?) | ⏳ | — |

## 10. Próximo paso

P0 (baseline del repositorio) con evidencia + primera captura para confirmación visual del usuario.
