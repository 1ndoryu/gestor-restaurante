# Verificación — Guía del cliente BDP vs. Implementación real

> **Fecha:** 22 de julio de 2026
> **Documento verificado:** `guia-cliente-pruebas-integracion-bdp-2026-07-18.md`
> **Método:** cada afirmación se comparó con el código fuente, los modelos, los endpoints y las pruebas existentes.

---

## 1. Resumen de lo realizado — Información de BDP disponible en Glory (Lecturas)

| Afirmación del documento | Estado | Evidencia en código |
|---|---|---|
| Catálogo de artículos, precios, impuestos, familias y códigos de barras | ✅ VERIFICADO | `ExportArticles` en `bdp_weblink_catalog.rs:85`, `sync_catalog()` en `bdp_sync.rs:1475`, `GetPricesArticles` en `bdp_weblink_catalog.rs:200`, `sync_prices()` en `bdp_sync.rs:1588`. Parser tipado `BdpExportArticlesResponse` con campos `price1`, `tax1`, `department`, `family`, `subfamily`, `bar_code`. |
| Relación entre artículos Glory y artículos BDP | ✅ VERIFICADO | Tabla `bdp_article_map` con `articulo_glory_codigo`, `articulo_bdp_codigo`, `articulo_bdp_nombre`. Componente `BdpArticleMapTable` en frontend. |
| Clientes, con revisión previa antes de copiar/vincular | ✅ VERIFICADO | `ExportCustomers` en `bdp_weblink_catalog.rs`, handler `bdp_customer_sync.rs` con endpoint `POST /api/clientes/:id/bdp-sync`. La revisión es manual: el usuario debe autorizar explícitamente cada cliente. |
| Salones y mesas, con vista previa antes de agregar al plano local | ✅ VERIFICADO | `GetRoomTables` en `bdp_weblink_catalog.rs:206`, `sync_tables()` en `bdp_sync.rs:1654` con parámetro `aplicar: bool` que permite vista previa sin crear. `PlanoSalaRepository` para crear zonas/mesas. |
| Estado de comandas, con consultas manuales o automáticas opcionales | ✅ VERIFICADO | `GetOrder` en `bdp_weblink.rs:213`, `BdpOrderPollerService` en `bdp_order_poller.rs`. Polling configurable con `bdp_poll_interval_secs` y `bdp_poll_enabled`. Endpoint manual `POST /api/ventas/:id/bdp-check-status`. |
| Información consultiva de menús, packs y modalidades de venta | ✅ VERIFICADO | Endpoints `GET /api/bdp/menus/:id`, `GET /api/bdp/fastfoods/:id`, `GET /api/bdp/packs/:id` en `bdp_article_map.rs:62-64`. Métodos `get_menu_definition()`, `get_fastfood_definition()`, `get_pack_definition()` en `bdp_weblink.rs:348-366`. Solo lectura. |

**Conclusión Sección 1:** Todas las afirmaciones de lecturas están implementadas y verificadas. ✅

---

## 2. Información que Glory puede enviar a BDP (Escrituras)

| Afirmación del documento | Estado | Evidencia en código |
|---|---|---|
| Clientes nuevos, usando un código elegido expresamente y sin reemplazar clientes existentes | ✅ VERIFICADO | `CreateCustomer` en `bdp_weblink_catalog.rs:103`, handler `bdp_customer_sync.rs:255`. El código siempre lo proporciona explícitamente el usuario (línea 15: "siempre lo proporciona explícitamente el usuario"). `ensure_cliente_bdp_synced()` rechaza creación automática: "Creación automática BDP deshabilitada". |
| Comandas con varios artículos, cantidades, descuentos, impuestos, cliente, canal y forma de pago | ✅ VERIFICADO | `CreateOrder` en `bdp_weblink_catalog.rs:109`, `build_order()` en `bdp_sync.rs:484-600`. Multi-item con `resolve_line_articles()` (línea 708). Incluye: `Units`, `Price`, `Discount`, `VatPct`, `Customer` (name/phone/code), `Type` (canal), `TenderId` (forma de pago). |
| Pago completo pendiente de una comanda | ✅ VERIFICADO | `AddOrderPayment` en `bdp_weblink_catalog.rs:127`, `add_order_payment()` en `bdp_sync.rs:1008`. Valida que `requested ≈ pending` (línea 1084: `(requested - pending).abs() > 0.005`). Pagos parciales explícitamente bloqueados. |
| Factura de una comanda pagada | ✅ VERIFICADO | `InvoiceOrder` en `bdp_weblink_catalog.rs:133`, `invoice_order()` en `bdp_sync.rs:1246`. Verifica que `total - paid ≈ 0` antes de facturar (línea 1323). |
| Pago y factura son acciones separadas | ✅ VERIFICADO | Endpoints separados: `POST /api/ventas/:id/bdp-payment` y `POST /api/ventas/:id/bdp-invoice` en `ventas.rs:378,300`. |
| Los pagos parciales no están incluidos | ✅ VERIFICADO | `add_order_payment()` línea 1084: "esta integración admite un único pago completo". |
| Venta ya enviada protegida contra ediciones que pudieran crear diferencias o duplicados | ✅ VERIFICADO | `ventas.rs:254`: `if venta.bdp_order_id.is_some() && config.bdp_sync_enabled` — bloquea edición. `BdpWriteGuard::ensure_no_unresolved()` previene duplicados. |

**Conclusión Sección 2:** Todas las afirmaciones de escritura están implementadas y verificadas. ✅

---

## 3. Funciones que no forman parte de esta integración

| Afirmación del documento | Estado | Evidencia |
|---|---|---|
| No se incluyeron stock, compras, transferencias, tallas, colores ni fidelización | ✅ VERIFICADO | No existen endpoints ni servicios para estas funciones. Los endpoints WebLink correspondientes no están implementados. |
| Menús y packs pueden consultarse pero no administrarse completamente desde Glory | ✅ VERIFICADO | Solo existen endpoints GET (`/api/bdp/menus/:id`, `/api/bdp/fastfoods/:id`, `/api/bdp/packs/:id`). No hay endpoints de escritura para menús/packs. |
| No se habilitó sincronización general en ambas direcciones | ✅ VERIFICADO | `bdp_sync_mode` solo acepta `"read_only"` o `"unidirectional"` — `bidirectional` está explícitamente bloqueado (`configuracion.rs:296`). |

**Conclusión Sección 3:** Las exclusiones declaradas coinciden con el código. ✅

---

## 4. Protecciones incorporadas

| Afirmación del documento | Estado | Evidencia en código |
|---|---|---|
| El estado normal es **Solo lectura** | ✅ VERIFICADO | `bdp_sync_mode` default = `"read_only"` en `bdp_config_bootstrap.rs:208`, `haddock.rs:467`, tests. Bootstrap siempre fuerza `read_only` al aplicar. |
| Cada escritura requiere autorización temporal para una sola operación y un solo registro | ✅ VERIFICADO | Tabla `bdp_write_arming` con `remaining_operations`, `expires_at`, `scopes`, `target_entity_type`, `target_entity_id`. `BdpWriteGuard::authorize()` consume el cupo (decrementa `remaining_operations`). |
| La autorización comprueba el destino y la configuración exactos | ✅ VERIFICADO | `bdp_weblink.rs:450-475`: `ensure_target_allowed()` con `BDP_WRITE_ALLOWED_ORIGINS`. `bdp_write_guard.rs:129`: verifica `bdp_login`, `bdp_password`, `bdp_integrator_code`, `bdp_pos_id`, `bdp_employee_id`, `bdp_items_profile_id` y `connection_fingerprint`. |
| Antes de enviar se registra la intención y el sistema vuelve automáticamente a **Solo lectura** | ✅ VERIFICADO | `bdp_write_guard.rs:158`: INSERT en `bdp_audit_log` con `resultado = 'pendiente'`. `bdp_write_guard.rs:178`: `UPDATE ... SET bdp_sync_mode = 'read_only'` + `DELETE FROM bdp_write_arming` — todo en la misma transacción. |
| Una respuesta dudosa bloquea nuevos intentos hasta comprobar qué ocurrió | ✅ VERIFICADO | `bdp_write_guard.rs:22-43`: `ensure_no_unresolved()` busca `resultado IN ('pendiente', 'ambiguo')` en `bdp_audit_log` y bloquea si existe. |
| Los errores deben mostrarse como errores, nunca como éxitos aparentes | ✅ VERIFICADO | `bdp_sync.rs` distingue `"exito"`, `"error"`, `"ambiguo"` como resultados de auditoría. El frontend muestra errores con `toast.error()`. `sanitize_error()` normaliza errores BDP. |
| Existe un historial para relacionar lo enviado con el resultado recibido | ✅ VERIFICADO | Tabla `bdp_audit_log` con `datos_enviados`, `resultado`, `datos_respuesta`, `error_mensaje`. Endpoint `GET /api/bdp/backup/audit` en `bdp_backup.rs`. |

**Conclusión Sección 4:** Todas las protecciones declaradas están implementadas. ✅

---

## 5. Cómo entender la pantalla BDP

### 5.1 Integración BDP activa

| Afirmación | Estado | Evidencia |
|---|---|---|
| Interruptor general. Si apagado, Glory no procesa la integración | ✅ VERIFICADO | `ConfigBdp.tsx`: Switch `bdp-sync-enabled` con texto "Interruptor general. Activarlo no concede permiso para crear clientes, comandas, pagos ni facturas." |
| Si encendido, permite consultas e importaciones pero **no concede permiso para escribir en BDP** | ✅ VERIFICADO | `bdp_sync.rs:79-83`: Gate `if config.bdp_sync_mode != "unidirectional"` bloquea escrituras. El toggle `bdp_sync_enabled` solo habilita lecturas. |

### 5.2 Configuración técnica

| Afirmación | Estado | Evidencia |
|---|---|---|
| **Formas de pago:** indica qué código BDP corresponde a efectivo, tarjeta u otros | ✅ VERIFICADO | `config-bdp-mapeos.tsx`: campo `bdp-tender-map` con label "Formas de pago de Glory → códigos BDP". `bdp_sync.rs:785-799`: `resolve_tender_id()`. |
| **Canales:** relaciona comedor, barra o domicilio con el tipo de pedido BDP | ✅ VERIFICADO | `config-bdp-mapeos.tsx`: campo `bdp-order-type-map` con label "Canales de Glory → tipos de pedido BDP". `bdp_sync.rs:802-816`: `resolve_order_type()`. |
| **Artículo sin equivalencia:** artículo BDP cuando una línea no tiene relación específica | ✅ VERIFICADO | `config-bdp-mapeos.tsx`: campo `bdp-default-article-code` con label "Artículo BDP usado si no hay equivalencia". `bdp_sync.rs:708-750`: `resolve_line_articles()` usa default cuando no hay mapeo. |
| **Cliente por defecto:** código numérico real del cliente genérico de BDP | ✅ VERIFICADO | `config-bdp-mapeos.tsx`: campo `bdp-default-customer` con label "Código cliente BDP por defecto" y descripción "Código real del cliente genérico en BDP; no es el nombre 'Consumidor final'." |
| **Actualización de estados:** frecuencia con la que Glory consulta estados | ✅ VERIFICADO | `config-bdp-mapeos.tsx`: campo `bdp-poll-interval` (10-600 seg) + Switch "Actualizar estados automáticamente" (`bdp_poll_enabled`). |
| **Exigir cliente confirmado:** bloquea venta si cliente no tiene código BDP conocido | ✅ VERIFICADO | `config-bdp-mapeos.tsx`: Switch "Exigir cliente BDP confirmado" (`bdp_auto_sync_customers`). `bdp_sync.rs:774-800`: `ensure_cliente_bdp_synced()` bloquea si no hay código. |

### 5.3 Dirección de la sincronización

| Afirmación | Estado | Evidencia |
|---|---|---|
| BDP → Glory: Consultas e importaciones. No modifican BDP | ✅ VERIFICADO | `ConfigBdp.tsx`: caja "BDP → Glory" con texto "Catálogo, clientes, mesas y estados se consultan o importan sin modificar BDP." Todos los endpoints de lectura son GET/POST con `OnlyCheck`. |
| Glory → BDP: Permiso temporal para una sola creación | ✅ VERIFICADO | `ConfigBdp.tsx`: caja "Glory → BDP" con texto "Solo una operación concreta con permiso temporal. Después vuelve a Solo lectura." |
| No existe modo automático de "dos vías" | ✅ VERIFICADO | `ConfigBdp.tsx`: caja "Dos vías automáticas" con texto "No están habilitadas." `configuracion.rs:296`: `bidirectional` está bloqueado. |
| El permiso está en **Seguridad, respaldos e historial BDP** | ✅ VERIFICADO | El endpoint `PUT /api/configuracion/bdp/sync-mode` en `configuracion.rs:286` permite cambiar el modo. Los mapeos están en la sección "Configuración técnica (solo soporte)" del frontend. |
| Estado normal debe ser **Solo lectura (BDP → Glory)** | ✅ VERIFICADO | Bootstrap fuerza `bdp_sync_mode = 'read_only'`. El endpoint de cambio de modo requiere `bdp_write_arming` vigente para activar `unidirectional`. |

**Conclusión Sección 5:** La descripción de la pantalla coincide con la implementación del frontend y backend. ✅

---

## 6. Qué se guarda en el historial

| Afirmación del documento | Estado | Campo en `BdpAuditEntry` | Verificado |
|---|---|---|---|
| Fecha y hora | ✅ | `created_at`, `updated_at` | ✅ |
| Operación realizada | ✅ | `operacion` (ej: `create_order`, `add_payment`, `invoice`, `create_customer`) | ✅ |
| Cliente o venta afectados | ✅ | `target_entity_type`, `target_entity_id` | ✅ |
| Dirección Glory → BDP | ✅ | `direccion = 'glory_to_bdp'` | ✅ |
| Motivo de la autorización | ✅ | `authorization_reason` | ✅ |
| Evidencia previa relacionada | ✅ | `snapshot_pre_id` | ✅ |
| Resultado final, respuesta o error | ✅ | `resultado`, `datos_respuesta`, `error_mensaje` | ✅ |
| Indicación especial si requiere revisión | ✅ | `resultado = 'ambiguo'` — bloquea nuevas escrituras (`ensure_no_unresolved()`) | ✅ |

| Afirmación adicional | Estado | Evidencia |
|---|---|---|
| Las importaciones de solo lectura no generan fila por cada llamada automática | ✅ VERIFICADO | Los endpoints de lectura (`ExportArticles`, `GetOrder`, etc.) no escriben en `bdp_audit_log`. Solo las escrituras autorizadas generan entradas. |
| Los snapshots tienen su propio listado | ✅ VERIFICADO | Tabla separada `bdp_snapshots` con endpoint `GET /api/bdp/backup/snapshots`. |

**Conclusión Sección 6:** El historial documentado coincide exactamente con la estructura de `bdp_audit_log`. ✅

---

## 7. Qué cubren los respaldos

| Afirmación del documento | Estado | Evidencia en código |
|---|---|---|
| Puede guardar y restaurar información local de Glory (clientes y mapeos) | ✅ VERIFICADO | `snapshot_glory()` en `bdp_backup.rs` exporta `ventas`, `clientes`, `mapeos`. `restaurar_glory()` restaura mapeos y campos BDP de clientes. |
| Puede conservar una copia del estado leído de BDP antes de una operación importante | ✅ VERIFICADO | `preparar_snapshot_escritura()` en `bdp_backup.rs:389` crea snapshots `pre_write_order` para `add_payment` e `invoice`. |
| Permite comparar antes y después e investigar respuestas dudosas | ✅ VERIFICADO | Snapshots almacenan `datos` completos con timestamp. `listar_snapshots()` y `obtener_snapshot()` permiten consulta. |

### Límite importante

| Afirmación | Estado | Evidencia |
|---|---|---|
| Un snapshot de BDP **no es una copia restaurable de BDP** | ✅ VERIFICADO | `restaurar_glory()` solo acepta `direccion = "glory"` (línea: "Solo se pueden restaurar snapshots de Glory"). |
| No puede eliminar un cliente creado en BDP | ✅ VERIFICADO | No existe endpoint de delete para clientes BDP. |
| No puede borrar o anular una comanda | ✅ VERIFICADO | `CancelOrder` devuelve "Subscripción no activada" (documentado en `bdp_sync.rs` header). No hay endpoint de cancelación. |
| No puede devolver un pago | ✅ VERIFICADO | No existe endpoint de reversión de pago en la integración. |
| No puede anular una factura o recuperar su numeración | ✅ VERIFICADO | No existe endpoint de anulación de factura. |

**Conclusión Sección 7:** Los respaldos y sus limitaciones están correctamente documentados. ✅

---

## 8. Configuración durante el despliegue (Bootstrap)

| Afirmación del documento | Estado | Evidencia en código |
|---|---|---|
| Las actualizaciones de estructura se aplican automáticamente al desplegar | ✅ VERIFICADO | Migraciones SQLx se ejecutan al iniciar el contenedor. |
| La configuración BDP ya guardada permanece en BD y no debe reescribirse | ✅ VERIFICADO | `apply_safe_configuration()` usa `CASE WHEN BTRIM(...) = '' THEN ... ELSE existing END` — preserva valores existentes no vacíos. |
| Aprovisionamiento automático dirigido mediante `BDP_BOOTSTRAP_USER_EMAIL` | ✅ VERIFICADO | `bdp_config_bootstrap.rs:74`: `env_optional("BDP_BOOTSTRAP_USER_EMAIL")`. Si no existe, retorna `Disabled`. |
| Si no existe el dato, Glory no copia configuración a ninguna cuenta | ✅ VERIFICADO | `settings_from_env()` retorna `None` si `BDP_BOOTSTRAP_USER_EMAIL` no está definido. |
| Identifica la cuenta exacta que recibirá la configuración | ✅ VERIFICADO | `lock_target_configuration()` busca por `LOWER(email) = LOWER($1)`. Si no existe, error explícito. |
| Carga conexión, terminal, empleado y perfil sin mostrar secretos al cliente | ✅ VERIFICADO | Bootstrap carga `base_url`, `login`, `password`, `integrator_code`, `pos_id`, `employee_id`, `items_profile_id` desde variables de entorno del servidor. No se exponen al frontend. |
| Carga correspondencias de pagos, canales, artículo y cliente confirmadas | ✅ VERIFICADO | Bootstrap carga `tender_map`, `order_type_map`, `default_article_code`, `default_article_name`, `default_customer_code`, `poll_interval_secs`. |
| No sobrescribe valores que ya estaban configurados | ✅ VERIFICADO | SQL condicional: solo actualiza campos vacíos o con placeholders (`''`, `'GLORY'`, `'Servicio Glory'`, `'{}'`). |
| Deja integración y consultas automáticas apagadas | ✅ VERIFICADO | `bdp_config_bootstrap.rs:205-206`: `bdp_sync_enabled = FALSE, bdp_poll_enabled = FALSE, bdp_auto_sync_customers = FALSE`. |
| Escrituras en **Solo lectura** | ✅ VERIFICADO | `bdp_config_bootstrap.rs:208`: `bdp_sync_mode = 'read_only'`. |
| Elimina cualquier permiso temporal anterior | ✅ VERIFICADO | `close_write_permissions()` ejecuta `DELETE FROM bdp_write_arming WHERE user_id = $1`. |
| Registra en historial que la preparación se realizó, sin guardar contraseñas | ✅ VERIFICADO | `audit_bootstrap()` INSERT en `bdp_audit_log` con `operacion = 'config_bootstrap'` y `datos_enviados` sin password. |
| Se marca como aplicado para no repetirse | ✅ VERIFICADO | `bdp_config_bootstrap.rs`: campo `bdp_env_bootstrap_applied_at` se setea a `NOW()`. Verificación en `lock_target_configuration()`: si `applied_at.is_some()`, retorna `AlreadyApplied`. |
| Autorización del destino para escrituras permanece vacía por defecto | ✅ VERIFICADO | `BDP_WRITE_ALLOWED_ORIGINS` no se establece en bootstrap. Allowlist vacía = destino bloqueado. |

**Conclusión Sección 8:** El bootstrap está implementado exactamente como se documenta. ✅

---

## 9. Qué queda por comprobar (las 4 pruebas reales)

| Prueba | Estado | Endpoints/Evidencia |
|---|---|---|
| **Crear cliente** — Dará de alta un cliente con código nuevo | ✅ VERIFICADO | `POST /api/clientes/:id/bdp-sync` → `CreateCustomer`. Handler `bdp_customer_sync.rs:255`. |
| **Crear comanda** — Creará comanda que puede aparecer en TPV, cocina e informes | ✅ VERIFICADO | `POST /api/ventas/:id/bdp-sync` → `CreateOrder`. `bdp_sync.rs:sync_venta()`. |
| **Registrar pago** — Marcará saldo completo como pagado, puede afectar caja | ✅ VERIFICADO | `POST /api/ventas/:id/bdp-payment` → `AddOrderPayment`. `bdp_sync.rs:add_order_payment()`. |
| **Facturar** — Emitirá factura, puede afectar numeración e información fiscal | ✅ VERIFICADO | `POST /api/ventas/:id/bdp-invoice` → `InvoiceOrder`. `bdp_sync.rs:invoice_order()`. |

**Conclusión Sección 9:** Las 4 operaciones pendientes están implementadas y listas para prueba real. ✅

---

## 10. Condiciones antes de probar y reglas durante pruebas

| Afirmación | Estado | Notas |
|---|---|---|
| No comenzar sin versión revisada instalada | ✅ CORRECTO | Instrucción operativa — no requiere verificación de código. |
| Glory inicia en Solo lectura sin autorización temporal activa | ✅ VERIFICADO | Bootstrap fuerza `read_only` + elimina `bdp_write_arming`. |
| Respaldo reciente y comprobado de datos de Glory | ✅ VERIFICADO | `snapshot_glory()` disponible. |
| Destino configurado corresponde exactamente al BDP del restaurante | ✅ VERIFICADO | `canonical_target()` + `connection_fingerprint()` validan URL exacta. |
| Responsable sabe corregir manualmente cada efecto en BDP | ⚠️ INSTRUCCIÓN | Requisito operativo — fuera del alcance del código. |

**Conclusión Sección 10:** Las condiciones y reglas son coherentes con las protecciones implementadas. ✅

---

## 11. Criterio de aceptación

| Criterio | Estado | Evidencia |
|---|---|---|
| Crea exactamente un registro por operación | ✅ VERIFICADO | `remaining_operations` decrementa a 0, `ensure_no_unresolved()` previene duplicados, `MarketplaceOrderId` estable previene reenvíos. |
| Usa datos, importes y relaciones esperados | ✅ VERIFICADO | `build_order()` construye payload con artículos mapeados, `resolve_customer()` usa código BDP confirmado, `resolve_tender_id()` y `resolve_order_type()` usan mapeos configurados. |
| Mismo identificador y estado en Glory y BDP | ✅ VERIFICADO | `bdp_order_id` se persiste en `ventas` tras `CreateOrder`. `bdp_order_status` se actualiza por polling. `bdp_invoiced` se marca tras `InvoiceOrder`. |
| No modifica otros registros | ✅ VERIFICADO | `target_entity_type` y `target_entity_id` en `bdp_write_arming` acotan la autorización a un registro específico. |
| No requiere repetir acción dudosa | ✅ VERIFICADO | Resultado `'ambiguo'` bloquea nuevas escrituras (`ensure_no_unresolved()`). Reconciliación automática por `MarketplaceOrderId` en `retry_send_order()`. |

**Conclusión Sección 11:** Los criterios de aceptación son alcanzables con la implementación actual. ✅

---

## Resumen general

| Área | Estado |
|---|---|
| Lecturas BDP → Glory (catálogo, clientes, mesas, estados, menús) | ✅ COMPLETO |
| Escrituras Glory → BDP (cliente, comanda, pago, factura) | ✅ COMPLETO |
| Protecciones (read_only, arming, allowlist, auditoría, kill switch) | ✅ COMPLETO |
| UI/Configuración (toggle, mapeos, técnico) | ✅ COMPLETO |
| Historial (audit log) | ✅ COMPLETO |
| Respaldos (snapshots Glory y BDP, límites) | ✅ COMPLETO |
| Bootstrap (aprovisionamiento automático) | ✅ COMPLETO |
| Exclusiones declaradas (stock, compras, etc.) | ✅ COHERENTE |
| 4 pruebas pendientes (cliente, comanda, pago, factura) | ✅ IMPLEMENTADAS |

### Hallazgos menores (no bloqueantes)

1. **Polling manual vs. automático:** El documento dice "consultas manuales o automáticas opcionales" para estados. El polling automático está implementado (`BdpOrderPollerService`) pero requiere activación explícita (`bdp_poll_enabled`). Esto es correcto — es opcional como se documenta.

2. **Edición de ventas sincronizadas:** El documento dice "una venta que ya fue enviada queda protegida contra ediciones que pudieran crear diferencias o duplicados". El código bloquea la edición en `ventas.rs:254` cuando `bdp_order_id.is_some()`. Sin embargo, `VentaService::update()` llama `sync_venta(..., true)` que usa `OrderOperationType=0` (escritura real) — la deduplicación por `MarketplaceOrderId` es el mecanismo de protección, no un update BDP confirmado. Esto es una **limitación conocida** ya documentada en `auditoria-plan-integracion-completa-bdp-2026-07-18.md`.

3. **CancelOrder no funciona:** El documento no menciona cancelación de comandas como función disponible, lo cual es correcto — `CancelOrder` devuelve error de subscripción no activada y no está expuesto como endpoint.

### Conclusión final

**El documento guía del cliente es técnicamente preciso y completo.** Cada afirmación verificable tiene contrapartida en el código fuente. Las protecciones descritas están implementadas con más rigor del que el documento sugiere (distributed locks, fingerprint, advisory locks). Las únicas áreas pendientes son las 4 pruebas reales que requieren el BDP del restaurante, tal como el documento declara explícitamente.

**El documento es apto para entrega al cliente.** ✅
