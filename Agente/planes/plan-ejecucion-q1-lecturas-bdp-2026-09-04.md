# Paquete de ejecución — Parte 2 (F3): lecturas Q1 contra el BDP real (SOLO LECTURA)

> **Plan padre:** `Agente/planes/plan-revision-integral-bdp-2026-09-03.md` (039A-1), §7 bloque Q1.
> **Fuente del inventario:** `Agente/planes/completados/plan-prueba-lecturas-bdp-2026-08-18.md` (138A-2).
> **Fecha de preparación:** 2026-09-04. **Rama:** `glory-rs-rest`.
> **Regla dura:** este paquete solo ejecuta **lecturas**. Cero escrituras, cero `Create*`/`Update*`/
> `Modify*`/`Add*`/`Cancel`/`Invoice`/`Tip`/`Points`/`Call`/`Regularizations`/`Transfers`.
> Cualquier escritura del bloque Q2 queda diferida a S3/F5 con autorización por operación.

---

## 1. Restricciones y protocolo de arranque (Q0)

1. **Solo lecturas.** Si un flujo terminaría escribiendo, se recorre hasta el punto anterior a la
   escritura (regla S2.3 del plan padre).
2. **La única vía de red es la aplicación** (handlers que usan `BdpWeblinkClient`). Prohibido
   `curl`/`psql`/SSH directo contra `100.83.196.35` — los guards de destino del cliente son parte
   de lo que se verifica (Q5.2).
3. **Antes de tocar BDP:** confirmar con el usuario que el BDP está online + credenciales válidas.
   Tomar snapshot de configuración local (`POST /api/bdp/backup/parcial`) y registro de evidencia
   redactada.
4. **Redacción obligatoria (Q5.3):** nunca registrar `bdp_password`, token de sesión ni
   `CodigoIntegrador`; en evidencia solo "login OK (expira 59 min)". Enmascarar NIF/fiscal,
   teléfonos, emails y nombres completos de clientes. Los cuerpos de error HTTP ya se registran
   solo con tamaño (`sanitize_body`). Respuestas grandes (catálogo/clientes) → resumen con
   contadores + muestra de ≤3 items con campos sensibles enmascarados.
5. **Errores de suscripción** ("Subscripción no activada" y similares) = bloqueo externo `⏸`,
   no fallo del producto. Paths especulativos (N6, Q1.22/23) rechazados por el contrato real se
   marcan **"especulativo"** (regla del plan Q1.22/23), no como defecto.
6. **Throttling:** respetar el guard `bdp_throttle` (Q5.5); no lanzar las 24 lecturas en ráfaga:
   ejecutar por flujo de la app (Q1.25) que ya las agrupa de forma natural.

## 2. Parámetros reales verificados (BD local `glory_backend_glory_rs_rest`, 2026-09-04)

| Parámetro | Valor | Origen |
| --- | --- | --- |
| `bdp_base_url` | `http://100.83.196.35:8068` | `configuracion_restaurante` (fila real) |
| `bdp_pos_id` | `31` | idem |
| `bdp_employee_id` | `1` | idem |
| `bdp_items_profile_id` | `1` | idem |
| `bdp_catalog_price_type` | `1` (IVA incluido) | idem; fix 048A-7 (no usar pos_id) |
| `bdp_almacen_default` | `1` | idem |
| `bdp_purchase_notes_profile_id` | **`NULL`** ⚠️ | Q1.21 exige `ExportProfileCode` explícito o configurar este campo |
| `bdp_sync_enabled` | `false` (→ standalone) | al activar Parte 2 se pasa a modo `bdp`/`auto` con BDP online |
| `bdp_sync_mode` | `read_only` | coherente con "solo lecturas" |
| Credenciales | **no se registran** | `bdp_login`/`bdp_password`/`bdp_integrator_code` solo en la config del entorno |

## 3. Inventario Q1.1–Q1.24 (contrato del cliente `BdpWeblinkClient`)

Convenciones: **método** = `POST` en todos (WebLink REST API). **Auth** = público (sin token) o
`Bearer <token>` obtenido de `POST /Auth/Login` (cache 55 min, `BDP_SESSION_MINUTES=59`).
Body siempre serializado **PascalCase**. Constantes de ruta: `src/services/bdp_weblink_catalog.rs`
(`BDP_PATH_*`). Formas de respuesta: structs tipados en `bdp_weblink_catalog.rs` / `bdp_weblink.rs`
o keys aceptadas por `validate_snapshot_response` (`bdp_backup.rs:961`).

### Q1.1 — ServiceHealth ✅ pre-validado
- **Ruta:** `POST {base}/Service/Health` — **Auth:** público — **Body:** `{}`
- **Respuesta:** `{ "IsAlive": bool }` (`BdpHealthResponse`)
- **Vía app:** `GET /api/configuracion/bdp/diagnostico` (paso 1 del preflight)
- **PASS:** HTTP 200 + `IsAlive:true`. **FAIL:** timeout/error HTTP/`false` → hallazgo o bloqueo externo.
- **Redacción:** n/a (no lleva datos).

### Q1.2 — GetVersion ✅ pre-validado
- **Ruta:** `POST {base}/Service/GetVersion` — **Auth:** Bearer — **Body:** `{}`
- **Respuesta:** `BdpVersionResponse` `{ Version:i32, Subversion:i32, Revision:String, Application:String, ApplicationDescription:String, ErrorMessage:String }`
- **Vía app:** `GET /api/configuracion/bdp/diagnostico` (paso 2)
- **PASS:** 200 + `ErrorMessage` vacío + `Version` > 0 parseable. **FAIL:** ErrorMessage no vacío → hallazgo.
- **Redacción:** registrar solo versiones, nunca credenciales.

### Q1.3 — Login ✅ pre-validado
- **Ruta:** `POST {base}/Auth/Login` — **Auth:** público — **Body:** `{ "Login": <config>, "Password": <config>, "TiempoSession": 59, "CodigoIntegrador": <config> }` (`BdpLoginRequest`)
- **Respuesta:** `BdpLoginResponse` `{ ErrorMessage:String, AuthSession:{ Token, ExpiresIn_InSecconds } }`
- **Vía app:** implícito en toda llamada autenticada (primera del diagnóstico)
- **PASS:** `ErrorMessage` vacío + `Token` no vacío. **FAIL:** credenciales/error remoto → hallazgo (nunca registrar el token ni la contraseña; evidencia = "login OK, expira 59 min").
- **Nota:** verificado contra BDP real en 2026-05-21 (doc `Agente/documentacion/api/bdp-weblink-2026-05-06.md`).

### Q1.4 — GetArticle (F9.2, [157A-9])
- **Ruta:** `POST {base}/API/Articles/Get` — **Auth:** Bearer — **Body:** `{ "ArtCode": <código real> }` (`BdpGetArticleRequest{art_code:i64}`)
- **Código a usar:** primer `Code` devuelto por Q1.6 (catálogo real); nunca un código local inventado.
- **Respuesta:** `ArticleData` JSON extenso (DeptCode, TAVPer, Price1..5…; sin struct tipado completo — se contrasta contra datos reales)
- **Vía app:** flujo de mapeo de artículo (`bdp_sync.rs:740` dentro de sync-catalog); no hay ruta directa dedicada.
- **PASS:** 200 + `ErrorMessage` vacío + `ArticleData` con código pedido y campos esperados presentes.

### Q1.5 — GetPricesArticles (F9.3)
- **Ruta:** `POST {base}/API/Articles/GetPrices` — **Auth:** Bearer — **Body:** `{ "ArtCode": <código real> }`
- **Respuesta:** `BdpGetPricesArticlesResponse` `{ Prices:[Decimal;5], Discounts:[Decimal;5] (alias "Disconts"), ErrorMessage }`
- **Vía app:** `POST /api/bdp/article-maps/sync-prices`
- **PASS:** 200 + `ErrorMessage` vacío + arrays de 5 números parseables.

### Q1.6 — ExportArticles ✅ pre-validado (contrato wiremock)
- **Ruta:** `POST {base}/API/Articles/Export` — **Auth:** Bearer
- **Body:** `BdpExportArticlesRequest::all_web_articles(config.bdp_catalog_price_type=1)` → `{ "Dept1":1, "Dept2":999, "Art1":1, "Art2":9999999999999, "Modified":false, "TypePrice":1, "Disc":0 }`
- **Respuesta:** `BdpExportArticlesResponse` `{ Articles:[{ Code|ItemCode, Name|Description, Family, Subfamily, Department, Tax1, Tax2, Price1..5, Discount, BarCode, Active, CurrentStock|Stock (alias), PricesTableData:[{CurrentStock}] }] }`
- **Vía app:** `POST /api/bdp/catalogo` (sync-catalog) · `POST /api/bdp/catalogo/:tipo` · explorar · backup parcial
- **PASS:** 200 + `ErrorMessage` vacío + `Articles` array (vacío válido) + parseo tipado OK.
- **Redacción:** muestra ≤3 artículos, enmascarar BarCode si aplica; guardar totales.

### Q1.7 — GetPOSArticlesList ✅ pre-validado (uso interno)
- **Ruta:** `POST {base}/API/Articles/GetPOSList` — **Auth:** Bearer
- **Body:** `BdpGetPosArticlesRequest::first_page(profile=config.bdp_items_profile_id=1, items=1)` → `{ "Art1":1, "Art2":9999999999999, "Dept1":1, "Dept2":999, "Description":"", "DescriptionQueryType":0, "ItemsPerPage":1, "ActualPage":1, "nField":1, "nOrder":0, "ProfileCode":1 }`
- **Respuesta:** keys aceptadas: `ArticlesListData | ArticleListData | Articles | ArticleList`
- **Vía app:** preflight (`GET /api/configuracion/bdp/sync-dry-run`, paso 6) · resolución de artículo (`bdp_sync.rs:782`)
- **PASS:** 200 + `ErrorMessage` vacío + alguna key de colección presente (array vacío válido).

### Q1.8 — ExportCustomers ✅ pre-validado
- **Ruta:** `POST {base}/API/Customers/Export` — **Auth:** Bearer — **Body:** `{ "Customer1":1, "Customer2":999999 }` (Default)
- **Respuesta:** keys aceptadas: `Customers | CustomerList`
- **Vía app:** `POST /api/bdp/customers/import` · explorar · backup parcial
- **PASS:** 200 + `ErrorMessage` vacío + colección presente.
- **Redacción:** ⚠️ datos personales: enmascarar NIF, teléfono, email, nombre completo en cualquier evidencia; solo contadores + muestra mínima.

### Q1.9 — GetOrder ⚠️ limitación API gratuita
- **Ruta:** `POST {base}/API/Orders/Get` — **Auth:** Bearer
- **Body:** `{ "OrderIdentifier": { "OrderId": <id> } }` (`BdpOrderIdentifier::by_order_id`); alternativas `by_market`/`by_table`
- **Id a usar:** un `bdp_order_id` real (el primero que exista en `ventas` tras Q2.4 en S3, o el de una comanda BDP existente conocida). **Sin id real → `⏸` documentada** (no inventar ids).
- **Respuesta:** **limitado en API gratuita: solo `Status` de la comanda** (+ mensaje del manual); keys esperadas: `Order` (objeto) en el resto de casos (`bdp_backup.rs:815` exige `Order` objeto para pre-write).
- **Vía app:** poller `bdp_order_poller.rs` · `bdp_sync.rs` · backup pre-write
- **PASS:** 200 + `ErrorMessage` vacío + `Status` presente (o `Order` objeto). La respuesta "limitada" documentada es válida (`⏸` por diseño de la suscripción, no fallo).

### Q1.10 — ExportDepartment ✅ pre-validado
- **Ruta:** `POST {base}/API/Departments/Export` — **Auth:** Bearer
- **Body:** `{ "Dept1":1, "Dept2":999, "Description":"", "DescriptionQueryType":0, "ItemsPerPage":0, "ActualPage":1, "nField":1, "nOrder":0 }` (Default)
- **Respuesta:** keys aceptadas: `Departments | Department | DepartmentList | DepartmentListData` (BDP real devuelve `DepartmentListData`; ErrorMessage posicional de .NET tolerado — 048A-7)
- **Vía app:** explorar · backup parcial
- **PASS:** 200 + colección presente (array vacío válido).

### Q1.11 — DepartmentsExportFromProfile
- **Ruta:** `POST {base}/API/Departments/ExportFromProfile` — **Auth:** Bearer — **Body:** `{ "ProfileId": 1 }` (`config.bdp_items_profile_id`)
- **Respuesta:** keys aceptadas: `Departamentos | Departments`
- **Vía app:** preflight (paso 5)
- **PASS:** 200 + `ErrorMessage` vacío + colección presente.

### Q1.12/13/14 — GetMenuDefinition / GetFastfoodDefinition / GetPackDefinition (F9.5)
- **Rutas:** `POST {base}/API/Menus/Get` · `/API/FastFoods/Get` · `/API/Packs/Get` — **Auth:** Bearer
- **Body:** `{ "MenuId" | "FastfoodId" | "PackId": <id> }` (ids reales: desde el catálogo Q1.6 o los menús/packs locales con código BDP)
- **Respuesta:** JSON raw informativo (sin struct tipado; se contrasta contra datos reales — "endpoints informativos")
- **Vía app:** `GET /api/bdp/menus/:id` · `GET /api/bdp/fastfoods/:id` · `GET /api/bdp/packs/:id`
- **PASS:** 200 + `ErrorMessage` vacío + JSON con la definición del id pedido. **Sin id real → `⏸`.**

### Q1.15 — GetPOS
- **Ruta:** `POST {base}/API/POS/Get` — **Auth:** Bearer — **Body:** `{ "Id": 31 }` (`config.bdp_pos_id`)
- **Respuesta:** key aceptada: `POS`
- **Vía app:** preflight (paso 3)
- **PASS:** 200 + `ErrorMessage` vacío + objeto `POS` con el id 31.

### Q1.16 — GetPOSes ⚠️ sin caller en la app
- **Ruta:** `POST {base}/API/POSes/Get` — **Auth:** Bearer — **Body:** `{}` (`BdpEmptyRequest`)
- **Respuesta:** JSON con lista de terminales (contrastar en vivo; key esperada tipo `POSes`/lista)
- **Vía app:** **ninguna** (método definido, sin uso). Ejecutar vía verificación directa del cliente (harness) o **`⏸` documentada**.
- **PASS:** 200 + `ErrorMessage` vacío + lista no-nula.

### Q1.17 — GetEmployee
- **Ruta:** `POST {base}/API/Employee/Get` — **Auth:** Bearer — **Body:** `{ "Id": 1 }` (`config.bdp_employee_id`)
- **Respuesta:** key aceptada: `Employee`
- **Vía app:** preflight (paso 4)
- **PASS:** 200 + `ErrorMessage` vacío + `Employee` con id 1. (Campos personales del empleado → enmascarar en evidencia.)

### Q1.18 — GetEmployees
- **Ruta:** `POST {base}/API/Employees/Get` — **Auth:** Bearer — **Body:** `{ "Ids": [], "OnlySalespeople": null }` (omits ambos si vacíos)
- **Respuesta:** keys aceptadas: `Employees | Employee | EmployeeList`
- **Vía app:** explorar · backup parcial
- **PASS:** 200 + colección presente. **Redacción:** enmascarar datos personales de empleados.

### Q1.19 — GetPOSEmployees
- **Ruta:** `POST {base}/API/POS/Employees/Get` — **Auth:** Bearer — **Body:** `{ "POSId": 31 }`
- **Respuesta:** key aceptada: `Employees`
- **Vía app:** preflight (paso 4, valida que `employee_id=1` está permitido en el POS)
- **PASS:** 200 + colección presente + el empleado configurado aparece en ella.

### Q1.20 — GetPOSTenderList ✅ pre-validado
- **Ruta:** `POST {base}/API/Tenders/GetPOSList` — **Auth:** Bearer — **Body:** `{ "POSId": 31 }`
- **Respuesta:** keys aceptadas: `TenderList | Tenders`
- **Vía app:** preflight (paso 4, valida mapeo `bdp_tender_map`)
- **PASS:** 200 + colección presente. (Complemento sin caller: `POST /API/Tenders/GetList` con `{}` — contrato wiremock verificado.)
- **Redacción:** los tender codes del mapeo local no son secretos; solo registrarlos enmascarando cualquier etiqueta comercial innecesaria.

### Q1.21 — ExportPurchaseNotes (Compras, [247A-11])
- **Ruta:** `POST {base}/API/ExportProfiles/PurchaseNotes` — **Auth:** Bearer
- **Body:** `BdpExportPurchaseNotesRequest`: `{ "ExportProfileCode": <OBLIGATORIO>, "InitialDate": <YYYY-MM-DD>, "FinalDate": <YYYY-MM-DD>, "InitialSupplier": 1, "FinalSupplier": 999999 }` — BDP real **rechaza proveedores omitidos** (403900) → rango completo por defecto (287A-4); rango de fechas ≤ 31 días (validación del handler)
- **⚠️ `bdp_purchase_notes_profile_id` = NULL en config:** Q1.21 requiere configurar el perfil de exportación o pasar `export_profile_code` explícito; sin ello el handler devuelve error de validación → `⏸` documentada.
- **Respuesta:** `BdpExportPurchaseNotesResponse` `{ DocumentsLists:[{ SerieAlbaran, NumAlbaran, FechaAlbaran, CodProveedor, NomProveedor, TotalAlbaran, …(flatten extra) }], ErrorMessage }`
- **Vía app:** `POST /api/bdp/purchase-notes/sync` (gate: `ff_bdp_purchase_notes_read`)
- **PASS:** 200 + `ErrorMessage` vacío + `DocumentsLists` array. **Redacción:** albaranes con datos de proveedor → máscara de NIF y contacto; solo serie/nº/total/fecha como evidencia.

### Q1.22/23 — GetStock / GetListStock ⚠️ paths ESPECULATIVOS (N6, [128A-1/F3])
- **Rutas:** `POST {base}/API/Warehouse/GetStock` · `/API/Warehouse/GetListStock` — **Auth:** Bearer
- **Body GetStock:** `{ "Article": <código real>, "Altern": 0, "Store": 1 }` (`bdp_almacen_default`) → `BdpGetStockResponse { Stock:Decimal, ErrorMessage }`
- **Body GetListStock:** `{ "Store": 1, "Articles": [{ "Article": <código>, "Altern": 0 }] }` (≤3 artículos del catálogo real) → `BdpGetListStockResponse { Stock:[{ Article, Altern, Units, ErrorMessage }], ErrorMessage }`
- **Vía app:** **ninguna** (métodos definidos y probados con wiremock, sin caller en producción). Ejecutar vía verificación directa del cliente o `⏸`.
- **PASS:** 200 + `ErrorMessage` vacío + `Stock` presente. **Si el contrato real rechaza el path/contrato → marcar "especulativo"** (regla del plan), no defecto.

### Q1.24 — GetRoomTables / GetRoomsTables (plano de sala, F9.4)
- **Rutas:** `POST {base}/API/Room/GetTables` · `/API/Rooms/GetTables` — **Auth:** Bearer
- **Body:** `{ "Id": <salón real> }` (individual) / `{}` (todos, Default)
- **Respuesta:** `{ Tables:[i32], ErrorMessage }` / `BdpGetRoomsTablesResponse { Rooms:[{ Id, Name, Tables:[i32] }], ErrorMessage }`
- **Bloqueo conocido:** BDP real sin suscripción de salones → `Remote("Subscripción no activada")` → **`⏸` externo tolerado** (`[]`, ya tratado en `fetch_rooms` 048A-7)
- **Vía app:** `POST /api/bdp/sync-tables` · explorar · backup parcial
- **PASS:** 200 + `ErrorMessage` vacío + `Rooms`/`Tables` presente (vacío válido si colección existe).

## 4. Q1.25 — Verificación por flujo (lecturas agrupadas por dominio)

| Flujo | Vía app | Lecturas que ejercita |
| --- | --- | --- |
| Catálogo | `POST /api/bdp/catalogo` · `/api/bdp/article-maps/sync-catalog` · `sync-prices` | Q1.6, Q1.4, Q1.5 |
| Clientes | `POST /api/bdp/customers/import` | Q1.8 |
| Explorador | `GET /api/bdp/explorar` | Q1.6, Q1.8, Q1.10, Q1.24, Q1.18 |
| Plano de sala | `POST /api/bdp/sync-tables` | Q1.24 |
| Compras | `POST /api/bdp/purchase-notes/sync` | Q1.21 |
| Preflight / diagnóstico | `GET /api/configuracion/bdp/sync-dry-run` · `GET /api/configuracion/bdp/diagnostico` | Q1.1, Q1.2, Q1.3, Q1.15, Q1.17, Q1.19, Q1.20, Q1.11, Q1.7 |
| Backup / snapshot (pre-write) | `POST /api/bdp/backup/parcial` · `GET /api/bdp/backup/snapshots` | Q1.6, Q1.8, Q1.10, Q1.24, Q1.18, Q1.9 |

## 5. Q1.26 — Limpieza y cierre

- Verificar que **ninguna escritura** se ejecutó: cola `bdp_push_pendientes` sin filas nuevas,
  `bdp_audit_log` sin entradas de escritura en la ventana, config restaurada (snapshot previo),
  cero datos creados/modificados en BDP (no hay forma de escribirlo desde lecturas; la
  verificación es por construcción + auditoría local).
- Cierre: evidencia redactada por lectura en §10b del plan padre, checklist Q1 marcado, bloque
  registrado en `Agente/completados/`, commit local.

## 6. Validación previa hecha al preparar este paquete (2026-09-04, sin tocar BDP)

- **Vía C:** rutas/payloads/estructuras verificadas contra el código real: constantes `BDP_PATH_*`
  (`bdp_weblink_catalog.rs`), structs tipados, `post_authenticated` (Bearer) / `post_public`,
  serialización PascalCase, `sanitize_body` (errores HTTP sin contenido), guards
  `ensure_target_allowed` (loopback permitido sin allowlist; host externo exige
  `BDP_WRITE_ALLOWED_ORIGINS` — solo escrituras; **las lecturas no tienen allowlist, por diseño**).
- **Vía T:** la suite wiremock del contrato ya está verde con el código actual del cliente
  (153/0 en F0 y F2, commits `76068ee`/`b4c41af`): health, login PascalCase, Bearer,
  ExportArticles, ExportPurchaseNotes, GetStock/GetListStock, AddOrderTip, GetApplicationVersion.
  `git log` confirma que `bdp_weblink.rs`/`bdp_weblink_catalog.rs` no cambian desde `188f6b3`
  (anterior a esas corridas verdes). **Re-run bloqueado por entorno:** `C:\tmp` sobre presupuesto
  (6,4 GB tras poda de `glory-backend`; libre en C: 5,9 GB < umbral del wrapper `run-cargo.mjs`);
  pendiente de entorno, no de código.
- **Vía B:** parámetros reales (§2) leídos de la BD local (fila real de configuración); sandbox
  `:3100` vivo (`/api/health` 200). El entorno local no expone endpoints BDP (fail-closed), por lo
  que la validación de contrato es la suite T + el cruce con `# WEBLINK RESTAPI.md`
  (manual oficial, raíz del repo) y `Agente/documentacion/api/bdp-weblink-2026-05-06.md`.
- **Verificaciones de red hechas:** cero conexiones salientes al BDP real (nada de este paquete ha
  contactado `100.83.196.35`); todo el trabajo previo es loopback/local.

## 7. Siguiente paso verificable

1. Usuario confirma: BDP online + credenciales válidas + autorización para **lecturas**.
2. Ejecutar F3 siguiendo §1–§4, una lectura/flujo a la vez, evidencia redactada por caso.
3. Marcar Q1 en §7 del plan padre, registrar en completados, commit local.
4. S2 (Parte 3) reutiliza estas lecturas; S3 (escrituras Q2) queda para el final con
   autorización por operación.