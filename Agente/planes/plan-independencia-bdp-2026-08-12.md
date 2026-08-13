# Plan — Independencia total del BDP (funcionar con o sin BDP, 100% operacional)

> **Fecha:** 2026-08-12 (revisión profunda 2026-08-12)
> **Rama:** `glory-rs-rest`
> **ID de bloque:** `128A-1`
> **Estado:** Activo (en ejecución). F0–F9 completados; F10 en curso.
> **Skills aplicadas:** `supervisor-thinking` (diseño y desafío) y `supervisor-review` (revisión dura) —
> veredicto en el Anexo B.
>
> **Terminología (resuelta 2026-08-12):** no existe "bridge" en el repositorio; el alcance es
> **exclusivamente RESTAURANTE** y su **integración BDP (WebLink REST API)**, tema de los planes
> 048A-11/12. Este documento no usa el término "bridge".
>
> **Decisiones del usuario (D1–D8):** todas resueltas el 2026-08-12 — ver §13. Ninguna pendiente
> bloqueante.
> **Revisión de auditoría 2026-08-12:** incorporados C1–C6 y notas de F1/F3/F4/F6/F8 (ver §15 y Anexo B);
> checklist de correcciones en `Agente/documentacion/bdp/auditoria-plan-independencia-bdp-2026-08-12.md`.
> **Objetivo (cita del usuario):** "todo lo que depende del BDP tiene que funcionar con o sin el BDP;
> todo tiene que estar 100% operacional". Además pidió: antes de ejecutar, **revisar el plan otra vez,
> mitigar problemas y conflictos que no se ven, y anticipar cosas** → §14 contiene ese análisis
> profundo (M1–M18), con su mitigación por conflicto.

---

## 0. Tabla resumida (para revisión rápida)

| Funcionalidad | ¿Depende de BDP hoy? | Estado de la integración | Independencia planificada (sin BDP) | Esfuerzo |
| --- | --- | --- | --- | --- |
| Catálogo de artículos | Sí (sync BDP) | ✅ operativo | **Catálogo local CRUD** sobre `bdp_article_map` ampliada (origen local/bdp, `local_dirty`) | M |
| Stock | Sí (lectura BDP) | ⚠️ parcial (`CurrentStock`; `GetStock` no impl.) | **`stock_local` editable** + `CurrentStock` + `GetStock`/`GetListStock` (BDP) | M |
| Clientes | Parcial (import) | ✅ operativo | Ya local; import BDP opcional | S |
| Plano de sala | Parcial (sync) | ✅ operativo | Ya local; sync BDP opcional | S |
| Ventas / comandas | Parcial (push) | ✅ operativo | Ya local; push BDP opcional | S |
| Pago | Sí (BDP + suscripción) | ❌ real no verificado (2.3) | Pago local = registro de venta; **parciales locales** (F6, A6) | S |
| Factura | Sí (BDP + suscripción) | ❌ real no verificada (2.4) | **Factura local mínima** (F6, decisión D9) | S |
| Pagos parciales | Sí (flag) | ✅ implementado (BDP) | Ledger local listo; **endpoint de escritura local** (F6, A8) | S–M |
| Cancelar comanda / anular venta | Sí (BDP + suscripción) | ❌ bloqueado por BDP | **Anulación local configurable** (`anulacion_modalidad`) + liberar mesa | M |
| Compras (albaranes) | Sí (lectura BDP) | ⚠️ solo Fase 1 (lectura) | **Albaranes locales** sobre `bdp_purchase_notes` (origen) | M |
| Historial / auditoría / snapshots | Sí | ✅ operativo (BDP) | Auditoría local de operaciones (extender `bdp_audit`) | S |
| Explorador (menús/packs) | Sí | ⚠️ sin verificar real | **Menús/packs locales** (CRUD sobre catálogo) | L |
| Polling de estados | Sí | ✅ operativo | Solo en modo bdp | — |
| Badge de estado | Sí | ✅ operativo | Indica modo `standalone`/`bdp`/degradado | S |
| **Eliminación de ventas** | **Bloqueada** si BDP on | ⚠️ dependencia inversa (`venta::delete` 409) | Desbloquear si no sincronizada ni anulada; anuladas nunca se borran | S |
| **Permisos por acción** (catálogo/stock/albaranes/anulación) | Nueva | — | **Configurables en Configuración** (default admin) | S |

**Deuda de integración que NO bloquea la independencia** (se audita en F0; no es pre-requisito): deploy
1e, activación de flags en producción, suscripción WebLink (1c), datos de prueba en TPV (1d), lecturas
reales (1b). El objetivo es que el restaurante opere 100% aunque esa deuda siga abierta.

---

## 1. Problema real, objetivo y no-goals

**Problema real:** varias áreas de la aplicación solo existen o solo se ven útiles cuando BDP está
configurado y disponible. Sin BDP (nunca configurado, credenciales vacías, BDP caído, suscripción sin
activar, deploy pendiente), esas áreas quedan vacías, deshabilitadas o bloqueadas (p. ej. eliminar ventas
devuelve 409, catálogo/stock/compras/explorador no tienen datos, anulación no existe, pagos parciales son
solo de comandas BDP). La operación diaria del restaurante no puede depender de un tercero ni de una
suscripción que gestiona el cliente.

**Objetivo (resultado deseado):** con o sin BDP, todas las funciones del restaurante están operativas al
100%. BDP pasa de ser un requisito a una **capa opcional** que se suma cuando está configurado y
disponible, sin degradar nada.

**No-goals:**
- No se modifica el contrato WebLink ni los servicios de conexión `bdp_*` (la capa de conexión BDP se
  conserva intacta).
- No se implementa sincronización bidireccional (rechazado firme, D3 del plan de pendientes).
- No se depende de la suscripción WebLink para cerrar la independencia.
- No se migran/escriben datos en el BDP del cliente sin autorización explícita.
- No se toca la integración Haddock (fuera de alcance; no empeorarla — M14).
- No se promete multi-instancia ni escalado horizontal (modelo de carga en §5).

---

## 2. Hechos confirmados vs supuestos

**Confirmados (código/documentación/migraciones):**
- `bdp_sync.rs` hace no-op si `!bdp_sync_enabled || !bdp_configurado(config)` (líneas 95, 1314, 1653).
- `venta.rs::delete` devuelve 409 mientras `bdp_sync_enabled` (y lo mismo con `haddock_sync_enabled`).
- No existe anulación local de ventas (solo `bdp_order_status='cancelled'`; `CancelOrder` bloqueado por
  BDP). Reservas sí tienen `cancelada`.
- **Roles:** `UserRole::{Admin, Trabajador}` (derivado de `trabajador_id`); middleware `AuthUser` con
  `require_role(&[UserRole])` y `effective_role` (incl. impersonación). Base para permisos (D8).
- **`bdp_article_map`:** tabla de mapeo enriquecida (`articulo_glory_codigo`, `articulo_bdp_codigo`,
  `articulo_bdp_nombre`, stock; `UNIQUE(user_id, articulo_glory_codigo)`). Sirve de base ampliable para
  el catálogo unificado (M5).
- **`venta_lineas`:** líneas con `articulo_codigo TEXT` libre (sin FK) + descripción/cantidad/precio/IVA.
  Compatible con catálogo local sin migrar datos.
- **`bdp_purchase_notes`:** `serie`, `numero`, `fecha`, `codigo_proveedor`, `nombre_proveedor`, `total`,
  `datos_bdp JSONB`, lifecycle de estados. Apta para albaranes locales con series locales (M18).
- **`bdp_pagos`:** ledger de pagos (parciales) — hoy BDP-only (M13).
- **`bdp_audit` / `bdp_backup` / `bdp_write_arming`:** auditoría, snapshots y arming existentes.
- Los 6 feature flags BDP son `false` por defecto; activación en producción pendiente (roadmap 2). Los
  checks de flags deben quedar condicionados al modo bdp (M12).
- Pendientes de la integración (N1–N14, Anexo A).

**Supuestos (riesgo abierto si cambian):**
- Un único restaurante por instalación, decenas de usuarios, ~10²–10³ artículos, cientos de ventas/día.
  Sin objetivo de carga mayor declarado (§5).
- En standalone, las ediciones locales sobre artículos importados de BDP son la excepción; el import no
  debe pisarlas (M6).
- El cliente no usa el Explorador de BDP (fuera del criterio de entrega) — por eso D2=A es opcionalidad
  local, no reemplazo del BDP.

---

## 3. Arquitectura de la independencia

### 3.1 Conmutador de modo operativo (con invariantes — M1)

Nuevo campo en `configuracion_restaurante`: **`modo_operacion`** (`auto` default | `standalone` | `bdp`).

| Valor | Comportamiento |
| --- | --- |
| `auto` | Si `bdp_configurado()` y preflight OK → `bdp`; si no → `standalone`. Reevaluación con TTL (30–60 s) + on-demand. **Histéresis (M2):** no cambia de modo por un único fallo/éxito; requiere N consecutivos (N=3 default) y nunca cambia a mitad de una operación. |
| `standalone` | Nunca se llama a BDP. Proveedores locales en todas las pantallas. |
| `bdp` | Fuerza modo BDP; si BDP cae o rechaza → **degradación a standalone con aviso** (badge + banner), sin romper operaciones locales. |

**Invariantes (M1 — evita matriz de estados contradictoria):**
1. `modo_operacion` es el **switch maestro**. `bdp_sync_enabled` y `bdp_sync_mode` solo se interpretan
   cuando el modo efectivo es `bdp`; en `standalone` se tratan como inactivos (sin borrarlos).
2. `bdp_sync_enabled` se mantiene por compatibilidad (columna existente) y pasa a derivarse del modo en
   las comprobaciones nuevas: `modo_efectivo() == bdp` ⟺ sync activo.
3. Guard: al guardar configuración no se permite un estado incoherente (p. ej. `modo_operacion=standalone`
   con `bdp_sync_enabled=true` explícito → se normaliza a `auto` o se avisa y se aplica `standalone`).
4. Migración aditiva: `modo_operacion` default `auto`; no altera filas existentes (M15).

**Servicio:** `ServicioModoOperacion` (SRP): decide el modo por `user_id`, cachea con TTL, expone
`modo_efectivo()`, `evento_fallo_bdp()` (degradación reactiva) y **se invalida al actualizar
configuración** (M3). Reutiliza `bdp_configurado()` del preflight; no duplica lógica de credenciales.

**Decisión F1 (auditoría):** en modo `auto`, la reevaluación por TTL debe usar **preflight ligero**
(credenciales + health), no el dry-run completo (`BdpSyncPreflightService::execute`, ~8 llamadas remotas
incluyendo create-order dry-run); el dry-run completo queda on-demand. Definir también el **almacenamiento
de la histéresis M2** (estado en memoria por proceso vs persistido) y el comportamiento con 2 TPV
(riesgo multi-proceso documentado, §5).

### 3.2 Proveedor por dominio (DIP/OCP)

Para cada dominio BDP-only (catálogo, stock, compras, historial, pagos parciales): **una única fuente de
datos por dominio con columna `origen ('local'|'bdp')`** y un selector según el modo. Handlers y UI
dependen del repositorio del dominio, nunca de `BdpWeblinkClient` directamente. Nuevos dominios se
añaden sin tocar el flujo core (OCP). Detalle por dominio en §4.

### 3.3 Reglas de degradación (invariantes)

1. **Ninguna operación local se bloquea por estado BDP.** Único bloqueo mantenido: eliminar físicamente
   una venta sincronizada con BDP (integridad) → sustituido por anulación local (§4.7).
2. **Nada queda vacío sin explicación.** Toda pantalla BDP muestra su modo local o un estado
   vacío/aviso claro (patrón U1–U8 ya implementado).
3. **Fail-closed en escrituras BDP.** En modo `bdp`: allowlists, arming/auto-arming y backup pre-write.
   En `standalone`: no existe path de escritura a BDP.
4. **Origen siempre visible.** Badge/columna de origen en filas mixtas (local vs BDP).
5. **Los feature flags BDP solo gatean en modo `bdp`** (M12): en `standalone` ninguna pantalla local se
   bloquea por un flag apagado.

### 3.4 Alternativas consideradas (tradeoffs)

| Opción | Descripción | Tradeoff | Veredicto |
| --- | --- | --- | --- |
| A. Condicionales por feature | Cada pantalla ramifica "¿hay BDP?" | Rápido, condicionales dispersos, viola OCP, fácil olvidar un caso | ❌ Descartada |
| B. Tabla única + `origen` + selector de modo | Una fuente por dominio, origen por fila | Reutiliza tablas/migraciones; conviven local y BDP; exige `local_dirty` y reglas de import (M6) | ✅ **Elegida** |
| C. Tablas espejo (local/BDP paralelas) | Dos fuentes por dominio | Duplica esquema y lógica de sync; divergencia casi segura | ❌ Descartada |
| D. Trait provider (impl local/bdp) | Abstracción total tipo adapter | Más limpio SOLID, pero sobreingeniería para 1 consumidor y escala actual (YAGNI §8) | ⚠️ Evolución futura |

---

## 4. Diseño por dominio (con BDP → sin BDP)

Cada dominio: estado actual → diseño sin BDP → diseño con BDP → cambios concretos → **criterio de
aceptación observable**. Los conflictos anticipados (M#) se detallan en §14.

### 4.1 Modo operativo + badge (base)

- **Sin BDP:** badge "Modo independiente" en navbar (reutilizar `BdpStatusIndicator`); opciones BDP
  ocultas o deshabilitadas con motivo.
- **Con BDP:** badge actual + degradación automática con banner (M2).
- **Cambios:** campo `modo_operacion` + migración; `ServicioModoOperacion`; badge extendido; avisos
  U2/U8 ya existentes.
- **Aceptación:** sin credenciales BDP la app entera opera; con credenciales y BDP caído degrada sin
  errores y el badge lo indica.

### 4.2 Catálogo local (M5, M6, M7)

- **Sin BDP:** CRUD completo de artículos sobre `bdp_article_map` ampliada (`origen='local'`): alta,
  edición, desactivación, precio/IVA/familia/código de barras. `resolve_article` (R12) se extiende para
  resolver desde el catálogo local antes del fallback por defecto.
- **Con BDP:** import `ExportArticles` hace upsert por `articulo_bdp_codigo` respetando `local_dirty`
  (D3=A: el import es fuente inicial pero no pisa ediciones locales — M6); artículos desactivados
  localmente se conservan como `activo=false` y el import no los reactiva (M7).
- **Cambios (C4):** migración aditiva sobre `bdp_article_map`: **solo `origen` y `local_dirty`** — el
  resto ya existe (`descripcion`, `precio_tarifa1`, `iva_pct`, `departamento`, `familia`, `subfamilia`,
  `activo`, `barcode`, `stock_actual`, `ultima_sync_at`; migraciones 20260714, 20260715200000,
  20260723000000); repositorio catálogo; endpoints CRUD (`GET/POST/PUT/DELETE` de artículos); UI sobre
  `BdpStock.tsx` extendido (o nueva pantalla Catálogo reutilizando sus patrones). Semántica documentada:
  tabla = "artículos del catálogo + mapeo Glory↔BDP" (M5).
- **Aceptación:** crear/editar/desactivar un artículo sin BDP y verlo en stock/ventas; tras un import BDP
  con el mismo código, la edición local no se pisa y el desactivado local se conserva.

### 4.3 Stock local + BDP (D7 = B)

- **Sin BDP:** stock local editable por artículo/almacén (ajuste manual entrada/salida) con auditoría.
- **Con BDP:** columna `CurrentStock` del catálogo **+ `GetStock`/`GetListStock` (N6)** para stock por
  artículo/almacén con datos frescos; el stock local queda como respaldo/diferencia visible, nunca se
  pisa.
- **Cambios (alineado con A3):** `bdp_article_stock` ya existe (por almacén, "General") y `stock_actual`
  en `bdp_article_map` (migración 20260723000000); endpoint de ajuste sobre el almacén elegido;
  `get_stock`/`get_list_stock` en `bdp_weblink.rs` + structs en `bdp_weblink_catalog.rs` (patrón D1);
  handler `GET /api/bdp/stock` (Opción B de D1); UI en `BdpStock.tsx`.
- **Decisión F3 (resuelta 2026-08-13):** fuente de verdad del stock local = `bdp_article_stock`
  (por almacén, default `warehouse_id='0'`/"General"). `stock_actual` de `bdp_article_map` es un
  snapshot de BDP que nunca se pisa. `CurrentStock` de BDP se consulta vía `GetStock`/`GetListStock`
  (N6) y la UI muestra el origen (`local`/`bdp`) de cada valor.
- **Aceptación:** ajustar stock sin BDP y verlo reflejado; con BDP, stock por almacén desde `GetListStock`
  sin pisar `stock_local`.

### 4.4 Clientes y 4.5 Plano de sala

- Ya locales. Solo verificación: import/sync BDP como acciones opcionales con aviso "requiere BDP
  conectado" (patrón U8). Sin cambios de modelo.
- **Aceptación:** crear cliente/editar plano sin BDP (ya funciona) y sin botones rotos.

### 4.6 Ventas, pagos, facturas y pagos parciales (A6–A8, M13)

- **Ventas:** locales desde siempre (CRUD + líneas). El push BDP es opcional (no-op sin BDP). Sin cambios
  salvo verificar que ningún botón BDP bloquee el flujo local en `standalone`.
- **Pago local (A6):** hoy el pago se registra al crear/editar la venta (`metodo_pago`, importes); no
  existe una operación de pago local separada. Diseño: pago completo = venta con `metodo_pago` (como
  hoy); **pagos parciales locales = nuevo endpoint de escritura** sobre el ledger existente.
- **Pagos parciales locales (A8/M13):** el ledger `bdp_pagos` ya admite filas sin `bdp_order_id` y el
  `GET /api/ventas/:id/bdp-payments` ya calcula total/pagado/pendiente **sin BDP**. Falta solo el
  **endpoint local de escritura** (POST pago parcial local) con idempotencia, saldo pendiente y guards.
  Con BDP se conserva el flujo actual (`GetOrder` + flag, solo modo bdp). No se renombra el ledger
  (compatibilidad).
- **Factura local (A7):** **no existe** hoy (`bdp-invoice` exige `bdp_order_id`). Para 100% operacional
  se añade **factura local mínima** en F6: numeración local secuencial + estado `facturada` + auditoría
  (decisión D9, default implementar). Con BDP, `InvoiceOrder` sigue el flujo actual.
- **Aceptación:** pagar una venta local en dos partes sin BDP con saldo correcto; facturar localmente sin
  BDP (número local + estado); con BDP, flujo actual intacto.

### 4.7 Anulación local de ventas + eliminación (M8–M11, D4, D5)

- **Modalidad configurable (D4):** campo `anulacion_modalidad` (`credito_completo` default |
  `estado_solo`), elegible en Configuración (patrón feature flags). Ambas comparten confirmación
  dinámica (patrón auto-arming: texto/monto/nº de venta) y auditoría obligatoria.
- **Sin BDP:** anulación 100% local según modalidad. En `credito_completo`: estado `anulada`, motivo
  obligatorio, **reversión de IVA idempotente** (M10) y exclusión del resumen diario, liberación de mesa
  **solo si la venta es la ocupante actual** (M11).
- **Con BDP:** si la comanda está sincronizada, intenta `CancelOrder`; si la suscripción no lo permite →
  venta **anulada localmente** con "pendiente de anular en BDP" + reintento (poller M8). Nunca se finge
  éxito BDP.
- **Reglas de transición (M9):** solo se anulan ventas **no facturadas**; ventas facturadas requieren
  flujo de nota de crédito fiscal aparte (documentado, no en esta fase). Transición de estado única y
  guardada (pendiente/pagada → anulada); idempotencia por petición (doble click seguro).
- **Eliminación (`venta::delete`, D5=A):** se desbloquea el 409 **solo para ventas no sincronizadas con
  BDP ni anuladas**. Las anuladas nunca se borran físicamente (registro histórico con motivo). Si está
  sincronizada y BDP no responde → bloqueo con mensaje accionable.
- **Cambios:** migración (`anulada`, `anulada_at`, `anulacion_motivo`, `anulacion_usuario`;
  `anulacion_modalidad` en config), `AnulacionVentaService` (2 modalidades), `POST /api/ventas/:id/anular`,
  ajuste de `venta.rs::delete`, poller excluye anuladas-pendientes-BDP (M8), UI en `venta-row-actions.tsx`,
  toggle en Configuración.
- **Notas de auditoría (F4, incorporadas):**
  - **C1:** `ventas` NO tiene columna `estado` → la máquina de estados (pendiente/pagada → anulada) se
    crea de cero; enumerar agregados afectados (resumen diario `total_periodo`, listados, filtros,
    export) antes de implementar.
  - **C2/M11:** `ventas` NO tiene `mesa_id`; la cadena real es venta → `reserva_id` → `mesas`, con
    fallback `num_mesa` en reservas sin `mesa_id` (`services/plano_sala.rs:351-353`). Liberación de mesa
    = solo si la venta anulada es la ocupante actual.
  - **C3/M8:** decidir en F4 la vía de cancelación BDP: (a) `CancelOrder` requiere ampliar scopes/arming
    y el CHECK de `bdp_write_arming` (hoy `cancel_order` no está contemplado; alinear §6 y U6), o
    (b) estado `anulada_local_pendiente_bdp` sin llamada API + reintento manual/poller cuando haya
    suscripción. Nunca fingir éxito BDP. **DECISIÓN F4: opción (b).** `VALID_BDP_WRITE_SCOPES`
    (`src/services/bdp_write_guard.rs:10`) no incluye `cancel_order` y el CHECK
    `bdp_write_arming_scopes_safe` tampoco; no se amplían scopes/arming en F4. El estado
    "pendiente BDP" se deriva: `anulada=true AND bdp_synced=true AND bdp_order_status NOT IN
    ('cancelled','invoiced')`. El poller excluye esas ventas (M8) y el reintento vía `CancelOrder`
    queda condicionado a una fase futura con scopes/arming ampliados.
  - **C5:** `bdp_pagos` tiene `ON DELETE CASCADE` sobre `ventas` → al desbloquear `venta::delete`,
    documentar y probar la semántica (historial de pagos de ventas borradas).
- **Aceptación:** anular sin BDP en cada modalidad; delete no bloqueado en caso seguro; venta sincronizada
  con BDP caído se anula local con "pendiente BDP"; anular dos veces seguidas no duplica (guard).

### 4.8 Compras locales (M18)

- **Sin BDP:** CRUD local de albaranes sobre `bdp_purchase_notes` (`origen='local'`, **series locales
  tipo `L-...`** para no chocar con el UNIQUE serie/numero de BDP — M18): proveedor (nombre/código local),
  fecha, líneas (en `datos_bdp` o columnas), estados del lifecycle ya existentes. La conciliación con
  gastos ya es local y no depende de BDP.
- **Con BDP:** `ExportPurchaseNotes` importa sobre el mismo almacén (origen `bdp`).
- **Cambios:** migración (`origen`, columnas locales si faltan), repositorio compras, endpoints CRUD,
  UI sobre `BdpCompras.tsx`; flags `ff_bdp_purchase_notes_*` solo gatean en modo bdp (M12).
- **Aceptación:** crear un albarán local, conciliarlo con un gasto y verlo en Compras, todo sin BDP.

### 4.9 Historial / auditoría / snapshots locales

- Extender `bdp_audit` con `origen_operacion ('local'|'bdp')` para registrar operaciones locales
  (anulaciones, ajustes de stock, CRUD catálogo, pagos parciales) y que Historial las muestre sin BDP.
  Snapshots de configuración (`bdp_backup`) ya son locales; verificar visibilidad sin BDP.
- **Aceptación:** una anulación/ajuste local aparece en Historial sin BDP conectado.

### 4.10 Explorador (menús/packs) — decisión D2 = A

- **Sin BDP:** menús/packs **locales**: agrupaciones de artículos del catálogo local (CRUD con líneas,
  precios, activación), reutilizando el patrón de `venta_lineas`.
- **Con BDP:** el Explorador actual de BDP se conserva y convive con los locales indicando origen.
- **Cambios:** migración (menús/packs locales + líneas), endpoints CRUD, UI sobre `BdpExplorador.tsx`
  extendido. F7.
- **Aceptación:** crear un menú local con artículos del catálogo y verlo en el Explorador sin BDP.

### 4.11 Permisos configurables (decisión D8)

- **Diseño:** permisos **por acción** configurables en Configuración, con enforcement en backend
  (nunca solo UI — M17):
  `permisos_catalogo_edicion`, `permisos_stock_ajuste`, `permisos_albaranes_gestion`,
  `permisos_anulacion_ventas`. Cada uno con valores `admin` (default) | `admin_trabajador` | `todos`.
- **Implementación:** columnas en `configuracion_restaurante` (patrón feature flags) + helper
  `permiso_habilitado(config, accion, user)` que combina `require_role` (existente) y el toggle;
  middleware/guard por endpoint. UI en Configuración BDP.
- **F8 (auditoría):** enumerar en F8 los endpoints de escritura a proteger (article-maps CRUD,
  sync-prices, ajuste stock, purchase-notes draft/reconcile, bdp-payment, bdp-invoice, anular,
  customers/import, clientes/:id/bdp-sync, sync-tables, bdp-poll) y añadir tests 403 por permiso.
- **Aceptación:** con default (admin), un trabajador recibe 403 al ajustar stock/anular; al ampliar el
  permiso a `todos` en Configuración, puede hacerlo. Ninguna acción sensible queda solo protegida por UI.

### 4.12 Polling, arming, preflight, flags

- Solo aplican en modo `bdp` (`bdp_order_poller` ya guarda con `bdp_sync_enabled`). En `standalone`
  permanecen inactivos y ocultos. El poller se ajusta para **excluir ventas anuladas-pendientes-BDP**
  (M8). Sin otros cambios.

---

## 5. Modelo de escala, rendimiento y operación

- **Modelo de carga (declarado):** 1 restaurante por instalación, decenas de usuarios, ~10²–10³ artículos,
  cientos de ventas/día, máx. 2 TPV. **Riesgo abierto:** sin objetivo de multi-instance/N restaurantes
  (R2-nota sigue abierto) — no se vende como diseño escalable.
- **Rendimiento:** standalone = cero llamadas de red a BDP (el mayor coste desaparece). CRUD con índices
  por código/estado y paginación (ya existe en `BdpStock.tsx`). Sin N+1: queries SQLx preparadas y joins
  acotados. Modo `auto`: reevaluación con TTL e histéresis, sin polling agresivo.
- **Operación/observabilidad:** badge de modo + log de transiciones (`standalone`→`bdp`→degradado) con
  motivo; `/api/configuracion/bdp/diagnostico` (endpoint real, `handlers/configuracion.rs:448`) ampliado
  con `modo_operacion` y `ultima_comprobacion`.
- **Recursos:** timeouts y límites existentes se conservan; en standalone no aplica throttling BDP.
- **M2 (auditoría):** definir el almacenamiento del estado de histéresis (memoria por proceso vs
  persistido) y el comportamiento con 2 TPV antes de F1; el plan declara multi-instancia fuera de
  alcance, pero 2 procesos conviven en el modelo de carga actual.

---

## 6. Seguridad

- Nuevos endpoints CRUD (catálogo, stock, compras, anulación, permisos): validación con `validator`
  (patrón existente), SQLx preparado (nunca interpolado), paginación y límites, timeouts.
- **Permisos (D8):** enforcement en backend con `require_role` + toggles configurables (M17); ninguna
  acción sensible protegida solo por UI.
- Anulación: confirmación dinámica (no texto fijo), auditoría obligatoria (`usuario`, `motivo`, `ip`,
  `timestamp`), idempotencia por petición, transición de estado guardada (M9/M10).
- `local_dirty`/origen: el import respeta ediciones locales; sin sobreescritura silenciosa (M6/M7).
- Secretos: credenciales BDP siguen en env/BD; nada nuevo se registra en documentación ni logs. **No se
  tocan allowlists ni arming, salvo decisión F4/C3** (si se elige `CancelOrder` para anulaciones BDP, se
  amplían scopes/arming explícitamente y con evidencia).
- Fail-closed: en `standalone` no existe path de escritura a BDP; la degradación nunca relaja allowlists.

---

## 7. UI y reutilización (front)

- **Reutilizar antes de crear:** `BdpStock.tsx`, `BdpCompras.tsx`, `BdpHistorial.tsx`,
  `BdpExplorador.tsx`, `venta-row-actions.tsx` (anulación), `BdpStatusIndicator` (badge),
  `BdpDemoToggle`/`useBdpDemoMode` (patrón conmutador), `bdp-mocks.ts` (demo). Los CRUD nuevos son
  **extensiones de las pantallas existentes** (modo local), no componentes paralelos.
- **Modo demo ≠ modo independiente (M16):** el demo muestra datos simulados de BDP; el standalone muestra
  datos reales locales. Textos/avisos distintos para no confundir ("Modo demo" vs "Modo independiente").
- **Design system:** tokens existentes (CSS en español/camelCase), sin estilos inline, sin hex/fuentes
  literales en componentes (reglas del proyecto).
- **Estados:** carga, vacío (con aviso de modo), error, teclado, responsive y contraste en cada pantalla
  tocada; validación visual en local al cerrar cada fase.
- Badge/columna de origen reutiliza el patrón de indicador de estado BDP existente.

---

## 8. Núcleo / abstracción (YAGNI)

- **Decisión:** no abstraer ahora al núcleo compartido (glory-rs/framework). Criterios: (1) lógica de
  modo local acoplada al producto (restaurante/BDP), no genérica; (2) sin segundo consumidor real hoy;
  (3) el framework no expone API para ello. El selector de modo podría extraerse si aparece un segundo
  consumidor (decisión registrada, no bloqueante).
- Reutilización interna: `bdp_configurado()`, preflight, `bdp_article_map`, `bdp_purchase_notes`,
  `bdp_pagos`, `bdp_audit`.

---

## 9. Documentación / entropía (archivos que tocará la tarea)

- `roadmap.md`: ya registrado `128A-1` (en curso) — se actualiza al cerrar cada fase y se retira al cierre.
- Este plan → `Agente/planes/completados/` al cerrar.
- `Agente/completados/tareas-YYYY-MM-DD.md`: registro con evidencia por bloque.
- `Agente/documentacion/bdp/feature-flags-bdp-2026-07-26.md`: añadir `modo_operacion`, permisos por
  acción, y aclarar que los flags solo aplican en modo bdp (M12).
- `Agente/usuario/mapeo-visual-integracion-bdp-2026-07-23.md`: entradas de modo local/origen/permisos.
- Guía del cliente: sección "modo independiente" y permisos (si aplica).
- `Agente/prevencion/` si surge un fallo repetible (p. ej. sobreescritura de ediciones locales en import).

---

## 10. Gate y evidencia

- **Por fase:** `cargo fmt --check`, `cargo check`, `cargo test --lib bdp`, `tests/bdp_service_integration.rs`
  + `tests/bdp_simulator_integration.rs`, suite Python del simulador (`tools/bdp-weblink-simulator`),
  frontend `tsc` + build Vite.
- **Gate canónico de cierre:** `npm run task:check -- <task-id>` (Sentinel 0.7.0, doctor/lock PASS) con
  reporte reproducible (reporte, rama, commit), separando deuda base de regresión nueva.
- **C6 (auditoría):** antes de prometer `task:check`, verificar en F0 el gate local: `sentinel doctor`
  readyForGate, identidad del gate vs submódulo (`0.7.0` pineado vs local 0.7.1 sucio / origin 0.7.4) y
  lock/submodule alineados.
- **Evidencia funcional:** por fase, recorrido con/sin BDP (sin credenciales → standalone; con
  credenciales y simulador → bdp) con resultados en `Agente/completados/`.

---

## 11. Fases y checklist ejecutable (orden D6 = natural)

| Fase | Contenido | Salida verificable | Depende de |
| --- | --- | --- | --- |
| **F0** | Auditoría del estado real: ¿se ejecutó el deploy 1e? ¿flags en producción? Estado de N1–N14; verificar que nada de la integración cambió | Inventario A/B actualizado con fecha y evidencia | — |
| **F1** | Conmutador `modo_operacion` + invariantes (M1) + histéresis (M2) + invalidation (M3) + badge + degradación + avisos | Sin credenciales: app 100% operativa, badge "independiente"; BDP caído: degrada sin errores | F0 |
| **F2** | Catálogo local: migración sobre `bdp_article_map`, repositorio, CRUD, fallback `resolve_article`, reglas import (M5/M6/M7) | CRUD local sin BDP; import no pisa ediciones ni reactiva desactivados | F1 |
| **F3** | Stock local + `GetStock`/`GetListStock` (N6), `POST /api/bdp/article-stock/ajustar`, UI con origen (D7) | Ajuste local sin BDP; con BDP stock por almacén sin pisar `stock_local` | F2 |
| **F4** | Anulación local (modalidades D4), reglas M8–M11, desbloqueo delete (D5), auditoría | Anular sin BDP según modalidad; delete no bloqueado en caso seguro; "pendiente BDP" | F1 |
| **F5** | Compras locales: CRUD albaranes + conciliación local (M18), flags solo bdp | Albarán local → conciliación con gasto sin BDP | F1 |
| **F6** | Historial/auditoría local (`origen_operacion`) + pagos parciales locales (A8) + **factura local mínima** (A7/D9) | Operaciones locales visibles; pago parcial y factura local sin BDP | F4 |
| **F7** | Menús/packs locales (D2) sobre catálogo + convivencia BDP | CRUD de menús sin BDP; origen visible | F2 |
| **F8** | Permisos configurables (D8) + enforcement backend (M17) | 403 para rol sin permiso; toggle en Configuración lo habilita | F2–F7 |
| **F9** | Pruebas con/sin BDP: standalone completo, simulador, regresión del gate | Suites + `task:check` PASS con reporte | F1–F8 |
| **F10** | Cierre documental: roadmap, completados, feature-flags, mapeo visual, plan a `planes/completados/` | Documentación actualizada y evidencia registrada | F9 |

**SIGUIENTE ACCIÓN (verificable):** ejecutar **F10** (cierre documental: roadmap con 128A-1
cerrado, completados con evidencia, feature-flags/mapeo visual actualizados y plan movido a
`planes/completados/`) en el ciclo local completo.
Autorizado: todo el ciclo local. No autorizado sin usuario: deploy a producción, escrituras al BDP
real, SSH (prohibido siempre).

**Estado 2026-08-13:** F0/F1 **completados** en rama `glory-rs-rest` (commit
`[128A-1] F0/F1 ...`). Evidencia: `cargo test` (unit + integración) PASS con
`CARGO_BUILD_JOBS=2` y `GLORY_CARGO_MIN_FREE_MB=1024` (evita E0786 ambiental),
`task:check 128A-1 --allow-heavy` PASS, type-check frontend PASS. F2
**completado** (commit `[128A-1] F2: catálogo local ...`): migración
`20260814000000_bdp_article_map_catalogo_local`, modelo con `origen`/
`local_dirty`/`omitidos_ediciones_locales`/`desactivados_localmente`, CRUD y
fallback `resolve_article` (M5), import que no pisa ediciones locales (M6) ni
reactiva desactivados (M7), UI de catálogo con origen y edición inline.
Evidencia: `task:check 128A-1 --allow-heavy` PASS (sentinel, varsense, rust,
frontend type-check, docs), tests `bdp_article_map` 26/26 + integración 8/8.

**Estado 2026-08-13 (F3):** **completado** — stock local editable con
auditoría (`bdp_article_stock` por almacén, `POST /api/bdp/article-stock/ajustar`
con idempotencia vía `bdp_audit_log` y transacción con upsert), weblink N6
especulativo `GetStock`/`GetListStock` con structs y tests wiremock, UI de stock
con badge de origen (`local`/`bdp`) y diálogo de ajuste. Evidencia: tests
`bdp_article_map` 30/30, `bdp_backup` 27/27, `--lib` 134/134, type-check
frontend PASS, `task:check 128A-1` PASS. Siguiente acción: **F4** (anulación
local).

**Estado 2026-08-13 (F4):** **completado** — anulación local de ventas con
modalidades (D4): migración `20260815000000_venta_anulacion`
(`anulada`, `anulada_at`, `anulacion_motivo`, `anulacion_usuario`;
`anulacion_modalidad` en config, `credito_completo` default), `VentaService::anular`
(motivo obligatorio en crédito completo, bloqueo de facturadas M9, guard de
transición única + idempotencia C1 vía `bdp_audit_log`), `total_periodo` excluye
anuladas (reversión de IVA idempotente M10), poller BDP excluye
anuladas-pendientes (M8, C3=b sin `CancelOrder`), liberación de mesa solo si es
la ocupante actual (M11), delete desbloqueado solo para ventas no sincronizadas
y no anuladas (D5, las anuladas nunca se borran), UI: botón Anular con
confirmación `ANULAR {id}` + motivo en `venta-row-actions.tsx`, badge «Anulada»,
selector de modalidad en Configuración BDP. Evidencia: `task:check 128A-1` PASS
(sentinel, varsense, rust, frontend type-check, docs). Siguiente acción: **F5**
(compras locales).

**Estado 2026-08-13 (F5):** **completado** — compras locales (M18, M12): migración
`20260816000000_bdp_purchase_notes_local` (`origen` con CHECK `local|bdp`, default `bdp`,
índice `idx_bdp_purchase_notes_user_origen`), modelo `BdpPurchaseNote` con `origen`,
structs `BdpPurchaseNoteLineaLocal`/`CrearBdpPurchaseNoteRequest`/`ActualizarBdpPurchaseNoteRequest`,
repositorio `crear_local` (serie `L`, secuencial por usuario `COUNT(*) origen='local'`),
`actualizar_local` (COALESCE por campo, recalcula `datos_bdp` con líneas), `eliminar_local`
(solo `pendiente`/`borrador`), handlers POST/GET `/bdp/purchase-notes` y PUT/DELETE
`/bdp/purchase-notes/:id` con gates de flags condicionales al modo efectivo bdp (M12) y
conciliación con IVA por línea (A10). Frontend: tipos + hooks CRUD en `api/bdp.ts`,
`BdpComprasLocalModal` (serie/proveedor/fecha/total/líneas con IVA por línea),
`BdpCompras` con badge de origen (`local`/`bdp`), botón «Nuevo albarán», editar/eliminar
solo origen local y `purchaseFeatureEnabled` según modo efectivo (standalone sin flags).
Evidencia: tests `bdp_purchase_notes_lifecycle` 18/18, `task:check 128A-1 --full` PASS
(sentinel, varsense, rust, frontend type-check, docs). Siguiente acción: **F6**
(historial/auditoría local, pagos parciales y factura local mínima).

**Estado 2026-08-13 (F6):** **completado** — historial/auditoría local (A11),
pagos parciales locales (A8/M13) y factura local mínima (A7/D9): migración
`20260817000000_bdp_audit_origen_local` (`bdp_audit_log.origen_operacion`
`local|bdp` default `bdp` + índice por usuario/origen; `ventas.facturada_local`,
`factura_numero`, `factura_fecha` + UNIQUE parcial `(user_id, factura_numero)`).
Auditoría local en anular, ajuste de stock, `pago_parcial_local` y
`factura_local` con `origen_operacion='local'` (Historial visible sin BDP).
`POST /api/ventas/:id/pagos-locales` (ledger `bdp_pagos` sin renombrar, saldo
pendiente, idempotencia por clave con normalización de claves vacías) y
`POST /api/ventas/:id/factura-local` (numeración `F-{año}-{n:04}` por usuario,
guards M9: no anuladas, sin doble facturación local/BDP, pagos parciales que
cubran el total; retry ante colisión de número). M9 extendido: anular bloquea
`facturada_local`; `bdp_invoice` rechaza ventas facturadas localmente. Frontend:
badge de origen en Historial con filtro, tipos `origen_operacion`/factura local,
botones y diálogos de pago local (`PAGO LOCAL {id} {amount}`) y factura local
(`FACTURA LOCAL {id}`) cuando no aplican los botones BDP, badge «Facturada».
Evidencia: tests `bdp_f6_local_pagos_factura` 11/11 (más `bdp_pagos`,
`bdp_backup`, `bdp_service_integration`, `bdp_venta_lineas` en verde), clippy
limpio, type-check frontend PASS. Siguiente acción: **F7** (menús/packs locales).

**Estado 2026-08-13 (F7):** **completado** — menús/packs locales (D2, §4.10, A12/M12) con
CRUD sobre catálogo local: migración `20260818000000_bdp_menu_local`
(`bdp_menus_locales` con tipo CHECK `menu|pack`, UNIQUE `(user_id, tipo, nombre)` +
`bdp_menu_local_lineas` con FK CASCADE y `orden` determinista), modelo con
`BdpMenuLocalTipo`/`BdpMenuLocalConLineas`, repositorio dinámico (sin macro, sin cache `.sqlx/`):
`listar` con filtros tipo/activo/búsqueda y líneas `ANY($1)`, `find_by_id`, `crear` (tx),
`actualizar` (COALESCE, reemplazo de líneas y recálculo de precio), `eliminar`; handlers
`GET/POST /bdp/menus-locales` y `GET/PUT/DELETE /bdp/menus-locales/:id` con validaciones y
23505 → Conflict; frontend `BdpMenuLocalModal` (líneas con Select de artículos del catálogo
`useBdpArticleMaps`) y sección «Menús y packs locales» en `BdpExplorador` con badge `Local`,
siempre disponible en standalone (M12, sin gates de flags); Explorador BDP conservado.
Evidencia: tests `bdp_f7_menus_locales` 15/15, `task:check 128A-1 --full` PASS (sentinel, varsense,
rust, frontend type-check, docs), suite completa en verde. Siguiente acción: **F8** (permisos
operativos configurables D8/M17).

**Estado 2026-08-13 (F8):** **completado** — permisos operativos configurables (D8, §4.11, M17):
4 columnas `permisos_*` (`catalogo_edicion`, `stock_ajuste`, `albaranes_gestion`,
`anulacion_ventas`) `VARCHAR(20) NOT NULL DEFAULT 'admin'` con CHECK en
`configuracion_restaurante` (migración `20260819000000_bdp_permisos_operativos`, aditiva M15);
servicio `src/services/permisos.rs` con `AccionPermiso`/`NivelPermiso`
(`admin|admin_trabajador|todos`, `desde_valor` fail-closed → Admin),
`permiso_habilitado` sobre `effective_role` y guard `verificar_permiso` (403); enforcement en
`bdp_article_map` (catálogo/stock), `bdp_purchase_note` (albaranes) y `anular_venta`
(anulación); validación de valores en PATCH + CHECK en BD; UI «Permisos operativos» en
`ConfigBdp.tsx` con 4 selects y sync server→local con default `'admin'`. Alcance: se gatean las
acciones locales del bloque; sync-prices/sync-tables/bdp-payment/bdp-invoice/etc. siguen
protegidos por guards BDP existentes (sync_enabled, modo bdp, feature flags, BdpWriteGuard).
Evidencia: tests `bdp_f8_permisos` 13/13, suite completa en verde, clippy `-D warnings` PASS,
type-check frontend PASS, `task:check 128A-1 --full` PASS. Siguiente acción: **F9** (pruebas
con/sin BDP: standalone completo, simulador, regresión del gate).

**Estado 2026-08-13 (F9):** **completado** — verificación integral F1–F8 con/sin BDP: suite
standalone completa `run-with-db test` PASS (exit 0; `bdp_f8_permisos` 13/13, `bdp_f7_menus_locales`
15/15, resto en verde); simulador BDP Python 92/92 OK; integración Rust contra simulador
24/24 PASS (`--include-ignored`); regresión `task:check 128A-1 --full` PASS con reporte
reproducible (`.quality-reports/branches/glory-rs-rest--f100af0a041e6e8a/128A-1/latest.md`).
Siguiente acción: **F10** (cierre documental: roadmap, completados, feature-flags, mapeo visual,
plan a `planes/completados/`).

---

## 12. Criterios de aceptación globales (Definition of Done)

1. **Sin BDP** (sin credenciales, modo auto→standalone): catálogo CRUD, stock editable, ventas, pago
   completo y parciales locales, **factura local mínima**, anulación según modalidad, compras locales,
   menús locales, historial local, clientes, plano de sala y reservas — todo operativo, sin botones BDP
   rotos ni pantallas vacías sin explicación.
2. **Con BDP conectado** (simulador o real): comportamiento actual preservado (import, push, pago,
   factura, arming/auto-arming, allowlists, polling) y datos mixtos con origen visible.
3. **Con BDP caído/indisponible**: degradación automática a standalone con aviso (histéresis); ninguna
   operación local falla ni se bloquea.
4. **Eliminación de ventas**: desbloqueada solo para no sincronizadas/no anuladas; anuladas conservadas.
5. **Permisos**: cada acción sensible protegida en backend; toggle en Configuración cambia el acceso.
6. **Evidencia**: `cargo test --lib bdp` + suites simulador + `tsc`/build frontend + `task:check` PASS,
   reportes reproducibles en `Agente/completados/`.

---

## 13. Decisiones del usuario (D1–D8 resueltas 2026-08-12; D9 pendiente)

| # | Decisión | Estado |
| --- | --- | --- |
| D1 | Alcance = RESTAURANTE, integración BDP (no existe "bridge") | ✅ Resuelta |
| D2 | Explorador: **A — menús/packs locales** | ✅ Resuelta |
| D3 | Catálogo: **A — import BDP como fuente inicial + ediciones locales con `local_dirty`** | ✅ Resuelta |
| D4 | Anulación: **configurable** — `anulacion_modalidad` (`credito_completo` default \| `estado_solo`) en Configuración | ✅ Resuelta |
| D5 | Ventas anuladas: **no se borran físicamente** | ✅ Resuelta |
| D6 | Fases: **orden natural** F0→F10 | ✅ Resuelta |
| D7 | Stock: **B — lo más completo** (`CurrentStock` + `GetStock`/`GetListStock` + `stock_local`) | ✅ Resuelta |
| D8 | Permisos: **configurables por acción** en Configuración (default admin) | ✅ Resuelta |
| D9 | Factura local: ¿implementar **factura local mínima** (numeración local + estado) en F6? | ✅ **Resuelta (default: sí, mínima)** — implementada en F6 (`facturada_local` + `F-{año}-{n:04}` + auditoría local); con BDP `InvoiceOrder` sigue intacto |

**Sin decisiones pendientes bloqueantes salvo D9 (requerida antes de F6).** Cualquier ajuste posterior
se registra aquí con fecha.

---

## 14. Análisis profundo — conflictos anticipados y mitigaciones (M1–M18)

> Resultado del pase de revisión pedido por el usuario ("mitigar problemas y conflictos que no vemos,
> anticipar cosas"). Cada ítem: conflicto/riesgo → impacto → mitigación concreta (no solo lista).

| # | Conflicto / riesgo anticipado | Impacto | Mitigación |
| --- | --- | --- | --- |
| **M1** | **Matriz de estados contradictoria**: `modo_operacion` + `bdp_sync_enabled` + `bdp_sync_mode` + 6 flags pueden decir cosas distintas (p. ej. standalone con sync=true) | Comportamiento impredecible, bugs difíciles de trazar | `modo_operacion` = switch maestro (§3.1); `bdp_sync_enabled`/`bdp_sync_mode` solo se interpretan en modo bdp; guard de coherencia al guardar config; normalización a `auto` |
| **M2** | **Flapping de modo**: BDP intermitente hace alternar standalone/bdp constantemente, confundiendo UI y disparando escrituras fallidas | UX rota, operaciones BDP fallidas intermitentes | **Histéresis**: N=3 éxitos consecutivos para subir a bdp, N=3 fallos para degradar; nunca cambiar a mitad de una operación; TTL 30–60 s |
| **M3** | **Caché de modo desincronizada** tras PATCH de configuración | UI y backend ven modos distintos hasta expirar TTL | `ServicioModoOperacion` se invalida explícitamente al guardar configuración (además de TTL) |
| **M4** | **BDP cae a mitad de una operación** (sync/pago/factura en curso) | Venta local intacta pero estado sync ambiguo | Ya existe: la venta local se crea primero; sync fallido → `sync_error` + reconcile (R1). Se documenta y se conserva; en degradación no se reintenta automáticamente escritura a medias |
| **M5** | **`bdp_article_map` es tabla de mapeo, no catálogo**: reutilizarla como catálogo mezcla dos responsabilidades | Acoplamiento y confusión semántica; columnas que no existen | Ampliar la misma tabla (ya tiene nombre/stock) con las columnas del catálogo y **documentar la semántica** ("artículos del catálogo + mapeo Glory↔BDP"); `UNIQUE(user_id, articulo_glory_codigo)` se mantiene como identidad local; no crear tabla paralela (opción C rechazada) |
| **M6** | **Import BDP pisa ediciones locales** | Pérdida silenciosa de cambios del dueño | `local_dirty=true` al editar localmente; upsert del import **no sobrescribe** filas dirty (mantiene versión local y registra el conflicto en el reporte de import, visible en UI) |
| **M7** | **Artículos desactivados localmente reaparecen** en cada import | El dueño "oculta" un artículo y el sync lo reactiva | `activo=false` es local y el import no lo reactiva; el import reporta "N artículos desactivados localmente" |
| **M8** | **Anulación local vs poller de reconciliación**: venta anulada localmente pero abierta en BDP podría marcarse `ambiguo` otra vez | Falsos positivos, ruido en auditoría | Estado explícito `anulada_local_pendiente_bdp`; el poller lo **excluye** de la reconciliación y, cuando la suscripción exista, intenta `CancelOrder`. **DECISIÓN F4 (C3=b):** solo estado local sin llamada API; `cancel_order` no está en `VALID_BDP_WRITE_SCOPES` ni en el CHECK `bdp_write_arming_scopes_safe`, y no se amplían scopes/arming en F4. Reintento vía `CancelOrder` condicionado a fase futura (§6/U6) |
| **M9** | **Anular una venta facturada** | Descuadre contable (factura emitida) | Regla: solo se anulan ventas **no facturadas**; facturadas → flujo de nota de crédito fiscal aparte (documentado, no en F4) |
| **M10** | **Reversión de IVA duplicada o mal aplicada** (doble anulación, ediciones concurrentes) | Caja descuadrada | Transición de estado única y guardada (pendiente/pagada → anulada) con idempotencia por petición; en `credito_completo` el resumen diario excluye/revierte la venta exactamente una vez |
| **M11** | **Liberación de mesa equivocada** (la mesa la ocupa otra venta/comanda) | Plano de sala inconsistente | Solo se libera si la venta anulada es la ocupante actual de la mesa; si no, se avisa y no se toca el plano |
| **M12** | **Feature flags BDP bloquean funciones locales** (todos `false` por defecto): compras locales, pagos parciales locales | Modo standalone roto por flags apagados | Checks de flags **condicionales al modo bdp** (M12): en standalone las pantallas locales no consultan flags; se centraliza en el selector de modo |
| **M13** | **Pagos parciales solo existen para comandas BDP** | "100% operacional" falla en standalone (no se puede cobrar venta local en partes) | **Extender el ledger existente `bdp_pagos` (sin renombrar — §4.6)**: filas locales sin `bdp_order_id` con `origen`; local: saldo pendiente + idempotencia; bdp: validación `GetOrder` + flag |
| **M14** | **Haddock**: `venta::delete` también bloquea con Haddock on | Confusión si se toca delete sin ver Haddock | Fuera de alcance: no cambiar Haddock; solo documentar que el desbloqueo de delete considera ambos flags (si Haddock on, sigue bloqueado por Haddock) |
| **M15** | **Migraciones incompatibles con la BD de producción** (filas existentes, columna nueva sin default) | Deploy roto | Migraciones **aditivas** con defaults (p. ej. `modo_operacion='auto'`, `anulacion_modalidad='credito_completo'`, permisos `admin`); sin borrar/renombrar columnas existentes; verificar con `sqlx` y migraciones inmutables (prevención existente) |
| **M16** | **"Modo demo" vs "Modo independiente" confusos** en la UI | El dueño cree que los datos son simulados (o al revés) | Textos/avisos distintos; el demo queda para pruebas (datos simulados), el standalone usa datos reales locales; ambos con badge claro |
| **M17** | **Permisos solo en la UI** (ocultar botón) | Cualquiera llama el endpoint y pasa | Enforcement en **backend** (`require_role` + toggle por acción); la UI solo refleja el permiso |
| **M18** | **`bdp_purchase_notes` con UNIQUE(serie, numero)**: albaranes locales podrían chocar con series de BDP | Imports y altas locales en conflicto | Series locales reservadas (`L-...` / prefijo configurable); `origen` en la tabla; import BDP conserva sus series |

**Conflictos que se descartan conscientemente (no aplican):** bidireccional (rechazado D3), multi-instance
(no existe hoy, R2-nota documentado), WebSockets/tiempo real (fuera de alcance), migración de datos del
cliente (no autorizada).

---

## 15. Auditoría profunda por funcionalidad (estado verificado en código — 2026-08-12)

> Verificación directa en handlers, repositorios y migraciones. Cada entrada: estado real verificado,
> hallazgo (A#), corrección al diseño. Los hallazgos corrigen afirmaciones anteriores del plan.

| Funcionalidad | Verificado en código | Hallazgo | Corrección al diseño |
| --- | --- | --- | --- |
| **Modo operativo** | `ConfiguracionService::obtener` carga config por request; PATCH `/api/configuracion` existe | A1: punto de invalidación del modo confirmado | M3 se implementa invalidando el modo en el PATCH |
| **Catálogo** | `/api/bdp/article-maps` CRUD (crear/actualizar/eliminar/listar, upsert por `articulo_glory_codigo`); `import-catalog`/`sync-catalog` (glory code = BDP Code); `sync-prices` | A2: **el CRUD de artículos ya existe** (branding BDP, sin campos locales) | F2 = ampliar el modelo con `origen/local_dirty/precio/iva/familia/barcode/activo` + UI de catálogo; los endpoints ya están (menos trabajo del previsto) |
| **Stock** | `bdp_article_stock` (por almacén, "General") + `GET /api/bdp/article-stock`; `stock_actual` en article_map | A3: **el almacén de stock local ya existe** | F3: `stock_local` editable sobre `bdp_article_stock` (ajuste por almacén); `GetStock`/`GetListStock` (N6) solo para refresh BDP |
| **Clientes** | `bdp_customer_sync.rs` import + push controlado | A4: sin cambios | — |
| **Plano de sala** | `sync-tables` con confirmación "IMPORTAR MESAS BDP" | A5: sin cambios | — |
| **Pago** | Solo venta con `metodo_pago`; `bdp-payment` exige `bdp_order_id` | A6 (crítico): **no existe pago local como operación**; el ledger `bdp_pagos` + `GET /bdp-payments` ya sirven local | F6: pago completo = venta con `metodo_pago`; parciales = nuevo endpoint local de escritura |
| **Factura** | `bdp-invoice` exige `bdp_order_id` | A7 (crítico): **no existe factura local** (el plan la daba por existente) | F6: **factura local mínima** (numeración local + estado) → **D9 pendiente (default implementar)** |
| **Pagos parciales** | `bdp_pagos` admite `bdp_order_id` NULL; `GET /bdp-payments` calcula saldo sin BDP | A8: almacenamiento local listo; falta la escritura local | M13 se reduce a: endpoint local de escritura con guards; no renombrar el ledger |
| **Anulación** | No existe nada; `venta::delete` bloquea con BDP **y Haddock** | A9: confirmado; M14 (Haddock) verificado en código | F4 como diseñado; delete considera ambos flags |
| **Compras** | `bdp_purchase_note.rs`: flags bloquean read/draft/reconcile; reconcile pone IVA=0 (sin desglose BDP); rango ≤31 días solo en sync | A10: M12 confirmado; el local debe capturar IVA por línea | F5: CRUD local sin flags (solo modo bdp consulta flags); reconcile local con IVA por línea |
| **Historial/auditoría** | `bdp_audit_log` con `direccion` glory_to_bdp/bdp_to_glory; snapshots ya incluyen direccion 'glory' | A11: las operaciones locales puras (anulación) no encajan en `direccion` actual | Añadir `origen_operacion ('local'\|'bdp')` o valor 'local' en `direccion` |
| **Explorador/menús** | Solo `GET /bdp/menus|fastfoods|packs/:id` (lectura BDP) | A12: sin modelo local | F7 desde cero (migración menús/packs + líneas) |
| **Permisos** | `require_role` definido en `middleware/auth.rs`, **sin ningún uso** | A13: no hay endpoints protegidos por rol hoy | F8: wiring completo + helper `verificar_permiso`; aplicar a endpoints sensibles nuevos y existentes de escritura |
| **Polling/preflight/arming** | `bdp_order_poller`, preflight, `bdp_write_guard` existentes | A14: sin cambios salvo M8 (excluir anuladas-pendientes-BDP del poller) | Ajuste en F4 |

---

## Anexo A — Inventario de lo NO completado de la integración (N1–N14)

| # | Funcionalidad | Estado real (2026-08-12) | Bloqueo | Ref. |
| --- | --- | --- | --- | --- |
| N1 | Pago real BDP (2.3) | ❌ `Payment/Add` → "Subscripción no activada" | Suscripción WebLink | roadmap 1/1c |
| N2 | Factura real BDP (2.4) | ❌ en espera de 2.3 | Suscripción WebLink | roadmap 1 |
| N3 | `CancelOrder` | ❌ método existe, no expuesto; BDP rechaza | Suscripción BDP | roadmap 3 / D5 |
| N4 | Compras F2 (borradores → BDP) | ⚠️ solo borradores locales | Decisión cliente | D2 |
| N5 | Compras F3 (recepción BDP) | ⚠️ no implementada | Decisión cliente | D2 |
| N6 | `GetStock`/`GetListStock` | ✅ En alcance de 128A-1 (F3, D7=B) | — | D1 / F3 |
| N7 | Explorador verificación real | ⚠️ visible, sin verificar | Cliente | roadmap 1b |
| N8 | Pruebas reales de lectura BDP | ⚠️ procedimiento listo, no ejecutado | BDP no conectado | 267A-6 |
| N9 | Deploy a producción conectado | ❌ pendiente (envs, bootstrap, allowlists) | Autorización + BDP online + gate | roadmap 1e |
| N10 | Flags activados en producción | ❌ todos `false` | Rollout | roadmap 2 |
| N11 | Tarifa/plantilla Compras del cliente | ⚠️ sin aportar | Cliente | roadmap 1b |
| N12 | Verificación suscripción WebLink | ⚠️ sin confirmar | Cliente/proveedor | roadmap 1c |
| N13 | Limpieza datos prueba TPV | ⚠️ pendiente | Cliente | roadmap 1d |
| N14 | Bidireccional | ❌ rechazado por diseño (no backlog) | Decisión firme (D3) | D3 |

## Anexo B — Revisión (supervisor-thinking / supervisor-review)

**VEREDICTO (tras el pase profundo §14): `VIABLE CON RESERVAS` → las reservas se incorporaron al plan.**

- **SOLID:** SRP con `ServicioModoOperacion` y repositorios por dominio; OCP con origen/selector; DIP
  (handlers dependen de repositorios, no de `BdpWeblinkClient`); ISP (interfaces mínimas por dominio);
  LSP (local/bdp deben mapear errores con el mismo contrato — M4/M8, nunca 500 en UI local).
- **Eficiencia:** sin tablas espejo (C rechazada); reutiliza `bdp_article_map`, `bdp_purchase_notes`,
  `bdp_pagos`, `bdp_audit`; standalone = 0 llamadas BDP. YAGNI en núcleo (§8).
- **Rendimiento/escala:** modelo de carga declarado (1 restaurante); histéresis y TTL acotados; riesgos
  abiertos (multi-instance/N restaurantes) documentados, no bloquean.
- **Seguridad:** permisos en backend (M17), validación + SQLx preparado, confirmación dinámica en
  anulación, auditoría obligatoria, `local_dirty` sin sobreescritura, fail-closed en standalone.
- **UI:** extensión de pantallas existentes, tokens del design system, M16 (demo ≠ independiente),
  estados obligatorios.
- **Documentación/gate:** roadmap ya actualizado; gate `task:check` + suites simulador previstas por fase.
- **Hallazgos de la revisión (todos incorporados):** M1 (matriz de estados), M2 (flapping), M5 (semántica
  de `bdp_article_map`), M8 (poller vs anulación), M12 (flags vs standalone), M13 (pagos parciales
  locales), M17 (permisos backend). **La auditoría profunda por funcionalidad (§15, A1–A14) añadió:**
  A6 (no existe pago local como operación), A7 (**no existe factura local** → D9), A2/A3 (CRUD de
  artículos y almacén de stock ya existen → F2/F3 con menos migración), A13 (`require_role` sin uso →
  wiring completo en F8).
- **Segunda pasada (auditoría 2026-08-12, C1–C6):** incorporadas al plan: C1 (estado de `ventas` desde
  cero), C2 (cadena venta→reserva→mesa, fallback `num_mesa`), C3 (vía CancelOrder ↔ arming), C4 (F2 solo
  `origen`/`local_dirty`), C5 (cascade `bdp_pagos`), C6 (gate Sentinel 0.7.x) + decisiones F1/F3/F4/F6/F8
  (preflight ligero, fuente de stock local, D9 antes de F6, endpoints protegidos). Detalle en el MD de
  auditoría (`Agente/documentacion/bdp/auditoria-plan-independencia-bdp-2026-08-12.md`).
- **`FALTA EVIDENCIA` (esperado en fase de plan):** no hay evidencia de ejecución porque no se ha
  implementado. F0 produce la primera evidencia (estado real) y F9 el gate completo.

**Conclusión:** plan viable y **autorizado para ejecutar el ciclo local** (editar/probar/gate/commit).
**No autorizados** hasta el usuario: deploy a producción, escrituras al BDP real, SSH (prohibido).

---

## Checklist de cierre

- [ ] F0: inventario A/B verificado contra el estado real con evidencia
- [x] F1: modo operativo + invariantes (M1) + histéresis (M2) + badge + degradación, probados
- [x] F2: catálogo local (origen/local_dirty, CRUD sin BDP, resolve_article M5, import M6/M7), probado
      con gate PASS
- [x] F3: stock local (ajuste manual con auditoría, GetStock/GetListStock N6, UI con origen),
      probado con gate PASS
- [x] F4: anulación local (modalidades D4), reglas M8–M11, desbloqueo delete (D5), auditoría,
      probado con gate PASS
- [x] F5: compras locales (CRUD albaranes + conciliación local M18, flags solo bdp M12,
      IVA por línea A10), probado con gate PASS
- [x] F6: historial/auditoría local (`origen_operacion` A11), pagos parciales locales (A8/M13)
      y factura local mínima (A7/D9), probado con gate PASS
- [x] F7: menús/packs locales (D2) sobre catálogo local + convivencia BDP, probado con gate PASS
- [x] F8: permisos operativos (D8/M17: catálogo, stock, albaranes, anulación) sin BDP,
      probado con gate PASS
- [x] F9: pruebas con/sin BDP + simulador + gate `task:check` PASS con reporte reproducible
- [ ] F10: roadmap actualizado (128A-1 cerrado), completados con evidencia, feature-flags/mapeo visual
      actualizados, plan movido a `planes/completados/`
- [ ] Auditoría por funcionalidad (§15, A1–A14) aplicada en sus fases (A2/A3 reducen F2/F3; A6–A8 y
      A7/D9 en F6; A13 en F8; A10 en F5; A11 en F6)
- [ ] M1–M18 revisados en implementación (cada mitigación aplicada en su fase)
- [ ] Decisiones D1–D8 reflejadas en este documento (todas resueltas)
- [ ] C1–C6 y notas de auditoría incorporadas (estado ventas, cadena mesa, CancelOrder/arming, F2
      columnas, cascade, gate Sentinel) y verificadas al ejecutar cada fase
