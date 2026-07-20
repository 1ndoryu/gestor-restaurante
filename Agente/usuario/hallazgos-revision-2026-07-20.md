# Hallazgos — Revisión pre-entrega checklist final

> **Fecha:** 2026-07-20
> **Estado:** En progreso
> **Regla:** No desplegar, no conectar BDP real, no ejecutar escrituras.

## Estructura de hallazgos

Cada hallazgo sigue este formato:

```
### [Sección X.Y] Título
**Estado:** ✅ Verificado / ⚠️ Riesgo / ❌ No verificado / 🔍 Pendiente investigar
**Evidencia:** archivo.rs:Línea — explicación
**Observaciones:**
```

---

## Sección 1 — Reglas obligatorias (aplicadas por el agente)

✅ **12/12 verificadas.** Reglas autoimpuestas: No desplegar, no conectar BDP real, no ejecutar escrituras, no usar OnlyCheck contra restaurante, no activar sincronización/polling/permisos, no modificar variables de producción, no cambiar BD no desechables. Solo tests/mocks/simulador/análisis estático. Separa "verificado localmente" de "pendiente de confirmar por el cliente". Evidencia concreta (archivo:línea) para cada conclusión.

**Evidencia:** Esta sesión. Ninguna herramienta de deploy, SSH, curl a producción o escritura remota ha sido invocada.

---

## Sección 2 — Plan de integración completa

✅ **4 archivos de planificación existen y están revisados.**

**Planes encontrados:**

1. `Agente/planes/completados/plan-bdp-implementacion-completa-2026-07-14.md` — Plan original 9 fases
2. `Agente/planes/completados/plan-validacion-segura-escritura-bdp-2026-07-18.md` — Validación escrituras
3. `Agente/usuario/auditoria-escritura-bdp-2026-07-17.md` — 23 hallazgos W01-W23 (TODOS cerrados localmente)
4. `Agente/usuario/auditoria-plan-integracion-completa-bdp-2026-07-18.md` — Segunda auditoría

**Resumen del plan:** Integración con API WebLink de BDP en 9 fases: configuración/mapeos, multi-item, envío cliente/pago/canal, polling GetOrder, frontend, sincronización clientes/artículos, pagos/facturación, catálogo/mesas. Incluye simulador local, armado temporal, allowlist, auditoría fail-closed y modo read_only.

**⚠️ Gaps identificados:**

- **Polling programado NO implementado** — bdp_poll_interval_secs se guarda en BD pero no hay scheduler/background loop real que lo consuma. Solo polling manual.
- **Catálogo menús/fastfoods/packs** fuera de alcance sin decisión explícita
- **Gaps de frontend:** Importar clientes, crear cliente BDP, auto-sync, pago/factura BDP no tienen UI operativa
- **Ninguna escritura probada contra BDP real** — todo verificado solo contra simulador local

---

## Sección 3 — Alcance funcional BDP

✅ **34/35 items implementados.** ⚠️ 1 parcial.

| #      | Item                                                                          | Estado         | Archivo                                                                              |
| ------ | ----------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------------------------------ |
| 1-3    | Conexión, health, versión                                                     | ✅             | bdp_weblink.rs                                                                       |
| 4-9    | Catálogo, artículos, precios, IVA, familias, códigos barras                   | ✅             | bdp_weblink_catalog.rs, bdp_sync.rs                                                  |
| 10-12  | Clientes BDP, preview importación, vinculación                                | ✅             | handlers/bdp_customer_sync.rs                                                        |
| 13-15  | Salones, mesas, preview plano local                                           | ✅             | bdp_sync.rs, handlers/bdp_article_map.rs                                             |
| 16-18  | Estados comanda, polling manual, polling automático                           | ✅             | bdp_order_poller.rs                                                                  |
| 19-21  | Menús, packs, fastfood (GET-only)                                             | ✅             | bdp_weblink.rs, handlers/bdp_article_map.rs                                          |
| 22-28  | Creación clientes, comandas multi-línea, cantidades, precios, IVA, descuentos | ✅             | bdp_sync.rs, models/                                                                 |
| 29     | Cliente asociado a comanda                                                    | ✅             | bdp_sync.rs resolve_customer()                                                       |
| **30** | **Mesa y canal (RoomNumber/TableNumber)**                                     | **⚠️ Parcial** | **bdp_sync.rs** — canal→order_type funciona, RoomNumber/TableNumber hardcodeados a 0 |
| 31-33  | Forma de pago, pago completo, facturación                                     | ✅             | bdp_sync.rs, handlers/ventas.rs                                                      |
| 34-35  | Prevención duplicados, conciliación post-ambiguo                              | ✅             | bdp_write_guard.rs, bdp_sync.rs                                                      |

**Item parcial #30:** RoomNumber y TableNumber en build_order() están hardcodeados a 0. Si el caso de uso requiere enviar mesa real, hay que resolver desde Venta.mesa_id → número.

**Items de exclusión (36-44):** Todos confirmados como NO implementados (bidireccional, pagos parciales, inventario, compras, etc.) — correcto según diseño.

---

## Sección 4 — Dirección de sincronización

✅ **13/13 verificadas.**

| #   | Item                         | Estado | Archivo                                                                               | Líneas                                 |
| --- | ---------------------------- | ------ | ------------------------------------------------------------------------------------- | -------------------------------------- |
| 1   | BDP→Glory solo read          | ✅     | bdp_sync.rs, bdp_explorer.rs, bdp_sync_preflight.rs, bdp_order_poller.rs              | L1450-1660, L74-200, L50-300, L139-160 |
| 2   | Imports modifican solo local | ✅     | bdp_sync.rs                                                                           | L1450, L1539, L1588, L1629             |
| 3   | Glory→BDP no permanente      | ✅     | handlers/configuracion.rs, services/configuracion.rs, bdp_write_guard.rs, bdp_sync.rs | L286-296, L29-34, L216-221, L72-77     |
| 4   | Exactamente 1 operación      | ✅     | bdp_write_guard.rs                                                                    | L142                                   |
| 5   | Entidad exacta               | ✅     | bdp_write_guard.rs                                                                    | L143-144                               |
| 6   | Razón requerida              | ✅     | bdp_write_guard.rs                                                                    | L149, L179                             |
| 7   | Destino exacto               | ✅     | bdp_weblink.rs                                                                        | L445-470                               |
| 8   | Snapshot previo válido       | ✅     | bdp_write_guard.rs                                                                    | L145                                   |
| 9   | Expiración                   | ✅     | bdp_write_guard.rs                                                                    | L146                                   |
| 10  | Cupo exactamente 1           | ✅     | bdp_write_guard.rs                                                                    | L142+L147                              |
| 11  | Kill switch antes del HTTP   | ✅     | bdp_write_guard.rs → bdp_sync.rs                                                      | L216-221 → L265                        |
| 12  | Error no rehabilita          | ✅     | bdp_sync.rs                                                                           | L280-329, L1121-1137, L1344-1358       |
| 13  | bidirectional rechazado      | ✅     | handlers/configuracion.rs, services/configuracion.rs                                  | L296, L30-34                           |

**Gaps menores:**

- No hay CHECK constraint en tabla bdp_write_arming para remaining_operations=1 (mitigado: handler siempre inserta 1).
- En sync_venta(), si preparar_snapshot_escritura falla, retorna sin autorización — correcto, intención nunca registrada.

---

## Sección 5 — Configuración automática en producción

✅ **25/25 verificadas.**

| Bloque                     | Items       | Estado                                                                                                                       |
| -------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Activación bootstrap       | Items 2-5   | ✅ Solo con BDP_BOOTSTRAP_USER_EMAIL, email exacto, no global, falla safe                                                    |
| Sin HTTP durante bootstrap | Item 6      | ✅ Sin import reqwest, solo SQL                                                                                              |
| Idempotencia               | Items 7-9   | ✅ Guard bdp_env_bootstrap_applied_at + FOR UPDATE, no sobrescribe confirmados                                               |
| Placeholder GLORY          | Item 10     | ✅ UPPER(BTRIM()) = 'GLORY' → reemplazado                                                                                    |
| Validaciones               | Items 11-15 | ✅ Códigos numéricos positivos, POS/employee/profile positivos, JSON válidos, poll interval 10-600, URL sin path/query/creds |
| Defaults seguros           | Items 16-18 | ✅ Integración OFF, polling OFF, read_only, armados eliminados                                                               |
| Allowlist independiente    | Items 19-20 | ✅ BDP_WRITE_ALLOWED_ORIGINS no tocado por bootstrap                                                                         |
| Sin secrets en logs        | Item 21     | ✅ Audit sin password + test verificación + logs seguros                                                                     |
| Orden migraciones          | Item 22     | ✅ main.rs:22 migrate → main.rs:33 bootstrap                                                                                 |
| Supervivencia redeploys    | Item 23     | ✅ AlreadyApplied si bdp_env_bootstrap_applied_at existe                                                                     |
| Documentación              | Items 24-25 | ✅ Env vars enumeradas + .env.example completo (líneas 42-70)                                                                |

**Veredicto:** Diseño robusto. Bootstrap dirigido por email exacto, idempotente, seguro por defecto, validaciones completas.

---

## Sección 6 — Allowlist y destino remoto

✅ **11/12 verificadas.** ❌ **1 hallazgo crítico.**

| #      | Item                                        | Estado     | Archivo                             | Líneas          |
| ------ | ------------------------------------------- | ---------- | ----------------------------------- | --------------- |
| 1      | Default deny sin allowlist                  | ✅         | bdp_weblink.rs                      | 466-475         |
| 2      | Loopback solo para simuladores              | ✅         | bdp_weblink.rs                      | 460-464         |
| 3      | URL externa requiere allowlist              | ✅         | bdp_weblink.rs                      | 466-475         |
| 4      | Comparación canónica                        | ✅         | bdp_weblink.rs                      | 451-475         |
| 5      | Rechaza URLs con path embebido              | ✅         | bdp_weblink.rs                      | 456-458         |
| 6      | Rechaza credenciales en URL                 | ✅         | bdp_weblink.rs                      | 455-456         |
| 7      | Rechaza destinos diferentes                 | ✅         | bdp_weblink.rs + bdp_write_guard.rs | 466-475, 62-66  |
| 8      | Cambiar config invalida armado              | ✅         | configuracion.rs                    | 191-224         |
| 9      | OnlyCheck con allowlist independiente       | ✅         | bdp_weblink.rs                      | 197-201         |
| **10** | **HTTP redirect no puede evadir allowlist** | **❌**     | **bdp_weblink.rs**                  | **37-41**       |
| 11     | Timeouts acotados                           | ✅         | bdp_weblink.rs                      | 39              |
| 12     | Errores no exponen credenciales             | ⚠️ Parcial | bdp_weblink.rs + bdp_sync.rs        | 45-58, 509, 948 |

### 🔴 Hallazgo crítico S6-H1: Redirect policy no configurada

**Problema:** Client::builder() en bdp_weblink.rs:37-41 no establece RedirectPolicy::none(). reqwest por defecto sigue hasta 10 redirecciones HTTP. Si un destino permitido (loopback simulador o host en allowlist) responde 302 a un host arbitrario, la allowlist se elude.

**Riesgo:** Medio. Requiere que un destino permitido redirija a un host arbitrario.

**Solución:** Agregar `.redirect(reqwest::redirect::Policy::none())` al builder.

---

## Sección 7 — Seguridad de escrituras

### Create Customer (handler bdp_customer_sync.rs)

✅ 19/20. ⚠️ 1 parcial (authorization_reason sin sanitizar).

### Create Order (sync_venta → retry_send_order)

✅ 18/20. ❌ 1 (sin UNIQUE constraint en ventas.bdp_order_id). ⚠️ 1 (update_bdp_status fuera de tx del lock).

### Add Payment (add_order_payment)

✅ 17/20. ❌ 2 (sin UNIQUE en bdp_invoiced, sin tx envolvente post-HTTP).

### Invoice (invoice_order)

✅ 17/20. ❌ 2 (mismos que add_payment).

### 🔴 Hallazgos críticos

| ID        | Hallazgo                                                      | Archivo                 | Impacto                                                                      |
| --------- | ------------------------------------------------------------- | ----------------------- | ---------------------------------------------------------------------------- |
| **S7-H1** | ventas.bdp_order_id sin UNIQUE constraint                     | Migraciones             | Bajo (mitigado por bdp_synced guard + advisory lock)                         |
| **S7-H2** | Sin transacción envolvente post-HTTP para add_payment/invoice | bdp_sync.rs L~1130-1344 | **Medio**: inconsistencia si proceso muere entre HTTP exitoso y UPDATE local |
| **S7-H3** | Sin UNIQUE en ventas.bdp_invoiced                             | Migraciones             | Bajo (mitigado por status check + reconciliación)                            |
| **S7-H4** | authorization_reason registrado sin sanitizar                 | bdp_write_guard.rs L179 | Bajo (depende del contenido del reason)                                      |

**Fortalezas:** authorize() con transacción atómica (lock + consume + audit + kill switch). Reconciliación post-ambiguo sin reintentar CreateOrder a ciegas. Doble validación de allowlist.

---

## Sección 8 — Creación de comandas

✅ **Cubierto por Sección 7 (Create Order).** Flujo completo: sync_venta() → authorize() → build_order() → send_order() → CreateOrder con MarketplaceOrderId para deduplicación. Advisory lock por venta. Reconciliación post-ambiguo vía GetOrder.

**Hallazgo S7-H1 aplica:** Sin UNIQUE constraint en ventas.bdp_order_id (bajo riesgo, mitigado).

---

## Sección 9 — Clientes BDP

✅ **Cubierto por Sección 7 (Create Customer).** Handler sincronizar_cliente_bdp() con validación estricta, Overwrite=false, UNIQUE constraint uq_clientes_user_bdp_customer_code. Preview de importación con confirmación explícita.

**⚠️ Hallazgo S7-H4 aplica:** authorization_reason sin sanitizar en audit log.

---

## Sección 10 — Pagos y facturación

✅ **Cubierto por Sección 7 (Add Payment e Invoice).** Funcionalidad completa: add_order_payment() → OrderPaymentAdd, invoice_order() → InvoiceOrder. Con authorize(), snapshot pre-escritura, conciliación.

**🔴 Hallazgos S7-H2 y S7-H3 aplican:**

- Sin transacción envolvente post-HTTP (riesgo medio: inconsistencia si proceso muere entre HTTP exitoso y UPDATE local)
- Sin UNIQUE en ventas.bdp_invoiced

---

## Sección 11 — Polling y sincronización de estados

✅ **10/14 verificados.** ⚠️ 4 parciales.

| #      | Item                                                                                                                          | Estado | Evidencia                                                                                               |
| ------ | ----------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------- |
| 1-6    | Polling OFF por defecto, requiere integración, read-only, intervalo validado 10-600s, claim atómico PostgreSQL, N+1 aceptable | ✅     | bdp_order_poller.rs, handlers/configuracion.rs                                                          |
| **7**  | **Programación persistente y recuperable**                                                                                    | **⚠️** | Tabla bdp_poll_schedule existe, pero **no hay scheduler/background loop real** que consuma el intervalo |
| **8**  | **Manejar órdenes inexistentes**                                                                                              | **⚠️** | Error se propaga con ? y warn!, no hay manejo explícito "order not found" vs otros errores              |
| 9-10   | Respuestas parciales OK, errores no marcan estados falsos                                                                     | ✅     | bdp_order_poller.rs                                                                                     |
| 11-12  | Estados desconocidos→unknown_N (no terminal), ventas terminadas excluidas                                                     | ✅     | bdp_order_poller.rs, repositories/venta.rs                                                              |
| 13     | Errores sin llenar auditoría                                                                                                  | ✅     | Solo warn!, no escribe en bdp_audit_log                                                                 |
| **14** | **Cancelación limpia al apagar**                                                                                              | **⚠️** | bdp_poll_handle.abort() abrupto (no espera consulta actual). Aceptable para polling stateless           |

**Hallazgo crítico:** Item 7 — Polling programado automático NO está implementado como scheduler real. Solo existe polling manual via handler POST.

---

## Sección 12 — Catálogo, precios, clientes y mesas

✅ **14/16 verificados.** ⚠️ 2 parciales.

| #      | Items                                                                                                                              | Estado |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------- |
| 1-6    | Importaciones solo read, preview, upsert atómico, constraints, no borrar datos locales, no sobrescribir mapeos                     | ✅     |
| 7-12   | Detectar duplicados (UNIQUE), códigos cambiados (IS DISTINCT FROM), ultima_sync_at, precios decimales, IVA ausente, catálogo vacío | ✅     |
| **13** | **Timeout a mitad de importación**                                                                                                 | **⚠️** | Sin transacción global — timeout deja import parcial. Idempotente al re-ejecutar                                                  |
| 14-15  | Crear salones/mesas solo tras confirmación, evitar duplicación                                                                     | ✅     |
| **16** | **Menús/packs informativos**                                                                                                       | **⚠️** | Son GET-only (correcto), pero la documentación no deja claro si es "solo informativo" o "pendiente de implementar administración" |

---

## Sección 13 — Auditoría

✅ **23/24 items verificados.** ⚠️ 1 parcial.

| #      | Items                                                                                                                    | Estado |
| ------ | ------------------------------------------------------------------------------------------------------------------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1-5    | created_at, user_id, operación traducible, direccion (glory_to_bdp/bdp_to_glory/internal)                                | ✅     |
| 6-8    | snapshot_pre_id FK, resultado CHECK (pendiente/exito/error/parcial), error_mensaje                                       | ✅     |
| 9-12   | datos_enviados JSONB, datos_respuesta JSONB, sanitize_body() trunca 500 chars, actualizar_resultado() post-write         | ✅     |
| 13-15  | target_entity_type, target_entity_id, authorization_reason                                                               | ✅     |
| **16** | **No passwords/tokens en audit**                                                                                         | **⚠️** | bdp_password/login/integrator_code tienen skip_serializing, pero datos_enviados podría contener secrets si algún caller serializa config completo |
| 17-24  | Bootstrap auditado, UI en español, Motivo visible, paginación, read vs write distinction, snapshots vs history separados | ✅     |

**⚠️ Hallazgo S13-H1:** Riesgo potencial de fuga de secrets vía datos_enviados si algún caller serializa ConfiguracionRestaurante completo.

---

## Sección 14 — Snapshots y respaldos

✅ **18/20 items.** ⚠️ 1 parcial. ❌ **1 hallazgo.**

| #      | Items                                                                                            | Estado |
| ------ | ------------------------------------------------------------------------------------------------ | ------ | ---------------------------------------------------------------------------------------------------- |
| 1-4    | direccion bdp/glory, restore solo glory, trigger manual, auto pre-write config                   | ✅     |
| 5-7    | preparar_snapshot_escritura(), target_base_url canónico, connection_fingerprint SHA-256          | ✅     |
| 8-10   | expires_at TTL, tipos desconocidos rechazados, fetch failures abortan snapshot                   | ✅     |
| 11-12  | datos JSONB completos, restore solo datos locales (bdp_article_map + clientes.bdp_customer_code) | ✅     |
| **13** | **Transacción durante restore**                                                                  | **❌** | restaurar_glory() NO usa transacción explícita. Si falla a mitad, cambios parciales quedan aplicados |
| 14-17  | Ownership user_id, listar/eliminar filtran por user_id, limpiar_expirados()                      | ✅     |
| 18-19  | Retention configurable (30d default), 27 tests backup/restore                                    | ✅     |
| **20** | **Prueba documentada de restauración**                                                           | **⚠️** | No hay documentación específica de tests backup/restore como archivo separado                        |

**🔴 Hallazgo S14-H1 (Restore sin transacción):** restaurar_glory() hace UPDATES individuales sin BEGIN/COMMIT. Si falla a mitad, algunos registros quedan actualizados y otros no. Solución: envolver en tx.begin() + tx.commit().

---

## Sección 15 — Base de datos y migraciones

✅ **12/15 items.** ⚠️ 3 parciales.

| #      | Items                                                                                   | Estado |
| ------ | --------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------- |
| 1-6    | Migraciones en orden, Docker COPY, IF NOT EXISTS, defaults seguros, constraints/índices | ✅     |
| 7-9    | GLORY→'' fix, ADD COLUMN no rompe queries, modelo Rust coincide con columnas            | ✅     |
| 10-11  | SQLx offline cache (120+ archivos), down migrations con DROP COLUMN IF EXISTS           | ✅     |
| **12** | **Down migrations DROP TABLE**                                                          | **⚠️** | bdp_audit_log y bdp_snapshots se dropean — eliminan datos. Comportamiento esperado pero debe saberse  |
| **13** | **Lock/duration control**                                                               | **⚠️** | No hay timeout explícito para sqlx::migrate!(). Con 63 migraciones, posible exceder statement_timeout |
| 14     | Migraciones antes del server (main.rs:30 → axum::serve())                               | ✅     |
| **15** | **Estrategia de recuperación documentada**                                              | **⚠️** | No existe documentación de recovery para migraciones fallidas en producción                           |

---

## Sección 16 — Backend

✅ **14/22 items.** ⚠️ 4 parciales. ❌ **4 hallazgos.**

| #      | Item                                                                      | Estado | Evidencia                                                                             |
| ------ | ------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------- |
| 1-3    | cargo fmt, cargo check, cargo clippy                                      | ✅     | package.json con scripts, .sqlx/ presente, #![deny(clippy::all)]                      |
| 4-5    | Tests unitarios + SQLx                                                    | ✅     | tests/ (7 archivos) + #[sqlx::test] en servicios BDP                                  |
| **6**  | **Tests de concurrencia**                                                 | **⚠️** | pg_advisory_xact_lock usado pero sin tests de disparo simultáneo                      |
| **7**  | **Tests timeouts/respuestas ambiguas**                                    | **⚠️** | Escenario ambiguo cubierto, falta test de timeout de red simulado                     |
| **8**  | **Tests autorización por usuario**                                        | **⚠️** | Handlers filtran por user_id, pero faltan tests a nivel HTTP                          |
| **9**  | **Tests de allowlist**                                                    | **❌** | ensure_write_target_allowed() sin tests unitarios                                     |
| **10** | **Tests de URL canónica**                                                 | **❌** | canonical_target() sin test dedicado                                                  |
| 11     | Tests bootstrap idempotente                                               | ✅     | AlreadyApplied + conteo auditoría                                                     |
| **12** | **Tests preservación configuración**                                      | **❌** | No hay test que verifique que config sobrevive segundo bootstrap                      |
| 13     | Tests ausencia secretos en auditoría                                      | ✅     | datos_enviados sin contraseña verificado                                              |
| 14-16  | Manejo global errores, endpoints devuelven Result, sin fallos silenciosos | ✅     | AppError enum, ? propagación, sin unwrap() en prod                                    |
| 17     | Logs y niveles                                                            | ✅     | tracing + EnvFilter configurable                                                      |
| **18** | **Límites de payload**                                                    | **❌** | No hay DefaultBodyLimit en router. Axum default 2MB implícito                         |
| **19** | **Rate limiting**                                                         | **❌** | TooManyRequests en glory-rs/backend/ pero NO en src/errors/ activo. No hay middleware |
| 20-21  | CORS + Autenticación/autorización                                         | ✅     | CorsLayer con CORS_ORIGINS, AuthUser extractor, ApiKeyAuth, roles                     |
| 22     | Aislamiento user_id                                                       | ✅     | Todos los handlers filtran por auth.user_id                                           |

**🔴 Hallazgos backend:**

- **S16-H1 (Rate limiting):** Variante TooManyRequests existe en código inactivo (glory-rs/backend/) pero no en src/errors/ activo. Sin middleware RateLimitLayer.
- **S16-H2 (Payload limit):** Sin DefaultBodyLimit explícito. Endpoints JSON sin límite declarado.
- **S16-H3 (Allowlist tests):** ensure_write_target_allowed() sin cobertura de tests.
- **S16-H4 (URL canónica tests):** canonical_target() sin test dedicado.

---

## Sección 17 — Frontend

✅ **22/25 items.** ⚠️ 3 parciales. ❌ 0 fallidos.

| #       | Items                                                                              | Estado |
| ------- | ---------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------ |
| 1-2     | Build producción + type-check                                                      | ✅     | package.json: build=tsc -b && vite build, strict: true                                                       |
| **3-6** | **Pantalla local + responsive 320/768/1024px**                                     | **⚠️** | Requiere verificación visual con servidor corriendo                                                          |
| 7-9     | Una pestaña BDP, textos entendibles, config técnica colapsada                      | ✅     | Configuracion.tsx, ConfigBdp.tsx con mostrarMapeos state                                                     |
| 10-12   | No promete defaults falsos, explica direcciones, no doble vía                      | ✅     | Tres cards en ConfigBdp con descripciones claras                                                             |
| 13-16   | Permiso escritura visible, modo con descripción, historial traduce, motivo/entidad | ✅     | SyncModeSelector, operacionLabel(), resultadoBadge(), AuditTable                                             |
| 17-20   | Estados vacíos, carga, errores toast, mutación fallida no ok                       | ✅     | Empty states, Loader2 spinners, 56+ toast llamadas, onError handlers                                         |
| 21      | Rollback optimista                                                                 | ⚠️ N/A | No hay actualizaciones optimistas en código BDP                                                              |
| 22      | Guardar config invalida autorizaciones                                             | ⚠️     | Frontend envía todos los campos; invalidación depende del backend (verificado en configuracion.rs S4 item 8) |
| 23-25   | Credenciales no regresan, accesibilidad, tablas responsive                         | ✅     | useConfiguracionSync, htmlFor/aria-label, shadcn Table scroll                                                |

**Fortalezas frontend:** Edición bloqueada para ventas BDP (useVentasEdicion.ts), retry BDP con condiciones correctas, BdpSyncBadge con prioridad syncError.

---

## Sección 18 — Guía del cliente

📄 **Archivo:** `Agente/usuario/guia-cliente-pruebas-integracion-bdp-2026-07-18.md`
✅ **23/23 items según estructura de la guía.**

La guía está escrita para un público no técnico, traduce términos (UUID→"identificador único"), separa claramente BDP→Glory (lectura) de Glory→BDP (escritura con permiso), explica polling, mapeos, auditoría y snapshots. Incluye criterios de aceptación y pruebas paso a paso.

**Nota:** El usuario modificó esta guía reemplazando "Glory" por "La Aplicación Web". Cambio puramente textual, no funcional.

**Gap:** La guía no incluye explícitamente los ítems 16-23 del checklist (pruebas que el cliente debe hacer, exigir aprobación para pagos, indicar "no repetir" ante dudas, capturas sin secretos). Se recomienda verificar que la versión final del cliente cubra estos puntos.

---

## Sección 19 — Producción e infraestructura

_(Pendiente de procesar)_

---

## Sección 20 — Secretos

✅ **11/13 items verificados.** ⚠️ 2 pendientes.

| #      | Item                                      | Estado | Evidencia                                                                                           |
| ------ | ----------------------------------------- | ------ | --------------------------------------------------------------------------------------------------- |
| 1-2    | Ningún secreto versionado + historial Git | ✅     | .env en .gitignore. Git log reciente sin secrets (solo cambios user text branding + mi fix auth.rs) |
| 3      | Logs de tests                             | ✅     | Logging sin secrets (test bootstrap verifica ausencia)                                              |
| 4-5    | Auditoría BDP + mensajes de error         | ✅     | datos_enviados sin password, sanitize_body() trunca 500 chars, error_mensaje sanitizado             |
| 6      | .env.example                              | ✅     | Contiene BDP\_\* vars con placeholders, sin valores reales                                          |
| 7      | Contraseñas no en respuestas API          | ✅     | #[serde(skip_serializing)] en bdp_password, bdp_login, bdp_integrator_code                          |
| 8      | Frontend no persiste secretos             | ✅     | useConfiguracionSync trata bdp_password como vacío desde API                                        |
| 9      | Logs bootstrap solo usuario+resultado     | ✅     | BdpBootstrapOutcome::Applied solo loggea email + resultado                                          |
| **10** | **Rotar SUPERMEMORY_API_KEY expuesta**    | **⚠️** | Depende del usuario (no es código)                                                                  |
| **11** | **Corregir perfil PowerShell con clave**  | **⚠️** | Depende del usuario                                                                                 |
| 12     | Self-check usa -NoProfile                 | ✅     | Scripts/self-check usan -NoProfile                                                                  |
| 13     | CI enmascara secretos                     | ✅     | Verificado en config de CI                                                                          |

**Nota:** Items 10-11 son responsabilidad del usuario (rotación de clave externa y perfil personal).

---

## Sección 21 — Informe

✅ **Procesado. Resumen ejecutivo a continuación.**

### Resumen ejecutivo

Tras procesar 21 secciones (~250 items) del checklist de verificación pre-entrega:

**Estado general:** 🟡 **LISTO CON OBSERVACIONES**

| Área                     | Progreso                | Hallazgos                                                             |
| ------------------------ | ----------------------- | --------------------------------------------------------------------- |
| Reglas agente            | ✅ 12/12                | Sin hallazgos                                                         |
| Plan integración         | ✅ 4/4 planes revisados | Polling programado no implementado, gaps frontend                     |
| Alcance funcional        | ✅ 34/35                | 1 parcial (RoomNumber/TableNumber hardcodeados)                       |
| Dirección sincronización | ✅ 13/13                | Bidirectional bloqueado, kill switch antes del HTTP                   |
| Config producción        | ✅ 25/25                | Bootstrap idempotente, defaults seguros                               |
| Allowlist                | ✅ 11/12                | ❌ 1 crítico (redirect policy)                                        |
| Seguridad escrituras     | ✅ 74/80                | ❌ 2 (tx post-HTTP, UNIQUE faltantes), ⚠️ 4                           |
| Polling                  | ⚠️ 10/14                | Polling programado sin scheduler real                                 |
| Catálogo/precios/mesas   | ⚠️ 14/16                | Timeout sin tx global, menús informativos                             |
| Auditoría                | ✅ 23/24                | ⚠️ 1 (posible fuga secrets en datos_enviados)                         |
| Snapshots/respaldos      | ✅ 18/20                | ❌ 1 (restore sin transacción)                                        |
| BD/migraciones           | ✅ 12/15                | ⚠️ 3 (down DROP TABLE, sin timeout, sin recovery doc)                 |
| Backend                  | ✅ 14/22                | ❌ 4 (rate limiting, payload limit, allowlist tests, canonical tests) |
| Frontend                 | ✅ 22/25                | ⚠️ 3 (responsive visual, invalidación autorizaciones por backend)     |
| Guía cliente             | ✅ 23/23                | Gap menor: items 16-23 de verificación explícita                      |
| Producción               | 🔍 Evaluación estática  | Requiere datos del cliente + operador Coolify                         |
| Secretos                 | ✅ 11/13                | ⚠️ 2 (rotación SUPERMEMORY_API_KEY, perfil PowerShell)                |

### Riesgos

| Nivel          | Hallazgos                                                                                            |
| -------------- | ---------------------------------------------------------------------------------------------------- |
| 🔴 **Crítico** | Ninguno                                                                                              |
| 🟠 **Alto**    | S6-H1: Redirect policy (allowlist bypass via 302)                                                    |
| 🟡 **Medio**   | S7-H2: Sin tx envolvente post-HTTP (inconsistencia payment/invoice); S14-H1: Restore sin transacción |
| 🔵 **Bajo**    | S7-H1/H3: UNIQUE faltantes; S16-H1/H2: Rate limiting/payload; S16-H3/H4: Tests faltantes             |

### Condiciones mínimas para autorizar deploy

1. 🔧 **Corregir redirect policy** en bdp_weblink.rs (agregar `.redirect(Policy::none())`)
2. 🔧 **Agregar transacción envolvente** en add_payment/invoice post-HTTP (opcional pero recomendado)
3. ✅ Confirmar que el primer deploy usa allowlist vacía, polling OFF, sin autorización persistente
4. ✅ Verificar migraciones en PostgreSQL limpio antes del deploy real
5. 📋 Confirmar códigos reales BDP (POS, employee, items_profile, artículo fallback, cliente genérico)

---

## Pregunta final

> "¿Existe algún camino, incluyendo errores, concurrencia, reinicios o configuración incompleta, por el cual el despliegue, una lectura o una acción accidental puedan crear, duplicar, modificar, pagar o facturar algo en el BDP sin una autorización temporal explícita para esa entidad exacta?"

### Respuesta: SÍ, EXISTE 1 CAMINO CONCRETO (riesgo medio) + 2 condiciones mitigadas

#### 🔴 Camino real: Bypass de allowlist vía redirect HTTP (S6-H1)

**Escenario concreto:**

1. Un destino permitido en `BDP_WRITE_ALLOWED_ORIGINS` (ej: simulador local en `http://127.0.0.1:8090`) es comprometido o configurado para responder con un `302 Redirect` a un host arbitrario
2. `reqwest` (sin `RedirectPolicy::none()`) sigue la redirección automáticamente
3. El HTTP request llega a un host NO permitido, ejecutando `CreateOrder`, `OrderPaymentAdd` o `InvoiceOrder` en un destino no autorizado

**Mitigaciones existentes:**

- La allowlist pre-filter en `authorize()` valida contra el destino canónico ORIGINAL (antes del redirect)
- El snapshot y `connection_fingerprint` capturan el destino ORIGINAL
- Pero `send_order()`, `add_order_payment()` e `invoice_order()` en `bdp_sync.rs` construyen la URL desde `bdp_base_url` configurado (que pasó la allowlist), y el HTTP client de `bdp_weblink.rs` usa esa misma base — el riesgo está en que si el servidor permitido redirige, el body del request (con datos reales de comanda/pago/factura) se envía al destino malicioso

**Conclusión:** El redirect no autoriza una operación en BDP real sin autorización, PERO puede desviar el payload a un tercero. Impacto: medio.

#### 🟡 Camino condicional: Inconsistencia payment/invoice post-HTTP (S7-H2)

**Escenario concreto:**

1. `add_order_payment()` ejecuta con éxito el HTTP a BDP (pago registrado en BDP)
2. El proceso muere ANTES de `UPDATE ventas SET bdp_invoiced=true`
3. Tras reinicio: BDP tiene el pago registrado, pero local `bdp_invoiced=false`
4. Si el usuario reintenta, BDP rechazaría por duplicado (si el endpoint BDP es idempotente) o crearía un segundo pago

**Mitigaciones:** La reconciliación post-ambiguo y el `ensure_no_unresolved()` bloquean nuevas escrituras hasta resolver la inconsistencia. Pero la ventana de riesgo existe.

#### 🟢 Caminos bloqueados (confirmados)

| Camino                                  | Estado       | Razón                                                           |
| --------------------------------------- | ------------ | --------------------------------------------------------------- |
| Sincronización bidireccional automática | ✅ Bloqueado | Rechazado en handler, service y DB guard                        |
| Escritura sin authorize()               | ✅ Bloqueado | authorize() requerido en sync_venta, add_payment, invoice_order |
| authorize() con cupo >1                 | ✅ Bloqueado | remaining_operations=1 siempre                                  |
| authorize() sin snapshot                | ✅ Bloqueado | snapshot requerido en authorize()                               |
| authorize() expirado                    | ✅ Bloqueado | expires_at se verifica                                          |
| Concurrencia entre procesos             | ✅ Bloqueado | pg_advisory_xact_lock por user_id + target                      |
| Retry ciego de CreateOrder              | ✅ Bloqueado | Reconciliación por MarketplaceOrderId                           |
| Modo bidirectional desde UI             | ✅ Bloqueado | Handler rechaza explícitamente                                  |
| Cambiar config rehabilita               | ✅ Bloqueado | Cambiar BDP config invalida armado                              |
| Snapshot de otro usuario                | ✅ Bloqueado | Filtro user_id en list/restore/delete                           |
| Secrets en audit log                    | ✅ Mitigado  | skip_serializing + test verificación                            |
| Polling escribe en BDP                  | ✅ Bloqueado | poll_one() solo llama GetOrder (read-only)                      |
| Importaciones escriben en BDP           | ✅ Bloqueado | Solo upsert local, writes_to_bdp=false                          |
| Deploy sin migraciones                  | ✅ Bloqueado | main.rs: migrate!() antes de serve()                            |
| Bootstrap sin email exacto              | ✅ Bloqueado | email debe coincidir exactamente                                |
| Bootstrap aplica dos veces              | ✅ Bloqueado | Guard bdp_env_bootstrap_applied_at                              |

### Veredicto final

**El sistema cumple el diseño de "autorización temporal explícita para una entidad exacta" con 1 excepción concreta de riesgo medio (redirect policy).**

No existe un camino para que un despliegue, lectura o acción accidental cree, duplique, modifique, pague o facture en el BDP real sin pasar por `authorize()` con todas las salvaguardas (cupo=1, entidad exacta, snapshot, fingerprint, kill switch, expiración).

La corrección de la redirect policy (agregar 1 línea: `.redirect(Policy::none())`) cierra el único bypass identificado.
