# Auditoría cruzada: endpoints BDP backend vs. manifestación en frontend

> **Fecha:** 2026-07-23
> **Método:** Se rastrearon TODOS los `.route()` en los handlers BDP del backend y se cruzaron con los hooks/componentes/frontend que los consumen.

---

## Mapa completo de endpoints BDP

### 1. Configuración y diagnóstico

| # | Endpoint | Handler | Frontend consume | Componente | Estado |
|---|---|---|---|---|---|
| 1 | `GET /api/configuracion/bdp/diagnostico` | `configuracion.rs` | ✅ `axios.get` | `ConfigBdp.tsx` → botón "Probar conexión" | ✅ Completo |
| 2 | `GET /api/configuracion/bdp/sync-dry-run` | `configuracion.rs` | ✅ `axios.get` | `ConfigBdp.tsx` → botón "Validar con simulador" | ✅ Completo |
| 3 | `PUT /api/configuracion/bdp/sync-mode` | `configuracion.rs` | ✅ `useSetSyncMode` | `PanelBdpBackup.tsx` → `SyncModeSelector` | ✅ Completo |

### 2. Catálogo y artículos

| # | Endpoint | Handler | Frontend consume | Componente | Estado |
|---|---|---|---|---|---|
| 4 | `GET /api/bdp/article-maps` | `bdp_article_map.rs` | ✅ `useListarArticleMaps` | `bdp-article-map-table.tsx` | ✅ Completo |
| 5 | `POST /api/bdp/article-maps` (crear) | `bdp_article_map.rs` | ✅ `useCrearArticleMap` | `bdp-article-map-table.tsx` → formulario inline | ✅ Completo |
| 6 | `PUT /api/bdp/article-maps/:id` | `bdp_article_map.rs` | ✅ `useActualizarArticleMap` | `bdp-article-map-table.tsx` | ✅ Completo |
| 7 | `DELETE /api/bdp/article-maps/:id` | `bdp_article_map.rs` | ✅ `useEliminarArticleMap` | `bdp-article-map-table.tsx` → botón eliminar | ✅ Completo |
| 8 | `POST /api/bdp/article-maps/import-catalog` | `bdp_article_map.rs` | ✅ `useImportarCatalogo` | Generado por Orval | ✅ Completo |
| 9 | `POST /api/bdp/article-maps/sync-catalog` | `bdp_article_map.rs` | ✅ `useSyncCatalog` | `bdp-article-map-table.tsx` → botón "Sync catálogo" | ✅ Completo |
| 10 | `POST /api/bdp/article-maps/sync-prices` | `bdp_article_map.rs` | ✅ `useSyncPrices` | `bdp-article-map-table.tsx` → botón "Sync precios" | ✅ Completo |

### 3. Mesas y plano de sala

| # | Endpoint | Handler | Frontend consume | Componente | Estado |
|---|---|---|---|---|---|
| 11 | `POST /api/bdp/sync-tables` | `bdp_article_map.rs` | ✅ `useSyncTables` | `PlanoSala.tsx` → botón "Sync BDP" | ✅ Completo |

### 4. Menús, packs y fastfoods

| # | Endpoint | Handler | Frontend consume | Componente | Estado |
|---|---|---|---|---|---|
| 12 | `GET /api/bdp/menus/:id` | `bdp_article_map.rs` | ✅ `useGetMenuDefinition` | `bdp-menu-explorer.tsx` (nuevo 237A-3) | ✅ Completo |
| 13 | `GET /api/bdp/fastfoods/:id` | `bdp_article_map.rs` | ✅ `useGetFastfoodDefinition` | `bdp-menu-explorer.tsx` (nuevo 237A-3) | ✅ Completo |
| 14 | `GET /api/bdp/packs/:id` | `bdp_article_map.rs` | ✅ `useGetPackDefinition` | `bdp-menu-explorer.tsx` (nuevo 237A-3) | ✅ Completo |

### 5. Clientes

| # | Endpoint | Handler | Frontend consume | Componente | Estado |
|---|---|---|---|---|---|
| 15 | `POST /api/bdp/customers/import` | `bdp_customer_sync.rs` | ✅ `useImportarClientesBdp` | `ListaClientes.tsx` → botón "Importar BDP" + diálogo | ✅ Completo |
| 16 | `POST /api/clientes/:id/bdp-sync` | `bdp_customer_sync.rs` | ✅ `customInstance` directo | `ListaClientes.tsx` → botón "BDP" por cliente + diálogo | ✅ Completo |

### 6. Ventas y comandas

| # | Endpoint | Handler | Frontend consume | Componente | Estado |
|---|---|---|---|---|---|
| 17 | `POST /api/ventas/:id/bdp-sync` (retry) | `ventas.rs` | ✅ `useReintentarSyncBdp` | `venta-row-actions.tsx` → botón retry BDP | ✅ Completo |
| 18 | `GET /api/ventas/:id/bdp-status` | `ventas.rs` | ✅ `fetchBdpStatus` | `venta-row-actions.tsx` → botón "Consultar estado" (nuevo 237A-3) | ✅ Completo |
| 19 | `POST /api/ventas/:id/bdp-payment` | `ventas.rs` | ✅ `customInstance` directo | `venta-row-actions.tsx` → botón 💳 + diálogo pago | ✅ Completo |
| 20 | `POST /api/ventas/:id/bdp-invoice` | `ventas.rs` | ✅ `customInstance` directo | `venta-row-actions.tsx` → botón 📄 + diálogo factura | ✅ Completo |
| 21 | `POST /api/ventas/bdp-poll` | `ventas.rs` | ✅ `useBdpPoll` | `useListaVentas.ts` | ✅ Completo |

### 7. Explorador general BDP

| # | Endpoint | Handler | Frontend consume | Componente | Estado |
|---|---|---|---|---|---|
| 22 | `GET /api/bdp/explorar` | `bdp_backup.rs` | ❌ **NO** | Ninguno | ⚠️ **SIN UI** |

### 8. Respaldos y auditoría

| # | Endpoint | Handler | Frontend consume | Componente | Estado |
|---|---|---|---|---|---|
| 23 | `GET /api/bdp/backup/snapshots` | `bdp_backup.rs` | ✅ `useBdpSnapshots` | `PanelBdpBackup.tsx` → tabla snapshots | ✅ Completo |
| 24 | `POST /api/bdp/backup/completo` | `bdp_backup.rs` | ✅ `useCreateSnapshotCompleto` | `PanelBdpBackup.tsx` → botón | ✅ Completo |
| 25 | `POST /api/bdp/backup/parcial` | `bdp_backup.rs` | ✅ `useCreateSnapshotParcial` | `PanelBdpBackup.tsx` → botón | ✅ Completo |
| 26 | `POST /api/bdp/backup/glory` | `bdp_backup.rs` | ✅ `useCreateSnapshotGlory` | `PanelBdpBackup.tsx` → botón | ✅ Completo |
| 27 | `DELETE /api/bdp/backup/snapshots/:id` | `bdp_backup.rs` | ✅ `useDeleteSnapshot` | `PanelBdpBackup.tsx` → botón eliminar | ✅ Completo |
| 28 | `POST /api/bdp/backup/restaurar/:id` | `bdp_backup.rs` | ✅ `useRestoreSnapshot` | `PanelBdpBackup.tsx` → botón restaurar | ✅ Completo |
| 29 | `GET /api/bdp/audit` | `bdp_backup.rs` | ✅ `useBdpAudit` | `PanelBdpBackup.tsx` → tabla auditoría | ✅ Completo |

---

## Resumen

| Categoría | Endpoints | Con UI | Sin UI |
|---|---|---|---|
| Configuración | 3 | 3 | 0 |
| Catálogo/artículos | 7 | 7 | 0 |
| Mesas | 1 | 1 | 0 |
| Menús/packs/fastfoods | 3 | 3 | 0 |
| Clientes | 2 | 2 | 0 |
| Ventas/comandas | 5 | 5 | 0 |
| Explorador general | 1 | **0** | **1** |
| Respaldos/auditoría | 7 | 7 | 0 |
| **TOTAL** | **29** | **28** | **1** |

---

## El único endpoint sin UI: `GET /api/bdp/explorar`

**Qué hace:** El servicio `BdpExplorerService` (`src/services/bdp_explorer.rs`) ejecuta un barrido completo de BDP: login, health, catálogo, clientes, tenders, empleados, menús, fastfoods, packs, departamentos. Devuelve un resumen estructurado de todo lo que hay en BDP.

**Por qué no tiene UI:** Es un endpoint de diagnóstico avanzado que consulta múltiples endpoints de BDP en secuencia. Probablemente se creó como herramienta interna de desarrollo/debugging.

**¿Necesita UI?** Depende del uso:
- **Si es para soporte técnico/debugging:** No necesita UI visible para el cliente. El endpoint de diagnóstico (`/api/configuracion/bdp/diagnostico`) ya cubre Health+Login+Version.
- **Si se quiere dar al cliente una visión completa de qué hay en BDP:** Sería útil una pantalla "Explorar BDP" que muestre el inventario completo.

**Recomendación:** Dejarlo como herramienta interna. El cliente no necesita ver el explorador completo; las funciones individuales (catálogo, clientes, mesas, menús) ya tienen sus propias UIs.

---

## Pendientes para decisión del usuario

### Pendiente A — Stock (solo lectura)

**Estado:** ✅ **IMPLEMENTADO (237A-4)** — Opción rápida aplicada.

**Qué se hizo:**
- `BdpExportArticleItem` ganó campo `current_stock: Option<Decimal>` con aliases `CurrentStock` y `Stock`
- `sync_catalog()` mapea `current_stock` → `stock_actual` en la tabla `bdp_article_map`
- Columna Stock + Precio añadida a `bdp-article-map-table.tsx`
- Si el módulo de almacén no está activo, `CurrentStock` viene `None` y la columna muestra "—"
- Warn log añadido en sync_catalog para detectar si el campo nunca viene poblado

**Bloqueo:** Ninguno. El stock aparece tras ejecutar "Sync catálogo". Los endpoints dedicados `GetStock`/`GetListStock` quedan disponibles como mejora futura (pantalla completa ~8h).

### Pendiente B — Compras

**Estado:** No implementado. Excluido por decisión de producto.

**Esfuerzo:** ~20-30h (proveedores, albaranes, recepciones).

**Bloqueo:** Pendiente de tu consulta al cliente.

### Pendiente C — Auto-arming (C1)

**Estado:** No implementado. El selector de modo en ConfigBdp muestra info cards + pointer a PanelBdpBackup.

**Qué haría:** Que los botones de pago/factura activen automáticamente el arming sin ir a Configuración.

**Riesgo:** Modifica el modelo de seguridad fail-closed. Requiere rediseñar `BdpWriteGuard`.

**Esfuerzo:** ~10-12h.

**Bloqueo:** Pendiente de tu decisión.

### Pendiente D — Bidireccional automática

**Estado:** Bloqueado explícitamente en `configuracion.rs:296`.

**Bloqueo:** Pendiente de tu consulta al cliente.

### Pendiente E — Pagos parciales

**Estado:** Bloqueado explícitamente en `bdp_sync.rs:1084`.

**Bloqueo:** Pendiente de tu consulta al cliente.

### Pendiente F — CancelOrder

**Estado:** BDP devuelve "Subscripción no activada". No implementado.

**Bloqueo:** Requiere que BDP active el módulo.

---

## Cosas que se implementaron y YA tienen manifestación en frontend (verificar)

| Implementación | Frontend | Verificar visualmente |
|---|---|---|
| Catálogo BDP expuesto como sección de primer nivel | `ConfigBdp.tsx` → sección "Catálogo de artículos BDP" | Que se vea la tabla + botones sync |
| Mapeos técnicos (tender, canales) fuera del colapsable | `ConfigBdp.tsx` → sección "Correspondencias Glory ↔ BDP" | Que se vean los campos |
| Polling toggle en vista principal | `ConfigBdp.tsx` → sección "Actualización de estados" | Que el switch funcione |
| Info cards de modo autorización | `ConfigBdp.tsx` → sección "Modo de operaciones BDP" | Que muestre el modo actual |
| Botón "Consultar estado BDP" por venta | `venta-row-actions.tsx` → ícono 🔍 | Que aparezca en ventas sincronizadas |
| Indicador BDP en navbar | `site-header.tsx` → badge "BDP: off/lectura/escritura" | Que se vea en la barra superior |
| Explorador de menús/packs/fastfoods | `bdp-menu-explorer.tsx` → card en ConfigBdp | Que los 3 tipos funcionen |
