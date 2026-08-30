# Plan — Verificación de las 24 funciones de lectura BDP "en uso" (LECTURA REAL)

> **Fecha:** 2026-08-18
> **Rama:** `glory-rs-rest` (git pendiente de reparación por otro agente — sin operaciones git)
> **Alcance:** comprobar contra el **BDP REAL del restaurante** que las 24 funciones de lectura marcadas "✅ En uso" en la tabla final responden correctamente. **Solo lecturas — cero escrituras, cero cambios en BDP, sin deploy.**
> **Destino BDP:** `http://100.83.196.35:8068` (Tailscale `restaurante-bdp`)
> **Fuente de la tabla:** inventario verificado contra `# WEBLINK RESTAPI.md`, `bdp_weblink.rs`, `bdp_weblink_catalog.rs` y call sites (sesión 2026-08-18).

## 1. Objetivo

Ejecutar las 24 funciones de lectura "en uso" contra el BDP real (una a la vez, siguiendo la guía del cliente: detenerse ante el primer resultado inesperado) y dejar un reporte por función con evidencia. Detectar regresiones de las fases F0–F10 (128A-1) en los flujos de lectura.

## 2. Inventario de las 24 funciones (alcance)

| # | Función | Método cliente | Flujo que la usa |
| --- | --- | --- | --- |
| 1 | `ServiceHealth` | `health()` | Preflight / probar conexión |
| 2 | `GetVersion` | `get_version()` | Diagnóstico de conexión |
| 3 | `Login` | `login()` | Sesión base de toda llamada |
| 4 | `GetArticle` | `get_article()` | Resolución de artículo al enviar venta |
| 5 | `GetPricesArticles` | `get_prices_articles()` | Refresh de precios |
| 6 | `ExportArticles` | `export_articles()` | Sync catálogo / tablas de mapeos |
| 7 | `GetPOSArticlesList` | `get_pos_articles()` | Preflight y artículos por perfil TPV |
| 8 | `ExportCustomers` | `export_customers()` | Importar clientes |
| 9 | `GetOrder` | `get_order()` | Polling y reconciliación |
| 10 | `ExportDepartment` | `export_departments()` | Sync departamentos |
| 11 | `DepartmentsExportFromProfile` | `export_departments_from_profile()` | Departamentos por perfil |
| 12 | `GetMenuDefinition` | `get_menu_definition()` | Explorador |
| 13 | `GetFastfoodDefinition` | `get_fastfood_definition()` | Explorador |
| 14 | `GetPackDefinition` | `get_pack_definition()` | Explorador |
| 15 | `GetPOS` | `get_pos()` | Preflight / mapeos |
| 16 | `GetPOSes` | `get_poses()` | Terminales disponibles |
| 17 | `GetEmployee` | `get_employee()` | Preflight |
| 18 | `GetEmployees` | `get_employees()` | Preflight / Explorador |
| 19 | `GetPOSEmployees` | `get_pos_employees()` | Preflight |
| 20 | `GetPOSTenderList` | `get_pos_tenders()` | Formas de pago del terminal |
| 21 | `ExportPurchaseNotes` | `export_purchase_notes()` | Compras (sync albaranes) |
| 22 | `GetStock` | `get_stock()` | Stock por almacén (N6, path especulativo) |
| 23 | `GetListStock` | `get_list_stock()` | Stock por almacén (N6, path especulativo) |
| 24 | `GetRoomTables` / `GetRoomsTables` | `get_room_tables()` / `get_rooms_tables()` | Sync plano de sala |

## 3. No-alcance

- **Escrituras BDP** (pago, factura, CancelOrder, las 15 nuevas) — prohibidas en esta sesión.
- Nada que modifique datos del restaurante (armings, allowlists de escritura, syncs que escriban).
- Deploy, producción (servidor web), migraciones, git.
- Haddock.

## 4. Pre-requisitos (F0)

- [ ] BDP online: `tailscale status` → `restaurante-bdp` / `100.83.196.35` activo. **BLOQUEADO — el restaurante no se ha conectado (online + credenciales sin confirmar).**
- [ ] Credenciales y config en `.env` local: `BDP_BASE_URL=http://100.83.196.35:8068`, `BDP_POS_ID=31`, usuario/clave del integrador `VBW2MBM5`. **Pendiente de confirmar con el cliente.**
- [x] `cargo check` OK (baseline de compilación) — verificado 2026-08-19 (`cargo check --lib --tests` limpio, exit 0).
- [x] Suite unit local (opcional, baseline) — verificado 2026-08-19 (`cargo test --lib`: **153 passed**, 0 failed).

## 5. Fases

### F1 — Ejecución de lecturas reales (una a la vez)
Para cada una de las 24 funciones, usando el cliente real (`BdpWeblinkClient`) contra `100.83.196.35:8068`:
- [ ] Llamada puntual con payload mínimo según contrato del manual.
- [ ] Validar respuesta tipada (parseo) y contenido esperado.
- [ ] Registrar evidencia: función, payload, HTTP status, resumen de la respuesta (sin secretos ni datos personales completos), resultado.
- **Detenerse ante el primer resultado inesperado** (regla de oro de la guía del cliente).

### F2 — Verificación por flujo (cruce)
- [ ] Confirmar que las funciones responden dentro de sus flujos reales donde sea viable: catálogo (6), clientes (8), explorador (12/13/14), plano (24), compras (21), preflight (1/2/3/7/15/17/18/19/20).
- [ ] Limpieza: verificar que no se creó ningún dato (solo lecturas).

### F3 — Reporte final
- [ ] Tabla por función con estado ✅/⚠️/❌ y evidencia real.
- [ ] Checklist de aceptación.
- [ ] Actualizar `roadmap.md` y `Agente/completados/`.

## 6. Criterios de aceptación

- Las 24 funciones responden contra BDP real con respuesta válida (HTTP 200 y parseo tipado OK) — o se documenta la limitación real (p. ej. `GetOrder` en API gratuita solo devuelve `Status`, Hallazgo 048A-11; `GetStock`/`GetListStock` especulativos).
- Ninguna escritura ejecutada; ningún dato creado/modificado en BDP.
- Evidencia reproducible por función.

## 7. Riesgos

| Riesgo | Mitigación |
| --- | --- |
| BDP caído / Tailscale desconectado | F0 confirma online antes de empezar; si cae a mitad, detener y documentar |
| API gratuita limita alguna lectura (p. ej. `GetOrder` sin `Total`/`Payments`) | Documentar como limitación real, no como fallo (criterio 048A-11) |
| Payload de contrato especulativo (GetStock/GetListStock) rechazado | Marcar como "especulativo, pendiente de confirmar contrato real" |
| Error de permisos del integrador `VBW2MBM5` para alguna lectura | Registrar y reportar; no intentar escrituras ni cambios de permisos |
| Datos personales en respuestas (clientes, empleados) | Redactar en evidencia; no volcar respuestas completas |
