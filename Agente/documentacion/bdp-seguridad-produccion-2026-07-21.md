# Seguridad BDP en Producción — Auditoría Pre-Deploy

> **Fecha:** 2026-07-21
> **Contexto:** Antes de desplegar la integración BDP a producción, se audita si el código puede hacer cambios automáticos en el BDP.
> **Respuesta corta:** NO. Es seguro desplegar. Nada se escribe en BDP sin configuración explícita + confirmación manual.

---

## 1. Qué pasa al desplegar (secuencia de arranque)

### 1.1 Bootstrap (`BdpConfigBootstrapService`)

Se ejecuta **una sola vez** al iniciar el contenedor, solo si `BDP_BOOTSTRAP_USER_EMAIL` está definida.

**Qué hace:**
- Busca el usuario por email en la tabla `users` (query local a PostgreSQL)
- Inserta/actualiza fila en `configuracion_restaurante` con valores del `.env`
- Registra en `bdp_audit_log` que se aplicó el bootstrap

**Qué NO hace:**
- ❌ NO hace ninguna llamada HTTP al BDP
- ❌ NO intenta conectar, loguear ni verificar el BDP
- ❌ NO envía datos, clientes, ventas ni nada al BDP

**Configuración que deja (hardcoded en el código):**
| Campo | Valor | Significado |
|---|---|---|
| `bdp_sync_enabled` | `false` | Sync automático DESACTIVADO |
| `bdp_poll_enabled` | `false` | Polling automático DESACTIVADO |
| `bdp_auto_sync_customers` | `false` | Sync clientes DESACTIVADO |
| `bdp_sync_mode` | `read_only` | SOLO LECTURA — escrituras bloqueadas |
| `bdp_write_arming` | ELIMINADO | Sin arming activo = sin posibilidad de escritura |

**Veredicto:** ✅ 100% seguro. Solo toca la BD local.

### 1.2 Migraciones SQL

Se ejecutan automáticamente al arrancar el binario (SQLx `run_pending_migrations`).

**Qué hacen:** CREATE TABLE, ALTER TABLE, CREATE INDEX — puramente DDL en PostgreSQL local.

**Veredicto:** ✅ 100% seguro. No toca red externa.

### 1.3 Background tasks

Dos procesos se lanzan en `tokio::spawn` al arrancar:

| Task | Condición de activación | Estado post-bootstrap |
|---|---|---|
| `BdpOrderPollerService::poll_due()` | `bdp_poll_enabled=true AND bdp_sync_enabled=true` | **NO se activa** (ambos=false) |
| `BdpSyncService::spawn_bdp_sync()` | `bdp_sync_enabled=true AND bdp_sync_mode=unidirectional` | **NO se activa** (enabled=false, mode=read_only) |

**Veredicto:** ✅ Los background tasks están inactivos por defecto.

---

## 2. Capas de protección contra escritura accidental

La integración tiene **5 capas** de seguridad. TODAS deben pasar para que una escritura llegue al BDP:

### Capa 1 — Variables de entorno (`BDP_WRITE_ALLOWED_ORIGINS`)

- Si está vacía o no definida → **TODAS las escrituras están bloqueadas**
- En producción esta variable **NO se va a configurar** inicialmente
- Efecto: aunque todo lo demás fallara, las escrituras no llegan al BDP

### Capa 2 — `bdp_sync_mode` = `read_only`

- Bootstrap setea `read_only` por defecto
- Para habilitar escritura: `PUT /api/configuracion/bdp/sync-mode` con `confirmar_escritura=true` + URL destino exacta
- Post-escritura: el sistema **vuelve automáticamente a `read_only`** (kill switch)

### Capa 3 — Write Arming (`bdp_write_arming`)

- Sin un arming record válido (con caducidad, alcance y fingerprint), ninguna escritura procede
- Bootstrap **elimina** cualquier arming previo
- El arming se consume tras cada escritura (single-use)

### Capa 4 — Confirmación explícita del usuario

Cada endpoint de escritura requiere un string de confirmación literal:

| Endpoint | Confirmación requerida |
|---|---|
| `POST /api/ventas/:id/bdp-invoice` | `FACTURAR {id}` |
| `POST /api/ventas/:id/bdp-payment` | `PAGAR {id} {amount:.2}` |
| `POST /api/clientes/:id/bdp-sync` | `CREAR CLIENTE {id} {code}` |

Sin el string exacto, la operación se rechaza.

### Capa 5 — Audit trail + Snapshot pre-write

- Cada escritura genera snapshot del estado BDP antes de mutar
- Registro inmutable en `bdp_audit_log`
- Permite auditoría post-mortem

---

## 3. Qué NO puede pasar tras el deploy

| Escenario | ¿Puede pasar? | Por qué no |
|---|---|---|
| Bootstrap envía datos al BDP | ❌ NO | Bootstrap solo escribe en BD local, no hace HTTP |
| Sync automático de ventas al BDP | ❌ NO | `bdp_sync_enabled=false`, `bdp_sync_mode=read_only` |
| Polling automático de órdenes BDP | ❌ NO | `bdp_poll_enabled=false` |
| Crear cliente en BDP | ❌ NO | Requiere API call + allowlist + arming + confirmación |
| Facturar/pagar en BDP | ❌ NO | Requiere API call + allowlist + arming + confirmación |
| Importar datos del BDP a la app | ❌ NO | Requiere API call explícita del usuario |
| Login automático al BDP | ❌ NO | Solo ocurre como parte de llamadas explícitas |
| Conexión automática al BDP al arrancar | ❌ NO | No hay health check ni login en startup |

---

## 4. Qué necesita hacer el usuario para HABILITAR escrituras en el futuro

Para que la integración BDP pueda escribir, el usuario debe (en este orden):

1. Configurar `BDP_WRITE_ALLOWED_ORIGINS` en envs del contenedor
2. Llamar `PUT /api/configuracion/bdp/sync-mode` con `confirmar_escritura=true`
3. Para cada operación individual: pasar por WriteGuard + confirmación explícita

**Nada de esto ocurre automáticamente.**

---

## 5. Escenario: BDP offline durante el deploy

Si la máquina BDP está apagada (como es el caso actual: `restaurante-bdp` offline en Tailscale):

- ✅ Bootstrap funciona normalmente (no necesita BDP)
- ✅ Migraciones funcionan normalmente (solo BD local)
- ✅ La app arranca y sirve requests normales
- ✅ Los background tasks están inactivos (no intentan conectar)
- ⚠️ Si alguien llama a un endpoint de lectura BDP (ej: `/api/configuracion/bdp/diagnostico`), fallará con timeout/error — pero esto es un endpoint de administración que nadie llama automáticamente
- ✅ Los endpoints de escritura están bloqueados por allowlist + sync_mode=read_only

**Veredicto:** ✅ Es seguro desplegar con BDP offline. Cuando el restaurante abra y la máquina se encienda, la integración estará lista para activarse.

---

## 6. Gaps identificados (no bloquean el deploy)

| Gap | Riesgo | Descripción | ¿Bloquea deploy? |
|---|---|---|---|
| GAP-1: `cancel_order()` sin handler | Medio | Método existe pero no está expuesto vía REST | No |
| GAP-2: Localhost bypass en allowlist | Medio-bajo | `localhost/127.0.0.1` permitido sin env var | No (BDP no es localhost) |
| GAP-3: Sin rate limiting en polling | Medio-bajo | N usuarios × polling = N llamadas/10s | No (polling desactivado) |
| GAP-4: `spawn_bdp_sync` sin timeout | Bajo | tokio::spawn sin timeout explícito | No (sync desactivada) |

---

## 7. Conclusión

**Es seguro desplegar a producción.** El código tiene una arquitectura de seguridad robusta con 5 capas de protección. El bootstrap solo toca la BD local. Todos los mecanismos de interacción con BDP están desactivados por defecto. Nada se envía al BDP sin configuración explícita + confirmación manual del usuario.
