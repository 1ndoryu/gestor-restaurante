# Plan — Escrituras BDP completas (todo lo que BDP puede escribir, se escribe; independencia intacta)

> **Fecha:** 2026-08-19
> **Rama:** `glory-rs-rest`
> **ID de bloque:** `198A-1`
> **Estado:** **Completado 2026-08-19** (plan archivado en `Agente/planes/completados/`).
> F1–F9 implementadas, incluido el wiring de `CancelOrder` (F6). D1–D10 resueltas.
> Implementado y verificado (`cargo check --lib --tests` limpio, `cargo test --lib` 153 OK,
> `cargo test --test bdp_push` 13 OK, `cargo test --test bdp_inventario` 3 OK,
> `cargo fmt --check` limpio, frontend `tsc --noEmit` limpio):
> catálogo de endpoints (20 nuevas), `BdpWeblinkClient` (20 métodos), worker de flush
> `BdpPushFlushService` con guards (arming/backup/auditoría) y no-op en standalone, wiring de artículo
> (modificar + alta D3 con rango reservado) y stock, handlers locales nuevos: departamento/familia
> (D7 código secuencial), propina por venta (D8), puntos de fidelización (D9) e inventario físico
> (D6=A, `POST /api/bdp/inventario` → `UpdateMassiveInventory`) — todos encolan en `BdpPushService`.
> UI: CallWaiter (D10) en el plano de sala (oculto en standalone), propina por venta, puntos en ficha
> de cliente, catálogo departamentos/familias e inventario. Botón "Sincronizar a BDP" en el indicador
> BDP del header (flush manual `POST /api/bdp/push/flush`, requerido por D1/D2; reintento tras
> suscripción solo manual). Tests de payloads encolados + endpoint de inventario.
> Migraciones `bdp_push_escrituras` + correctiva `bdp_push_estado_ancho` + `bdp_catalogo_propina_puntos`
> + `bdp_write_arming_ampliar` (corrige las CHECK de scopes/dominios que bloqueaban el arming del push).
> **Pendiente (diferido por diseño, no bloqueante):** verificación real contra BDP
> (suscripción/datos del cliente; BDP offline). `CancelOrder` real sujeto a suscripción activa.
> **Base:** bloque **128A-1** completado (2026-08-13) — independencia total (conmutador standalone/bdp,
> catálogo local, stock local, anulación local, compras locales, pagos/factura local, menús locales,
> permisos configurables). Este plan **añade la capa de escritura BDP** sobre esos datos locales, sin
> tocar la independencia.
> **Skills aplicadas:** `supervisor-thinking` (diseño y desafío) y `supervisor-review` (revisión dura) —
> veredicto en el Anexo B.
>
> **Objetivo (cita del usuario):** "todo lo que puede ser escritura en el BDP, lo sea, y que siga
> funcionando independiente del BDP también si se quiere". Verificado contra el manual WebLink REST
> (`# WEBLINK RESTAPI.md`, índice líneas 300–460 + rutas) y contra `src/`: **15 escrituras sin
> implementar** (cero referencias en código, incluida `CallWaiter`) + **`CancelOrder` implementada pero
> bloqueada** por suscripción (Hallazgo 048A-11) = **16 escrituras pendientes** que este plan cubre.

---

## 0. Tabla resumida (para revisión rápida)

| Área | Escrituras BDP | Qué hace | Estado hoy | Esfuerzo |
| --- | --- | --- | --- | --- |
| **Infraestructura push** | — | Servicio de push con `local_dirty`, cola de pendientes, guards (arming/backup/auditoría), reporte de sync | ❌ No existe | L |
| **Catálogo → BDP** | `CreateArticlesAndUpdateProfiles`, `ModifyArticleAndUpdateProfile`, `ModifyPricesArticles` | Crear/editar artículo y precios en BDP desde el catálogo local (F2 de 128A-1) | ❌ 3 sin implementar | L |
| **Stock → BDP** | `UpdateStock`, `UpdateMassiveStock`, `UpdateMassiveInventory`, `Regularizations`, `Transfers`, `CreateFamily`, `CreateSubfamily` | Regularizar/traspasar stock y crear familias en BDP desde el stock local (F3 de 128A-1) | ❌ 7 sin implementar | L |
| **Departamentos → BDP** | `CreateDepartment`, `CreateDepartmentAndupdateProfiles` | Crear departamento en BDP desde el catálogo local | ❌ 2 sin implementar | M |
| **Comandas → BDP** | `AddOrderTip`, `CancelOrder` | Propina en ventas locales + cancelar comanda BDP | ❌ 1 sin impl. + 1 bloqueada por suscripción | S–M |
| **Plano de sala → BDP** | `CallWaiter` | Botón "llamar camarero" en el plano → aviso emergente en el TPV BDP | ❌ 1 sin implementar | S |
| **Fidelización → BDP** | `AddPoints` (+ `GetPoints` lectura de soporte) | Sumar/restar puntos a cliente en BDP | ❌ 1 sin implementar | S |
| **Lecturas de soporte** | `GetApplicationVersion`, `GetProfilesListCreateArticleList`, `GetProfileListModifyArticleList`, `GetProfilesListCreateDepartmentList`, `GetPoints` | Diagnóstico de suscripción, perfiles para crear/modificar, saldo de puntos | ❌ 5 sin implementar (lecturas) | M |
| **Simulador + pruebas** | Ampliar simulador con las 15 rutas nuevas | Verificación local sin BDP real (mismo enfoque que 128A-1 F9) | ⚠️ 16 rutas hoy | M |

**Totales:** **16 escrituras BDP cubiertas** (15 nuevas: 14 de push sobre datos locales + `CallWaiter`
del plano; + `CancelOrder` ya implementada pero pendiente de suscripción), 5 lecturas de soporte
nuevas y la infraestructura de push. Catálogo de endpoints: 32 → **52** (20 nuevas). En modo
`standalone` **nada** de esto se invoca — la independencia se conserva íntegra.

**Fuera de alcance (BDP no lo expone — verificado en el manual):** menús/fastfoods/packs (solo
lectura `GetMenuDefinition`/`GetFastfoodDefinition`/`GetPackDefinition`) y compras/albaranes (solo
`ExportPurchaseNotes`). Sus CRUD permanecen locales.

---

## 1. Problema real, objetivo y no-goals

**Problema real:** tras 128A-1, Glory edita localmente catálogo, stock, albaranes, anulaciones, menús y
facturas — pero **solo 4 de las 20 escrituras de negocio que BDP soporta están en uso**
(`CreateCustomer`, `CreateOrder`, `AddOrderPayment`, `InvoiceOrder`). El resto del trabajo del restaurante (crear un
artículo, ajustar stock, regularizar, traspasar, crear una familia o departamento, añadir una propina,
dar puntos de fidelización, cancelar una comanda) queda **solo local**: si el cliente opera con BDP
conectado, esos cambios no se reflejan en BDP y el TPV del restaurante y Glory divergen.

**Objetivo (resultado deseado):** todo dato que el BDP pueda aceptar como escritura se escribe en BDP
cuando está conectado y el módulo de suscripción está activo — y sigue siendo 100% editable y funcional
sin BDP. BDP sigue siendo una **capa opcional**: en `standalone` nada se envía; en `bdp` los cambios
locales se empujan con `local_dirty`, sin bloquear jamás la operación local.

**No-goals:**
- No se implementa sincronización bidireccional automática compleja (mantiene la decisión D3 de
  128A-1): el push es **unidireccional Glory→BDP** de cambios marcados `local_dirty`. El import BDP→Glory
  sigue sin pisar ediciones locales (M6 de 128A-1).
- No se escribe en el BDP real sin autorización explícita ni sin suscripción verificada (lección
  048A-11: BDP responde "Subscripción no activada").
- No se implementa menús/fastfoods/packs ni compras como escritura BDP (BDP no tiene endpoint).
- No se toca la capa de conexión `bdp_*` existente más allá de añadir endpoints al catálogo y métodos al
  cliente (mismo patrón que las escrituras actuales).
- No se promete verificación contra BDP real si el BDP está offline o sin módulos activos; se entrega
  con verificación local (simulador ampliado + wiremock) y el plan de verificación real queda registrado.
- No se fuerza multi-almacén ni talla/color (BDP lo soporta, Glory no lo modela — M5/M6 de este plan).

---

## 2. Hechos confirmados (verificados en código y manual — 2026-08-19)

**Escrituras pendientes — 0 referencias en `src/` (verificado):**

| Función | Ruta en el manual | Estado en código |
| --- | --- | --- |
| `AddOrderTip` | `/API/Orders/Tip/Add` | ❌ 0 refs |
| `CreateArticlesAndUpdateProfiles` | `/API/Articles/CreateAndUpdateProfiles` | ❌ 0 refs |
| `ModifyArticleAndUpdateProfile` | `/API/Articles/ModifyAndUpdateProfiles` | ❌ 0 refs |
| `ModifyPricesArticles` | `/API/Articles/ModifyPrices` | ❌ 0 refs |
| `CreateDepartment` | `/API/Departments/Create` | ❌ 0 refs |
| `CreateDepartmentAndupdateProfiles` | `/API/Departments/CreateAndUpdateProfiles` | ❌ 0 refs |
| `AddPoints` | `/API/Loyalty/AddPoints` | ❌ 0 refs |
| `CreateFamily` | `/API/Warehouse/CreateFamily` | ❌ 0 refs |
| `CreateSubfamily` | `/API/Warehouse/CreateSubfamily` | ❌ 0 refs |
| `Regularizations` | `/API/Warehouse/Regularizations` | ❌ 0 refs |
| `Transfers` | `/API/Warehouse/Transfers` | ❌ 0 refs |
| `UpdateMassiveStock` | `/API/Warehouse/UpdateMassiveStock` | ❌ 0 refs |
| `UpdateStock` | `/API/Warehouse/UpdateStock` | ❌ 0 refs |
| `UpdateMassiveInventory` | `/API/Warehouse/UpdateMassiveInventory` | ❌ 0 refs |
| `CallWaiter` | `/API/Waiters/Call` | ❌ 0 refs (aviso emergente de atención de camarero en el TPV BDP para una mesa/salón — encaja con el plano de sala local) |
| `CancelOrder` | `/API/Orders/Cancel` | ✅ Implementada (`BDP_PATH_CANCEL_ORDER` + `cancel_order`), bloqueada por BDP real ("Subscripción no activada", 048A-11); no se llama desde anulación local (decisión C3=b de 128A-1) |

**Lecturas de soporte — 0 refs (verificado):** `GetApplicationVersion` (`/Service/GetApplicationVersion`,
devuelve estado de la suscripción extendida — clave para diagnosticar módulos), `GetProfilesListCreateArticleList`
(`/API/ProfilesLists/GetCreateArticleList`), `GetProfileListModifyArticleList`
(`/API/ProfilesLists/GetModifyArticleList`), `GetProfilesListCreateDepartmentList`
(`/API/ProfilesLists/GetCreateDepartmentList`), `GetPoints` (`/API/Loyalty/GetPoints`).

**Patrones de código que se reutilizan (verificados):**
- Cliente: cada escritura usa `self.ensure_write_target_allowed()?` + `post_authenticated_json(path, req)`
  (`src/services/bdp_weblink.rs`, ej. `create_customer`/`create_order`).
- Catálogo de endpoints: `BdpEndpointSpec { name, area, path, purpose }` + `BdpEndpointArea`
  (`src/services/bdp_weblink_catalog.rs`, líneas 51–68).
- Guards de escritura: `BdpWriteGuard::authorize` / `try_auto_arm` / `ensure_no_unresolved`
  (`src/services/bdp_write_guard.rs`), backup pre-write (`bdp_backup.rs`), auditoría (`bdp_audit_log`),
  arming/auto-arming.
- Datos locales base (de 128A-1): `bdp_article_map` ampliada (`origen`, `local_dirty`,
  `omitidos_ediciones_locales`, `desactivados_localmente`, `precio_tarifa1`, `iva_pct`, `departamento`,
  `familia`, `subfamilia`, `barcode`, `activo`, `articulo_bdp_codigo`); `bdp_article_stock` por almacén;
  anulación local (`anulada`, `anulacion_motivo`, `anulacion_modalidad`); `bdp_pagos` ledger;
  `bdp_menus_locales`; permisos `permisos_*` + `verificar_permiso`.
- Simulador: dispatch por path en `tools/bdp-weblink-simulator/server.py` (`_dispatch`, ~16 rutas hoy);
  tests Python `test_server.py` (92 hoy) + integración Rust `tests/bdp_simulator_integration.rs` (24 hoy).

**Hechos de BDP real (lección 048A-11, pruebas 2026-08-04/05):** la API gratuita rechaza pago/factura con
"Subscripcion no activada" aunque el cliente afirma tener suscripción activa → **cada módulo (almacén,
fidelización, pagos) puede rechazar igual**. Por eso el diseño de este plan trata "módulo no activo" como
**estado pendiente visible, no como fallo**.

---

## 3. Arquitectura: capa de push Glory→BDP

### 3.1 Principio

Sobre los datos locales de 128A-1 (que ya tienen `origen` y `local_dirty`), se añade un **servicio de
push unidireccional** con esta invariante: *ninguna escritura BDP se intenta fuera de modo `bdp`
efectivo, y ninguna operación local se bloquea por el estado del push*.

### 3.2 Disparador configurable (`push_modalidad` — D1 resuelta)

Nuevo campo en `configuracion_restaurante`: **`push_modalidad`** (`automatico` default | `manual`).

| Valor | Comportamiento |
| --- | --- |
| `automatico` | Las ediciones locales encolan; cuando modo bdp efectivo **y** modo escritura armado (arming/auto-arming), la cola se sincroniza sola por dominio (orden por dependencias, M12) |
| `manual` | La cola acumula; solo se sincroniza con el botón "Sincronizar a BDP" por pantalla (con arming) |

Invariantes: el push **nunca** escribe sin arming (fail-closed de 128A-1 §3.3 se conserva); en `standalone`
la cola acumula `pendiente` sin llamadas, en ambas modalidades. El botón manual existe siempre (forzar/
reintentar) aunque la modalidad sea automática.

**Política de reintentos (D2 resuelta 2026-08-19):**
- **Error transitorio** (timeout, BDP caído, error de red) → reintento automático **acotado** (tope
  configurable, default 5, con backoff).
- **Bloqueo por suscripción** ("Subscripción no activada") → estado `pendiente_suscripcion` y **SOLO
  reintento manual** (botón "Sincronizar a BDP"). La suscripción no es un fallo transitorio: puede no
  activarse nunca; reintentar automáticamente generaría ruido y llamadas inútiles indefinidas.
- `GetApplicationVersion` **no** se usa para auto-reintentar (M23): exige un código `Application` por
  módulo que no conocemos; queda como diagnóstico/manual en Configuración.

### 3.3 Flujo de push (por dominio)

```
edición local (CRUD existente) → marca local_dirty + registra en cola de pendientes
  ↓ (cuando modo_efectivo() == bdp y flag del módulo activo y BDP alcanzable)
push: guards (permiso + arming + backup pre-write + auditoría) → construir request BDP → llamar
  ↓ éxito        → limpiar local_dirty + estado 'sincronizado' + auditoría (origen_operacion='bdp')
  ↓ suscripción  → estado 'pendiente_suscripcion' + aviso UI (reintento SOLO manual — D2)
  ↓ error BDP    → estado 'error' + motivo + reintento acotado (backoff) + reporte visible
  ↓ BDP caído    → estado 'pendiente' + se reintenta con la histéresis del modo (M2 de 128A-1)
```

### 3.4 Tabla nueva: cola de push (`bdp_push_pendientes`)

| Columna | Tipo | Notas |
| --- | --- | --- |
| `id` | PK | — |
| `user_id` | FK | por instalación |
| `dominio` | enum | `articulo` \| `stock` \| `departamento` \| `familia` \| `venta` \| `cliente_puntos` \| `propina` |
| `entidad_id` | BIGINT | id local del registro (articulo_glory_codigo, venta id, …) |
| `operacion` | enum | `crear` \| `modificar` \| `precios` \| `regularizar` \| `traspasar` \| `inventario` \| `cancelar` \| `puntos` \| `propina` |
| `payload_json` | JSONB | request BDP construido (para reintento idéntico e idempotencia) |
| `estado` | enum | `pendiente` \| `pendiente_suscripcion` \| `error` \| `sincronizado` \| `descartado` |
| `reintentos` | int | con tope; solo para errores transitorios, no consume en suscripción (D2) |
| `ultimo_error` | text | ErrorMessage de BDP |
| `creado_at` / `actualizado_at` | timestamptz | — |

**UNIQUE parcial `(user_id, dominio, entidad_id, operacion)`** con upsert (M19): una sola fila pendiente por
entidad+operación; nuevas ediciones actualizan el `payload_json` en vez de duplicar filas. Migración
**aditiva** (M18), sin tocar tablas existentes.

**Alternativa considerada y descartada:** push síncrono sin cola (cada edición llama a BDP en línea).
Tradeoff: una edición local no debe depender de la latencia/estado de BDP; la cola da idempotencia,
reintentos y reporte. La escritura síncrona se usa solo para acciones explícitas del usuario
("Sincronizar ahora" — D1).

### 3.5 Guards y seguridad (mantiene el fail-closed de 128A-1)

- Cada endpoint nuevo entra en el catálogo con su área y **se marca como escritura** (extender
  `BdpEndpointSpec` si hace falta con `tipo: lectura|escritura`).
- Toda escritura pasa por: `modo_efectivo() == bdp` → flag de módulo (si aplica) → permiso operativo
  (reutilizar `verificar_permiso` de 128A-1 F8, ampliar con `stock_ajuste`, `catalogo_edicion`) →
  `BdpWriteGuard` (arming/auto-arming) → backup pre-write → auditoría.
- `ensure_write_target_allowed()` ya está en el cliente; los paths nuevos heredan la protección de
  allowlist. `CancelOrder` y `AddOrderTip` usan allowlist de escritura normal.
- La cola de push **no reintenta en bucle**: respeta tope de reintentos, backoff y nunca en
  `standalone`.

### 3.6 Coherencia con 128A-1 (sin contradicción)

| Mecanismo de 128A-1 | Cómo lo respeta este plan |
| --- | --- |
| `modo_operacion` switch maestro (M1) | El push solo corre con `modo_efectivo()==bdp`; en standalone la cola acumula `pendiente` sin llamadas |
| `local_dirty` (M6: import no pisa ediciones) | El push **limpia** `local_dirty` tras éxito; mientras haya push pendiente el import sigue sin pisar |
| Histéresis M2 | El push usa el mismo `ServicioModoOperacion`; un fallo de BDP no degrada por sí solo el modo |
| Permisos (F8) | Reutiliza `verificar_permiso` con las mismas acciones |
| Flags de módulo (M12) | Nuevos flags o reutilización de los 6 existentes según el área (D1 de 128A-1 no toca flags de almacén) |
| Anulación sin `CancelOrder` (C3=b) | `CancelOrder` se **añade como push programado** (solo cuando suscripción activa) sin cambiar la anulación local |
| Auditoría local (A11) | El push escribe auditoría con `origen_operacion='bdp'` (o `'local'` si solo quedó local) |

---

## 4. Diseño por dominio (con BDP → escritura BDP)

### 4.1 Catálogo → BDP (`CreateArticlesAndUpdateProfiles`, `ModifyArticleAndUpdateProfile`, `ModifyPricesArticles`)

- **Origen:** catálogo local `bdp_article_map` (F2 de 128A-1).
- **Mapeo de códigos (M1 de este plan, corregido por M11):** BDP usa `ArtCode` entero (hasta 13 dígitos);
  Glory usa `articulo_glory_codigo` TEXT. **Verificado en el manual: `CreateArticlesAndUpdateProfiles` NO
  devuelve el código asignado en la respuesta** (solo `ErrorMessage` + `ListaErroresArticulo`) →
  `AutomaticCode` dejaría el artículo en BDP sin que Glory sepa su código (divergencia silenciosa).
  Reglas finales:
  - Si el artículo local tiene `articulo_bdp_codigo` → `ModifyArticleAndUpdateProfile` /
    `ModifyPricesArticles` con ese código.
  - Si no (creado solo localmente) → asignar automáticamente un **código numérico de un rango
    reservado** (configurable, default `90 000 000` en adelante, ≤13 dígitos), **pre-check** con
    `GetArticle`/`GetPOSArticlesList` para evitar colisión (M22), y enviar explícito con
    `AutomaticCode=false` (D3 resuelta así: ni fricción de pedir código, ni código desconocido).
  - Si el código local ya es numérico ≤13 dígitos y libre en BDP, se usa directamente.
- **Campos mínimos de `ArticleListDataType` a mapear:** `DeptCode`, `DeptDescription`, `TAVCode`,
  `TAVPer` (IVA de venta), `Is_Inventoriable`, `ModifiablePrice`, `WebArticle`, `MenuDish`,
  `BuyTAVCode/Per` (opcional), `POS_*ID` (opcional, solo si el artículo es menú/fastfood/pack — D4).
- **Mapeo de IVA (M13):** el local guarda `iva_pct` (%), pero BDP pide `TAVCode` (int) + `TAVPer`.
  `ExportArticles` devuelve `SalesVAT` (%) pero no el `TAVCode` de venta → nueva configuración
  `bdp_tav_map` (`iva_pct` → `TAVCode`), con defaults orientativos (p. ej. 10→1, 21→2 según los ejemplos
  del manual con `BuyTAVCode` 1=10,0 y 2=21,0), editable en Configuración y auto-aprendida si `GetArticle`
  devuelve el código al consultar un artículo importado.
- **`ModifyPricesArticles`:** mapear `precio_tarifa1..5` → `Price1..5`, `Dct1..5` (descuentos,
  localmente no modelados → 0 o configurables — D4).
- **Perfiles:** `GetProfilesListCreateArticleList` / `GetProfileListModifyArticleList` devuelven los
  perfiles disponibles; el push envía la lista de perfiles del terminal. **D4 simplificada:** la variante
  `CreateDepartmentAndupdateProfiles` admite `AllProfiles=true` (añade a todos los posibles) → en
  departamentos no hace falta resolver la lista; en artículos se usa la lista de perfiles POS activos
  (F0 la confirma).
- **UI:** en el catálogo local, columna/estado "BDP: pendiente / sincronizado / error (motivo)" +
  botón "Sincronizar a BDP" (acción explícita, D1) + filtro de pendientes.
- **Aceptación:** crear un artículo local con BDP conectado (simulador) → aparece en `ExportArticles`
  del simulador; modificar precio → `ModifyPricesArticles` recibe el nuevo PVP; sin BDP → nada cambia.

### 4.2 Stock → BDP (`UpdateStock`, `UpdateMassiveStock`, `UpdateMassiveInventory`, `Regularizations`, `Transfers`, `CreateFamily`, `CreateSubfamily`)

- **Origen:** `bdp_article_stock` + ajuste local existente (`POST /api/bdp/article-stock/ajustar`, F3 de
  128A-1) y familias/subfamilias del catálogo.
- **Ajuste puntual:** al ajustar stock local con BDP conectado → push `UpdateStock` (artículo, unidades
  +/−, almacén `Store`, motivo `CodReg`, fecha). Almacén y motivo: **configurables** (D5) con defaults
  (Store=1 "General", CodReg=1).
- **Regularización masiva:** `UpdateMassiveStock` para el caso "ajuste de varios artículos a la vez"
  (misma pantalla con lote); `Regularizations` es la variante por artículo con motivo explícito.
- **Inventario:** `UpdateMassiveInventory` — conteo físico de un conjunto de artículos (D6: alcance —
  UI de inventario completa vs mínima).
- **Traspasos:** `Transfers` requiere 2 almacenes (StoreFrom/StoreTo) — solo se expone si la
  configuración tiene ≥2 almacenes (D5).
- **Familias:** `CreateFamily`/`CreateSubfamily` al crear familia/subfamilia en el catálogo local con
  BDP conectado. **Códigos (M14):** BDP pide `Code` int de 1–3 dígitos y la familia local es texto libre
  → asignación secuencial automática (misma lógica que departamentos, D7 ampliada a familias/
  subfamilias), guardando el código asignado junto al nombre local.
- **Talla/color:** BDP pide `sD1..sD3` — Glory no modela talla/color → se envían vacíos (M5 de este
  plan). Fechas en ISO-8601 correcto (el manual muestra ejemplos con espacios — gotcha M6).
- **UI:** sección stock con badge de origen (ya existe) + estado de push + botones por operación.
- **Aceptación:** ajustar stock local con simulador → simulador refleja el cambio vía `UpdateStock`;
  crear familia → aparece en `GetListStock`/catálogo del simulador.

### 4.3 Departamentos → BDP (`CreateDepartment`, `CreateDepartmentAndupdateProfiles`)

- **Origen:** `departamento` del catálogo local (texto libre hoy) → al guardar un departamento nuevo con
  BDP conectado, push `CreateDepartment` (Code 1–3 dígitos — el local no tiene código numérico → asignar
  secuencial o pedir código — D7) + `CreateDepartmentAndupdateProfiles` para perfiles (o la variante
  simple si no hay perfiles configurados).
- **Aceptación:** crear departamento local con simulador → aparece en `ExportDepartment` del simulador.

### 4.4 Comandas → BDP (`AddOrderTip`, `CancelOrder`)

- **Propina:** campo `propina` en `ventas` (migración aditiva) + UI para añadir propina antes/después del
  pago; con BDP conectado → push `AddOrderTip` (OrderIdentifier de la comanda BDP, `Amount`, `AddTip`
  según D8: sumar o sustituir). Validación del manual: el total con propina no puede ser inferior a los
  pagos (M10 de este plan). **Pre-requisito M16:** solo se push si la venta tiene identificadores BDP
  (`bdp_order_id`/Room/Table); si no → estado `pendiente` con aviso "comanda no sincronizada" (no es un
  error del push).
- **CancelOrder:** ya implementada en el cliente. Se añade: cuando una venta anulada localmente tiene
  comanda BDP (`bdp_order_id`), queda `anulada_local_pendiente_bdp` (estado M8 de 128A-1 ya existente) y
  el push la reintenta **solo manualmente** (D2=B; botón "Sincronizar a BDP") cuando el cliente confirme
  tener la suscripción activa; al confirmarse la cancelación en BDP → se marca `cancelada_en_bdp` y se
  cierra la auditoría. **No** se reintenta automáticamente mientras BDP siga rechazando; se muestra estado
  "pendiente de suscripción".
- **Aceptación:** anular venta con simulador y suscripción simulada activa → simulador marca la comanda
  `Status=2` (cancelada); sin suscripción → estado visible pendiente, cero reintentos en bucle.

### 4.5 Plano de sala → BDP (`CallWaiter`)

- **Origen:** plano de sala local (mesas/salones ya sincronizados).
- **UI:** botón "Llamar camarero" por mesa en el plano (solo con BDP conectado; en standalone se oculta
  con motivo, no es una operación local). Push directo `CallWaiter` con `Table`/`Room` (mismos
  identificadores del plano) — es una acción puntual, no va a la cola (D10).
- **Aceptación:** pulsar "Llamar camarero" con simulador → el simulador registra la llamada
  (`/API/Waiters/Call` en el historial) sin error.

### 4.6 Fidelización → BDP (`AddPoints` + `GetPoints`)

- **Origen:** clientes locales + ventas (regla de puntos — D9).
- `GetPoints` (lectura de saldo) para mostrar el saldo en la ficha de cliente; `AddPoints` con
  `Customer` (código BDP del cliente — `bdp_customer_code`), `PointsAdded` (positivo/negativo),
  `Reason` (motivo obligatorio).
- **UI:** ficha de cliente → sección "Puntos" (saldo + añadir/restar con motivo). Gating por módulo de
  fidelización (D9).
- **Aceptación:** sumar puntos con simulador → saldo reflejado en `GetPoints` del simulador.

### 4.7 Lecturas de soporte nuevas

| Función | Ruta | Para qué |
| --- | --- | --- |
| `GetApplicationVersion` | `/Service/GetApplicationVersion` | Diagnóstico: estado de la suscripción extendida → decide `pendiente_suscripcion` vs `error` |
| `GetProfilesListCreateArticleList` | `/API/ProfilesLists/GetCreateArticleList` | Perfiles para `CreateArticlesAndUpdateProfiles` |
| `GetProfileListModifyArticleList` | `/API/ProfilesLists/GetModifyArticleList` | Perfiles para `ModifyArticleAndUpdateProfile` |
| `GetProfilesListCreateDepartmentList` | `/API/ProfilesLists/GetCreateDepartmentList` | Perfiles para `CreateDepartmentAndupdateProfiles` |
| `GetPoints` | `/API/Loyalty/GetPoints` | Saldo de puntos del cliente |

Todas siguen el patrón de lectura del cliente existente (post_authenticated_json + struct de respuesta
`Value` o tipada según convención) y se añaden al catálogo con su área.

---

## 5. Fases y checklist ejecutable (orden propuesto D-orden — natural)

| Fase | Contenido | Salida verificable | Depende de |
| --- | --- | --- | --- |
| **F0** | Auditoría del estado real: suscripciones/módulos del cliente (pagos, almacén, fidelización), perfiles TPV, almacenes, motivos de regularización; confirmar los datos que las decisiones D4/D5 asumen (POS activos, almacenes/motivos) | Inventario actualizado con fecha y evidencia | — |
| **F1** | Infraestructura push: migración `bdp_push_pendientes` (UNIQUE parcial M19), `push_modalidad` en config (D1 resuelta), servicio `BdpPushService` (orden por dependencias M12, reintentos con tope), guards/arming/backup/auditoría integrados, catálogo con `tipo`, flags de módulo, `bdp_tav_map` (M13) | Cola + push de prueba unitario (crear→sincronizar→limpiar dirty; suscripción→pendiente; colisión de códigos M22) | F0 |
| **F2** | Lecturas de soporte: `GetApplicationVersion` + 3 de perfiles + `GetPoints` (structs, catálogo, tests wiremock) | 5 lecturas en catálogo + tests PASS | F1 |
| **F3** | Catálogo → BDP: `CreateArticlesAndUpdateProfiles`, `ModifyArticleAndUpdateProfile`, `ModifyPricesArticles` + mapeo de códigos (M1) + UI estado push | Push de alta/modificación/precios contra simulador PASS; standalone intacto | F2 |
| **F4** | Stock → BDP: `UpdateStock`, `Regularizations`, `UpdateMassiveStock`, `UpdateMassiveInventory`, `Transfers`, `CreateFamily`, `CreateSubfamily` + config almacenes/motivos | Push de ajuste, regularización, traspaso, familia contra simulador PASS | F2 |
| **F5** | Departamentos → BDP: `CreateDepartment`, `CreateDepartmentAndupdateProfiles` + asignación de código | Push de departamento contra simulador PASS | F2 |
| **F6** | Comandas/plano → BDP: campo `propina` + `AddOrderTip`; wiring de `CancelOrder` con estado `pendiente_suscripcion`; `CallWaiter` en el plano de sala | Propina, cancelación y llamada a camarero contra simulador (con/sin suscripción simulada) PASS | F1 |
| **F7** | Fidelización → BDP: `AddPoints` + `GetPoints` + UI ficha cliente | Push de puntos contra simulador PASS | F2 |
| **F8** | Simulador ampliado (16 rutas nuevas + fixture) con **inyección de fallos por módulo** (M17: `__simulator/fault` devuelve "Subscripción no activada" por ruta) + pruebas integrales: suite standalone completa, integración Rust contra simulador, wiremock para suscripción/errores | Simulador Python nuevo PASS, integración Rust PASS, `task:check` PASS | F3–F7 |
| **F9** | Cierre documental: roadmap, completados, feature-flags, mapeo visual, plan a `planes/completados/`; registro del plan de verificación real (BDP online + suscripciones) | Documentación actualizada y evidencia registrada | F8 |

**SIGUIENTE ACCIÓN verificable:** resolver D1–D9 (§6) → F0.

---

## 6. Decisiones del usuario (D1–D10 resueltas 2026-08-19)

> Cada decisión se pregunta 1×1 con recomendación (estilo 128A-1). Ninguna bloquea el inicio de F0.

| # | Decisión | Opciones | Recomendación |
| --- | --- | --- | --- |
| **D1** | ¿Push automático o manual? | Configurable: `push_modalidad` (`automatico` \| `manual`) | ✅ **Resuelta 2026-08-19** (decisión del usuario: "lo que sea mejor; si se puede elegir con configuración, mejor"): `push_modalidad` en Configuración, default **`automatico`**, botón manual siempre disponible; el push nunca escribe sin arming |
| **D2** | Cuando BDP rechaza por suscripción ("Subscripción no activada") | A) Estado `pendiente_suscripcion` + reintento automático cuando `GetApplicationVersion` muestre módulo activo; B) Estado + reintento solo manual | ✅ **Resuelta 2026-08-19 (B, decisión del usuario):** la suscripción no se resuelve pronto ni garantizado → `pendiente_suscripcion` con **reintento SOLO manual**; el reintento automático queda reservado a errores transitorios (timeout/red), con tope y backoff. `GetApplicationVersion` no se usa para auto-reintentar (M23)
| **D3** | Artículo local sin `articulo_bdp_codigo` (creado solo en Glory): ¿`AutomaticCode=true` (BDP asigna) o código explícito? | A) Automático siempre; B) Automático si el código local no es numérico, explícito si lo es; C) Siempre pedir código al usuario | ✅ **Resuelta 2026-08-19 (C, decisión del usuario):** código explícito de **rango reservado automático** (configurable, default `90xxxxxxx`), editable si el usuario quiere un código concreto; pre-check `GetArticle` para evitar colisión (M20/M22); `AutomaticCode` queda descartado (M11) |
| **D4** | Perfiles para crear/modificar artículos y departamentos | A) Todos los POS activos (`GetProfilesList*`); B) Perfil configurado en Configuración; C) Ninguno (solo la variante sin perfiles) | ✅ **Resuelta 2026-08-19 (A, decisión del usuario — "lo que sea mejor para el cliente"):** todos los POS activos; en departamentos `AllProfiles=true` (sin resolver lista); se confirma en F0 cuántos TPV hay |
| **D5** | Almacenes y motivos de regularización/traspaso | A) Configurables en Configuración (lista Store/CodReg) con defaults; B) Fijos (Store=1, CodReg=1) | ✅ **Resuelta 2026-08-19 (A, decisión del usuario):** configurables en Configuración con defaults (Store=1 "General", CodReg=1); el dueño los ajusta una vez según su BDP |
| **D6** | Alcance del inventario (`UpdateMassiveInventory`) | A) UI completa de inventario (conteo físico de artículos); B) Mínima (endpoint + listado simple con ajuste por lote) | ✅ **Resuelta 2026-08-19 (A, decisión del usuario):** UI completa — conteo físico con filtros, listado de unidades esperadas/contadas, diferencias y envío por lotes a BDP |
| **D7** | Código de departamento **y familia/subfamilia** local (BDP pide int 1–999 / 1–3 dígitos) | A) Asignación secuencial automática local; B) Campo código en el formulario | ✅ **Resuelta 2026-08-19 (A, decisión del usuario):** asignación secuencial automática sin fricción; el código se guarda y se usa en el push (ampliada a familias por M14) |
| **D8** | Propina (`AddOrderTip`): ¿`AddTip=true` (sumar) o false (sustituir)? | A) Configurable por venta; B) Fijo sumar | ✅ **Resuelta 2026-08-19 (A, decisión del usuario):** configurable por venta (sumar/sustituir), default sumar |
| **D9** | Fidelización (`AddPoints`): regla de puntos y módulo | A) Implementar con gating por módulo (si no hay módulo → `pendiente_suscripcion`); B) Posponer hasta confirmar módulo con el cliente | ✅ **Resuelta 2026-08-19 (A, decisión del usuario):** implementar con gating por módulo — UI de puntos en ficha de cliente (saldo + sumar/restar con motivo), suma automática por venta configurable; si no hay módulo → `pendiente_suscripcion` con reintento solo manual (D2) |
| **D10** | `CallWaiter` (llamar camarero en el plano): ¿incluir? | A) Incluir — botón por mesa, push directo, esfuerzo S; B) Excluir (la operación es del TPV BDP, Glory no la necesita) | ✅ **Resuelta 2026-08-19 (A, decisión del usuario):** incluir — botón "Llamar camarero" por mesa en el plano, push directo; en `standalone` se oculta |

---

## 7. Análisis profundo — conflictos anticipados y mitigaciones (M1–M26)

| # | Conflicto / riesgo anticipado | Impacto | Mitigación |
| --- | --- | --- | --- |
| **M1** | **Códigos de artículo**: Glory TEXT vs BDP `ArtCode` int ≤13 dígitos | Push con código inválido → rechazo de BDP | Reglas de mapeo de §4.1 (D3): `articulo_bdp_codigo` existente → modify; si no → `AutomaticCode` y guardar el código devuelto |
| **M2** | **`local_dirty` limpiado antes de confirmar** | Cambio local "sincronizado" que BDP nunca recibió → divergencia silenciosa | Solo se limpia `local_dirty` con respuesta OK del endpoint; con error se conserva + estado en cola |
| **M3** | **Idempotencia/duplicados** en crear/modificar (reintento tras timeout) | Artículos duplicados en BDP | `AutomaticCode` + cola con payload fijo; antes de reintentar crear, consultar `GetArticle`/`GetPOSArticlesList` por coincidencia; auditoría con idempotency key |
| **M4** | **Módulo por dominio**: almacén/fidelización/pagos pueden estar sin activar | Falla global del push si se trata como un único estado | Estado por dominio en la cola (`pendiente_suscripcion` por área); `GetApplicationVersion` decide el estado; nunca degrada el modo por un módulo |
| **M5** | **Talla/color**: BDP pide `sD1..sD3`; Glory no modela talla/color | Push rechazado si se omiten | Enviar vacíos (el manual permite vacío) y documentarlo; si el cliente usa talla/color en BDP → tarea aparte |
| **M6** | **Fechas**: el manual muestra `DateReg` con espacios ("2020- 10 - 29") | Parseo/ISO incorrecto | Normalizar siempre ISO-8601 válido (serde `datetime`); test en el simulador con el formato real |
| **M7** | **Concurrencia**: 2 TPV editan el mismo artículo/stock a la vez | Last-write-wins o pérdida de ajuste | Cola por entidad con transacción; `updated_at` local y conflicto → reporte (misma línea que M6 de 128A-1) |
| **M8** | **`AddOrderTip` con pagos**: total con propina < pagos existentes | BDP rechaza | Validación local previa (total+propina ≥ pagos) + manejo del ErrorMessage de BDP como mensaje claro |
| **M9** | **Perfiles inexistentes** en `Create...AndUpdateProfiles` | Error de BDP opaco | Preflight con `GetProfilesList*` al configurar; error mapeado a mensaje en español |
| **M10** | **Cola creciendo sin BDP** (standalone prolongado) | Acumulación de pendientes sin fin | Límite de antigüedad + resumen/limpieza (descartar con auditoría); nunca bloquear operaciones locales |
| **M11** | **`CreateArticlesAndUpdateProfiles` no devuelve el `ArtCode` asignado** (verificado en el manual: respuesta solo `ErrorMessage` + `ListaErroresArticulo`) | `AutomaticCode` crea el artículo en BDP y Glory no sabe su código → divergencia silenciosa | Descartar `AutomaticCode`; asignar código explícito de rango reservado (D3) y guardarlo en `articulo_bdp_codigo` tras éxito |
| **M12** | **`DeptCode` obligatorio al crear artículo**: el departamento debe existir en BDP antes | Push de artículo rechazado si el departamento es local | **Orden por dependencias en la cola**: `departamento`/`familia` antes que `articulo` (topológico por dominio); si el departamento no existe en BDP, encolar `CreateDepartment` primero (M14 asigna su código) |
| **M13** | **Mapeo IVA**: local `iva_pct` (%) vs BDP `TAVCode` (int) + `TAVPer`; `ExportArticles` no trae `TAVCode` de venta | Push de artículo con IVA incorrecto o rechazado | Config `bdp_tav_map` (pct→code) editable + auto-aprendida con `GetArticle`; defaults orientativos (10→1, 21→2) |
| **M14** | **Códigos de familia/subfamilia**: BDP pide `Code` int 1–3 dígitos; el local es texto libre | Colisiones o rechazo | Asignación secuencial automática (D7 ampliada) + tabla de mapeo nombre→código; pre-check por descripción |
| **M15** | **`CreateDepartmentAndupdateProfiles` sin lista de perfiles** | Error si se envía `ProfileList` vacía o inexistente | Usar `AllProfiles=true` (manual lo admite); evitar `GetProfilesListCreateDepartmentList` salvo que F0 muestre lo contrario |
| **M16** | **`AddOrderTip`/`CancelOrder` necesitan identificadores BDP** (`bdp_order_id`/Room/Table) | Push imposible para ventas nunca enviadas a BDP | Pre-condición: solo encolar si la venta tiene identificador BDP; si no → estado `pendiente` con aviso "comanda no sincronizada"; no es error |
| **M17** | **Simulador sin suscripción por módulo** | No se puede probar el estado `pendiente_suscripcion` localmente | Extender `__simulator/fault` para devolver "Subscripción no activada" por ruta (por módulo) y tests de ambos caminos |
| **M18** | **Migraciones**: nuevas tablas/columnas rotas o alteraciones de tablas existentes | Rollback/entropía | `bdp_push_pendientes` y `push_modalidad`/`bdp_tav_map` como migraciones **aditivas** (patrón 128A-1 M15); nunca alterar tablas existentes; inmutabilidad de migraciones aplicadas (prevención existente) |
| **M19** | **Concurrencia de push** (2 TPV empujan la misma entidad; worker + manual a la vez) | Filas duplicadas en la cola, doble escritura a BDP | UNIQUE parcial `(user_id, dominio, entidad_id, operacion)` + upsert (una fila por entidad+op) + `SELECT FOR UPDATE` al procesar + idempotencia por payload fijo (M3) |
| **M20** | **Colisión del rango reservado de códigos** (dos artículos locales reciben el mismo código; el código ya existe en BDP) | Push rechazado o artículo equivocado en BDP | Pre-check con `GetArticle`/`GetPOSArticlesList` antes de enviar; contador de rango persistido por usuario con transacción (evita asignar dos veces el mismo); si BDP rechaza por código existente → nuevo intento con el siguiente libre |
| **M21** | **Límites de recurso del push**: payload grande, reintentos sin tope, cola indefinida | CPU/red/BD degradadas | Tope de reintentos (configurable, default 5) con backoff **solo para errores transitorios**; `pendiente_suscripcion` no consume reintentos (D2); payload cap por operación (rechazar con error claro si el artículo excede campos BDP); límite de antigüedad + limpieza de `sincronizado`/`descartado` |
| **M22** | **`AutomaticCode` sin retorno de código** (verificado en el manual) | Divergencia silenciosa | Ya mitigado en M11/D3: código explícito de rango reservado; `AutomaticCode` queda descartado |
| **M23** | **`GetApplicationVersion` exige código `Application` por módulo** (ej. `"Application": 84`) — no existe un check genérico de suscripción | Auto-detección de "módulo ahora activo" no fiable | No se usa para auto-reintentar (D2=B); queda como diagnóstico manual con config opcional `bdp_modulos_map` (módulo→Application code) cuando el cliente/proveedor la facilite |
| **M24** | **Resultado parcial en `UpdateMassiveStock`/`UpdateMassiveInventory`**: BDP devuelve `ErrorList` por artículo (unos OK, otros fallan) | Cola marcada toda `sincronizado` o toda `error` incorrectamente | Procesar `ErrorList`: los artículos sin error → `sincronizado` (limpiar `local_dirty`); los que fallan → sub-estado `error` con el mensaje del artículo, sin bloquear el resto |
| **M25** | **Requisito `WebArticle`/`Inventariable`**: el manual exige artículo "tipo Web" (y "Inventariable" para inventario) o rechaza con "EL ARTÍCULO 1003 NO ES DEL TIPO WEB" | Stock/inventario rechazado por artículos locales creados sin esos flags | Al crear artículo local (`CreateArticlesAndUpdateProfiles`) fijar `WebArticle=true`; para inventario fijar `Is_Inventoriable=true` y validar en UI; documentar que el stock push aplica solo a artículos Web |
| **M26** | **`OrderIdentifier` para `AddOrderTip`/`CancelOrder`**: el local solo guarda `bdp_order_id` (no Room/Table/Market) | No se puede construir el identificador si se asume Room/Table | Usar `OrderIdentifier { OrderId: bdp_order_id }` (el manual lo admite); si no hay `bdp_order_id` → estado `pendiente` con aviso "comanda no sincronizada" (M16) |

---

## 8. Evidencia y gate

- **Por fase:** `cargo test --lib bdp` + tests de integración nuevos + simulador Python + wiremock para
  suscripción/errores + `tsc`/build frontend.
- **Gate canónico:** `task:check` del proyecto (wrapper Sentinel) con reporte reproducible en
  `.quality-reports/` y registro en `Agente/completados/`.
- **Verificación real (diferida):** plan de verificación contra BDP real cuando esté online y con
  módulos activos — mismas condiciones que el bloque 138A-2 (lecturas) y 048A-11 (escrituras). No es
  pre-requisito de entrega; queda registrada como pendiente no bloqueante.

---

## Anexo A — Inventario completo del bloque (verificado 2026-08-19)

| Área | Función | Ruta | Tipo | Estado final previsto |
| --- | --- | --- | --- | --- |
| Servicios | `GetApplicationVersion` | `/Service/GetApplicationVersion` | Lectura | 🆕 Soporte suscripción |
| Artículos | `CreateArticlesAndUpdateProfiles` | `/API/Articles/CreateAndUpdateProfiles` | Escritura | 🆕 F3 |
| Artículos | `ModifyArticleAndUpdateProfile` | `/API/Articles/ModifyAndUpdateProfiles` | Escritura | 🆕 F3 |
| Artículos | `ModifyPricesArticles` | `/API/Articles/ModifyPrices` | Escritura | 🆕 F3 |
| Perfiles | `GetProfilesListCreateArticleList` | `/API/ProfilesLists/GetCreateArticleList` | Lectura | 🆕 F2 |
| Perfiles | `GetProfileListModifyArticleList` | `/API/ProfilesLists/GetModifyArticleList` | Lectura | 🆕 F2 |
| Perfiles | `GetProfilesListCreateDepartmentList` | `/API/ProfilesLists/GetCreateDepartmentList` | Lectura | 🆕 F2 |
| Departamentos | `CreateDepartment` | `/API/Departments/Create` | Escritura | 🆕 F5 |
| Departamentos | `CreateDepartmentAndupdateProfiles` | `/API/Departments/CreateAndUpdateProfiles` | Escritura | 🆕 F5 |
| Comandas | `AddOrderTip` | `/API/Orders/Tip/Add` | Escritura | 🆕 F6 |
| Comandas | `CancelOrder` | `/API/Orders/Cancel` | Escritura | ✅ Cliente listo; push F6 (pendiente suscripción) |
| Salones | `CallWaiter` | `/API/Waiters/Call` | Escritura | 🆕 F6 (plano de sala) |
| Fidelización | `GetPoints` | `/API/Loyalty/GetPoints` | Lectura | 🆕 F2/F7 |
| Fidelización | `AddPoints` | `/API/Loyalty/AddPoints` | Escritura | 🆕 F7 |
| Stock | `CreateFamily` | `/API/Warehouse/CreateFamily` | Escritura | 🆕 F4 |
| Stock | `CreateSubfamily` | `/API/Warehouse/CreateSubfamily` | Escritura | 🆕 F4 |
| Stock | `Regularizations` | `/API/Warehouse/Regularizations` | Escritura | 🆕 F4 |
| Stock | `Transfers` | `/API/Warehouse/Transfers` | Escritura | 🆕 F4 |
| Stock | `UpdateMassiveStock` | `/API/Warehouse/UpdateMassiveStock` | Escritura | 🆕 F4 |
| Stock | `UpdateStock` | `/API/Warehouse/UpdateStock` | Escritura | 🆕 F4 |
| Stock | `UpdateMassiveInventory` | `/API/Warehouse/UpdateMassiveInventory` | Escritura | 🆕 F4 |

**Totales:** **16 escrituras BDP cubiertas** (15 nuevas —14 de push sobre datos locales + `CallWaiter`—
y `CancelOrder` pendiente de suscripción) + 5 lecturas de soporte + la infraestructura de push. **20
funciones nuevas en el catálogo** sobre las 32 actuales → **52 en total** con este bloque.

---

## Anexo B — Revisión (supervisor-thinking / supervisor-review)

**Revisión 1 (2026-08-19, creación):**
- *¿Es la mejor opción arquitectónica?* Sí: reutiliza la base de 128A-1 (origen/local_dirty/guards) en
  vez de crear un camino paralelo; la cola de push es el patrón estándar para unidireccional con
  idempotencia y reintentos.
- *¿Riesgo de romper independencia?* No: invariante 3.1 — el push solo corre en modo bdp efectivo; la
  cola acumula en standalone. Las escrituras síncronas explícitas del usuario pasan los mismos guards.
- *¿Complejidad justificada?* Sí: las escrituras comparten un solo servicio de push (SRP/OCP); sin él
  serían flujos duplicados por dominio.

**Revisión 3 (2026-08-19, segunda pasada pedida por el usuario — antes de comenzar):**
- **D2 resuelta como B (manual para suscripción):** la suscripción no es un fallo transitorio; puede no
  activarse nunca → `pendiente_suscripcion` con reintento **solo manual**; el auto-reintento queda
  reservado a errores transitorios (timeout/red) con tope y backoff.
- **Verificado que `GetApplicationVersion` exige código `Application` por módulo** (M23) → no sirve para
  auto-detección; queda como diagnóstico manual.
- **Verificado `ErrorList` por artículo** en `UpdateMassiveStock`/`UpdateMassiveInventory` (M24): éxito
  parcial, y requisitos `WebArticle`/`Inventariable` (M25) que hay que fijar al crear artículos.
- **Verificado `CreateSubfamily` sin código de familia padre** (M14 simplificada) y **ventas solo guardan
  `bdp_order_id`** (M26) → usar `OrderIdentifier { OrderId }` para tip/cancel.

**Revisión 2 (2026-08-19, pase profundo pedido por el usuario — "mitigar cualquier posible fallo"):**
- **Corregido M1/D3:** verificado en el manual que `CreateArticlesAndUpdateProfiles` **no devuelve el
  código** → descartado `AutomaticCode`; códigos explícitos de rango reservado con pre-check (M11, M22).
- **Corregida la dependencia departamento→artículo** (`DeptCode` obligatorio): orden topológico por
  dominio en la cola (M12).
- **Añadido mapeo de IVA** `bdp_tav_map` (M13) — el local solo tiene `iva_pct` y `ExportArticles` no
  expone el `TAVCode` de venta.
- **Añadidos 12 conflictos nuevos (M11–M22)**: códigos no devueltos, dependencias de dominio, IVA,
  códigos de familia, `AllProfiles`, identificadores BDP para tip/cancel, simulación de suscripción por
  módulo, migraciones aditivas, concurrencia (UNIQUE parcial + `FOR UPDATE`), colisión del rango
  reservado, límites de recurso y `AutomaticCode` descartado.
- **D1 resuelta como configurable** (`push_modalidad`, default `automatico`) — el push nunca escribe sin
  arming, coherente con el fail-closed de 128A-1.

**supervisor-review (riesgos de entrega):**
- Dependencia real: suscripciones del cliente (pagos/almacén/fidelización) — cubierta por diseño
  (estado `pendiente_suscripcion` por dominio, D2) y registrada como verificación diferida, no como
  bloqueo.
- BDP offline (Tailscale `restaurante-bdp` caído, credenciales ausentes en `.env`) — no bloquea: toda la
  verificación de F3–F8 es local (simulador + wiremock), mismo enfoque que 128A-1 F9.
- Riesgo de alcance (inventario masivo, fidelización) — acotado por D6/D9 con opción mínima recomendada.
- Evidencia: gate `task:check` + suites locales por fase, reportes reproducibles.

**Veredicto:** `VIABLE` — revisiones 2 y 3 incorporadas (M11–M26), **D1–D10 resueltas** (2026-08-19).
Sin decisiones pendientes. Autorizado: ciclo local completo (editar → probar → gate → commit), empezando
por F0 (auditoría del estado real).
