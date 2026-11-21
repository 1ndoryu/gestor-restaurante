# Diagnóstico BDP sobre el TPV demo — 2026-11-21

`GET /api/configuracion/bdp/sync-dry-run` en `rest-dev` (app en la 89, puerto 3105, modo build;
BDP demo del portátil por el túnel inverso `89:8071`). Usuario `demo@restaurante.com`.

- `listo_para_sincronizar`: **true**
- `escritura_real`: **false** (el dry-run no escribe ni comanda ni pago)
- `mensaje`: «El simulador aceptó el contrato local; esto no demuestra el comportamiento de un BDP real»

| # | Check | Endpoint | Resultado y mensaje |
|---|---|---|---|
| 1 | Mapeo de articulos | `bdp_article_map` | OK: 564 articulos mapeados Glory→BDP |
| 2 | Health | `/Service/Health` | OK: BDP responde IsAlive=true |
| 3 | Sesion y version | `/Auth/Login` + `/Service/GetVersion` | OK: Login y version correctos |
| 4 | Perfil de articulos | `/API/ProfilesLists/GetCreateArticleList` | OK: El perfil de articulos configurado existe en el TPV |
| 5 | Mapa de IVA | `/API/Articles/GetPOSList` | OK: 3 IVA mapeados a codigos TAV reales del perfil |
| 6 | Perfil de documentos | `/API/ExportProfiles/Documents` | OK: El perfil 1 exporta documentos en el TPV |
| 7 | Cliente por defecto | `bdp_default_customer_code` | OK: sin cliente por defecto (aviso verde, no bloqueo) |
| 8 | Tarifa del catalogo | `/API/Articles/Export` | OK: 2 articulos con precio en la tarifa 1 |
| 9 | Terminal POS | `/API/POS/Get` | OK |
| 10 | Empleado BDP | `/API/Employee/Get` | OK |
| 11 | Empleados del POS | `/API/POS/Employees/Get` | OK |
| 12 | Empleado permitido en POS | `/API/POS/Employees/Get` | OK: el empleado configurado aparece asociado al terminal |
| 13 | Formas de pago del POS | `/API/Tenders/GetPOSList` | OK |
| 14 | Mapeo de tenders | `bdp_tender_map` | OK: 2 tenders mapeados correctamente |
| 15 | Mapeo de order types | `bdp_order_type_map` | OK: 4 canales mapeados |
| 16 | Departamentos del perfil | `/API/Departments/ExportFromProfile` | OK |
| 17 | Articulos del perfil | `/API/Articles/GetPOSList` | OK |
| 18 | CreateOrder OnlyCheck | `/API/Orders/Create` | OK: el destino aceptó OnlyCheck |

## Qué demuestra

Los cinco checks nuevos de configuración (perfil de artículos, mapa de IVA, perfil de
documentos, cliente por defecto, tarifa del catálogo) se ejecutan de verdad y dan verde contra un
TPV real, y el arreglo `codigo_entero` quita los seis falsos positivos que daban los checks de
mapeo cuando el mapa está guardado como número (que es como lo guarda la UI).

Antes del arreglo, el mismo diagnóstico devolvía en rojo «Mapeo de tenders: valor no es string»
(×2) y «Mapeo de order types: valor '0' no es numérico válido» (×4) sobre una configuración que
funciona: el consumidor real (`resolve_tender_id`, `resolve_order_type`) acepta número y texto.

## Qué no demuestra

Que los valores de producción del restaurante sean válidos. Este TPV es el demo de BDP, con otros
artículos, otros perfiles y otro terminal. Esa vuelta es `21AB-4b`, desde
`restaurante.wandori.us`.
