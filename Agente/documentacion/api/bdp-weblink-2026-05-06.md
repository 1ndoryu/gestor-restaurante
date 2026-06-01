# BDP WebLink REST API — Integracion inicial

## Estado 2026-05-21

La base de integracion ya valida el PC real del restaurante por Tailscale sin guardar credenciales en el repositorio. La URL operativa actual es `http://100.83.196.35:8068`; `Health`, `Login` y `GetVersion` quedaron probados contra WebLink real. La documentacion completa pegada desde PDF vive en `# WEBLINK RESTAPI.md` y la configuracion operativa queda por restaurante en `configuracion_restaurante`.

El backend incorpora una prueba de sincronizacion segura: lecturas reales de WebLink y una comanda de prueba enviada a `/API/Orders/Create` con `OrderOperationType = 1` (`OnlyCheck`). Ese modo pide a BDP validar el payload de comanda sin crear comanda, pago ni factura. En produccion, el PC del restaurante ya valida todas las lecturas, pero BDP rechaza el `CreateOrder OnlyCheck` con `[300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA`; esto requiere configurar en BDP la serie/destino valida para comandas WebLink antes de activar escrituras reales.

## Implementado

- Columnas `bdp_*` en `configuracion_restaurante`: URL publica, login, password, codigo integrador, toggle sync y IDs operativos de POS/empleado/perfil de articulos.
- Cliente backend `BdpWeblinkClient` con `Health`, `Login`, `GetVersion`, timeout de 20s, manejo de `ErrorMessage` y sanitizado de bodies HTTP.
- Metodo `BdpWeblinkClient::check_order()` que fuerza `OrderOperationType = 1` aunque el caller envie otro valor.
- Catalogo backend de rutas/payloads BDP para articulos, clientes, comandas, pagos, departamentos, terminales y empleados.
- Endpoint `GET /api/configuracion/bdp/diagnostico` para probar Health + Login + GetVersion sin exponer credenciales.
- Endpoint `GET /api/configuracion/bdp/sync-dry-run` para validar sincronizacion sin escrituras reales.
- Pantalla de configuracion BDP con URL, credenciales, IDs operativos, toggle y boton de diagnostico.
- Boton `Probar sincronizacion segura` en Configuracion BDP: muestra checks por paso y el payload resumido que BDP aceptaria.
- Tags OpenAPI de reseñas renombrados a `resenas` y Orval configurado con `clean: true` para evitar carpetas generadas corruptas en Windows.

## Dry-run de sincronizacion

`GET /api/configuracion/bdp/sync-dry-run` ejecuta esta secuencia autenticada:

1. `Health` para comprobar que WebLink responde.
2. `Login` y `GetVersion` para validar credenciales, integrador y version.
3. `POS/Get`, `POS/Employees/Get`, `Tenders/GetPOSList` y `Articles/GetPOSList` para comprobar que los IDs operativos configurados existen y devuelven datos reales.
4. Validacion de que el `EmployeeId` configurado aparece permitido para el POS.
5. Construccion de una comanda minima con articulo real, sin pagos, para no activar validaciones de facturacion del TPV durante el dry-run.
6. `/API/Orders/Create` en modo `OnlyCheck`, con `escritura_real = false` en la respuesta propia.

Cuando `listo_para_sincronizar = true`, la plataforma puede decir al cliente que el circuito tecnico de sincronizacion esta validado sin haber creado datos en BDP. Lo unico no ejecutado por diseno es cambiar `OrderOperationType` a creacion real.

## Mapa tecnico extraido del manual

| Area | Ruta | Uso previsto |
| --- | --- | --- |
| Servicio | `/Service/Health`, `/Service/GetVersion`, `/Auth/Login` | Diagnostico remoto y sesion autenticada. |
| Articulos | `/API/Articles/Export` | Sincronizar catalogo web marcado como Articulo Web en BDP. |
| Articulos | `/API/Articles/GetPOSList` | Leer articulos filtrados por perfil de departamentos/articulos del TPV. |
| Clientes | `/API/Customers/Export`, `/API/Customers/Create` | Exportar clientes o crear/sobrescribir cliente antes de una venta/comanda. |
| Comandas | `/API/Orders/Create`, `/API/Orders/Get`, `/API/Orders/Cancel` | Crear, consultar o cancelar comandas por `OrderId`, marketplace o mesa. |
| Pagos | `/API/Orders/Payment/Add`, `/API/Orders/Invoice` | Registrar cobros y facturar comandas desde POS/empleado configurados. |
| Departamentos | `/API/Departments/Export`, `/API/Departments/ExportFromProfile` | Obtener departamentos generales o por perfil operativo. |
| Terminales | `/API/POS/Get`, `/API/POSes/Get` | Resolver terminales validos para cobros/cancelaciones. |
| Empleados | `/API/Employee/Get`, `/API/Employees/Get`, `/API/POS/Employees/Get` | Resolver camareros/vendedores validos para crear y cerrar comandas. |
| Formas de pago | `/API/Tenders/GetList`, `/API/Tenders/GetPOSList` | Mapear metodos de pago locales contra `TenderId` de BDP. |

## Mapeo operativo previsto

- Catalogo: `BdpExportArticlesRequest::all_web_articles(1)` cubre el barrido inicial; `BdpGetPosArticlesRequest::first_page(profile, page_size)` queda para perfiles de TPV.
- Cliente local: antes de enviar una comanda con datos fiscales, crear/sobrescribir cliente con `/API/Customers/Create` y guardar el codigo BDP si el entorno real lo exige.
- Venta/reserva: se convierte en `/API/Orders/Create` con `EmployeeId`, `ItemsProfileId`, `OrderEndType` y estructura `Order`. Para pruebas seguras se usa `OrderOperationType = 1`; para sincronizacion real se cambiara al modo de creacion que confirme BDP en la respuesta operativa.
- Pago: los metodos locales deben mapearse contra `/API/Tenders/GetPOSList` para enviar `TenderId` correcto a `/API/Orders/Payment/Add`.
- Facturacion: si el pago no factura automaticamente, usar `/API/Orders/Invoice` con `PosId`, `EmployeeId` y el `OrderIdentifier` persistido.

## Checklist remoto BDP

- Si no hay tecnico local, compilar y usar `remote_access_bootstrap.exe` desde el `release` del target activo de Cargo para dejar Tailscale, RustDesk y reporte final listos en el PC remoto.
- Confirmar que BDP-NET esta activo y sin modo demo.
- Confirmar subscripcion extendida de WebLink REST API.
- Abrir `Utilidades -> Configuracion Servicios Web` en el PC servidor BDP.
- Confirmar puerto publico, firewall de Windows y NAT/router.
- Confirmar que el servicio exige login/password en entorno real.
- Pedir a BDP el `CodigoIntegrador` y validar que Login devuelve `AuthSession.Token`.
- Ejecutar diagnostico desde Configuracion: Health debe responder `IsAlive=true`; Login debe devolver sesion; GetVersion debe devolver `ErrorMessage` vacio.

## Supuestos y gotchas

- El manual no explicita el header de sesion para comandos autenticados. El cliente usa `Authorization: Bearer <token>` y esta encapsulado en un unico punto para ajustar si BDP usa otro header.
- `TiempoSession` se fija en 59 minutos porque el manual lo declara como maximo.
- La API BDP usa nombres PascalCase y errores de negocio en `ErrorMessage`; no tratar HTTP 200 como exito sin revisar ese campo.
- Las respuestas de articulos, clientes, comandas y pagos quedan como JSON hasta probar contra el PC real. El manual es grande y no conviene cerrar structs definitivos sin ver datos reales de BDP-NET.
- `OrderOperationType = 1` es el modo seguro de validacion: si BDP responde OK, valido conectividad, credenciales, permisos, articulo, formas de pago disponibles y shape de comanda sin escribir datos.
- En el BDP real del restaurante, BDP exige `Order.AlreadyInvoiced` y despues devuelve `[300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA` incluso usando `OnlyCheck`, `Invoice=false` y una comanda sin `Payments`. `/API/POSSeries/GetList` devuelve series, pero el manual no expone un campo de serie en `CreateOrder`; por tanto falta una configuracion de serie/destino en el TPV/WebLink, no una escritura desde nuestra app. **Ver guia detallada de diagnostico y ubicaciones en `bdp-error-300035-serie-facturacion-2026-05-29.md`. Ver tambien el resumen consolidado en `bdp-300035-resumen-completo-2026-06-01.md`.**
- La validacion final antes de activar escrituras reales es ejecutar el boton `Probar sincronizacion segura` en produccion y guardar captura/resultado de `listo_para_sincronizar = true`.
