# Plan — Auditoría integral de la independencia BDP (revisión 1×1, sin parches)

> **Fecha:** 2026-08-27
> **Rama:** `glory-rs-rest`
> **ID de bloque:** `208A-1` (auditoría) — provisional hasta confirmar en roadmap
> **Motivo (cita del usuario):** "esto no puede ser así, no podemos ir simplemente parcheando
> cosas… necesitamos un plan completo que revise el plan de independencia completo y evitar hacer
> un desastre de nuevo… lo primero que quiero es un plan de revisión con check y que vayas
> revisando una por una cada cosa y vayas anotando lo que encuentres".
> **Regla de oro:** **NO se implementa nada** durante la auditoría. Solo se verifica, se anota el
> hallazgo y se clasifica. El plan de corrección sale DESPUÉS, con los hallazgos en la mano.

---

## 1. Por qué esta auditoría

El plan de independencia (128A-1) se cerró en 2026-08-13 con gate PASS y el de escrituras
(198A-1) el 2026-08-19. Sin embargo, al revisar la interfaz el usuario detectó que la
experiencia no es la de un producto "100% operacional con o sin BDP":

- **Stock**: no se puede crear un artículo desde la página Stock; "Ajustar" solo modifica
  artículos que ya existen. Sin catálogo (BDP off), la página está vacía y no hay nada que hacer.
- **Inventario**: el conteo es solo estado de la UI; no se guarda localmente y en standalone
  "Enviar" no envía nada → la pantalla es inútil sin BDP.
- **Catálogo**: el CRUD de artículos vive en **Configuración → BDP → Correspondencias**, no en
  la página "Catálogo" (que solo tiene departamentos/familias). Dos "catálogos" distintos y
  confusos; Configuración mezcla conexión con CRUD de negocio.
- **Compras**: el usuario sospecha que tampoco cumple ("siento que esto es un desastre").

La conclusión honesta: el **código** de independencia existe y está testeado, pero la **UX y la
ubicación de las funciones** no lo hacen evidente ni completo. Esta auditoría recorre 1×1 cada
dominio del plan de independencia + la integración de escritura, verifica el estado real y anota
hallazgos con evidencia. Con el inventario completo se decide el plan de corrección.

**Ámbitos revisados:** plan 128A-1 (F1–F8) + 198A-1 (escrituras) + UX estructural + integridad
de datos. **Fuera de alcance:** BDP real (suscripción/datos del cliente), deploy, 138A-2
(lecturas reales), Sentinel/coolify (alcance separado).

## 2. Método de verificación (cómo se audita cada ítem)

Cada ítem del checklist se verifica con **al menos una** de estas vías (se anota cuál):

| Vía | Descripción |
| --- | --- |
| **C** | Lectura de código (backend Rust / frontend TS) — contrato, guards, wiring |
| **A** | Llamada API real contra el stack local (`:3100`, BD de rama, seed demo) |
| **U** | Recorrido en el navegador (preview `:5180`) con rol admin y trabajador |
| **T** | Tests existentes (`cargo test --lib bdp*`, integración, type-check) |

Resultado por ítem: `✅ OK` / `⚠️ Parcial` / `❌ Fallo` / `⏸ Diferido (BDP real)` + hallazgo.

## 3. Checklist de auditoría (1×1)

> Se marca la casilla cuando el ítem está **verificado**, y se rellena la tabla de hallazgos (§5)
> con la evidencia. Los hallazgos usan IDs `H1…Hn` y se clasifican por severidad:
> **Crítico** (rompe la promesa de independencia), **Alto** (funcional pero UX rota o confusa),
> **Medio** (mejora necesaria), **Bajo** (pulido), **Deuda** (ya declarada/diferida por diseño).

### R0. Baseline del repositorio
- [ ] R0.1 `cargo check --lib --tests` limpio (exit 0)
- [ ] R0.2 Suite unit: `cargo test --lib` en verde (153+ esperados)
- [ ] R0.3 `npm run type-check` del frontend limpio
- [ ] R0.4 Estado git: cambios pendientes son los esperados (sin commitear, sin mezclar frentes)

### R1. Modo operativo / conmutador / badge / degradación (F1, M1–M3)
- [ ] R1.1 `modo_operacion` es el switch maestro; `bdp_sync_enabled` solo se interpreta en modo bdp (M1)
- [ ] R1.2 Guard de coherencia al guardar configuración (standalone con sync=true se normaliza)
- [ ] R1.3 Histéresis implementada y cableada (N=3 fallos → degradar; N=3 éxitos → subir) (M2)
- [ ] R1.4 Invalidación de caché del modo al guardar configuración (M3)
- [ ] R1.5 Sin credenciales → standalone, app 100% operativa, badge "BDP: off"/independiente
- [ ] R1.6 BDP caído → degradación con banner, sin errores en operaciones locales
- [ ] R1.7 Preflight ligero en auto (no dry-run completo) (decisión F1)

### R2. Catálogo de artículos (F2, D3, M5–M7) + ubicación
- [ ] R2.1 CRUD de artículos local sin BDP (alta, edición, desactivar, precio/IVA/familia/barcode)
- [ ] R2.2 **Ubicación**: el CRUD de artículos está en la página "Catálogo" del menú, NO en Configuración
- [ ] R2.3 Configuración solo tiene configuración (conexión, mapeos, permisos) — sin CRUD de negocio
- [ ] R2.4 Alta local asigna código del rango reservado (D3) y funciona sin BDP
- [ ] R2.5 Import BDP no pisa ediciones locales (`local_dirty`) (M6) y no reactiva desactivados (M7)
- [ ] R2.6 `resolve_article` resuelve desde el catálogo local antes del fallback (M5)
- [ ] R2.7 Con BDP on: el alta/edición encola `CreateArticlesAndUpdateProfiles` (198A-1)
- [ ] R2.8 Origen visible (local/bdp) en filas mixtas

### R3. Stock (F3, D7)
- [ ] R3.1 Ajustar stock local por almacén con motivo y auditoría, sin BDP
- [ ] R3.2 **Crear artículo desde la propia página Stock** ("Nuevo artículo") en standalone
- [ ] R3.3 Origen del valor de stock visible (local/bdp)
- [ ] R3.4 Con BDP: `GetStock`/`GetListStock` sin pisar `stock_local` (N6)
- [ ] R3.5 Con BDP on: el ajuste encola el push (198A-1)
- [ ] R3.6 Empty state accionable (no "sincroniza desde BDP" como única salida en standalone)

### R4. Inventario (198A-1, D6=A)
- [ ] R4.1 Conteo esperadas vs contadas con diferencias
- [ ] R4.2 **Persistencia local del conteo** (la pantalla no es inútil en standalone; recontable y auditable)
- [ ] R4.3 Envío a BDP solo en modo bdp (y omitidos sin código BDP reportados)
- [ ] R4.4 La diferencia contada se refleja en el stock local (o se decide explícitamente que no)
- [ ] R4.5 Estado del envío visible (encolado / enviado / error)

### R5. Anulación + eliminación (F4, D4/D5, M8–M11, F6 CancelOrder)
- [ ] R5.1 Anular venta local según modalidad configurada (`credito_completo`/`estado_solo`)
- [ ] R5.2 Confirmación dinámica + motivo obligatorio + auditoría (`anular_venta` local en Historial)
- [ ] R5.3 Venta facturada no anulable (M9); anuladas nunca se borran (D5); delete desbloqueado solo no-sincronizadas (D5)
- [ ] R5.4 Resumen diario excluye anuladas (M10)
- [ ] R5.5 Liberación de mesa solo si la venta es la ocupante actual (M11)
- [ ] R5.6 Con BDP on y `bdp_order_id`: anulación encola `cancel_order` y queda "pendiente BDP" sin fingir éxito (M8/C3=b, F6)
- [ ] R5.7 Poller excluye anuladas-pendientes-BDP

### R6. Compras (F5, M18, A10)
- [ ] R6.1 Crear albarán local (serie `L-`, proveedor, fecha, líneas con IVA) sin BDP
- [ ] R6.2 Editar/eliminar albarán local (solo pendiente/borrador)
- [ ] R6.3 Conciliar albarán local con gasto sin BDP (IVA por línea)
- [ ] R6.4 Origen visible y convivencia con albaranes importados BDP (M18: sin colisión de serie)
- [ ] R6.5 Flags `ff_bdp_purchase_notes_*` solo gatean en modo bdp (M12)

### R7. Pagos y factura local (F6, A6–A8)
- [ ] R7.1 Pago completo local (venta con `metodo_pago`) sin BDP
- [ ] R7.2 Pago parcial local: `POST /ventas/:id/pagos-locales` con saldo pendiente e idempotencia
- [ ] R7.3 Factura local mínima: numeración `F-{año}-{n:04}`, estado, auditoría, guards (no anulada, sin doble facturación)
- [ ] R7.4 Con BDP on: botones BDP (pagar/facturar en BDP) disponibles y no pisan los locales

### R8. Menús y packs (F7, D2)
- [ ] R8.1 CRUD local de menús/packs sobre catálogo local sin BDP
- [ ] R8.2 Líneas con artículos del catálogo y precio recalculado
- [ ] R8.3 Convivencia con el Explorador BDP y origen visible

### R9. Historial / auditoría (F6, A11)
- [ ] R9.1 Operaciones locales (anulación, ajuste stock, CRUD catálogo, pagos parciales, factura local) visibles con `origen_operacion='local'`
- [ ] R9.2 Snapshots de configuración visibles sin BDP
- [ ] R9.3 Filtros y badge de origen

### R10. Permisos (F8, M17)
- [ ] R10.1 4 permisos configurables (`catalogo_edicion`, `stock_ajuste`, `albaranes_gestion`, `anulacion_ventas`) con enforcement backend (403 real)
- [ ] R10.2 Default `admin`: trabajador recibe 403; al ampliar a `todos` puede operar
- [ ] R10.3 UI de permisos en Configuración refleja y cambia el acceso
- [ ] R10.4 El alta de artículo nuevo (si se añade en Stock) queda cubierto por `catalogo_edicion`

### R11. Escrituras 198A-1 (cola de push + flush)
- [ ] R11.1 Encolado en handlers locales: artículo (D3), departamento/familia (D7), propina (D8), puntos (D9), inventario (D6), cancel_order (F6)
- [ ] R11.2 Worker de flush respeta guards (arming, backup, auditoría) y es no-op en standalone
- [ ] R11.3 Flush manual "Sincronizar a BDP" disponible solo en modo bdp; reintento tras bloqueo por suscripción **solo manual** (D2)
- [ ] R11.4 Estado de la cola visible (filas pendientes/sincronizadas/error)
- [ ] R11.5 Test de invariante: standalone → no encola ni envía (existente)

### R12. Navegación y UX estructural
- [ ] R12.1 No existe sección "Integración BDP" en el menú; opciones integradas con nombres normales
- [ ] R12.2 Orden del menú lógico (Compras junto a Gastos; Catálogo→Menús y Packs; Historial al final)
- [ ] R12.3 Títulos de página sin el prefijo "BDP" (header)
- [ ] R12.4 Cada pantalla tiene: estado vacío con explicación, carga, error, y aviso de modo (U1–U8)
- [ ] R12.5 Botón de 3 puntos en Ventas con menú contextual (fix previo) y sin regresiones
- [ ] R12.6 Autocompletar no se abre al abrir modales en edición (fix previo) sin regresiones
- [ ] R12.7 "Modo demo" ≠ "Modo independiente" (M16): textos distintos, sin confusión

### R13. Independencia real e integridad de datos
- [ ] R13.1 En standalone: cero llamadas de red a BDP (logs de red de la pasada completa)
- [ ] R13.2 Ningún botón BDP se ofrece en standalone (o aparece deshabilitado con motivo)
- [ ] R13.3 Migraciones aditivas con defaults (M15) — sin borrar/renombrar columnas
- [ ] R13.4 Sin colisiones: serie local `L-` vs series BDP (M18); rango reservado de códigos (198A-1 M11)
- [ ] R13.5 `venta::delete` considera también Haddock (M14) — documentado

### R14. Con BDP conectado (solo verificable con simulador o BDP real)
- [ ] R14.1 Import de catálogo convive con locales (origen visible)
- [ ] R14.2 Pago/factura BDP (con suscripción) — **⏸ diferido: suscripción WebLink real**
- [ ] R14.3 CancelOrder real — **⏸ diferido: suscripción WebLink real**
- [ ] R14.4 Lecturas reales (24 funciones) — **⏸ diferido: BDP online (bloque 138A-2)**

## 4. Resultado por área (auditoría 2026-08-27)

> Vías usadas: **C** = código, **A** = API, **U** = UI (preview `:5180`, stack local `:3100`,
> BD de rama, seed demo, standalone), **T** = tests. Los hallazgos H1–H8 se detallan en §5.

| Área | Resultado | Evidencia | Hallazgos |
| --- | --- | --- | --- |
| **R0 Baseline** | ✅ | `cargo check --lib --tests` exit 0 (con BD de rama); `cargo test --lib` 153/153; `tsc --noEmit` limpio; git = solo cambios esperados | — |
| **R1 Modo operativo** | ✅ (1 menor) | `ServicioModoOperacion`: histéresis N=3 (UMBRAL_FALLOS_BDP), `invalidar()` al PATCH, sin red en auto (preflight ligero cumplido), wired a poller y escrituras; badge "BDP: off" y banners verificados en UI | H5 |
| **R2 Catálogo** | ⚠️ (1 alto) | Backend CRUD local completo + rango reservado + encolado (D3) + import respeta `local_dirty`/desactivados (M6/M7) + `resolve_article` local; **pero el CRUD de artículos solo vive en Configuración → BDP → "Catálogo de artículos BDP"** (`config-bdp-mapeos.tsx`), y la página "Catálogo" solo tiene departamentos/familias | H1 |
| **R3 Stock** | ⚠️ (2 altos) | Ajuste local con motivo+auditoría, origen visible, encolado si código BDP; **sin "Nuevo artículo"**; empty state solo sugiere "Sincroniza el catálogo desde BDP o Cargar demo"; **"Sync catálogo" habilitado en standalone** | H2, H7 |
| **R4 Inventario** | ❌ (2 altos) | Conteo = `useState` de la UI, **no se persiste** (el backend lo documenta: "Localmente no persiste el conteo"); en standalone "Enviar" es no-op y el toast dice "Inventario encolado: N artículos" (engañoso); la diferencia no se aplica al stock local | H3, H4 |
| **R5 Anulación** | ✅ | Modalidades D4, confirmación+motivo+auditoría (E2E 198A-2), no-facturadas (M9), anuladas nunca se borran (D5), delete desbloqueado, resumen excluye (M10), mesa solo ocupante (M11), `cancel_order` encolado en anulación fresca con `bdp_order_id` (F6/M16), poller excluye | — |
| **R6 Compras** | ✅ (1 menor) | CRUD local con serie L- e IVA por línea (tests 18/18 + UI: "Nuevo albarán" operativo); **empty state no menciona crear albarán local** (solo "Sync albaranes"/"Cargar demo") | H8 |
| **R7 Pagos/factura local** | ✅ | `pago_parcial_local` (ledger, saldo, idempotencia), factura local F-{año}-{n} con guards (tests 11/11); botones locales/BDP verificados en UI | — |
| **R8 Menús/packs** | ✅ | CRUD local sobre catálogo, convivencia con Explorador BDP (tests 15/15) | — |
| **R9 Historial** | ✅ | `origen_operacion='local'` (anular_venta Local visto en Historial 198A-2) | — |
| **R10 Permisos** | ✅ | 6 permisos configurables con enforcement backend (403 verificado; 13/13 tests); alta de artículo ya cubierto por `CatalogoEdicion` | — |
| **R11 Escrituras** | ⚠️ (1 medio) | Encolado en handlers (artículo, departamento/familia, propina, puntos, inventario, cancelar); flush no-op standalone con test de invariante; botón "Sincronizar a BDP" solo en modo bdp; **no hay visibilidad de la cola en la UI** (solo `POST /push/flush`; sin listar pendientes, sin reintento por ítem) | H6 |
| **R12 Navegación/UX** | ⚠️ | Menú integrado sin sección BDP ✅; títulos sin prefijo ✅; dropdown 3 puntos + autocompletar corregidos ✅; **empty states de Stock/Compras/Inventario orientan a BDP/demo en vez de a la acción local** | H2, H3, H8 |
| **R13 Independencia real** | ✅ | Cero tráfico a BDP verificado (198A-2, logs de red); botones BDP ocultos/deshabilitados salvo H7; migraciones aditivas (M15); series L- y rango reservado sin colisiones; Haddock documentado (M14) | H7 |
| **R14 Con BDP real** | ⏸ | Suscripción WebLink (1 mes, hasta 24/09) pendiente de activar por el cliente; lecturas reales = bloque 138A-2 | — |

**Conclusión de la auditoría:** el **núcleo de independencia está implementado y verde** (baseline
153 tests + UI). El "desastre" que percibe el usuario son **6 hallazgos de UX/ubicación** (H1–H4
son los que rompen la experiencia; H6–H8 la completan) y **no** fallos de lógica. La corrección
es un plan propio (no parches sueltos) que se decide con los hallazgos consolidados.

---

## 5. Tabla de hallazgos (auditoría 2026-08-27)

| ID | Área | Hallazgo (evidencia) | Severidad | Corrección propuesta | Estado |
| --- | --- | --- | --- | --- | --- |
| **H1** | R2/R12 | El CRUD de artículos (alta, edición inline, activo, origen) solo existe en **Configuración → BDP → "Catálogo de artículos BDP"** (`config-bdp-mapeos.tsx` + `bdp-article-map-table.tsx`); la página **"Catálogo"** del menú solo gestiona departamentos/familias. Dos "catálogos" distintos y confusos; Configuración mezcla conexión con CRUD de negocio | **Alto** | Unificar: mover el CRUD de artículos a la página "Catálogo" (junto a departamentos/familias); Configuración queda solo con conexión/mapeos/permisos | ⏳ Pendiente de decisión |
| **H2** | R3 | **Stock no permite crear artículos**: solo "Ajustar" sobre artículos existentes. En standalone sin catálogo, el empty state dice "Sincroniza el catálogo desde BDP o pulsa Cargar demo" — la única salida sugerida es BDP/demo | **Alto** | Añadir "Nuevo artículo" a la página Stock (alta local con código/nombre/precio/IVA/familia; encola en modo bdp vía `crear_article_map` que ya existe). Empty state accionable | ⏳ Pendiente de decisión |
| **H3** | R4 | **Inventario no persiste el conteo**: `contadas` es `useState` de la UI; el backend lo documenta explícitamente ("Localmente no persiste el conteo"); en standalone "Enviar inventario" es no-op pero muestra toast "Inventario encolado: N artículos". La pantalla es inútil sin BDP | **Alto** | Persistir conteos localmente (tabla de conteos fechada, auditable, recontable); deshabilitar/avisar claramente el envío en standalone | ⏳ Pendiente de decisión |
| **H4** | R4 | La diferencia contada **no se aplica al stock local** (solo se encola a BDP en modo bdp) | **Medio** | Decidir: al guardar un conteo, ¿aplica la diferencia al stock local con motivo "conteo"? (recomendado) | ⏳ Pendiente de decisión |
| **H5** | R1 | No hay guard de normalización al guardar configuración: se puede persistir `modo_operacion=standalone` con `bdp_sync_enabled=true` (M1 preveía normalizar a `auto`). Funciona porque el switch maestro gana, pero el almacenamiento puede ser contradictorio | **Bajo** | Normalizar en el PATCH (standalone+sync → auto o aviso) | Pendiente de plan |
| **H6** | R11 | **No hay visibilidad de la cola de push en la UI**: solo existe `POST /api/bdp/push/flush`; no hay listado de pendientes (aunque `BdpPushService::listar_pendientes` existe), ni estado por ítem (pendiente/suscripción/error), ni reintento individual (D2 dice reintento manual pero solo hay flush global) | **Medio** | Endpoint GET de cola + sección "Sincronización" con filas y acciones (reintentar, ver error), visible solo en modo bdp | Pendiente de plan |
| **H7** | R3/R13 | Botón **"Sync catálogo" habilitado en standalone** en Stock (`BdpStockActions` solo se deshabilita con `demoMode`): pulsa a BDP sin BDP → error. Viola R13.2 (no ofrecer BDP en standalone) | **Medio** | Deshabilitar con tooltip "requiere BDP conectado" cuando el modo efectivo no sea bdp (patrón U8) | Pendiente de plan |
| **H8** | R6 | Empty state de Compras no menciona crear un albarán local: "No hay albaranes importados. Selecciona un rango de fechas y pulsa Sync albaranes, o pulsa Cargar demo" — pero el botón "Nuevo albarán" sí existe y funciona | **Bajo** | Actualizar el empty state para ofrecer "Nuevo albarán" como primera acción | Pendiente de plan |

**Hallazgos descartados (verificados como correctos):** encolado en standalone (las filas quedan
pendientes y se envían al conectar BDP — comportamiento local-first deseado, no un bug); test de
invariante `flush_en_standalone_no_envia_ni_consume_la_cola`; migraciones aditivas; serie L- y
rango reservado.

## 6. Decisiones del usuario (resueltas 1×1 el 2026-08-27)

| # | Decisión | Resolución |
| --- | --- | --- |
| **D1** | Ubicación del CRUD de artículos (H1) | **Mover a la página "Catálogo"** (junto a departamentos/familias); Configuración queda solo con conexión/mapeos/permisos |
| **D2** | Alta de artículo en Stock (H2) | **Sí**: botón "Nuevo artículo" en Stock con el formulario de alta (código, nombre, precio, IVA, familia); en modo bdp encola el alta |
| **D3** | Persistencia del conteo de inventario (H3) | **Sí**: tabla de conteos local fechada y auditable, retomable y recontable; en modo bdp además se encola el envío |
| **D4** | Efecto del conteo en el stock local (H4) | **Sí**: al guardar el conteo, la diferencia (contado − esperado) ajusta el stock local con motivo "conteo" |
| **D5** | Visibilidad de la cola de push (H6) | **Sí**: sección "Sincronización" visible solo en modo bdp, con filas pendientes/sincronizadas/error, estado por ítem y reintento individual; el flush global se mantiene |
| **D6** | Qué queda en Configuración → BDP | **Solo configuración**: conexión, mapeos y permisos. Sin CRUD de negocio; si procede, atajos a Catálogo/Sincronización |

**Correcciones técnicas confirmadas (sin decisión de producto):** H5 (normalizar `standalone`+`sync=true` → `auto` al guardar), H7 (deshabilitar "Sync catálogo" en standalone con tooltip "requiere BDP conectado"), H8 (empty state de Compras ofrece "Nuevo albarán" como primera acción).

El plan de corrección se construye a partir de esta tabla (siguiente bloque).

## 7. Cierre documental

- [ ] Roadmap actualizado con el bloque de auditoría y su resultado
- [ ] Hallazgos consolidados en `Agente/documentacion/bdp/` (fuente canónica)
- [x] Plan de corrección separado (nuevo plan con fases y DoD) una vez cerrada la auditoría
      → creado el 2026-08-27: `plan-correccion-independencia-bdp-2026-08-27.md` (bloque 208A-2)
- [ ] Sin commits durante la auditoría (solo documentación del hallazgo); los cambios previos
      del entorno siguen sin commitear
