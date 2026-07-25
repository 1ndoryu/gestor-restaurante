# Plan: Stock BDP — página individual y solo lectura

> **Fecha:** 2026-07-25
> **Tarea roadmap:** UI4 — Stock BDP
> ** Alcance actual:** página individual `/bdp/stock`, **solo lectura**, con datos provenientes de `ExportArticles`/`CurrentStock` almacenados en `bdp_article_map.stock_actual`.

---

## 1. Alcance actual (solo lectura)

Se mantiene el stock como **información consultiva**. La página muestra lo que BDP devuelve a través del catálogo; no hay escritura sobre el inventario del restaurante.

| Aspecto | Decisión | Justificación |
|---------|----------|---------------|
| **Escritura de stock** |  No permitida | El cliente pidó "ver" stock, no gestionarlo. Las modificaciones se hacen en BDP. |
| **Página propia** | ✅ `/bdp/stock` | Tiene su ruta, menú lateral y URL independiente. |
| **Fuente de datos** | `bdp_article_map.stock_actual` | Se sincroniza con `ExportArticles`/`CurrentStock`. |
| **Sync catálogo** | ✅ Botón visible | Solo lectura desde BDP + upsert local en `bdp_article_map`. No escribe en BDP. |
| **Exportación** | ✅ CSV informativo | Exporta los registros filtrados a CSV localmente; no toca BDP. |

---

## 2. Página `/bdp/stock` (implementada)

### 2.1 Funcionalidades

- **Tabla paginada** de artículos mapeados con su stock.
- **Filtros**: texto libre (código/nombre), stock (con/sin/todos), estado (activo/inactivo/todos).
- **Ordenación** por código Glory, código BDP, nombre, precio y stock.
- **Paginación** configurable (10, 25, 50 por página).
- **Banner de solo lectura** que explica que el stock no se puede modificar desde Glory.
- **Exportación a CSV** de los resultados filtrados/ordenados.
- **Sync catálogo** para refrescar stock desde BDP a Glory.
- **Estados defensivos**: carga, vacío, error y datos inválidos.

### 2.2 Archivos

| Archivo | Responsabilidad |
|---------|-----------------|
| `frontend/src/componentes/bdp/BdpStock.tsx` | Página principal de stock. |
| `frontend/src/hooks/useBdpStockFilters.ts` | Lógica de filtros, ordenación y paginación. |
| `frontend/src/componentes/bdp/bdp-stock-utils.ts` | Formateo de precio/stock/fecha y exportación CSV. |
| `frontend/src/components/app-sidebar.tsx` | Menú lateral con acceso directo a `/bdp/stock`. |

---

## 3. Seguridad y mitigaciones

### 3.1 Sin operaciones de escritura sobre stock

- No existe endpoint `POST /api/bdp/stock/adjust` ni similar.
- La página solo consume `GET /api/bdp/article-maps` (lista de mapeos) y `POST /api/bdp/article-maps/sync-catalog` (importa desde BDP).
- `sync-catalog` es **importación**, no exportación: lee de BDP y actualiza registros locales.

### 3.2 Prevención de manipulación accidental

- **Banner visible** en la parte superior: "Solo lectura".
- **No hay botones de editar/ajustar/eliminar stock** en la interfaz.
- **Sync catálogo** muestra tooltip aclarando que no modifica BDP.

### 3.3 Defensas en datos

- `formatPrice` ignora valores nulos, `0` o no numéricos.
- `formatStock` trata `0`, `null`, `undefined` y strings no numéricos como "sin stock".
- `formatDate` guarda contra fechas inválidas.
- La tabla muestra "—" para datos faltantes en lugar de romper la UI.

### 3.4 Rendimiento

- `useMemo` para filtrado, ordenación y paginación.
- Página con paginación para evitar renderizar miles de filas de una vez.
- Hook y utilidades extraídos para mantener el componente manejable.

### 3.5 Mitigaciones futuras si se amplía el alcance

| Escenario | Mitigación |
|-----------|------------|
| Lectura por almacén (`GetStock`) | Nuevo endpoint `GET /api/bdp/stock` sin escritura; selector de almacén en UI. |
| Gestión de stock (`UpdateStock`) | Feature flag desactivado por defecto + arming + confirmación textual + idempotencia + audit log. |
| Race conditions en ajustes | Lock por `article_map_id + warehouse_id`. |
| Descuadre de inventario | Tabla de ajustes propios + audit log. |

---

## 4. Opciones futuras (pendientes de decisión del cliente)

### 4.1 Opción A — Stock por almacén (solo lectura)

Mostrar stock desglosado por almacén usando `GetStock`/`GetListStock`.

- **Endpoints BDP**: `GetStock`, `GetListStock`, `GetWarehouses`.
- **Cambios backend**: nuevos handlers `GET /api/bdp/stock` y `GET /api/bdp/warehouses`.
- **Cambios frontend**: selector de almacén, columnas de stock por almacén.
- **Esfuerzo estimado**: ~6-8h.
- **Riesgo**: bajo.

### 4.2 Opción B — Gestión de stock (escritura en BDP)

Permitir actualizar stock en BDP vía `UpdateStock`/`Regularizations`.

- **Requisitos de seguridad**: feature flag desactivado, arming, confirmación textual, idempotencia, audit log, permisos de admin/gerente, throttling.
- **Cambios backend**: servicio `BdpStockService::adjust`, handler `POST /api/bdp/stock/adjust`, tabla `bdp_stock_adjustments`.
- **Cambios frontend**: modal de ajuste, permisos, confirmación.
- **Esfuerzo estimado**: ~16-24h.
- **Riesgo**: alto.

---

## 5. Decisiones pendientes del cliente

| ID | Pregunta | Opción recomendada por defecto |
|----|----------|-------------------------------|
| D1 | ¿Necesitas ver stock por almacén? | No — mantener página consolidada actual. |
| D2 | ¿Quieres poder ajustar stock desde Glory? | No — solo lectura hasta nuevo aviso. |

---

## 6. Próximos pasos inmediatos

1. ✅ Implementar página `/bdp/stock` solo lectura con filtros, ordenación y paginación.
2. Validar con `type-check` y `build` del frontend.
3. Actualizar roadmap (`UI4`) a "Implementado (solo lectura)".
4. Si el cliente lo solicita, evaluar Opción A o B con sus respectivas salvaguardas.

---

## 7. Esfuerzo real invertido

| Tarea | Esfuerzo |
|-------|----------|
| Diseño de página read-only con seguridad y mitigaciones | ~1h |
| Implementación de `BdpStock.tsx`, hook y utilidades | ~2h |
| Type-check, build y revisiones | ~1h |
| Documentación del plan | ~30min |
| **Total** | **~4.5h** |
