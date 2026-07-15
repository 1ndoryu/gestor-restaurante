# Plan BDP — Fase 9: Catálogo, Plano de Sala y Menús

> **Fecha:** 2026-07-15
> **Autor:** Agente
> **Contexto:** Tras Fase 7.5+8 (customer sync + facturación), el usuario quiere aprovechar endpoints BDP que tienen impacto real: artículos (catálogo automático), plano de sala (sync con BDP) y menús (lectura informativa).
> **Estado:** Planificado, pendiente de ejecución

---

## Análisis de utilidad

### 1. Artículos — `ExportArticles` / `GetArticle` / `GetPricesArticles`

**Por qué es útil:**
- Hoy mantienes `bdp_article_map` **a mano** (CRUD manual desde frontend o directo en BD)
- Si el cliente añade un producto en el TPV de BDP, Glory no se entera hasta que alguien lo mapea manualmente
- `ExportArticles` trae el catálogo completo con: código, descripción, 5 tarifas, descuento, IVA, departamento
- `GetArticle` permite buscar un artículo individual (fallback cuando `resolve_article` no encuentra el código)
- `GetPricesArticles` refresca precios sin reimportar todo

**Impacto:** Elimina el mantenimiento manual del mapa + sincroniza precios automáticamente

### 2. Plano de Sala — `GetRoomTables` / `GetRoomsTables`

**Por qué es útil:**
- Glory **ya tiene** un sistema completo de plano de sala: `zonas_sala`, `mesas`, `paredes_sala`, `combinaciones_mesas`
- Tiene 17 endpoints CRUD + export/import JSON + ocupación
- **Pero** el plano se configura manualmente en Glory
- BDP ya tiene definidas las mesas del POS (con su configuración en el TPV)
- `GetRoomTables` trae las mesas de un salón específico; `GetRoomsTables` trae todas

**Impacto:** Pre-cargar la estructura de mesas de BDP en Glory sin configurarlas a mano. Sync bidireccional de ocupación sería Fase 10+.

**Consideración:** El modelo BDP (`RoomTableData`: `{Id, Name, RoomId, RoomName, MinDiners, MaxDiners, Shape, Width, Height}`) mapea directo al modelo Glory `Mesa` (`numero`, `forma`, `min_personas`/`max_personas`, `ancho`/`alto`). El `RoomId` mapea a `ZonaSala`.

### 3. Menús — `GetMenuDefinition` / `GetFastfoodDefinition` / `GetPackDefinition`

**Por qué es útil:**
- BDP define menús como agrupaciones de artículos (grupo + items + suplementos)
- Si Glory algún día quiere mostrar menús en vez de artículos sueltos, necesita esto
- **Pero Glory NO tiene modelo de menús** — habría que crearlo desde cero

**Impacto:** Bajo a corto plazo (requiere feature nueva). Útil a largo plazo como lectura informativa.

**Recomendación:** Solo leer y exponer la data de BDP como "información disponible", sin crear modelo Glory de menús todavía. Endpoint tipo `GET /api/bdp/menus/:id` que devuelva el JSON raw de BDP.

---

## Fases propuestas

### Fase 9.1 — ExportArticles: Sync de catálogo BDP → Glory 🔴 Alta utilidad

**Qué hace:** Lee todo el catálogo de BDP y sincroniza con Glory.

**Flujo:**
1. Login a BDP
2. `POST /API/Articles/Export` → recibe array de `ArticleExportData` (código, descripción, tarifa1-5, dto, IVA, departamento, familia, etc.)
3. Para cada artículo BDP:
   - Si existe en `bdp_article_map` → actualizar precios, descripción, IVA
   - Si NO existe → crear entrada nueva (opcional: auto-crear en Glory o solo registrar en mapa)
4. Devolver resumen: { creados, actualizados, sin_cambios }

**Modelo Glory actual:** `BdpArticleMap { id, glory_article_id, bdp_art_code, created_at, updated_at }`
**Campos que faltarían en `bdp_article_map`:** `descripcion`, `precio_tarifa1`, `iva_pct`, `departamento`, `familia`, `ultima_sync_at`

**Endpoint Glory nuevo:** `POST /api/bdp/sync-catalog` (o `POST /api/bdp-article-map/sync`)

**Archivos a crear/modificar:**
- `src/services/bdp_sync.rs` — nuevo método `sync_catalog()`
- `src/models/bdp_article_map.rs` — campos nuevos (descripcion, precio, iva, departamento)
- `src/repositories/bdp_article_map.rs` — método `upsert_batch()`
- `migrations/` — ALTER TABLE bdp_article_map ADD COLUMN ...
- `src/handlers/bdp_article_map.rs` — nuevo endpoint `POST sync`
- `src/handlers/mod.rs` — registrar schema/route
- Tests Category A (unit) + Category B (DB)

**Esfuerzo estimado:** 2-3h

---

### Fase 9.2 — GetArticle: Consulta individual de artículo 🟡 Útil

**Qué hace:** Busca un artículo en BDP por código cuando no está en el mapa local.

**Flujo:**
1. `resolve_article()` no encuentra el código en `bdp_article_map`
2. En vez de fallback al artículo default, intenta `POST /API/Articles/Get` con el código
3. Si BDP lo encuentra → crear entrada en `bdp_article_map` + devolver datos
4. Si no → usar artículo default (comportamiento actual)

**Archivos a modificar:**
- `src/services/bdp_sync.rs` — modificar `resolve_article()` para fallback a BDP
- `src/services/bdp_weblink.rs` — método `get_article()` (ya tiene path constante `ARTICLE_GET`)
- `src/services/bdp_weblink_catalog.rs` — `BdpGetArticleRequest { article_code: i32 }`

**Esfuerzo estimado:** ~1h

---

### Fase 9.3 — GetPricesArticles: Refresh de precios 🟡 Útil

**Qué hace:** Actualiza precios de artículos ya mapeados sin reimportar todo.

**Flujo:**
1. Login a BDP
2. `POST /API/Articles/GetPrices` con array de códigos BDP
3. Actualizar `precio_tarifa1` y `iva_pct` en `bdp_article_map`

**Archivos a modificar:**
- `src/services/bdp_sync.rs` — nuevo método `refresh_prices()`
- `src/handlers/bdp_article_map.rs` — nuevo endpoint `POST refresh-prices`

**Esfuerzo estimado:** ~1h

---

### Fase 9.4 — GetRoomTables: Sync de mesas BDP → Glory 🟡 Útil (plano de sala existe)

**Qué hace:** Pre-carga la estructura de mesas del POS desde BDP al plano de sala de Glory.

**Flujo:**
1. Login a BDP
2. `POST /API/Rooms/GetRoomsTables` → array de `RoomTableData`
3. Mapear:
   - `RoomId` → `ZonaSala` (buscar por nombre o crear)
   - `RoomTableData` → `Mesa` (numero, forma, min/max personas, dimensiones)
4. Upsert en tablas Glory (no borrar mesas existentes con reservas)
5. Devolver resumen: { zonas_creadas, mesas_creadas, mesas_actualizadas }

**Modelo BDP (`RoomTableData`):**
```json
{
  "Id": 1, "Name": "Mesa 1", "RoomId": 1, "RoomName": "Sala principal",
  "MinDiners": 2, "MaxDiners": 4, "Shape": 0, "Width": 80, "Height": 80
}
```

**Mapeo BDP → Glory:**
| BDP | Glory | Notas |
|-----|-------|-------|
| `RoomId` + `RoomName` | `ZonaSala { nombre: RoomName }` | Buscar por nombre, crear si no existe |
| `Name` | `Mesa.numero` | Extraer número del nombre ("Mesa 1" → 1) |
| `Shape` | `Mesa.forma` | 0=cuadrada, 1=redonda (verificar) |
| `MinDiners` | `Mesa.min_personas` | Directo |
| `MaxDiners` | `Mesa.max_personas` | Directo |
| `Width`/`Height` | `Mesa.ancho`/`Mesa.alto` | Directo (px del canvas) |

**Endpoint Glory nuevo:** `POST /api/bdp/sync-tables` (o `POST /api/plano-sala/sync-bdp`)

**Archivos a crear/modificar:**
- `src/services/bdp_sync.rs` — nuevo método `sync_room_tables()`
- `src/services/bdp_weblink.rs` — método `get_rooms_tables()` (ya tiene path `ROOM_GET_ROOMS_TABLES`)
- `src/services/bdp_weblink_catalog.rs` — `BdpRoomTableData` response struct
- `src/handlers/plano_sala.rs` — nuevo endpoint `POST sync-bdp`
- Tests

**Esfuerzo estimado:** 2-3h

---

### Fase 9.5 — GetMenuDefinition: Lectura informativa de menús 🟢 Futuro

**Qué hace:** Expone la definición de menús/packs/fast-food de BDP como datos raw.

**Flujo:**
1. `POST /API/Menus/Get` con `{ "MenuId": N }` → devuelve `MenuDataType`
2. `POST /API/FastFoods/Get` con `{ "FastfoodId": N }` → `FastfoodDataType`
3. `POST /API/Packs/Get` con `{ "PackId": N }` → `PackDataType`
4. Devolver el JSON tal cual (sin modelo Glory)

**Endpoint Glory nuevo:** `GET /api/bdp/menus/:id`, `GET /api/bdp/fastfoods/:id`, `GET /api/bdp/packs/:id`

**Archivos a crear/modificar:**
- `src/services/bdp_weblink.rs` — 3 métodos nuevos
- `src/handlers/` — nuevo handler o ampliar `configuracion.rs`

**Esfuerzo estimado:** 1-1.5h (solo lectura, sin modelo Glory)

---

## Orden recomendado de ejecución

```
9.1 ExportArticles (sync catálogo)     ← máximo impacto, bloquea 9.2 y 9.3
  └→ 9.2 GetArticle (fallback)         ← se beneficia de 9.1 (artículos ya mapeados)
  └→ 9.3 GetPricesArticles (refresh)   ← se beneficia de 9.1 (campos de precio existen)
9.4 GetRoomTables (sync mesas)          ← independiente, requiere auth BDP
9.5 GetMenuDefinition (lectura)         ← independiente, bajo impacto
```

## Estimación total

| Fase | Esfuerzo | Dependencia |
|------|----------|-------------|
| 9.1 ExportArticles | 2-3h | — |
| 9.2 GetArticle | 1h | 9.1 (por campos nuevos en mapa) |
| 9.3 GetPricesArticles | 1h | 9.1 (por campos de precio) |
| 9.4 GetRoomTables | 2-3h | — |
| 9.5 GetMenuDefinition | 1-1.5h | — |
| **Total** | **~7-9.5h** | |

## Pre-requisitos para Fase 9.4 y 9.5

- **Auth BDP requerida** (llamadas reales a la API). Se puede hacer el código sin auth (Category A tests con mocks), pero la validación final requiere acceso al TPV.
- Para 9.4: verificar que `GetRoomsTables` funciona (puede estar como `GetPOS` — devuelve error). Alternativa: probar con `GetRoomTables` por sala individual.

## Datos que BDP ofrece por artículo (ExportArticles)

```json
{
  "ArtCode": 1001,
  "Description": "CAFE BOMBON",
  "Family": 1,
  "Subfamily": 1,
  "Department": 1,
  "Tax1": 10.0,
  "Tax2": 0.0,
  "Price1": 2.50, "Price2": 0.0, "Price3": 0.0, "Price4": 0.0, "Price5": 0.0,
  "Discount": 0.0,
  "BarCode": "8412345678901",
  "Active": true
}
```
