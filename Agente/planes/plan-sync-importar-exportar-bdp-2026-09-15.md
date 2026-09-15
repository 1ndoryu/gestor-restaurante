# 159A-2 — Unificar botones sync en Importar/Exportar + importar departamentos/familias (2026-09-15)

Origen: el usuario siente complejo el botón "Sync" y pide 2 botones por sección
(Importar del BDP / Exportar al BDP). Además pide resolver por qué departamentos y
familias nunca se planificaron para importar y hacerlo posible si se puede.

## 0. Respuesta corta a "por qué nunca se importaron"

Sin motivo técnico. El BDP expone departamentos (`/API/Departments/Export` y
`/API/Departments/ExportFromProfile`, ya cableados en `src/services/bdp_weblink.rs:362,370`
y usados por backup/preflight/explorer; W-Q2.3 vio 55 deptos reales). El flujo se diseñó
local-primero con empuje al BDP (`POST /api/bdp/catalogo` encola push, `bdp_catalogo.rs:65-83`)
y la importación solo se implementó para artículos (F5.7: sync-catalog/sync-prices).
Conclusión del spike previo: importar departamentos es factible con endpoint nuevo;
familias queda pendiente del spike F0 (el BDP tiene CreateFamily/CreateSubfamily pero no
hay export de familias cableado; los artículos traen códigos Family/Subfamily).

## 1. Inventario real (2026-09-15, verificado por grep)

| # | Sección | Botón actual | Dirección | Endpoint / hook |
|---|---------|--------------|-----------|-----------------|
| 1 | Catálogo pest. Artículos | Sync catálogo | BDP→local | `POST sync-catalog` (`BdpArticleCatalogActions.tsx:87`) |
| 2 | Catálogo pest. Artículos | Sync precios | BDP→local | `POST sync-prices` (`BdpArticleCatalogActions.tsx:97`) |
| 3 | Stock | Sync catálogo | BDP→local | `useSyncCatalog` (`BdpStockActions.tsx:92`) |
| 4 | Stock | Exportar CSV | archivo (NO BDP) | descarga local (`BdpStockActions.tsx:81`) |
| 5 | Clientes | Importar BDP (modal preview+frase) | BDP→local | hook importar (`ListaClientes.tsx:104,320-335`) |
| 6 | Clientes | push por cliente | local→BDP | `POST /api/clientes/:id/bdp-sync` |
| 7 | Ventas | Polling BDP | BDP→local | `POST /api/ventas/bdp-poll` (`ventas.rs:388`) |
| 8 | Ventas | auto-push + Reintentar | local→BDP | `POST /api/ventas/:id/bdp-sync` (`ventas.rs:260`) |
| 9 | Plano Sala | Sincronizar mesas (diálogo preview) | BDP→local | `useSyncTables` (`PlanoSala.tsx:42-74`) |
| 10 | Plano Sala | Exportar/Importar plano | archivo JSON (NO BDP) | (`PlanoSala.tsx:182-183`) |
| 11 | Sincronización + header | Flush global + pendientes + reintento | local→BDP | `POST /api/bdp/push/flush` (`bdp_push.rs:38-44`, Admin) |
| 12 | Config BDP | Probar sincronización | ninguno (diagnóstico) | `GET sync-dry-run` (`ConfigBdp.tsx:302`) |
| 13 | Catálogo pest. Clasific. | (no existe importar) | — | alta local + push encolado |

## 2. Diseño objetivo: 2 botones por sección

Regla: toda sección con intercambio BDP muestra exactamente **Importar del BDP**
(Download) y **Exportar al BDP** (Upload), con tooltip de dirección y gating modo BDP.
Lo que es archivo se renombra para no confundir.

- **Catálogo/Artículos**: Importar = sync-catalog + sync-prices en UNA acción secuencial
  (un toast resumen "X nuevos, Y actualizados, Z precios"); se retiran los 2 botones.
  Exportar = flush de dominios departamento/familia/artículo (ver F0: filtro por dominio).
- **Clasificaciones (nuevo)**: Importar = endpoint nuevo F1 (deptos; familias si F0 OK),
  con preview conteo + respeto a ediciones locales (igual que clientes: no sobrescribir
  vínculos/nombres locales, marcar origen `bdp`). Exportar = ya existe por cola+flush;
  el botón dispara el flush de esos dominios (o informa "se envía solo" si está vacío).
- **Clientes**: Importar se mantiene (ya es modal con preview+confirmación).
  Exportar = botón global que hace flush del dominio cliente (hoy solo existe por fila).
- **Ventas**: Importar = renombrar "Polling BDP" a "Importar del BDP". Exportar = automático
  al guardar (ya) + "Reintentar" con tooltip "Exportar al BDP"; opcional botón Exportar
  pendientes si F0 lo hace barato.
- **Plano Sala**: Importar = renombrar sync-tables a "Importar del BDP". Exportar NO aplica
  (el plano es local); Exportar/Importar JSON → "Guardar archivo"/"Cargar archivo".
- **Stock**: Importar = renombrar a "Importar del BDP". "Exportar CSV" → "Descargar CSV".
- **Sincronización**: es el panel de Exportar global; renombrar flush a "Exportar al BDP".
- **Config**: "Probar sincronización" → "Probar conexión" (no mueve datos).
- Componente reutilizable `BdpImportExportButtons` (2 botones + gating + tooltips) para
  no duplicar lógica en 6 sitios.

## 3. Fases

- **F0 spike (solo lectura, sin código productivo)**: (a) rango real de códigos de depto
  del BDP (¿caben en CHECK 1..999 de `bdp_catalogo_clasificaciones`? ver los 55 de W-Q2.3);
  (b) ¿existe export de familias en el manual (`# WEBLINK RESTAPI.md` cat. Departamentos/
  Almacén) o se derivan de Family/Subfamily de ExportArticles?; (c) ¿`flush` admite filtro
  por dominio o hay que añadirlo? (`flush(pool, user_id, manual)` hoy es global).
  Salida: decisión familias (endpoint propio vs derivadas vs solo local+export) y diseño
  del filtro de flush.
- **F1 backend**: `POST /api/bdp/catalogo/importar-departamentos` (y familias si F0 OK):
  lee ExportDepartments/FromProfile, upsert con origen `bdp`, respeta filas locales
  (no pisa nombre local, solo vincula código), códigos fuera de 1..999 se reportan como
  omitidos (no se fuerza el CHECK). Filtro dominio en flush si F0 lo pide. Tests
  unitarios/integración contra simulador. Reutilizar guards modo BDP existentes.
- **F2 frontend**: `BdpImportExportButtons` + aplicar el par Importar/Exportar por sección
  (§2) + renombres archivo-vs-BDP. Importar departamentos con resumen
  (nuevos/vínculos/omitidos). Todo sigue en modales existentes donde los haya; los botones
  son acciones con confirmación/toast, no formularios inline.
- **F3 verificación**: type-check `src/` limpio, tests afectados verdes, preview visual
  sección por sección (1x1 con OK del usuario, como 149A-1). Importar = lectura BDP
  (sin armado). Exportar/flush contra BDP real = escritura real → regla oro 149A-2:
  solo con autorización explícita por operación; en simulador libre.

## 6. Hallazgos F0 (2026-09-15, solo lectura, cero escrituras)

- **Rango códigos reales**: `GET /api/bdp/departamentos/perfil` (ExportFromProfile) devuelve
  ~60 deptos con códigos 1..63 (CAFES=1, GINEBRAS=8, ...), árbol con SubDepartamentos.
  Caben de sobra en el CHECK 1..999. Nombres con eñes llegan mojibake (CO�AC): se
  importan tal cual (cosmético, preexistente en vistas de auditoría).
- **Familias**: el BDP NO expone export de familias (manual sin ExportFamilies; solo
  CreateFamily/CreateSubfamily + códigos Family/Subfamily sin nombre en ExportArticles).
  Decisión: familias quedan local+export; NO se importan (registrado, no es olvido).
- **Flush**: era global sin filtro; se añadió `flush_con_dominios` (wrapper conserva
  `flush` sin cambios: cero churn en tests existentes) + query `?dominios=` en
  `POST /api/bdp/push/flush`.
- Estado BD al medir: modo efectivo BDP (auto+enabled), push_modalidad=automatico,
  cola activa VACÍA (4 filas sincronizado) → lecturas seguras.

## 7. Bloque F1 commiteado (backend, sin escrituras reales BDP)

- `POST /api/bdp/catalogo/importar-departamentos` (modo BDP + CatalogoEdicion, solo lee
  BDP) + servicio `BdpImportDepartamentosService` + repo `buscar_por_code/nombre`,
  `crear_con_code`. Tests: `tests/bdp_import_departamentos.rs` (1 passed) + unit aplanar
  (1 passed) + regresión `bdp_push` (19 passed) + `bdp_push_cola` (5 passed).
- `flush_con_dominios` + `?dominios=` en flush_manual (wrapper manual `@/api/bdp`,
  sin orval). Familias: no importables (sin export en BDP) → local+export.

## 4. No alcance (original, vigente)

- No tocar lógica de encolado/push ni guards de modo; solo exponerla con 2 botones.
- No importar familias si F0 demuestra que el BDP no las expone (quedan local+export,
  registrado como pendiente real con motivo).
- No cambiar `Sincronización` más allá del renombre; no tocar dry-run (solo renombre).
- Sin migraciones destructivas; códigos BDP fuera de rango se omiten, no se remapean a mano.

## 5. Definition of Done

- Cada sección del §1 muestra como máximo el par Importar/Exportar (o 1 si no aplica,
  documentado en §2); cero botones "Sync/Polling/Probar sincronización" con nombre viejo.
- Importar departamentos (+familias si F0 OK) funciona contra simulador y respeta ediciones
  locales; omitidos fuera de rango reportados.
- Type-check + tests verdes; preview 1x1 con OK usuario; commit por bloque con ID 159A-2.
