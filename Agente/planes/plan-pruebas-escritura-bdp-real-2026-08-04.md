# Plan — Pruebas reales de escritura BDP (cliente, comanda, pago, factura)

> **Fecha:** 2026-08-04
> **Rama:** `glory-rs-rest` (HEAD `99b0eac6` = commit 028A-6 con mitigaciones)
> **Alcance:** pruebas LOCALES (repo local + PC local) contra el BDP real del restaurante.
> **Destino BDP:** `http://100.83.196.35:8068` (Tailscale, `restaurante-bdp` — ONLINE confirmado).
> **Base:** `Agente/usuario/auditoria-escrituras-bdp-mitigaciones-2026-08-03.md` (criterio de habilitación de 7 pasos).
> **Norma:** seguir la guía del cliente `guia-cliente-pruebas-integracion-bdp-2026-07-18.md` — una acción por vez, confirmar en BDP antes de continuar, detenerse ante el primer resultado inesperado.
> **Nota:** producción se actualiza en una fase posterior con autorización explicita de deploy; estas pruebas NO tocan producción.
> **Estado (2026-08-05):** Fase 0 ✅ · Fase 1 ✅ (datos definidos) · **Fase 2.1 ✅ (cliente 900001 creado y verificado en BDP real)** · **Fase 2.2 ✅ (comanda 5330 creada y verificada en BDP real)** · **Fase 2.3 ⏸ PENDIENTE de verificación con cliente (el cliente afirma que la suscripción WebLink de pago estaba activa; el BDP real responde "Subscripción no activada" — ver Hallazgo 048A-11 y sección Follow-up)** · Fase 2.4 en espera (depende de 2.3) · Correcciones 048A-8 y 048A-10 aplicadas y validadas.

---

## Fase 0 — Pre-requisitos y correcciones (ANTES de escribir)

| #   | Acción                                                                                                                                          | Estado | Evidencia                                                                                                   |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------- |
| 0.1 | Confirmar Tailscale: `restaurante-bdp` online                                                                                                   | ✅     | `tailscale status`: `100.83.196.35 active; direct`                                                          |
| 0.2 | PostgreSQL local corriendo                                                                                                                      | ✅     | Puerto 5432 escuchando (pid 6952)                                                                           |
| 0.3 | Config BDP en `.env` local (`BDP_BASE_URL`, `BDP_POS_ID=31`, credenciales)                                                                      | ✅     | Claves presentes en `.env`                                                                                  |
| 0.4 | **Añadir al `.env` local:** `BDP_WRITE_ALLOWED_ORIGINS=http://100.83.196.35:8068` y `BDP_CHECK_ORDER_ALLOWED_ORIGINS=http://100.83.196.35:8068` | ✅ | Aplicado 2026-08-04 — verificado por lectura de `.env` |
| 0.5 | Validación post-mitigación: `cargo fmt --check` + `cargo check` | ✅ | `fmt_exit=0`, `check_exit=0` (runner `scripts/run-cargo.mjs`) |
| 0.6 | Suite unit BDP: `cargo test --lib bdp` | ✅ | **85 passed / 0 failed** (exit 0) — incluye mitigaciones 028A-6 y el fix 048A-8 |
| 0.7 | (Opcional) Suite simulador E2E: 92 Python + 23 Rust                                                                                          | ⏳     | `tools/bdp-weblink-simulator/` — No bloqueante: contrato validado contra el BDP real (el simulador no cubre campos de gestión)                                                        |
| 0.8 | Backend + frontend local levantados | ✅ | Backend `http://localhost:3000` con **binario corregido (048A-8)** y allowlists forzadas en el entorno · Frontend `http://localhost:5174/` (el `5173` estaba ocupado por otro proyecto) — arrancado en puerto alternativo |

**Corrección 0.4 (imprescindible):** el código en `src/services/bdp_weblink.rs` (`ensure_target_allowed`, línea ~522) solo permite loopback (localhost) o hosts listados en `BDP_WRITE_ALLOWED_ORIGINS`/`BDP_CHECK_ORDER_ALLOWED_ORIGINS`. La IP Tailscale `100.83.196.35` NO es loopback → sin el allowlist, toda escritura falla con `Escritura BDP bloqueada`. Es la única corrección de entorno necesaria antes de probar.

**Corrección 0.4b (operativa):** `dotenvy` **no carga variables añadidas al `.env` con `Add-Content`** (problema de encoding). Al relanzar el backend hay que **forzar las allowlists en el entorno** antes de `npm run dev:back`:
```powershell
$env:BDP_WRITE_ALLOWED_ORIGINS="http://100.83.196.35:8068"
$env:BDP_CHECK_ORDER_ALLOWED_ORIGINS="http://100.83.196.35:8068"
```
Sin esto, toda escritura fallaría con 422 `destino no incluido en BDP_WRITE_ALLOWED_ORIGINS`.

**Corrección 048A-8 (imprescindible — descubierta al ejecutar 2.1):** el contrato `CreateCustomer` mínimo (solo `Code` + `FiscalName`) provoca en el BDP real con módulo gestión una `NullReferenceException` (.NET → HTTP 500 `"Referencia a objeto no establecida como instancia de un objeto."`). Fix aplicado y **validado contra el BDP real**:
- `BdpCreateCustomerRequest` en `src/services/bdp_weblink_catalog.rs` (~línea 489) ampliado a **todos los campos del contrato oficial** (`# WEBLINK RESTAPI.md`, sección CreateCustomer ~7038-7150): `code, fiscal_name, commercial_name, address, post_code, town, province, land_line, mobile_phone, fin, fin_type, email (key "Email"), per_discount, payment_mode, representative, area_code, tav_code, rate_code, overwrite`.
- **`FINType` en `i32`** (antes `f64`): el BDP real rechaza `1.0` con `"Input string '1.0' is not a valid integer. Path 'FinType'"`.
- **Técnica de validación segura del contrato:** enviar el payload completo contra el código existente (1) con `Overwrite=false` → respuesta de **duplicado** (sin NullReference) confirma el contrato sin crear datos. Usar siempre esta técnica para validar cualquier contrato antes de escribir.

**Corrección 048A-10 (imprescindible — descubierta al ejecutar 2.2):** el `Order.Total` del payload `Orders/Create` se construía como `importe_base + importe_iva` (p. ej. **5,50**), pero BDP valida el total de la comanda como la **suma de los `Item.Total` (bruto)** — con `Total=5,50` y `Item.Total=5,00` BDP rechaza con **`[300033]-EL IMPORTE TOTAL INDICADO DE LA COMANDA NO COINCIDE CON EL TEÓRICO`**. Fix aplicado y **validado contra el BDP real** en `src/services/bdp_sync.rs` (`build_order`): `Order.Total` = Σ `precio_unitario × cantidad − descuento` por línea (bruto). El IVA se transmite por `VatPct` de cada item y BDP lo calcula. Evidencia: el dry-run oficial (2026-06-01) pasaba validación de totales con `Item.Total=Price` y `Order.Total=Price` (sin IVA). Reintento exitoso → comanda **5330** creada.

**Hallazgo 048A-11 — LIMITACIÓN DE LA API GRATUITA DE WEBLINK (bloquea 2.3 y 2.4; PENDIENTE de verificación con el cliente):**
- `POST /API/Orders/Get` → `[301010]` en la API gratuita: solo devuelve `Status` (0 = abierta); **no devuelve** `Order`, `Total`, `Items` ni `Payments`. Glory necesita `Total`/`Payments` para reconciliar antes de pagar (diseño W12) → el pago vía Glory falla con 422 `no se pudo reconciliar la orden antes de escribir`.
- Probe de contrato `POST /API/Orders/Payment/Add` (amount=0, no escribe nada) → HTTP 200 con **`ErrorMessage: "Subscripción no activada"`** → el endpoint de pago no procesa en esta instalación.
- **Contradicción con el cliente:** el cliente afirma que la suscripción de pago estaba activa. Pendiente de verificar con cliente/proveedor WebLink: (a) que el módulo/licencia "WebLink REST API de pago" esté activado **en la instalación 100.83.196.35:8068** (no en otro entorno); (b) que el `CodigoIntegrador` (`VBW2MBM5`) tenga permiso de pago; (c) que no haya un subpath/puerto distinto para la API de pago.
- Implicación: las fases 2.3 (pago) y 2.4 (factura) **no son verificables en BDP real** hasta resolver la suscripción. Se registran como **pendientes de verificación**, no como fallidas (criterio del plan).
- Tenders del POS 31 (vía `Tenders/GetPOSList`): `Id=1 Contado` (efectivo del POS), 2 SABADELL, 3 La Caixa, 4 BANKINTER, 6 SANTANDER, 16 AMEX, 17 TAKE WAY, 19 UMAPPI, 22 CREDITO CLIENTE, 23 IBERCAJA. (`GetList` global añade Id=7 EFECTIVO, 18 JUST EAT, 20/21 EFECT. delivery, 24 GLOVO, 25 UBER). `bdp_tender_map` vacío → el pago habría usado `tender_id=1 (Contado)`.

## Follow-up cliente (2026-08-05)

### 1. Comanda 5330 — qué decirle al cliente para anularla en el TPV
- **Qué es:** comanda de prueba creada el 2026-08-05 (venta Glory `1070d6ef-714f-4fda-9c4c-f6f97017b438`, artículo de prueba, 5,00 + IVA = 5,50 €, tendencia efectivo, sin cliente asignado) mediante la prueba 2.2 del plan. **Status=0 (abierta)**, sin pagos ni ticket/factura.
- **Si le afecta:** una comanda abierta sin pagar puede aparecer en el listado de comandas pendientes del TPV y en conteos de caja abierta. No genera ticket ni movimiento de caja hasta que se cobre o anule.
- **Cómo anularla en el TPV de BDP:** abrir la comanda 5330 (o buscarla por comandas abiertas) y usar la opción de **anular comanda** estándar del TPV (la anulación manual es la única vía sin la API de pago; `CancelOrder` por API también responde "Subscripción no activada").
- **No bloquea nada en Glory:** la venta en Glory conserva su badge "Esperando validación · Orden: 5330" y no se altera; la anulación es solo higiene del TPV.

### 2. Suscripción WebLink de pago — qué pedir al cliente/proveedor
- Preguntar: ¿la licencia/módulo **"WebLink REST API de pago"** está activa en la instalación `100.83.196.35:8068` (Tailscale `restaurante-bdp`), con integrador `VBW2MBM5`?
- Si lo está: pedir cómo se activa el permiso de pago para ese integrador (puede requerir configuración por instalación o subpath/puerto distinto).
- Reproducción para el proveedor: `POST /API/Orders/Payment/Add` con un payload mínimo devuelve `{"InvoiceNumber":null,"ErrorMessage":"Subscripción no activada"}`.

---

## Fase 1 — Datos de prueba (sin escritura) — ✅ DEFINIDOS

> Estado: datos definidos y confirmados (2026-08-04/05). El cliente ya se creó y verificó en BDP (Fase 2.1).

1. **Cliente de prueba** definido: **código 900001**, FiscalName "CLIENTE ESCRITURA 2026-08-04 PRUEBA", CommercialName "PRUEBA", MobilePhone "699000004", `FINType=1` → **creado y verificado en BDP real (2026-08-05 01:22 UTC)**.
2. **Código verificado como libre** en BDP vía `ExportCustomers` (antes de la prueba solo existían 1,2,3,4).
3. **Venta pequeña** definida: 1× artículo **1001 "CAFE BOMBON"** (verificado en BDP: `Price1=5.00`, `TAVPer=10`, `TAVCode=1`) → importe base **5.00** + IVA 10% (**0.50**) = total **5.50**, descuento 0; descripción reconocible "PRUEBA COMANDA 2026-08-04".
4. **Forma de pago**: solo pago completo/único (NO parciales); `bdp_tender_map` vacío → la comanda irá sin tender explícito (forma con caja se confirma en Fase 2.3).
5. **Anulación manual**: confirmada en runbook (Fase 3) — comandas/facturas duplicadas se cancelan desde el TPV de BDP (`CancelOrder` no existe por API); una factura emitida es irreversible por API.

## Fase 2 — Ejecución de las 4 escrituras (UNA A LA VEZ)

> Regla de oro: autorizar solo la operación puntual → ejecutar una sola vez → confirmar en BDP → recién entonces pasar a la siguiente. Detenerse ante el primer resultado inesperado.

### 2.1 Crear cliente — ✅ COMPLETADA (2026-08-05 01:22 UTC)

- [x] Confirmar en BDP que el código aún no existe → **900001** verificado ausente en `ExportCustomers` (solo 1,2,3,4).
- [x] Autorizar únicamente la creación de ese cliente → arming `create_customer` (1 op, 10 min, snapshot vigente, `target_entity_type=cliente`, `target_entity_id=0991c7c8-5a9d-4867-9901-b98a646b79ac`, motivo documentado).
- [x] Ejecutar la acción una sola vez desde Glory → sync desde el cliente local **`0991c7c8-5a9d-4867-9901-b98a646b79ac`** ("PRUEBA / CLIENTE ESCRITURA 2026-08-04").
- [x] Confirmar → **TOTAL 5 clientes en BDP (1,2,3,4,900001)**; 900001: `FiscalName="CLIENTE ESCRITURA 2026-08-04 PRUEBA"`, `CommercialName="PRUEBA"`, `MobilePhone="699000004"`, `FINType=1`; cliente local `bdp_customer_code=900001` y `bdp_synced=true`; **ningún cliente preexistente modificado**.

**Evidencia:** primera escritura real BDP. Verificada programáticamente vía `/API/Customers/Export` (TOTAL 5) y lectura del registro nuevo. Requirió la corrección de contrato [048A-8] (ver Fase 0).

### 2.2 Crear comanda — ✅ COMPLETADA (2026-08-05 ~02:16 UTC)

- [x] Revisar artículos, cantidades, precios, impuestos, descuentos, cliente, canal, total → 1× "CAFE BOMBON" (1001, 5,00 + IVA 10%), total **5,50**, descuento 0; **venta sin `cliente_id`** (el request `POST /api/ventas` no admite cliente) → comanda sin `Customer` (aceptable para la prueba; `bdp_default_customer_code` vacío).
- [x] **Autorizar únicamente esa venta** → arming manual con alcance `create_order`, `target_entity_type=venta`, `target_entity_id=1070d6ef-714f-4fda-9c4c-f6f97017b438`, duración 10 min, `max_operaciones=1`, motivo documentado. (Nota: `ff_bdp_auto_arm` está en `false` en el restaurante → se usa arming manual por `PUT /api/configuracion/bdp/sync-mode`.)
- [x] **Crear la venta local** (una sola): `POST /api/ventas` → HTTP 201, **venta `1070d6ef-714f-4fda-9c4c-f6f97017b438`** (fecha 2026-08-04, descripcion "PRUEBA COMANDA 2026-08-04", base 5.00 + IVA 0.50, 1 línea 1001 CAFE BOMBON).
- [x] **Enviar una sola vez**: `POST /api/ventas/{id}/bdp-sync` con `{"auto_arm":false}`.
  - **Primer intento fallido:** `[300033]-EL IMPORTE TOTAL INDICADO DE LA COMANDA NO COINCIDE CON EL TEÓRICO` (Order.Total base+IVA → ver corrección 048A-10).
  - **Reintento correcto (tras aplicar fix 048A-10 y rebuild):** HTTP 200 → `bdp_synced=true`, **`bdp_order_id=5330`**, `bdp_sync_error=` vacío.
- [x] Confirmar en BDP:
  - `Orders/Get` por **OrderId=5330** → **Status=0** (comanda pendiente/abierta, existe). *(El WebLink gratuito no devuelve contenido: `[301010]` solo estado.)*
  - `Orders/Get` por **MarketId=9900 + MarketplaceOrderId=`G1070d6ef714f4f`** (15 chars) → **Status=0** (localizada; sin `[301000] no existe`).
  - Glory local `GET /api/ventas/{id}/bdp-status` → `bdp_order_id=5330`, `bdp_synced=true`, `bdp_sync_error=null`.

**Evidencia:** segunda escritura real BDP (primera comanda). Requirió la corrección de contrato [048A-10] (Order.Total = suma de Item.Total brutos, ver Fase 0). Impacto mínimo: comanda pendiente en el TPV, sin ticket ni factura.

### 2.3 Registrar pago (solo saldo completo pendiente de la comanda 2.2) — ⛔ BLOQUEADA (API gratuita sin suscripción de pago)

> Ejecutada la secuencia de la Fase 2.3 en 2026-08-05: confirmado comanda abierta, armado `add_payment`, intento de pago → **bloqueado por limitación de la API gratuita** (ver Hallazgo 048A-11 en Fase 0). No se escribió nada en BDP.

- [x] Confirmar en BDP que la comanda sigue abierta, no facturada, con el saldo esperado → `Orders/Get` OrderId=5330 → **Status=0** (abierta). Tenders POS 31 consultados (`GetPOSList`): efectivo = **Id=1 Contado**.
- [x] Autorizar el pago por ese importe exacto → arming `add_payment` (1 op, 10 min, `target_entity_id=1070d6ef-714f-4fda-9c4c-f6f97017b438`) → HTTP 200, modo `unidirectional`.
- [x] Ejecutar una sola vez → `POST /api/ventas/{id}/bdp-payment` `{amount:5.50, tender_id:1, confirmacion:"PAGAR … 5.50"}` → **422 `no se pudo reconciliar la orden antes de escribir: [301010]`** (la API gratuita no devuelve `Total`/`Payments`).
- [ ] Confirmar: 1 solo pago, importe/forma correctos, saldo 0, efecto en caja correcto → **NO EJECUTABLE**: probe `Payment/Add` (amount=0) → `"Subscripción no activada"`.
- [x] **Limpieza:** desarmado manual → `bdp_sync_mode=read_only` confirmado (el arming no se había consumido porque el fallo ocurrió antes del guard).

**Resultado:** Fase 2.3 **no verificable en BDP real** con la API gratuita. Queda registrada como pendiente de suscripción de pago (no como fallida).

### 2.4 Facturar (aprobación expresa de administración — consecuencias fiscales)

- [ ] Confirmar comanda pagada y aún no facturada.
- [ ] Autorizar únicamente la factura.
- [ ] Ejecutar una sola vez.
- [ ] Confirmar: exactamente 1 factura; número, serie, cliente, impuestos y total; Glory muestra el mismo número/estado.

### Evidencia por prueba

- Fecha y hora; operación; identificadores (cliente/comanda/pago/factura); resultado Glory; resultado BDP/TPV/caja; captura sin secretos ni datos personales completos; diferencias si las hay.

---

## Fase 3 — Mitigaciones ante fallo (de `runbook-operativo-bdp-2026-07-26.md`)

| Síntoma                                              | Acción                                                                                                                                                                    |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| BDP no responde / timeout                            | Verificar PC + Tailscale; no activar escritura manual mientras BDP inestable; reintentar sync cuando vuelva                                                               |
| Comanda duplicada                                    | No reintentar; BDP deduplica por `MarketplaceOrderId`; si hay 2 reales, cancelar la duplicada desde TPV de BDP (`CancelOrder` NO existe via API)                          |
| Pago registrado en BDP pero no en Glory (o al revés) | Estado debería quedar `ambiguo` → consultar estado real (`GetOrder`) → reconciliar o contactar soporte. **Nunca pulsar "Pagar" dos veces**                                |
| Factura sin número o duplicada                       | `InvoiceNumber` vacío = `ambiguo` → consultar estado; si hay duplicada, cancelarla en TPV. **Una factura emitida es irreversible por API** — corrección manual en BDP-NET |
| Error 300035 (serie no válida)                       | BDP-NET → Configuración TPV → Terminal 31 → Parámetros 6 → verificar serie destino (`00031TI`)                                                                            |
| Error 300005 (IVA incluido)                          | Verificar que la serie del terminal tenga IVA Incluido activo                                                                                                             |
| Throttling                                           | Esperar 30 s; el sistema lo marca `ambiguo` (no error permanente); reconciliar                                                                                            |
| Cliente sin código BDP bloquea venta                 | Vincular cliente en Glory, importar desde BDP, o usar `bdp_default_customer_code`                                                                                         |
| Resultado dudoso                                     | Detener la sesión de pruebas; documentar; NO reintentar a ciegas                                                                                                          |

**Reglas de seguridad amplificadas:**

- Si cualquier operación deja la venta en `ambiguo`, la sesión se detiene y se reconcilia antes de continuar.
- El arming es de un solo uso y vuelve automáticamente a `read_only` tras cada operación.
- No hay 2PC: una caída tras aceptación remota requiere reconciliación, nunca reintento automático.

---

## Fase 4 — Cierre

1. Verificar que el sistema quedó en modo `read_only` (sin arming residual).
2. Revisar "Historial BDP" (auditoría) — cada operación con resultado `exito`/`error`/`ambiguo`.
3. Recopilar evidencia completa (Fase 2).
4. Actualizar `Agente/completados/tareas-2026-08-04.md` y el roadmap (quitar el pendiente real #1 si todo pasó).
5. **Producción (NO hoy salvo instrucción):** con autorización explicita de deploy: push de `99b0eac6`, `npm run task:check`, deploy `coolify-manager deploy --name glory-rest --update --skip-backup` + health. Permitir allowlists en producción SOLO tras validación del cliente.

---

## Checklist de aceptación

- [ ] Cada operación autorizada crea exactamente un registro.
- [ ] Usa los datos, importes y relaciones esperados.
- [ ] Deja el mismo identificador y estado en Glory y BDP.
- [ ] No modifica otros registros.
- [ ] No requirió repetir una acción dudosa.
- Las pruebas no realizadas quedan registradas como **no verificadas en BDP real**, no como fallidas.

## Próximos pasos

1. [x] Ejecutar Fase 0 (allowlists + fmt/check + suite bdp + backend relanzado con binario corregido 048A-8).
2. [x] Confirmar datos de prueba con el responsable.
3. [x] **Fase 2.1** — cliente **900001** creado y verificado en BDP real (2026-08-05 01:22 UTC).
4. [x] **Fase 2.2** — comanda creada y verificada en BDP real (OrderId **5330**, venta `1070d6ef-…`, fix 048A-10).
5. [ ] **Fase 2.3** — pago 5,50: **⛔ bloqueada** por API gratuita (`Subscripción no activada`, Hallazgo 048A-11). Requiere suscripción de pago WebLink o entorno con ella activa.
6. [ ] **Fase 2.4** — facturar: en espera (depende de 2.3 y requiere aprobación expresa de administración).
7. [ ] Fase 4 — cierre: read_only confirmado, auditoría, evidencia, `Agente/completados/` y roadmap.
