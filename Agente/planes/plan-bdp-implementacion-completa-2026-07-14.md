# Plan: Implementación completa BDP WebLink REST API

> **Fecha:** 2026-07-14 (v2 — revisado con impacto frontend completo)
> **Estado:** � En progreso — Fases 1-4 backend completas (4/6 fases)
> **Riesgo:** ALTO — la integración actual funciona en producción. Cualquier cambio debe ser retrocompatible.

---

## 0. Principios rectores

1. **No romper lo que funciona.** `CreateOrder` con 1 artículo genérico hoy envía comandas a BDP. Cada fase se despliega de forma independiente y retrocompatible.
2. **OnlyCheck antes de escritura real.** Todo cambio al payload de `CreateOrder` se prueba con `OnlyCheck=true` (preflight) antes de escribir en BDP.
3. **Configuración gradual.** Los nuevos campos de config tienen defaults que replican el comportamiento actual. Si no se configuran, el flujo no cambia.
4. **Sin acceso al TPV.** No podemos tocar la configuración de BDP-NET en el POS. Todo lo que requiera cambios en el TPV queda documentado como "tarea cliente".
5. **Un commit por subtarea.** Si algo sale mal, se revierte un cambio pequeño, no toda la fase.

### ⚠️ REGLA CRÍTICA: Control de despliegues y pruebas

> **PROHIBIDO** realizar despliegues a producción o llamadas a la API de BDP sin autorización explícita del usuario.

| Acción                                                              | Permitido | Requiere autorización     |
| ------------------------------------------------------------------- | --------- | ------------------------- |
| Implementar código Rust (modelos, migraciones, servicios, handlers) | ✅ Sí     | —                         |
| Compilar localmente (`cargo check`, `cargo build`)                  | ✅ Sí     | —                         |
| Tests unitarios que NO llamen a BDP                                 | ✅ Sí     | —                         |
| Regenerar Orval codegen localmente                                  | ✅ Sí     | —                         |
| Implementar componentes React/frontend                              | ✅ Sí     | —                         |
| Deploy a producción (nakomi.studio)                                 | ❌        | ✅ Autorización requerida |
| Llamadas a API BDP (Login, CreateOrder, GetOrder, etc.)             | ❌        | ✅ Autorización requerida |
| Pruebas contra el TPV real (preflight, dry-run, escritura)          | ❌        | ✅ Autorización requerida |
| Crear comandas reales en BDP                                        | ❌        | ✅ Autorización requerida |

**Flujo de autorización:**

1. Implementar todo el código sin llamar a BDP
2. Compilar y validar localmente (cargo check, tests unitarios)
3. Presentar resumen de cambios al usuario
4. **Esperar autorización** para: deploy, pruebas contra BDP, pruebas en producción
5. Las pruebas contra BDP NO deben crear datos reales (comandas, clientes) que el cliente no espere

---

## 1. Estado actual (qué funciona hoy)

### Backend

| Componente          | Estado      | Detalle                                                          |
| ------------------- | ----------- | ---------------------------------------------------------------- |
| Login BDP           | ✅          | JWT con re-login automático                                      |
| CreateOrder         | ✅          | 1 artículo genérico, Type=0 (Barra), sin pagos                   |
| Serie `00031TI`     | ✅          | IVA incluido, asignada a POS 31                                  |
| Error 300035        | ✅ RESUELTO | Serie creada, cliente confirmó                                   |
| MarketplaceOrderId  | ✅          | Max 15 chars, prefijo `G`                                        |
| Preflight dry-run   | ✅          | OnlyCheck sin escritura                                          |
| Sync tracking en BD | ✅          | `bdp_synced`, `bdp_order_id`, `bdp_sync_error` en tabla `ventas` |
| Retry manual        | ✅          | `POST /api/ventas/:id/bdp-sync` existe en backend                |
| Multi-item          | ❌          | 1 sola línea por venta                                           |
| Mapeo artículos     | ❌          | Todo → `CAFE BOMBON` (1001)                                      |
| Cliente en comanda  | ❌          | No se envía `Customer`                                           |
| Pagos en comanda    | ❌          | `Payments[]` vacío                                               |
| Canal → Type        | ❌          | Siempre 0 (Barra)                                                |
| Polling estado      | ✅         | `bdp_order_poller.rs` — GetOrder polling + mapeo estados (F4.2)  |
| CancelOrder         | ❌          | API devuelve "Subscripción no activada"                          |

### Frontend

| Componente                 | Estado     | Detalle                                                                                                  |
| -------------------------- | ---------- | -------------------------------------------------------------------------------------------------------- |
| `ConfigBdp.tsx`            | ✅ Parcial | Credenciales + diagnóstico + dry-run. **Sin mapeos** (artículos, tenders, types)                         |
| `HaddockSyncBadge`         | ✅         | Badge visual para sync Haddock. **No existe equivalente BDP**                                            |
| `ListaVentas.tsx`          | ⚠️         | Columna Haddock ✅, pero **sin columna BDP**, sin filtro BDP, sin retry BDP                              |
| `FormularioVenta.tsx`      | ❌         | **Sin soporte para líneas de venta** — crea venta monolítica                                             |
| `VentaConCliente` (schema) | ⚠️         | **Faltan campos BDP** — `bdp_synced`, `bdp_order_id`, `bdp_sync_error` no están en el schema TS generado |
| Orval codegen              | ⚠️         | **Desactualizado** — no incluye campos BDP de `VentaConCliente` ni endpoint retry BDP                    |
| Hook `useListaVentas`      | ⚠️         | Tiene retry Haddock, **no retry BDP**                                                                    |
| Hook `useConfiguracion`    | ✅         | Guarda campos BDP correctamente                                                                          |

### Problema crítico: Orval codegen desactualizado

El schema TS generado (`gestionRestauranteAPI.schemas.ts`) NO tiene `bdp_synced`, `bdp_order_id`, `bdp_sync_error` en `VentaConCliente`. El backend los devuelve pero el frontend no los tipa. Esto significa que **ni siquiera podemos mostrar el estado BDP actual** sin regenerar el codegen.

### Arquitectura relevante

| Archivo                               | Líneas | Rol                                                      |
| ------------------------------------- | ------ | -------------------------------------------------------- |
| `src/services/bdp_sync.rs`            | ~480   | Orquestación: login → build_order → create_order → retry |
| `src/services/bdp_weblink.rs`         | ~750   | Cliente HTTP: 23 métodos, token management               |
| `src/services/bdp_weblink_catalog.rs` | ~200   | Constantes, structs request/response                     |
| `src/services/bdp_sync_preflight.rs`  | ~460   | 9 checks + dry-run CreateOrder                           |
| `src/models/venta.rs`                 | ~160   | Modelo Venta (monolítico, sin líneas)                    |
| `src/models/configuracion.rs`         | ~100   | Config BDP en tabla `configuracion`                      |

### Dato crítico: no existe `VentaLinea`

Las ventas en Glory son **monolíticas**: 1 registro con `importe_base` + `importe_iva`. No hay tabla `venta_lineas`. Para enviar múltiples artículos a BDP necesitamos crear esa tabla (migración + modelo + API + frontend).

---

## 2. Plan por fases

### Fase 1 — Configuración y mapeos (sin cambiar el flujo)

**Objetivo:** Preparar la infraestructura de configuración sin tocar `CreateOrder`. Si se despliega esta fase sola, el comportamiento es idéntico al actual.

| #   | Subtarea                                                                                                                   | Archivos                             | Riesgo | Effort |
| --- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ | ------ | ------ |
| 1.1 | **Nueva tabla `bdp_article_map`** — mapeo código Glory → código BDP                                                        | Nueva migración, modelo, repositorio | BAJO   | 2h     |
| 1.2 | **Nuevos campos en `configuracion`** — `bdp_tender_map` (jsonb), `bdp_order_type_map` (jsonb), `bdp_default_customer_code` | Migración, `models/configuracion.rs` | BAJO   | 1h     |
| 1.3 | **Endpoint admin: mapeo artículos** — CRUD para `bdp_article_map`                                                          | Handler, servicio                    | BAJO   | 2h     |
| 1.4 | **Endpoint admin: mapeo tenders** — CRUD para tender_map (efectivo→1, tarjeta→2, bizum→5)                                  | Handler, servicio                    | BAJO   | 1.5h   |
| 1.5 | **Endpoint admin: mapeo canal→Type** — CRUD para order_type_map (barra→0, comedor→1, terraza→0)                            | Handler, servicio                    | BAJO   | 1.5h   |
| 1.6 | **Tests unitarios** para todos los mapeos                                                                                  | `tests/`                             | BAJO   | 1h     |

**Total Fase 1:** ~9h
**Validación:** Deploy + verificar que la sync actual sigue funcionando igual.

**Notas para el cliente:**

- Necesitamos el catálogo real de artículos de BDP para poblar `bdp_article_map`. El endpoint `ExportArticles` o `GetPOSArticlesList` devuelve los artículos disponibles.
- Los IDs de tender se obtienen con `GetPOSTenderList` (ya probado en preflight).

---

### Fase 2 — Multi-item (el cambio más visible)

**Objetivo:** Que cada línea de una venta llegue como artículo separado a BDP.

| #   | Subtarea                                                                                                                                        | Archivos                                    | Riesgo | Effort |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ------ | ------ |
| 2.1 | **Nueva tabla `venta_lineas`** — FK a `ventas`, campos: `articulo_codigo`, `descripcion`, `cantidad`, `precio_unitario`, `iva_pct`, `descuento` | Nueva migración, modelo                     | MEDIO  | 2h     |
| 2.2 | **Modelo `VentaLinea`** en Rust                                                                                                                 | `models/venta.rs` o `models/venta_linea.rs` | BAJO   | 1h     |
| 2.3 | **Modificar `CrearVentaRequest`** — aceptar `lineas: Vec<CrearLineaRequest>` (opcional, retrocompatible)                                        | `models/venta.rs`, handler                  | MEDIO  | 1.5h   |
| 2.4 | **Repositorio: CRUD líneas** — crear, leer, borrar por venta                                                                                    | `repositories/venta_linea.rs`               | BAJO   | 1.5h   |
| 2.5 | **Modificar `bdp_sync.rs::build_order()`** — si hay líneas, iterar; si no, fallback al artículo genérico actual                                 | `services/bdp_sync.rs`                      | ALTO   | 3h     |
| 2.6 | **Modificar `resolve_article()`** — usar `bdp_article_map` si existe, fallback a `bdp_default_article_code`                                     | `services/bdp_sync.rs`                      | MEDIO  | 2h     |
| 2.7 | **Preflight: validar mapeo** — nuevo check que verifica que todas las líneas tienen artículo BDP mapeado                                        | `services/bdp_sync_preflight.rs`            | BAJO   | 1.5h   |
| 2.8 | **Tests: build_order con 1, 3 y 0 líneas**                                                                                                      | `tests/`                                    | BAJO   | 1.5h   |

**Total Fase 2:** ~14h
**Validación:** OnlyCheck con múltiples líneas → verificar que BDP acepta el payload.

**Riesgo principal:** `build_order()` es la función más delicada. Se modifica con fallback: si `venta_lineas` está vacío, usa el comportamiento actual (1 artículo genérico). Esto garantiza que ventas existentes no se rompen.

**Notas para el cliente:**

- Necesitamos saber qué artículos del catálogo Glory corresponden a qué artículos de BDP. Esto se puede hacer automáticamente con `ExportArticles` + coincidencia por nombre, o manualmente desde el admin.

---

### Fase 3 — Cliente y pagos en comanda

**Objetivo:** Enviar datos de cliente y forma de pago a BDP.

| #   | Subtarea                                                                                         | Archivos                         | Riesgo | Effort |
| --- | ------------------------------------------------------------------------------------------------ | -------------------------------- | ------ | ------ |
| 3.1 | **Modificar `build_order()`** — si `venta.cliente_id` existe, lookup cliente y enviar `Customer` | `services/bdp_sync.rs`           | MEDIO  | 2h     |
| 3.2 | **Modificar `build_order()`** — mapear `metodo_pago` → `TenderId` vía `bdp_tender_map`           | `services/bdp_sync.rs`           | MEDIO  | 1.5h   |
| 3.3 | **Modificar `build_order()`** — mapear `canal` → `Type` vía `bdp_order_type_map`                 | `services/bdp_sync.rs`           | MEDIO  | 1h     |
| 3.4 | **Preflight: check tender** — verificar que el tender mapeado existe en el POS                   | `services/bdp_sync_preflight.rs` | BAJO   | 1h     |
| 3.5 | **Preflight: check order type** — verificar que el Type mapeado es válido                        | `services/bdp_sync_preflight.rs` | BAJO   | 1h     |
| 3.6 | **Tests: build_order con cliente, pago y canal**                                                 | `tests/`                         | BAJO   | 1.5h   |

**Total Fase 3:** ~8h
**Validación:** OnlyCheck con cliente + pago + canal → verificar aceptación.

**Riesgo principal:** `Type=0` (Barra) es el único que sabemos que funciona en POS 31. Otros types (1=Mesa, 2=Comedor) pueden dar error 300008/300009. Se prueba con OnlyCheck primero.

**Notas para el cliente:**

- Los IDs de tender se obtienen del POS con `GetPOSTenderList`.
- Si el cliente no tiene datos (no hay `cliente_id`), se omite el campo `Customer` — comportamiento actual.

---

### Fase 4 — Lifecycle de comandas (polling + estados)

**Objetivo:** Saber en Glory si una comanda fue cobrada/facturada en el TPV.

| #   | Subtarea                                                                                                                     | Archivos                              | Riesgo | Effort | Estado |
| --- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- | ------ | ------ | ------ |
| 4.1 | **Nuevo campo `bdp_order_status`** en `ventas`                                                                               | Migración, modelo                     | BAJO   | 1h     | ✅     |
| 4.2 | **Servicio de polling** — consulta `GetOrder` periódicamente para ventas con `bdp_synced=true` y `bdp_order_status` no final | Nuevo: `services/bdp_order_poller.rs` | MEDIO  | 3h     | ✅     |
| 4.3 | **Endpoint manual: consultar estado** — `GET /api/ventas/:id/bdp-status`                                                     | Handler                               | BAJO   | 1h     | ✅     |
| 4.4 | **Reflejar facturación** — si `GetOrder` devuelve status=3 (facturada), marcar venta como cobrada                            | `services/bdp_order_poller.rs`        | MEDIO  | 2h     | ✅     |
| 4.5 | **Configuración: intervalo de polling** — `bdp_poll_interval_secs` en config (default 60)                                    | Migración, config                     | BAJO   | 0.5h   | ✅     |
| 4.6 | **Tests: polling con estados simulados**                                                                                     | `tests/`                              | BAJO   | 1.5h   | ⚠️*    |

**Total Fase 4:** ~9h
**Validación:** Crear venta → verificar que el polling actualiza el estado.

> *4.6: Test `test_map_status()` implementado y compilando. `cargo test` no ejecutable por errores preexistentes en `middleware/auth.rs` (Claims sin campos `role`/`effective_role`/`impersonator`/`trabajador_id`). Los tests correrán cuando se corrija el template base.

**Restricción conocida:** `CancelOrder` devuelve "Subscripción no activada". No se implementa cancelación hasta que BDP active el endpoint. Se documenta como limitación.

---

### Fase 5 — Frontend: visibilidad BDP (sin multi-item aún)

**Objetivo:** Que el usuario vea el estado de la sincronización BDP y pueda reintentar. No incluye multi-item (eso es Fase 6).

**Depende de:** Fase 1 (para mapeos en UI) + Orval codegen regenerado.

| #   | Subtarea                                                                                                                                                | Archivos                                              | Riesgo | Effort |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------ | ------ |
| 5.0 | **Regenerar Orval codegen** — `cd frontend && npm run codegen`. Esto actualiza `VentaConCliente` con campos BDP y genera hooks para endpoints nuevos    | `frontend/src/api/generated/`                         | MEDIO  | 0.5h   |
| 5.1 | **`BdpSyncBadge`** — componente visual (✅/❌/⏳) equivalente a `HaddockSyncBadge`, con tooltip con `bdp_order_id` y `bdp_sync_error`                   | Nuevo: `frontend/src/components/bdp-sync-badge.tsx`   | BAJO   | 1h     |
| 5.2 | **Columna BDP en `ListaVentas`** — añadir columna con `BdpSyncBadge`, igual que la columna Haddock existente                                            | `frontend/src/componentes/ListaVentas.tsx`            | BAJO   | 1.5h   |
| 5.3 | **Filtro BDP en `ListaVentas`** — `estadoBdp` (synced/error/pending) usando `ColumnFilterHeader` existente                                              | `frontend/src/componentes/ListaVentas.tsx`, hook      | BAJO   | 1h     |
| 5.4 | **Botón retry BDP en `ListaVentas`** — como el botón Haddock pero para BDP. Necesita hook `useRetryBdpSync` que llame a `POST /api/ventas/:id/bdp-sync` | `frontend/src/components/venta-row-actions.tsx`, hook | BAJO   | 1h     |
| 5.5 | **Hook `useRetryBdpSync`** — mutation que llama al endpoint retry BDP                                                                                   | Nuevo o ampliar `useListaVentas.ts`                   | BAJO   | 0.5h   |
| 5.6 | **`ConfigBdp` expandir** — añadir sección de mapeos: tabla de artículos Glory→BDP, selector de tenders, selector de order type                          | `frontend/src/componentes/ConfigBdp.tsx`              | MEDIO  | 3h     |
| 5.7 | **`ConfigBdp` expandir** — botón "Importar catálogo BDP" que ejecuta `ExportArticles` y precarga la tabla de mapeo                                      | `frontend/src/componentes/ConfigBdp.tsx`              | MEDIO  | 2h     |
| 5.8 | **`useConfiguracion` expandir** — añadir campos nuevos: `bdp_tender_map`, `bdp_order_type_map`, `bdp_default_customer_code`                             | `frontend/src/hooks/useConfiguracion.ts`              | BAJO   | 1h     |
| 5.9 | **Tests visuales** — verificar badge, columna, filtro, retry en ListaVentas                                                                             | Manual                                                | BAJO   | 0.5h   |

**Total Fase 5:** ~12h
**Validación:** Verificar que la columna BDP aparece, el filtro funciona, el retry envía la venta, y los mapeos se guardan.

**Riesgo principal:** Regenerar Orval codegen puede generar cambios en otros tipos si el OpenAPI spec cambió. Se hace como primer paso y se verifica diff antes de continuar.

---

### Fase 6 — Frontend: multi-item y formulario de ventas

**Objetivo:** Que el usuario pueda crear ventas con múltiples líneas y que cada línea se mapee a un artículo BDP.

**Depende de:** Fase 2 (backend `venta_lineas` + `build_order` multi-item).

| #   | Subtarea                                                                                                                                                                                                                         | Archivos                                                | Riesgo | Effort |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- | ------ | ------ |
| 6.1 | **Componente `LineasVentaEditor`** — editor de líneas dentro del formulario de venta. Cada línea: selector de artículo, cantidad, precio unitario, IVA%, descuento. Botón añadir/eliminar línea. Total calculado automáticamente | Nuevo: `frontend/src/componentes/LineasVentaEditor.tsx` | MEDIO  | 4h     |
| 6.2 | **CSS `LineasVentaEditor`** — estilos responsive (mobile ≥320px, tablet, desktop)                                                                                                                                                | Nuevo: `frontend/src/styles/lineas-venta-editor.css`    | BAJO   | 1h     |
| 6.3 | **Integrar en `FormularioVenta`** — renderizar `LineasVentaEditor` debajo de los campos existentes. Si hay líneas, los importes se calculan de las líneas. Si no hay líneas, se usan los campos manuales (retrocompatible)       | `frontend/src/componentes/FormularioVenta.tsx`          | MEDIO  | 2h     |
| 6.4 | **Hook `useFormularioVenta` expandir** — manejar estado de líneas, validación (mínimo 1 línea si se usan líneas), cálculo de totales                                                                                             | `frontend/src/hooks/useFormularioVenta.ts`              | MEDIO  | 2h     |
| 6.5 | **Modificar `CrearVentaRequest`** — enviar `lineas` al backend (campo opcional)                                                                                                                                                  | `frontend/src/api/generated/` (codegen)                 | BAJO   | 0.5h   |
| 6.6 | **Indicador de mapeo BDP** — en cada línea, mostrar si el artículo tiene mapeo BDP (✅/⚠️). Si no tiene mapeo, warning de que usará artículo genérico                                                                            | `LineasVentaEditor.tsx`                                 | BAJO   | 1.5h   |
| 6.7 | **Selector de artículo con búsqueda** — input con autocomplete que busca en el catálogo Glory (o BDP si está importado)                                                                                                          | `LineasVentaEditor.tsx`                                 | MEDIO  | 2h     |
| 6.8 | **Tests visuales** — crear venta con 1, 3 líneas, verificar totales, verificar que BDP recibe múltiples artículos                                                                                                                | Manual                                                  | BAJO   | 1h     |

**Total Fase 6:** ~14h
**Validación:** Crear venta con 3 líneas → verificar que BDP recibe 3 `OrderItems` separados.

**Riesgo principal:** El `FormularioVenta` actual es monolítico (un solo total). Añadir líneas cambia el flujo de creación. Se mantiene retrocompatibilidad: si no se añaden líneas, el comportamiento actual se preserva.

---

## 3. Resumen de esfuerzo

| Fase                            | Horas    | Riesgo     | Dependencias     |
| ------------------------------- | -------- | ---------- | ---------------- |
| 1 — Config y mapeos (backend)   | ~9h      | BAJO       | Ninguna          |
| 2 — Multi-item (backend)        | ~14h     | MEDIO-ALTO | Fase 1           |
| 3 — Cliente y pagos (backend)   | ~8h      | MEDIO      | Fase 1           |
| 4 — Lifecycle polling (backend) | ~9h      | MEDIO      | Fase 2+3         |
| 5 — Frontend visibilidad BDP    | ~12h     | BAJO-MEDIO | Fase 1 + codegen |
| 6 — Frontend multi-item         | ~14h     | MEDIO      | Fase 2 + Fase 5  |
| **Total**                       | **~66h** |            |                  |

---

## 4. Tareas que requiere el cliente (no podemos hacer nosotros)

| #   | Qué                                                     | Por qué                                                       | Cómo obtenerlo                                          |
| --- | ------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------- |
| C1  | **Lista de artículos BDP con códigos**                  | Para poblar `bdp_article_map`                                 | Ejecutar `GetPOSArticlesList` o pedir export al técnico |
| C2  | **IDs de formas de pago**                               | Para poblar `bdp_tender_map`                                  | Ejecutar `GetPOSTenderList` (ya probado)                |
| C3  | **Confirmar qué Order.Type funciona**                   | Type=0 funciona, otros pueden fallar                          | Probar con OnlyCheck o preguntar al técnico             |
| C4  | **Activar CancelOrder**                                 | Devuelve "Subscripción no activada"                           | Técnico BDP debe activar el módulo                      |
| C5  | **Decisión: ¿artículos genéricos o catálogo completo?** | Define si necesitamos Fase 2 completa o un mapeo simplificado | Reunión con cliente                                     |

---

## 5. Secuencia de ejecución recomendada

```
Fase 1 (9h) — Backend: tablas de mapeo → Deploy → Verificar sync actual intacta
    ↓
Fase 5.0-5.5 (4h) — Frontend: codegen + badge + columna + filtro + retry BDP
    ↓   (ya podemos ver el estado BDP en el panel)
Fase 3 (8h) — Backend: cliente + pagos + canal → Deploy → OnlyCheck
    ↓
Fase 5.6-5.8 (6h) — Frontend: expandir ConfigBdp con mapeos
    ↓   (ya podemos configurar tenders, types, importar catálogo)
Fase 2 (14h) — Backend: multi-item + venta_lineas → Deploy → OnlyCheck multi-item
    ↓
Fase 4 (9h) — Backend: polling de estado → Deploy
    ↓
Fase 6 (14h) — Frontend: editor de líneas + selector de artículos
    ↓   (flujo completo end-to-end)
```

**Secuencia alternativa (mínimo viable):**
Si el cliente quiere resultados rápidos:

```
Fase 1 → Fase 5.0-5.5 → Fase 3 → DONE (comandas con cliente+pago, visibles en panel)
Luego: Fase 2 → Fase 6 (multi-item)
```

Cada fase se puede deployar independientemente. Si Fase 2 tiene problemas, Fases 1+3+5 siguen funcionando.

---

## 6. Riesgos y mitigaciones

### Backend

| Riesgo                                                   | Probabilidad | Impacto | Mitigación                                                       |
| -------------------------------------------------------- | ------------ | ------- | ---------------------------------------------------------------- |
| `build_order()` con multi-item rompe comandas existentes | MEDIA        | ALTO    | Fallback: si no hay líneas, usa comportamiento actual            |
| `Type != 0` falla en POS 31                              | ALTA         | MEDIO   | OnlyCheck antes de escritura. Default = 0 (Barra)                |
| `bdp_article_map` vacío → artículos no mapeados          | MEDIA        | MEDIO   | Fallback a `bdp_default_article_code`. Preflight alerta          |
| `CancelOrder` sigue sin funcionar                        | ALTA         | BAJO    | No dependemos de él. Documentar como limitación                  |
| Polling genera carga en BDP                              | BAJA         | BAJO    | Intervalo configurable (default 60s), solo para ventas recientes |
| Migración `venta_lineas` rompe ventas existentes         | BAJA         | ALTO    | Tabla nueva con FK, no modifica `ventas`. Líneas son opcionales  |

### Frontend

| Riesgo                                                  | Probabilidad | Impacto | Mitigación                                                                                   |
| ------------------------------------------------------- | ------------ | ------- | -------------------------------------------------------------------------------------------- |
| Orval codegen rompe tipos existentes                    | MEDIA        | ALTO    | Hacer diff del codegen como primer paso. Si hay breaking changes, adaptar antes de continuar |
| `LineasVentaEditor` rompe flujo de crear venta          | MEDIA        | MEDIO   | Líneas son opcionales. Si no se añaden, el formulario actual funciona igual                  |
| Mapeos en UI no se guardan correctamente                | BAJA         | MEDIO   | Validar que `useConfiguracion` envía los campos nuevos. Tests manuales post-deploy           |
| `BdpSyncBadge` confunde con `HaddockSyncBadge`          | BAJA         | BAJO    | Diferenciar visualmente: BDP usa icono diferente (ej: `ArrowRightLeft` en vez de `Check`)    |
| Columna BDP + Haddock = tabla demasiado ancha en mobile | MEDIA        | BAJO    | Ocultar columnas menos importantes en mobile (responsive). Usar `hidden md:table-cell`       |
| `FormularioVenta` con líneas supera 300 líneas          | MEDIA        | MEDIO   | Extraer `LineasVentaEditor` a componente separado (ya previsto en plan)                      |

---

## 7. Checklist pre-implementación

### Backend

- [ ] Confirmar con cliente: ¿quieren catálogo completo o artículos genéricos mejorados?
- [ ] Ejecutar `GetPOSArticlesList` para obtener catálogo real de BDP
- [ ] Ejecutar `GetPOSTenderList` para obtener IDs de formas de pago
- [ ] Verificar que `OnlyCheck` sigue funcionando (no ha cambiado la API)
- [ ] Decidir: ¿Fase 1 sola o Fase 1+3 juntas?

### Frontend

- [ ] Regenerar Orval codegen y verificar diff (Fase 5.0)
- [ ] Confirmar que `bdp_synced`/`bdp_order_id`/`bdp_sync_error` aparecen en `VentaConCliente` tras codegen
- [ ] Verificar que el endpoint `POST /api/ventas/:id/bdp-sync` está documentado en OpenAPI (para que Orval genere el hook)
- [ ] Decidir si `LineasVentaEditor` usa selector de artículos del catálogo Glory o del catálogo BDP importado

---

## 8. Inventario de archivos frontend afectados

| Archivo                                          | Qué cambia                                                 | Fase    |
| ------------------------------------------------ | ---------------------------------------------------------- | ------- |
| `frontend/src/api/generated/**/*.ts`             | Regenerar con Orval (nuevos tipos, hooks)                  | 5.0     |
| `frontend/src/components/bdp-sync-badge.tsx`     | **NUEVO** — badge visual BDP                               | 5.1     |
| `frontend/src/componentes/ListaVentas.tsx`       | +columna BDP, +filtro BDP, +retry BDP                      | 5.2-5.4 |
| `frontend/src/components/venta-row-actions.tsx`  | +botón retry BDP                                           | 5.4     |
| `frontend/src/componentes/ConfigBdp.tsx`         | +mapeos (artículos, tenders, types), +importar catálogo    | 5.6-5.7 |
| `frontend/src/hooks/useConfiguracion.ts`         | +campos nuevos (tender_map, order_type_map, customer_code) | 5.8     |
| `frontend/src/hooks/useListaVentas.ts`           | +retry BDP mutation, +filtro estadoBdp                     | 5.3-5.5 |
| `frontend/src/componentes/LineasVentaEditor.tsx` | **NUEVO** — editor de líneas de venta                      | 6.1     |
| `frontend/src/styles/lineas-venta-editor.css`    | **NUEVO** — estilos del editor                             | 6.2     |
| `frontend/src/componentes/FormularioVenta.tsx`   | +integrar LineasVentaEditor                                | 6.3     |
| `frontend/src/hooks/useFormularioVenta.ts`       | +manejo de líneas, cálculo de totales                      | 6.4     |

---

## 9. Archivos de referencia

| Archivo                                                                 | Contenido                                                   |
| ----------------------------------------------------------------------- | ----------------------------------------------------------- |
| `Agente/documentacion/api/bdp-integration-status-2026-06-07.md`         | Inventario completo de endpoints, gap analysis, prioridades |
| `Agente/documentacion/api/bdp-cambios-analisis-problemas-2026-06-08.md` | Análisis de los 4 problemas del cliente y su resolución     |
| `Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md`    | Resolución del error 300035                                 |
| `frontend/src/components/haddock-sync-badge.tsx`                        | Referencia para crear `BdpSyncBadge` (patrón idéntico)      |
| `frontend/src/componentes/ConfigBdp.tsx`                                | Componente BDP existente a expandir                         |
| `frontend/src/componentes/ListaVentas.tsx`                              | Tabla de ventas donde añadir columna/filtro/retry BDP       |
