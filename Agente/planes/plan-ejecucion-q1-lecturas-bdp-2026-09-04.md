# Paquete de ejecución — Parte 2 (F3): lecturas Q1 contra el BDP real (SOLO LECTURA)

> **Plan padre:** `Agente/planes/plan-revision-integral-bdp-2026-09-03.md` (039A-1), §7 bloque Q1.
> **Fuente del inventario:** `Agente/planes/completados/plan-prueba-lecturas-bdp-2026-08-18.md` (138A-2).
> **Preparado:** 2026-09-04 · **Rama:** `glory-rs-rest` · **Ejecutable cuando el usuario confirme**
> BDP online + credenciales válidas + autorización de lecturas.

## 1. Reglas de ejecución

1. **Solo lecturas.** Cero escrituras (`Create*`/`Update*`/`Modify*`/`Add*`/`Cancel`/`Invoice`/
   `Tip`/`Points`/`Call`/`Regularizations`/`Transfers`); Q2 queda en S3/F5 con autorización por
   operación. Si un flujo terminaría escribiendo, se corta antes (regla S2.3 del plan padre).
2. **La única vía de red es la app** (`BdpWeblinkClient` vía los endpoints de §4). Prohibido
   curl/psql/SSH directo a `100.83.196.35`; los guards de destino del cliente son parte de lo que
   se verifica (Q5.2 del plan padre).
3. **Precondición:** tomar snapshot de config local (`POST /api/bdp/backup/parcial`) y confirmar
   con el usuario antes de la primera llamada.
4. **Redacción (Q5.3):** nunca registrar `bdp_password`, token de sesión ni `CodigoIntegrador` —
   evidencia = "login OK (expira 59 min)". NIF/teléfonos/emails/nombres de clientes y empleados
   enmascarados; errores HTTP solo con tamaño (`sanitize_body`); catálogos/clientes → contadores +
   muestra ≤3 items enmascarada.
5. **Throttle:** respetar el guard `bdp_throttle`; ejecutar por flujo de §4, no en ráfaga.
6. **Clasificación:** "Subscripción no activada" y similares = bloqueo externo `⏸`. Paths
   especulativos rechazados por el contrato real (Q1.22/23) = "especulativo", no defecto. Sin id
   real disponible → `⏸` (nunca inventar ids).

## 2. Parámetros reales (BD local `glory_backend_glory_rs_rest`, 2026-09-04)

| Parámetro | Valor | Nota |
| --- | --- | --- |
| `bdp_base_url` | `http://100.83.196.35:8068` | fila real `configuracion_restaurante` |
| `bdp_pos_id` | `31` | |
| `bdp_employee_id` | `1` | |
| `bdp_items_profile_id` | `1` | |
| `bdp_catalog_price_type` | `1` (IVA incluido) | fix 048A-7: no usar pos_id |
| `bdp_almacen_default` | `1` | |
| `bdp_purchase_notes_profile_id` | **`NULL`** ⚠️ | Q1.21 exige perfil explícito o configurar este campo |
| `bdp_sync_enabled` / `bdp_sync_mode` | `false` / `read_only` | al ejecutar F3, modo `bdp`/`auto` con BDP online |
| Credenciales (`bdp_login`/`bdp_password`/`bdp_integrator_code`) | **no se registran** | solo en la config del entorno |

## 3. Lecturas Q1.1–Q1.24

Convenciones: **POST** en todas (WebLink REST API); **auth** público o `Bearer <token>` de
`POST /Auth/Login` (cache 55 min); body serializado **PascalCase**; constantes de ruta en
`src/services/bdp_weblink_catalog.rs` (`BDP_PATH_*`); respuestas = structs tipados del mismo
módulo / keys de `validate_snapshot_response` (`src/services/bdp_backup.rs:961`) / contraste
manual contra datos reales. La **vía app** de cada lectura (endpoint local que la dispara) está
en la tabla §4; el **PASS/FAIL** se evalúa sobre la respuesta de la app.

### Q1.1 — ServiceHealth ✅ pre-validado
- **Ruta/Body:** `POST {base}/Service/Health` · `{}` · público → `{ "IsAlive": bool }`
- **Vía app:** `GET /api/configuracion/bdp/diagnostico` (paso 1 preflight). ⚠️ la app expone
  `health_ok`, no `IsAlive`; con BDP configurado pero inalcanzable este paso tarda **~20 s**
  (timeout cliente `bdp_weblink.rs:52`) y responde **200 con `health_ok:false`** + mensaje honesto
  — no es error HTTP.
- **PASS:** 200 + `health_ok:true`. **FAIL:** `health_ok:false`/timeout → bloqueo externo ⏸.

### Q1.2 — GetVersion ✅ pre-validado
- **Ruta/Body:** `POST {base}/Service/GetVersion` · `{}` · Bearer
- **Respuesta:** `BdpVersionResponse { Version, Subversion, Revision, Application, ApplicationDescription, ErrorMessage }`
- **Vía app:** mismo diagnóstico (paso 2). **PASS:** 200 + `ErrorMessage` vacío + `Version` > 0.
  **FAIL:** `ErrorMessage` no vacío → hallazgo.

### Q1.3 — Login ✅ pre-validado
- **Ruta/Body:** `POST {base}/Auth/Login` · `{ "Login": <config>, "Password": <config>, "TiempoSession": 59, "CodigoIntegrador": <config> }` · público
- **Respuesta:** `BdpLoginResponse { ErrorMessage, AuthSession: { Token, ExpiresIn_InSecconds } }`
- **Vía app:** implícito (primera llamada autenticada del diagnóstico). Verificado contra BDP real
  en 2026-05-21 (`Agente/documentacion/api/bdp-weblink-2026-05-06.md`).
- **PASS:** `ErrorMessage` vacío + `Token` no vacío. **FAIL:** credenciales → hallazgo. Evidencia:
  solo "login OK (expira 59 min)".

### Q1.4 — GetArticle
- **Ruta/Body:** `POST {base}/API/Articles/Get` · `{ "ArtCode": <código> }` · Bearer — código =
  primer `Code` real de Q1.6, nunca uno local inventado.
- **Respuesta:** `ArticleData` JSON extenso (sin struct tipado; contraste manual).
- **Vía app:** sync-catalog (`bdp_sync.rs:740`). **PASS:** 200 + `ErrorMessage` vacío + `ArticleData`
  con el código pedido y campos esperados.

### Q1.5 — GetPricesArticles
- **Ruta/Body:** `POST {base}/API/Articles/GetPrices` · `{ "ArtCode": <código real> }` · Bearer
- **Respuesta:** `BdpGetPricesArticlesResponse { Prices:[Decimal;5], Discounts:[Decimal;5] (alias "Disconts"), ErrorMessage }`
- **Vía app:** `POST /api/bdp/article-maps/sync-prices`. **PASS:** 200 + `ErrorMessage` vacío +
  arrays de 5 números parseables.

### Q1.6 — ExportArticles ✅ pre-validado (contrato wiremock)
- **Ruta/Body:** `POST {base}/API/Articles/Export` · `BdpExportArticlesRequest::all_web_articles(type_price=1)`
  → `{ "Dept1":1, "Dept2":999, "Art1":1, "Art2":9999999999999, "Modified":false, "TypePrice":1, "Disc":0 }` · Bearer
- **Respuesta:** `BdpExportArticlesResponse { Articles:[{ Code|ItemCode, Name|Description, Family, Subfamily,
  Department, Tax1, Tax2, Price1..5, Discount, BarCode, Active, CurrentStock|Stock, PricesTableData }] }`
- **Vía app:** `POST /api/bdp/catalogo` · `/api/bdp/article-maps/sync-catalog` · explorar · backup parcial.
- **PASS:** 200 + `ErrorMessage` vacío + `Articles` array (vacío válido). **Redacción:** ≤3 items, contadores.

### Q1.7 — GetPOSArticlesList ✅ pre-validado (uso interno)
- **Ruta/Body:** `POST {base}/API/Articles/GetPOSList` · `BdpGetPosArticlesRequest::first_page(profile=1, items=1)`
  → `{ "Art1":1, "Art2":9999999999999, "Dept1":1, "Dept2":999, "Description":"", "DescriptionQueryType":0,
  "ItemsPerPage":1, "ActualPage":1, "nField":1, "nOrder":0, "ProfileCode":1 }` · Bearer
- **Respuesta:** keys aceptadas: `ArticlesListData | ArticleListData | Articles | ArticleList`
- **Vía app:** preflight (`sync-dry-run`, paso 6) · resolución de artículo (`bdp_sync.rs:782`).
  **PASS:** 200 + `ErrorMessage` vacío + colección presente (vacía válida).

### Q1.8 — ExportCustomers ✅ pre-validado
- **Ruta/Body:** `POST {base}/API/Customers/Export` · `{ "Customer1":1, "Customer2":999999 }` (Default) · Bearer
- **Respuesta:** keys aceptadas: `Customers | CustomerList`
- **Vía app:** `POST /api/bdp/customers/import` · explorar · backup parcial.
- **PASS:** 200 + `ErrorMessage` vacío + colección presente. **Redacción:** ⚠️ datos personales —
  NIF/teléfono/email/nombre completo enmascarados; solo contadores + muestra mínima.

### Q1.9 — GetOrder ⚠️ limitación API gratuita
- **Ruta/Body:** `POST {base}/API/Orders/Get` · `{ "OrderIdentifier": { "OrderId": <id> } }` · Bearer
  (`by_order_id`; alternativas `by_market`/`by_table`)
- **Id a usar:** un `bdp_order_id` real de `ventas` (tras S3/Q2.4) o de una comanda BDP existente.
  **Sin id real → `⏸` documentada.**
- **Respuesta:** API gratuita limita a `Status` (+ mensaje del manual); en el resto, objeto `Order`
  (exigido como objeto en el pre-write, `src/services/bdp_backup.rs:826`).
- **Vía app:** poller `bdp_order_poller.rs` · backup pre-write. **PASS:** 200 + `ErrorMessage` vacío +
  `Status` presente (respuesta "limitada" documentada = `⏸` por diseño, no fallo).

### Q1.10 — ExportDepartment ✅ pre-validado
- **Ruta/Body:** `POST {base}/API/Departments/Export` · `{ "Dept1":1, "Dept2":999, "Description":"",
  "DescriptionQueryType":0, "ItemsPerPage":0, "ActualPage":1, "nField":1, "nOrder":0 }` (Default) · Bearer
- **Respuesta:** `Departments | Department | DepartmentList | DepartmentListData` (real: `DepartmentListData`;
  ErrorMessage posicional de .NET tolerado — 048A-7)
- **Vía app:** explorar · backup parcial. **PASS:** 200 + colección presente (vacía válida).

### Q1.11 — DepartmentsExportFromProfile
- **Ruta/Body:** `POST {base}/API/Departments/ExportFromProfile` · `{ "ProfileId": 1 }` · Bearer
- **Respuesta:** `Departamentos | Departments`. **Vía app:** preflight (paso 5).
- **PASS:** 200 + `ErrorMessage` vacío + colección presente.

### Q1.12/13/14 — GetMenuDefinition / GetFastfoodDefinition / GetPackDefinition
- **Rutas/Body:** `POST {base}/API/Menus/Get` · `/API/FastFoods/Get` · `/API/Packs/Get` · `{ "MenuId" |
  "FastfoodId" | "PackId": <id> }` (ids reales del catálogo Q1.6 o menús/packs locales con código BDP) · Bearer
- **Respuesta:** JSON raw informativo (sin struct tipado). **Vía app:** `GET /api/bdp/menus/:id` ·
  `/api/bdp/fastfoods/:id` · `/api/bdp/packs/:id`.
- **PASS:** 200 + `ErrorMessage` vacío + definición del id pedido. **Sin id real → `⏸`.**

### Q1.15 — GetPOS
- **Ruta/Body:** `POST {base}/API/POS/Get` · `{ "Id": 31 }` · Bearer
- **Respuesta:** key `POS`. **Vía app:** preflight (paso 3).
- **PASS:** 200 + `ErrorMessage` vacío + `POS` con id 31.

### Q1.16 — GetPOSes ⚠️ sin caller en la app
- **Ruta/Body:** `POST {base}/API/POSes/Get` · `{}` · Bearer
- **Respuesta:** lista de terminales (contrastar en vivo; key tipo `POSes`/lista).
- **Sin vía app** (método definido, sin uso): **por defecto `⏸` documentada** — no se añade harness.

### Q1.17 — GetEmployee
- **Ruta/Body:** `POST {base}/API/Employee/Get` · `{ "Id": 1 }` · Bearer
- **Respuesta:** key `Employee`. **Vía app:** preflight (paso 4).
- **PASS:** 200 + `ErrorMessage` vacío + `Employee` id 1. **Redacción:** datos personales enmascarados.

### Q1.18 — GetEmployees
- **Ruta/Body:** `POST {base}/API/Employees/Get` · `{ "Ids": [], "OnlySalespeople": null }` (omitidos
  si vacíos — `skip_serializing_if`) · Bearer
- **Respuesta:** `Employees | Employee | EmployeeList`. **Vía app:** explorar · backup parcial.
- **PASS:** 200 + colección presente. **Redacción:** datos personales enmascarados.

### Q1.19 — GetPOSEmployees
- **Ruta/Body:** `POST {base}/API/POS/Employees/Get` · `{ "POSId": 31 }` · Bearer
- **Respuesta:** key `Employees`. **Vía app:** preflight (paso 4, valida employee 1 en POS 31).
- **PASS:** 200 + colección presente + el empleado configurado aparece.

### Q1.20 — GetPOSTenderList ✅ pre-validado
- **Ruta/Body:** `POST {base}/API/Tenders/GetPOSList` · `{ "POSId": 31 }` · Bearer
- **Respuesta:** `TenderList | Tenders`. **Vía app:** preflight (paso 4, valida `bdp_tender_map`).
- **PASS:** 200 + colección presente. (Complemento sin caller: `POST /API/Tenders/GetList` con `{}` —
  contrato wiremock verificado.)

### Q1.21 — ExportPurchaseNotes (Compras, [247A-11])
- **Ruta/Body:** `POST {base}/API/ExportProfiles/PurchaseNotes` · `{ "ExportProfileCode": <OBLIGATORIO>,
  "InitialDate": <YYYY-MM-DD>, "FinalDate": <YYYY-MM-DD>, "InitialSupplier": 1, "FinalSupplier": 999999 }`
  · Bearer — proveedores omitidos rechazados por BDP real (403900, 287A-4); rango ≤ 31 días (validación del handler)
- **⚠️ `bdp_purchase_notes_profile_id` = NULL:** requiere configurar el perfil o pasar
  `export_profile_code` explícito; sin ello error de validación → `⏸` documentada.
- **Respuesta:** `BdpExportPurchaseNotesResponse { DocumentsLists:[{ SerieAlbaran, NumAlbaran, FechaAlbaran,
  CodProveedor, NomProveedor, TotalAlbaran, …flatten }], ErrorMessage }`
- **Vía app:** `POST /api/bdp/purchase-notes/sync` (gate: `ff_bdp_purchase_notes_read`).
  **PASS:** 200 + `ErrorMessage` vacío + `DocumentsLists`. **Redacción:** proveedores con NIF/contacto
  enmascarados; solo serie/nº/total/fecha.

### Q1.22/23 — GetStock / GetListStock ⚠️ paths ESPECULATIVOS (N6, [128A-1/F3])
- **Rutas/Body:** `POST {base}/API/Warehouse/GetStock` · `{ "Article": <código real>, "Altern": 0, "Store": 1 }`
  → `BdpGetStockResponse { Stock, ErrorMessage }` · `POST {base}/API/Warehouse/GetListStock` ·
  `{ "Store": 1, "Articles": [{ "Article": <código>, "Altern": 0 }] }` (≤3 artículos reales) →
  `BdpGetListStockResponse { Stock:[{ Article, Altern, Units, ErrorMessage }], ErrorMessage }` · Bearer
- **Sin vía app** (métodos probados con wiremock, sin caller en producción): **por defecto `⏸`**
  documentada. Si el contrato real rechaza path/contrato → marcar **"especulativo"**, no defecto.
- **PASS:** 200 + `ErrorMessage` vacío + `Stock` presente.

### Q1.24 — GetRoomTables / GetRoomsTables (plano de sala, F9.4)
- **Rutas/Body:** `POST {base}/API/Room/GetTables` · `{ "Id": <salón real> }` (individual) ·
  `POST {base}/API/Rooms/GetTables` · `{}` (todos, Default) · Bearer
- **Respuesta:** `{ Tables:[i32], ErrorMessage }` / `BdpGetRoomsTablesResponse { Rooms:[{ Id, Name, Tables }], ErrorMessage }`
- **Bloqueo conocido:** BDP sin suscripción de salones → `Remote("Subscripción no activada")` → `⏸` externo
  tolerado (ya tratado en `fetch_rooms`, 048A-7).
- **Vía app:** `POST /api/bdp/sync-tables` · explorar · backup parcial.
- **PASS:** 200 + `ErrorMessage` vacío + `Rooms`/`Tables` presente (vacío válido).

## 4. Q1.25 — Orden de ejecución por flujo (las lecturas agrupadas, sin ráfaga)

| Flujo | Vía app | Lecturas |
| --- | --- | --- |
| Catálogo | `POST /api/bdp/catalogo` · `/api/bdp/article-maps/sync-catalog` · `sync-prices` | Q1.6, Q1.4, Q1.5 |
| Clientes | `POST /api/bdp/customers/import` | Q1.8 |
| Explorador | `GET /api/bdp/explorar` | Q1.6, Q1.8, Q1.10, Q1.24, Q1.18 |
| Plano de sala | `POST /api/bdp/sync-tables` | Q1.24 |
| Compras | `POST /api/bdp/purchase-notes/sync` | Q1.21 |
| Preflight / diagnóstico | `GET /api/configuracion/bdp/sync-dry-run` · `diagnostico` | Q1.1, Q1.2, Q1.3, Q1.15, Q1.17, Q1.19, Q1.20, Q1.11, Q1.7 |
| Backup / snapshot (pre-write) | `POST /api/bdp/backup/parcial` · `GET /api/bdp/backup/snapshots` | Q1.6, Q1.8, Q1.10, Q1.24, Q1.18, Q1.9 |

## 5. Q1.26 — Cierre (cero escrituras)

Cola `bdp_push_pendientes` sin filas nuevas, `bdp_audit_log` sin entradas de escritura en la ventana,
config restaurada (snapshot previo). Cero datos en BDP por construcción (solo lecturas) + auditoría local.
Evidencia redactada por lectura en §10b del plan padre, checklist Q1 marcado, completados, commit local.

## 6. Validación previa (2026-09-04, sin tocar BDP)

- **Vía C:** rutas de §4, payloads (`all_web_articles`/`first_page`/Defaults) y guards
  (`ensure_target_allowed` — lecturas sin allowlist, por diseño; `ff_bdp_purchase_notes_read`;
  ≤31 días) verificados contra el código real. Las lecturas no tienen allowlist; solo escrituras
  y `check_order` la exigen.
- **Vía T:** suite wiremock del contrato verde (153/0 en F0 `76068ee` y F2 `b4c41af`) con el código
  actual (sin cambios desde `188f6b3`). Re-run bloqueado por entorno: `C:\tmp` 6,4 GB / disco 5,9 GB
  libres < umbral del wrapper `run-cargo.mjs`.
- **Vía B:** parámetros §2 leídos de BD local; sandbox `:3100` vivo; cero conexiones salientes a
  `100.83.196.35` en toda la preparación.

## 7. Siguiente paso verificable

1. Usuario confirma: BDP online + credenciales válidas + autorización de **lecturas**.
2. Ejecutar F3 por flujo (§4), evidencia redactada por lectura, `⏸` donde aplique.
3. Marcar Q1 en §7 del plan padre, completados, commit local.
4. S3 (escrituras Q2) al final, autorización por operación.