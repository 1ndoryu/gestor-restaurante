# Plan: Sistema de Copias de Seguridad BDP ↔ Glory

> **Fecha:** 2026-07-15
> **Prioridad:** 🔴 CRÍTICA — Sin este sistema, no podemos sincronizar datos con BDP sin riesgo de pérdida
> **Objetivo:** Garantizar que NINGUNA operación de Glory pueda causar pérdida de datos en BDP. Punto.

---

## ⚠️ REGLA CRÍTICA: Control de despliegues y pruebas

> **PROHIBIDO** realizar despliegues a producción o llamadas a la API de BDP sin autorización explícita del usuario.

| Acción | Permitido | Requiere autorización |
|---|---|---|
| Implementar código Rust (modelos, migraciones, servicios, handlers) | ✅ Sí | — |
| Compilar localmente (`cargo check`, `cargo build`) | ✅ Sí | — |
| Tests unitarios que NO llamen a BDP | ✅ Sí | — |
| Regenerar Orval codegen localmente | ✅ Sí | — |
| Implementar componentes React/frontend | ✅ Sí | — |
| Deploy a producción (restaurante) | ❌ | ✅ Autorización requerida |
| **Cualquier** llamada a API BDP (Login, ExportArticles, ExportCustomers, GetOrder, CreateOrder, etc.) | ❌ | ✅ Autorización requerida |
| Importar datos de BDP a Glory (ExportCustomers, ExportArticles) | ❌ | ✅ Autorización requerida |
| Push de datos de Glory a BDP (CreateCustomer, CreateOrder, etc.) | ❌ | ✅ Autorización requerida |
| Crear/Modificar/Eliminar datos reales en BDP | ❌ | ✅ Autorización requerida |

> **REGLA ABSOLUTA:** NO se toca el sistema BDP del restaurante sin autorización explícita.
> El código se implementa y compila localmente; las llamadas reales requieren tu OK.

---

## 1. Hallazgos de la investigación

### 1.1 BDP NO tiene sistema de backup nativo

La API WebLink REST de BDP **no tiene** endpoints de backup, snapshot, ni exportación completa en una sola llamada. No hay forma de hacer un "punto de restauración" mediante la API.

### 1.2 BDP SÍ tiene muchos endpoints de lectura (23+)

Estos son **read-only** y no modifican nada:

| Categoría         | Endpoint                                    | Qué devuelve                                                                   |
| ----------------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| **Artículos**     | `ExportArticles`                            | Catálogo completo (código, descripción, 5 tarifas, IVA, departamento, familia) |
| **Artículos**     | `GetPOSArticlesList`                        | Artículos por perfil (paginado, con toda la ficha)                             |
| **Artículos**     | `GetArticle`                                | Artículo individual por código                                                 |
| **Artículos**     | `GetPricesArticles`                         | Precios 1-5 y descuentos de un artículo                                        |
| **Clientes**      | `ExportCustomers`                           | Todos los clientes (código, nombre, NIF, teléfono, email, dirección)           |
| **Departamentos** | `ExportDepartment`                          | Departamentos con paginación                                                   |
| **Departamentos** | `DepartmentsExportFromProfile`              | Departamentos jerárquicos por perfil                                           |
| **Comandas**      | `GetOrder`                                  | Estado y contenido de una comanda individual                                   |
| **Documentos**    | `ExportDocumentsByExportProfile`            | Facturas/albaranes de venta por perfil y fecha                                 |
| **Documentos**    | `ExportManagmentDocumentsByExportProfile`   | Albaranes/facturas de gestión (cabeceras + líneas + vencimientos)              |
| **Documentos**    | `ExportPurchaseNotes`                       | Albaranes de compra                                                            |
| **Documentos**    | `ExportStockAndSalesSummaryByExportProfile` | Movimientos de stock por ventas                                                |
| **Stock**         | `GetStock`                                  | Stock de un artículo en un almacén                                             |
| **Stock**         | `GetListStock`                              | Stock de múltiples artículos                                                   |
| **Stock**         | `GetItemCostPrices`                         | Precios de coste (UPC/PMC) de un artículo                                      |
| **Stock**         | `GetItemsCostPrices`                        | Precios de coste de múltiples artículos                                        |
| **Salones**       | `GetRoomTables`                             | Mesas de un salón individual                                                   |
| **Salones**       | `GetRoomsTables`                            | Todos los salones con sus mesas                                                |
| **Menús**         | `GetMenuDefinition`                         | Definición completa de un menú (grupos + items + suplementos)                  |
| **Fast-food**     | `GetFastfoodDefinition`                     | Definición de un fast-food (ingredientes + precios)                            |
| **Packs**         | `GetPackDefinition`                         | Definición de un pack (grupos + items + precios)                               |
| **Config**        | `GetEmployees`                              | Empleados dados de alta                                                        |
| **Config**        | `GetPOSes`                                  | Terminales disponibles                                                         |
| **Config**        | `GetTenderList`                             | Formas de pago                                                                 |
| **Config**        | `GetPOSTenderList`                          | Formas de pago por terminal                                                    |
| **Config**        | `GetPOSSeriesList`                          | Series de facturación                                                          |
| **Config**        | `GetPOSEmployees`                           | Empleados por terminal                                                         |
| **Suplementos**   | `GetSupplementsProfiles`                    | Perfiles de suplementos (con alérgenos)                                        |
| **Talla&Color**   | `GetInfoSAC` / `GetItemSAC`                 | Dimensiones T&C por artículo                                                   |

### 1.3 Endpoints que Glory usa para ESCRIBIR en BDP (5)

Estos son los **peligrosos** — modifican datos en BDP:

| Endpoint          | Cuándo se llama                                      | Riesgo                                 |
| ----------------- | ---------------------------------------------------- | -------------------------------------- |
| `CreateOrder`     | `sync_venta()` — cada venta Glory → comanda BDP      | **ALTO** — crea comandas en el TPV     |
| `CreateCustomer`  | `ensure_cliente_bdp_synced()` — auto-sync de cliente | **MEDIO** — crea clientes nuevos       |
| `AddOrderPayment` | `add_order_payment()` — registrar pago               | **MEDIO** — modifica estado de comanda |
| `InvoiceOrder`    | `invoice_order()` — facturar comanda                 | **ALTO** — facturación irreversible    |
| `CancelOrder`     | No disponible (suscripción)                          | N/A                                    |

### 1.4 Endpoints que Glory usa solo para LEER de BDP (3)

Estos son **seguros** — solo leen de BDP y escriben en Glory:

| Endpoint            | Servicio Glory   | Qué hace                                          |
| ------------------- | ---------------- | ------------------------------------------------- |
| `ExportArticles`    | `sync_catalog()` | Lee catálogo BDP → upsert en `bdp_article_map`    |
| `GetPricesArticles` | `sync_prices()`  | Lee precios BDP → actualiza `precio_tarifa1`      |
| `GetRoomsTables`    | `sync_tables()`  | Lee salones/mesas BDP → crea zonas/mesas en Glory |

---

## 2. Arquitectura del sistema de backup

### 2.1 Principios

1. **BDP es la fuente de verdad** — antes de tocar BDP, SIEMPRE tomamos snapshot
2. **Glory puede romper, BDP NO** — todo lo que Glory envía a BDP debe ser deshacible
3. **Backup automático pre-escritura** — antes de `CreateOrder`, `CreateCustomer`, etc.
4. **Exploración segura primero** — snapshot completo de BDP ANTES de habilitar sync
5. **Auditoría inmutable** — cada operación queda registrada con timestamp y datos

### 2.2 Componentes

```
┌─────────────────────────────────────────────────────┐
│                   FRONTEND (React)                   │
│                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ Panel Backup │  │ Config Sync  │  │ Historial  │ │
│  │ (snapshots)  │  │ (dirección)  │  │ (audit log)│ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬─────┘ │
└─────────┼──────────────────┼──────────────────┼──────┘
          │                  │                  │
┌─────────┼──────────────────┼──────────────────┼──────┐
│         ▼                  ▼                  ▼      │
│  ┌──────────────────────────────────────────────┐    │
│  │           BACKUP SERVICE (Rust)               │    │
│  │                                               │    │
│  │  snapshot_bdp_completo()  ← exploración inicial│   │
│  │  snapshot_bdp_parcial(tipos) ← backup selectivo│   │
│  │  snapshot_glory(tipos)    ← backup BD Glory    │   │
│  │  pre_write_snapshot(tipo) ← auto antes de write│   │
│  │  listar_snapshots()       ← historial          │   │
│  │  restaurar_bdp(snapshot)  ← restaurar a BDP    │   │
│  │  restaurar_glory(snapshot)← restaurar en Glory │   │
│  └──────────────────────────────────────────────┘    │
│                                                      │
│  ┌──────────────────────────────────────────────┐    │
│  │           AUDIT LOG (inmutable)               │    │
│  │                                               │    │
│  │  Cada sync_venta, create_customer, etc.       │    │
│  │  → registro con timestamp, tipo, datos,       │    │
│  │    snapshot_pre_id, resultado                 │    │
│  └──────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────┘
          │                    │
          ▼                    ▼
   ┌─────────────┐     ┌─────────────┐
   │  BDP (TPV)  │     │ Glory (BD)  │
   │  Read+Write │     │ Read+Write  │
   └─────────────┘     └─────────────┘
```

### 2.3 Modos de sincronización

| Modo                         | Descripción                                                                         | Seguridad | Default |
| ---------------------------- | ----------------------------------------------------------------------------------- | --------- | ------- |
| **Solo lectura**             | Glory lee de BDP pero NUNCA escribe. Snapshot inicial + sync catálogo/precios/mesas | 🟢 Máxima | ✅ SÍ   |
| **Unidireccional Glory→BDP** | Glory envía ventas/clientes a BDP. Backup pre-write automático                      | 🟡 Media  | No      |
| **Bidireccional**            | Lectura + escritura en ambas direcciones. Backup completo                           | 🔴 Baja   | No      |

**El modo default es "Solo lectura".** Para activar escritura, el usuario debe:

1. Haber hecho un snapshot completo de BDP
2. Confirmar explícitamente en el frontend
3. Cada escritura genera backup pre-write **selectivo** (solo el recurso afectado, NO backup completo)

### Nota sobre costo del pre-write snapshot

El backup pre-write **NO** hace un ExportArticles/ExportCustomers completo antes de cada write. Eso sería prohibitivamente costoso.

En su lugar, el pre-write snapshot es **selectivo y minimal**:

| Operación | Pre-write snapshot | Costo API |
|---|---|---|
| `CreateOrder` | Datos que se envían (ya los tenemos) + cliente si se sync | 0-1 llamadas |
| `CreateCustomer` | Datos del cliente (ya los tenemos) | 0 llamadas |
| `AddOrderPayment` | Estado actual comanda (`GetOrder`) | 1 llamada |
| `InvoiceOrder` | Estado actual comanda (`GetOrder`) | 1 llamada |
| `sync_catalog` | Catálogo actual en Glory (query local) | 0 llamadas BDP |
| `sync_prices` | Precios actuales en Glory (query local) | 0 llamadas BDP |
| `sync_tables` | Mesas actuales en Glory (query local) | 0 llamadas BDP |

**Regla:** pre-write snapshot cuesta como máximo 1 llamada adicional a BDP.

---

## 3. Modelo de datos

### 3.1 Tabla `bdp_snapshots`

```sql
CREATE TABLE bdp_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES usuarios(id),
    tipo VARCHAR(50) NOT NULL,          -- 'completo', 'articulos', 'clientes', 'salones', 'mesas', 'glory_ventas', 'glory_clientes', 'glory_mapeos'
    direccion VARCHAR(20) NOT NULL,     -- 'bdp', 'glory'
    trigger VARCHAR(50) NOT NULL,       -- 'manual', 'pre_write', 'exploracion_inicial', 'scheduled'
    datos JSONB NOT NULL,               -- snapshot completo de los datos
    metadata JSONB,                     -- info adicional: endpoint usado, cantidad registros, etc.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,             -- retención opcional
    notas TEXT                          -- descripción opcional del usuario
);

CREATE INDEX idx_bdp_snapshots_user ON bdp_snapshots(user_id, created_at DESC);
CREATE INDEX idx_bdp_snapshots_tipo ON bdp_snapshots(tipo, created_at DESC);
```

### 3.2 Tabla `bdp_audit_log`

```sql
CREATE TABLE bdp_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES usuarios(id),
    operacion VARCHAR(50) NOT NULL,     -- 'create_order', 'create_customer', 'add_payment', 'invoice', 'sync_catalog', 'sync_prices', 'sync_tables'
    direccion VARCHAR(20) NOT NULL,     -- 'glory_to_bdp', 'bdp_to_glory'
    snapshot_pre_id UUID REFERENCES bdp_snapshots(id),  -- snapshot tomado ANTES de la operación
    datos_enviados JSONB,               -- lo que se envió a BDP o lo que vino de BDP
    resultado VARCHAR(20) NOT NULL,     -- 'exito', 'error', 'parcial'
    datos_respuesta JSONB,              -- respuesta de BDP o resultado de Glory
    error_mensaje TEXT,                 -- si hubo error
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bdp_audit_user ON bdp_audit_log(user_id, created_at DESC);
CREATE INDEX idx_bdp_audit_operacion ON bdp_audit_log(operacion, created_at DESC);
```

### 3.3 Configuración nueva en `configuracion`

```sql
ALTER TABLE configuracion ADD COLUMN IF NOT EXISTS bdp_sync_mode VARCHAR(20) DEFAULT 'read_only';
-- Valores: 'read_only', 'unidirectional', 'bidirectional'

ALTER TABLE configuracion ADD COLUMN IF NOT EXISTS bdp_backup_retention_days INTEGER DEFAULT 30;
-- Días que se conservan los snapshots

ALTER TABLE configuracion ADD COLUMN IF NOT EXISTS bdp_auto_backup_before_write BOOLEAN DEFAULT true;
-- Backup automático antes de cada escritura a BDP
```

---

## 4. Implementación por fases

### Fase 0 — Exploración segura (SIN llamadas write)

**Objetivo:** Ver exactamente qué hay en BDP antes de tocar nada.

**Implementación:**

1. Nuevo servicio `BdpExplorerService` con método `explorar_bdp_completo()`
2. Llama SOLO endpoints de lectura:
    - `ExportArticles` (con rango máximo) → catálogo completo
    - `ExportCustomers` (con rango máximo) → clientes completos
    - `ExportDepartment` (con rango máximo) → departamentos
    - `GetRoomsTables` → salones y mesas
    - `GetEmployees` → empleados
    - `GetTenderList` → formas de pago
    - `GetPOSes` → terminales
3. Devuelve resumen de cantidad de registros por categoría
4. Endpoint: `GET /api/bdp/explorar`
5. Frontend: botón "Explorar BDP" que muestra el inventario

**No modifica NADA en BDP.** Solo lectura.

**Esfuerzo:** ~2h

### Fase 1 — Motor de snapshots

**Objetivo:** Poder tomar y almacenar snapshots de BDP y Glory.

**Implementación:**

1. Migración: tablas `bdp_snapshots` + `bdp_audit_log` + campos nuevos en `configuracion`
2. Servicio `BdpBackupService`:
    - `snapshot_bdp_completo(pool, client)` → llama todos los endpoints read de BDP, guarda JSON
    - `snapshot_bdp_parcial(pool, client, tipos)` → solo los tipos seleccionados
    - `snapshot_glory(pool, user_id, tipos)` → exporta tablas Glory relevantes
    - `listar_snapshots(pool, user_id)` → historial de snapshots
    - `obtener_snapshot(pool, id)` → detalle de un snapshot
    - `eliminar_snapshot(pool, id)` → borrar snapshot (con confirmación)
3. Handlers:
    - `POST /api/bdp/backup/completo` → snapshot BDP completo
    - `POST /api/bdp/backup/parcial` → snapshot parcial (tipos)
    - `POST /api/bdp/backup/glory` → snapshot de tablas Glory
    - `GET /api/bdp/backup/snapshots` → listar snapshots
    - `GET /api/bdp/backup/snapshots/:id` → detalle
    - `DELETE /api/bdp/backup/snapshots/:id` → eliminar

**Esfuerzo:** ~4h

### Fase 2 — Backup automático pre-escritura (SELECTIVO)

**Objetivo:** Antes de CADA operación que escribe en BDP, registrar datos en audit log con snapshot mínimo del recurso.

**⚠️ IMPORTANTE:** NO se hace backup completo de BDP antes de cada write. Solo se captura:
- Los datos que se van a enviar (ya disponibles en la función)
- El estado actual del recurso que se modifica (1 llamada `GetOrder` como máximo)
- Datos locales de Glory afectados (queries locales, 0 llamadas BDP)

**Implementación:**

1. Modificar `BdpSyncService::sync_venta()` → antes de `CreateOrder`, registrar datos enviados en audit log (0 llamadas extra)
2. Modificar `BdpSyncService::ensure_cliente_bdp_synced()` → antes de `CreateCustomer`, registrar datos enviados (0 llamadas extra)
3. Modificar `BdpSyncService::add_order_payment()` → antes de `AddOrderPayment`, snapshot estado comanda con `GetOrder` (1 llamada)
4. Modificar `BdpSyncService::invoice_order()` → antes de `InvoiceOrder`, snapshot estado comanda con `GetOrder` (1 llamada)
5. Cada pre-write:
    - Registra en `bdp_audit_log` con `datos_enviados`
    - Si es AddOrderPayment o InvoiceOrder, guarda snapshot del estado actual de la comanda
    - Si el snapshot falla, la operación CONTINÚA (no bloquea) pero registra warning

**Esfuerzo:** ~3h

### Fase 3 — Modo de sincronización

**Objetivo:** Controlar si Glory puede o no escribir en BDP.

**Implementación:**

1. Configuración `bdp_sync_mode`:
    - `read_only`: Glory SOLO lee de BDP. `sync_venta()` no ejecuta. `sync_catalog/sync_prices/sync_tables` sí.
    - `unidirectional`: Glory puede enviar ventas/clientes a BDP. Backup pre-write activo.
    - `bidirectional`: Lectura + escritura en ambas direcciones.
2. Gate en `BdpSyncService`:
    ```rust
    if config.bdp_sync_mode == "read_only" {
        return Err(AppError::Validation("BDP en modo solo lectura. Cambia el modo en configuración."));
    }
    ```
3. Frontend: selector de modo en ConfigBdp con confirmación explícita

**Esfuerzo:** ~2h

### Fase 4 — Restauración

**Objetivo:** Poder restaurar datos desde un snapshot.

**Implementación:**

1. `restaurar_bdp_articulos(snapshot_id)` → re-importa artículos desde snapshot a `bdp_article_map`
2. `restaurar_bdp_clientes(snapshot_id)` → (limitado: BDP no tiene delete/update de clientes via API)
3. `restaurar_glory_ventas(snapshot_id)` → restaura campos BDP de ventas desde snapshot
4. `restaurar_glory_clientes(snapshot_id)` → restaura campos BDP de clientes desde snapshot
5. `restaurar_glory_mapeos(snapshot_id)` → restaura `bdp_article_map` desde snapshot
6. Handlers:
    - `POST /api/bdp/backup/restaurar/:id` → restaurar desde snapshot
7. Frontend: botón "Restaurar" en cada snapshot del historial

**Limitaciones importantes:**

- BDP **NO permite borrar ni actualizar** clientes/órdenes via API
- Restaurar en BDP = re-importar datos correctos a Glory (no modificar BDP)
- Para "deshacer" algo en BDP, hay que hacerlo manualmente desde el TPV
- Lo que SÍ podemos restaurar: datos locales de Glory que reflejan el estado de BDP

**Esfuerzo:** ~3h

### Fase 5 — Frontend completo

**Objetivo:** Panel de backup manejable desde el frontend.

**Implementación:**

1. Nuevo componente `PanelBdpBackup.tsx`:
    - Botón "Explorar BDP" → muestra inventario actual
    - Botón "Snapshot completo" → toma snapshot de todo
    - Botón "Snapshot parcial" → checkboxes por tipo de dato
    - Selector de modo de sync (read_only / unidirectional / bidirectional)
    - Historial de snapshots con fecha, tipo, tamaño
    - Botón "Restaurar" por cada snapshot
    - Botón "Eliminar" por cada snapshot
2. Integración en ConfigBdp como sección nueva
3. Indicador de estado: "BDP en modo solo lectura" / "BDP en modo escritura" / "BDP en modo bidireccional"

**Esfuerzo:** ~3h

### Fase 6 — Auditoría y alertas

**Objetivo:** Registro inmutable de todas las operaciones.

**Implementación:**

1. Cada operación de sync escribe en `bdp_audit_log`
2. Endpoint `GET /api/bdp/audit` → historial de operaciones con filtros
3. Frontend: sección de auditoría con tabla paginada
4. Alertas: si hay errores en operaciones de escritura, notificación al usuario

**Esfuerzo:** ~2h

---

## 5. Estimación total

| Fase      | Descripción            | Esfuerzo | Dependencia |
| --------- | ---------------------- | -------- | ----------- |
| 0         | Exploración segura     | ~2h      | —           |
| 1         | Motor de snapshots     | ~4h      | F0          |
| 2         | Backup pre-escritura   | ~3h      | F1          |
| 3         | Modo de sincronización | ~2h      | F1          |
| 4         | Restauración           | ~3h      | F1          |
| 5         | Frontend completo      | ~3h      | F1+F3       |
| 6         | Auditoría              | ~2h      | F1          |
| **Total** |                        | **~19h** |             |

### Orden de ejecución recomendado

```
F0: Exploración segura (lee BDP, no toca nada)
  ↓
F1: Motor de snapshots (base del sistema)
  ↓
F2 + F3 en paralelo: Backup pre-escritura + Modo sync
  ↓
F4 + F6 en paralelo: Restauración + Auditoría
  ↓
F5: Frontend (requiere F1+F3, usa F4+F6)
```

---

## 6. Preguntas para el usuario

Antes de implementar, necesito aclarar:

1. **¿Cuántos artículos/clientes hay en BDP?** → F0 nos dirá. Si hay miles, el snapshot puede ser pesado.
2. **¿Quieres que el modo default sea `read_only` y solo se active `unidirectional` con confirmación?** → Recomendado.
3. **¿Los snapshots de BDP se guardan en la BD de Glory (PostgreSQL) o en archivos JSON?** → Propongo PostgreSQL (JSONB) por simplicidad y consistencia.
4. **¿Retención de snapshots?** → Propongo 30 días por defecto, configurable.
5. **¿Quieres que F0 (exploración) se implemente primero para ver qué hay antes de construir el resto?** → Recomendado.

---

## 7. Riesgos y mitigaciones

| Riesgo                                                | Probabilidad | Impacto | Mitigación                                                     |
| ----------------------------------------------------- | ------------ | ------- | -------------------------------------------------------------- |
| BDP devuelve datos incompletos en ExportArticles      | Media        | Alto    | Validar respuesta, registrar warnings, no confiar ciegamente   |
| Snapshot muy grande (miles de artículos)              | Baja         | Medio   | Comprimir JSON, paginación, retención limitada                 |
| Pre-write snapshot ralentiza operaciones              | Media        | Bajo    | Snapshot asíncrono (no bloquea), timeout corto                 |
| BDP cambia estructura de respuesta                    | Baja         | Alto    | Versionado de parsers, tests contra respuestas reales          |
| Usuario activa `unidirectional` sin entender riesgos  | Media        | Alto    | UI con confirmación explícita + warning + snapshot obligatorio |
| Restore de Glory no coincide con estado actual de BDP | Media        | Alto    | Restore SIEMPRE preceded by fresh snapshot de BDP              |

---

## 8. Decisión arquitectónica clave

**¿Por qué no hacer backup de BDP "de verdad"?**

Porque BDP no permite:

- ❌ Crear snapshots/backups via API
- ❌ Eliminar órdenes o clientes
- ❌ Restaurar estado anterior de datos
- ❌ Exportar TODO en una sola llamada

Lo que SÍ podemos hacer:

- ✅ Leer todo lo que hay en BDP (23+ endpoints de lectura)
- ✅ Guardar esa lectura como snapshot local
- ✅ Controlar si Glory escribe o no en BDP
- ✅ Restaurar datos LOCALES de Glory desde snapshots
- ✅ Registrar cada operación en audit log
- ✅ Alertar si algo sale mal

**La protección real viene de:**

1. **Leer primero, escribir después** → snapshot antes de cada write
2. **Control de modo** → read_only por defecto, escritura solo con confirmación
3. **Auditoría** → saber exactamente qué se hizo y cuándo
4. **Restore local** → poder revertir cambios en Glory si BDP fue alterado incorrectamente
