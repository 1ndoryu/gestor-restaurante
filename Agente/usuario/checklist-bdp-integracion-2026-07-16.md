# Checklist: Integración completa BDP — Pruebas manuales

> **Fecha:** 2026-07-16 (v2)
> **Alcance:** Sync BDP + Backup/Restauración + Configuración + Multi-item + Clientes + Pagos + Facturación
> **URL local:** http://localhost:5174/configuracion
> **URL producción:** https://restaurante.wandori.us
>
> **Orden de pruebas:** Sin BDP → Solo lectura BDP → Escritura BDP

---

## 1️⃣ SIN BDP — UI + Backend local (no requiere conexión al TPV)

> Estas pruebas se pueden hacer sin tener el servidor BDP activo.
> Verifican que la interfaz, los snapshots, la auditoría y el flujo de ventas funcionan localmente.

### Pestañas de Configuración (BKP-008)
- [ ] **5 pestañas visibles:** General, Integraciones, Chatbot, BDP Conexión, BDP Backup
- [ ] **Pestaña "BDP Conexión":** Muestra formulario de conexión (URL, login, password, código integrador, POS, empleado, perfil artículos)
- [ ] **Pestaña "BDP Backup":** Muestra panel de snapshots sin crashes
- [ ] **Sin errores en consola:** DevTools → Console no muestra errores rojos al navegar entre pestañas

### Configuración BDP Conexión (Fase 1)
- [ ] **Campos visibles:** URL pública BDP, Login, Password, Código integrador, Terminal POS, Empleado, Perfil artículos
- [ ] **Toggle sync:** Switch "Sincronización BDP activa" funciona (on/off)
- [ ] **Mapeos colapsados:** "Configuración avanzada (mapeos)" con chevron derecho y nota informativa
- [ ] **Expandir mapeos:** Click → despliega JSON de tender_map, order_type_map, customer_code, poll_interval
- [ ] **Tabla de mapeo artículos:** `BdpArticleMapTable` visible al expandir mapeos
- [ ] **Importar catálogo BDP:** Botón visible (funcionalidad requiere BDP — sección 2)
- [ ] **Guardar conexión:** Click "Guardar conexión BDP" → "Guardando..." → éxito, persiste al recargar

### Configuración Backup & Seguridad BDP (BKP-005)
- [ ] **Sync mode selector:** Muestra modo actual (read_only / unidirectional / bidirectional)
- [ ] **Cambiar modo:** Seleccionar otro modo → se actualiza en el panel
- [ ] **Campos de retención:** `bdp_backup_retention_days` visible y editable
- [ ] **Toggle auto-backup:** `bdp_auto_backup_before_write` visible

### Snapshots — Crear (BKP-001, BKP-005)
- [ ] **Tab "Snapshots" visible:** Muestra tabla (vacía o con datos existentes)
- [ ] **Snapshot completo:** Click "Crear completo" → aparece con tipo `completo`, estado `disponible`, fecha actual
- [ ] **Snapshot parcial:** Seleccionar tipos (menú, productos, etc.) → "Crear parcial" → tipo `parcial`
- [ ] **Snapshot Glory:** Seleccionar tipos → "Crear Glory" → tipo `glory`
- [ ] **Notas opcionales:** Crear con y sin nota, verificar que se guarda
- [ ] **Loading state:** Botón muestra spinner/deshabilitado mientras procesa
- [ ] **Metadatos:** Cada snapshot muestra cantidad de registros, trigger, dirección

### Snapshots — Eliminar (BKP-001)
- [ ] **Eliminar:** Click botón eliminar → confirmar → snapshot desaparece
- [ ] **Confirmación:** Pide confirmación antes de eliminar

### Snapshots — Restaurar (BKP-004, BKP-005)
- [ ] **Restaurar Glory:** Click restaurar → confirmar → resultado con detalle de tablas restauradas
- [ ] **Restaurar no destructiva:** Datos del restaurante (reservas, ventas) NO se pierden
- [ ] **Error: snapshot inexistente:** Intentar restaurar eliminado → error claro

### Auditoría (BKP-001)
- [ ] **Tab "Auditoría" visible:** Muestra tabla (vacía o con datos)
- [ ] **Registros aparecen:** Después de crear/eliminar/restaurar snapshots, hay entradas
- [ ] **Detalle correcto:** Cada entrada muestra operación, resultado (éxito/error), timestamp, usuario
- [ ] **Operaciones registradas:** snapshot_crear, snapshot_eliminar, snapshot_restaurar

### Ventas — Multi-item (Fase 2, Fase 6)
- [ ] **Formulario de venta carga:** Sin errores, muestra campos habituales
- [ ] **LineasVentaEditor visible:** Editor de líneas debajo de los campos de venta
- [ ] **Añadir línea:** Click "+" → nueva línea con selector de artículo, cantidad, precio, IVA, descuento
- [ ] **Eliminar línea:** Click "−" → línea desaparece, total se recalcula
- [ ] **Múltiples líneas:** Añadir 3 líneas → total = suma correcta
- [ ] **Autocomplete artículos:** Buscar artículo → muestra sugerencias del catálogo Glory
- [ ] **Indicador mapeo BDP:** Cada línea muestra ✅/⚠️ si tiene/no tiene mapeo BDP
- [ ] **Retrocompatibilidad:** Si no se añaden líneas, el formulario funciona como antes (campo total manual)
- [ ] **Crear venta con líneas:** Submit → venta creada con líneas asociadas en BD

### Ventas — Lista de ventas con BDP (Fase 5)
- [ ] **Columna BDP visible:** `BdpSyncBadge` en la tabla de ventas (✅/❌/⏳)
- [ ] **Filtro BDP:** `estadoBdp` (synced/error/pending) en filtros de columna
- [ ] **Retry BDP:** Botón en acciones de fila → llama `POST /api/ventas/:id/bdp-sync`
- [ ] **Tooltip BDP:** Hover en badge muestra `bdp_order_id` y `bdp_sync_error`

### Ventas — Campos BDP en modelo (Fase 4, Fase 5)
- [ ] **Campos en BD:** `bdp_synced`, `bdp_order_id`, `bdp_sync_error`, `bdp_order_status` existen en tabla `ventas`
- [ ] **Campo `bdp_order_status`:** Mapea estados BDP (pendiente, enviada, cobrada, facturada, error)
- [ ] **Orval codegen:** `VentaConCliente` incluye campos BDP tras regenerar

### Clientes — Campos BDP (Fase 7)
- [ ] **Campo `bdp_customer_code`:** Existe en tabla `clientes` (VARCHAR)
- [ ] **Campo `bdp_synced`:** Existe en tabla `clientes` (BOOLEAN)
- [ ] **Campo `bdp_sync_error`:** Existe en tabla `clientes` (TEXT)

### Manejo de errores sin BDP
- [ ] **Botón "Probar conexión":** Error claro (no crash) porque no hay BDP
- [ ] **Botón "Probar sincronización segura":** Error o estado pendiente (no crash)
- [ ] **Crear snapshot sin BDP:** Funciona (es backup local, no depende de BDP)
- [ ] **Retry BDP sin BDP:** Botón retry → error manejado (no crash)

---

## 2️⃣ SOLO LECTURA BDP — Diagnóstico, import y validación

> Estas pruebas requieren que el servidor BDP esté activo pero NO escriben datos en él.
> Verifican la conexión, autenticación, importación de datos y estado del TPV.
>
> **Pre-requisito:** Tailscale conectado + BDP del restaurante encendido.

### Diagnóstico BDP (Fase 1)
- [ ] **Probar conexión:** Click → muestra estado (health_ok, login_ok, versión BDP)
- [ ] **Info de versión:** Versión, sub_version y aplicación del TPV
- [ ] **Credenciales incorrectas:** Cambiar password → probar conexión → error de autenticación

### Sync dry-run / Preflight (Fase 1, Fase 3)
- [ ] **Probar sincronización segura:** Click → ejecuta 9+ checks de lectura sin escribir
- [ ] **Checks individuales:** Cada check muestra nombre, endpoint, ok/error, cantidad de registros
- [ ] **Check tender:** Valida que el tender mapeado existe en el POS
- [ ] **Check order type:** Valida que el Type mapeado es válido
- [ ] **Check artículos:** Valida que todas las líneas tienen artículo BDP mapeado
- [ ] **Check cliente:** Valida que el cliente tiene código BDP (si aplica)
- [ ] **Dry-run CreateOrder:** OnlyCheck=true sin crear orden real
- [ ] **Estado "listo para sincronizar":** Muestra si todo está configurado

### Importar catálogo BDP → Glory (Fase 1, Fase 5)
- [ ] **Importar artículos:** Botón "Importar catálogo BDP" → ejecuta `ExportArticles` → precarga `bdp_article_map`
- [ ] **Artículos importados:** La tabla de mapeo muestra artículos Glory↔BDP
- [ ] **Import incremental:** Solo importa nuevos/actualizados (no duplica existentes)

### Importar clientes BDP → Glory (Fase 7.1)
- [ ] **Endpoint `POST /api/bdp/customers/import`:** Ejecuta `ExportCustomers` → crea/actualiza clientes en Glory
- [ ] **Matching por teléfono:** Clientes existentes se actualizan (no duplican)
- [ ] **Campo `bdp_customer_code`:** Se asigna automáticamente al importar
- [ ] **Campo `bdp_synced`:** Se marca como `true` tras import exitoso
- [ ] **Batch processing:** Import masivo (~43k registros) no crashea

### Consultar estado de comanda (Fase 4)
- [ ] **`GET /api/ventas/:id/bdp-status`:** Devuelve estado actual de la comanda en BDP
- [ ] **Mapeo de estados:** BDP status → Glory status (pendiente, enviada, cobrada, facturada)
- [ ] **Venta no sincronizada:** Devuelve error claro si la venta no fue enviada a BDP

### Restaurar snapshot (lectura local, escritura Glory)
- [ ] **Restaurar Glory desde snapshot:** Click restaurar → confirmar → tablas restauradas
- [ ] **Restaurar no destructiva:** Datos del restaurante NO se pierden
- [ ] **Error: snapshot inexistente:** Intentar restaurar eliminado → error claro

---

## 3️⃣ ESCRITURA BDP — Sync real, pagos y facturación

> ⚠️ **Estas pruebas modifican datos en el TPV/BDP.** Dejarlas para el final.
> Requieren BDP activo + sync_mode != read_only + autorización explícita del usuario.

### Pre-write audit (BKP-002, BKP-003)
- [ ] **Sync unidirectional:** Al escribir hacia BDP, se registra entrada de auditoría ANTES de la escritura
- [ ] **Sync bidirectional:** Igual que unidirectional pero en ambas direcciones
- [ ] **Sync read_only:** NO se permite escritura → endpoint devuelve error 403
- [ ] **Pre-write snapshot selectivo:** Solo datos que se envían (no backup completo)
- [ ] **Costo máximo 1 llamada:** Pre-write snapshot cuesta como máximo 1 llamada adicional a BDP

### Sync de clientes Glory → BDP (Fase 7.2, Fase 7.5)
- [ ] **Push cliente individual:** `POST /api/clientes/:id/bdp-sync` → ejecuta `CreateCustomer`
- [ ] **Auto-sync al crear venta:** Si cliente no tiene `bdp_customer_code`, push automático antes de `CreateOrder`
- [ ] **Campo `bdp_customer_code`:** Se asigna tras push exitoso
- [ ] **Campo `bdp_synced`:** Se marca como `true` tras push exitoso
- [ ] **Error handling:** Si `CreateCustomer` falla, error se guarda en `bdp_sync_error`

### CreateOrder — Escritura real (Fase 2, Fase 3)
- [ ] **Venta simple (1 línea):** Crear venta → `sync_venta()` → comanda aparece en BDP
- [ ] **Venta multi-item (3 líneas):** Crear venta con 3 líneas → BDP recibe 3 `OrderItems` separados
- [ ] **Artículo genérico fallback:** Si línea no tiene mapeo BDP → usa `bdp_default_article_code`
- [ ] **Artículo mapeado:** Si línea tiene mapeo → usa código BDP correcto
- [ ] **Cliente en comanda:** Si venta tiene `cliente_id` → `Customer` con Code/Name/Phone en BDP
- [ ] **Forma de pago:** `metodo_pago` → `TenderId` mapeado correctamente
- [ ] **Canal:** `canal` → `Type` mapeado (barra=0, comedor=1, domicilio=2)
- [ ] **MarketplaceOrderId:** Max 15 chars, prefijo `G`, único por venta
- [ ] **Serie `00031TI`:** IVA incluido, asignada a POS 31
- [ ] **Retry automático:** Si falla, reintenta 3 veces con backoff exponencial
- [ ] **Error en BD:** `bdp_sync_error` se guarda en la venta si falla tras reintentos

### AddOrderPayment — Pagos parciales (Fase 8.1)
- [ ] **Registrar pago:** `add_order_payment()` → envía pago parcial a BDP
- [ ] **Endpoint manual:** `POST /api/ventas/:id/bdp-payment` funciona
- [ ] **Mapeo tender:** Forma de pago Glory → TenderId BDP
- [ ] **Error handling:** Si falla, error se guarda en venta

### InvoiceOrder — Facturación (Fase 8.2, Fase 8.3)
- [ ] **Facturar comanda:** `invoice_order()` → factura en BDP
- [ ] **Endpoint manual:** `POST /api/ventas/:id/bdp-invoice` funciona
- [ ] **Campo `bdp_invoiced`:** Se marca como `true` tras facturación exitosa
- [ ] **Reflejar facturación automática:** Polling detecta status=3 → marca `bdp_invoiced`

### Polling de estado (Fase 4)
- [ ] **Polling automático:** Ventas con `bdp_synced=true` y status no final → consulta `GetOrder` periódicamente
- [ ] **Intervalo configurable:** `bdp_poll_interval_secs` (default 60s) se respeta
- [ ] **Actualización de estado:** Si BDP devuelve status cambiado → se actualiza en Glory
- [ ] **Solo ventas recientes:** No consulta ventas antiguas/finales (optimización)

### Sync catálogo BDP → Glory (Fase 1)
- [ ] **`sync_catalog`:** Lee `ExportArticles` → upsert en `bdp_article_map`
- [ ] **`sync_prices`:** Lee `GetPricesArticles` → actualiza `precio_tarifa1`
- [ ] **`sync_tables`:** Lee `GetRoomsTables` → crea zonas/mesas en Glory

### Edge cases de escritura
- [ ] **Rate limiting:** Demasiadas peticiones → error de rate limit claro
- [ ] **Timeout:** Operación larga → timeout (no spinner infinito)
- [ ] **Token expirado mid-sync:** Error de autenticación, re-login automático, reintento
- [ ] **Rollback parcial:** Escritura parcial falla → audit log lo registra
- [ ] **BDP caído durante sync:** Error manejado, venta queda con `bdp_sync_error`, retry disponible
- [ ] **CancelOrder:** Si devuelve "Subscripción no activada" → error claro, no crash

---

## 📋 Comandos útiles

```bash
# Frontend compila sin errores
cd frontend && npx tsc --noEmit

# Backend compila
cargo check

# Tests unitarios BDP (32 tests, sin DB ni BDP)
cargo test --test bdp_sync -- --test-threads=1

# Tests DB BDP (21 tests, requiere PostgreSQL local)
cargo test --test bdp_article_map
cargo test --test bdp_venta_lineas
cargo test --test bdp_backup

# Tests read-only BDP (6 tests, requiere Tailscale + BDP encendido)
cargo test --test bdp_readonly -- --ignored

# Suite completa del proyecto (113 tests)
cargo test

# Verificar que el dev server arranca
npm run dev
```

---

## 🐛 Bugs conocidos ya corregidos

- ✅ `snapshots.map is not a function` — Fix: customInstance wrapper extrae `.data` (BKP-008)
- ✅ FK `usuarios(id)` → `users(id)` en migración (BKP-007)
- ✅ `NaiveDateTime` → `DateTime<Utc>` para TIMESTAMPTZ (BKP-007)
- ✅ 25 tests backend pasando (BKP-007)
- ✅ Config BDP separada en pestaña propia, mapeos colapsados (BKP-008)
- ✅ Error 300035 — Serie creada, cliente confirmó
- ✅ Orval codegen regenerado con campos BDP (Fase 5.0)

---

## 📊 Resumen de tests existentes

| Categoría | Tests | Qué validan |
|---|---|---|
| **Cat A — Unit** | 32 | `build_order` JSON, PascalCase, mappings tender/order_type/customer, retry, error handling |
| **Cat B — DB** | 21 | `bdp_article_map` y `venta_lineas`: CRUD, upsert, aislamiento, FK constraints |
| **Cat B — Backup** | 25 | `bdp_snapshots` y `bdp_audit_log`: crear, listar, eliminar, restaurar, expirar |
| **Cat C — Read-only** | 6 | Login real, ExportArticles, GetTenders, GetOrder contra BDP real |
| **Total** | **84** | 84 pasan, 0 fallan, 0 ignored |
