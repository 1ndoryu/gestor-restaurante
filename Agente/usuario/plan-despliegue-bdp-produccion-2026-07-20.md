# Plan de despliegue BDP a producción — glory-rest (Wandori)

> **Fecha:** 2026-07-20
> **Estado:** Planificado (sin ejecutar)
> **Destino:** `https://restaurante.wandori.us` — VPS 66.94.100.241, stack `b8s0cks444o0sogo8kg8wcgw`

---

## 1. Estado actual

El contenedor de producción **no tiene ninguna variable BDP**. La integración existe en el código y las migraciones ya se aplicaron, pero el bootstrap BDP nunca corrió porque falta `BDP_BOOTSTRAP_USER_EMAIL`.

**ENVs actuales del contenedor** (verificadas con `exec`):

| Variable | Valor en producción |
|---|---|
| `DATABASE_URL` | `postgres://rust_app:***@postgres-b8s0cks444o0sogo8kg8wcgw:5432/rust_db` |
| `JWT_SECRET` | Presente (generado por Coolify) |
| `RUST_LOG` | `info` |
| `STATIC_DIR` | `/app/dist` |
| `HOST` | `0.0.0.0` |
| `SERVICE_FQDN_APP` | `restaurante.wandori.us` |
| **Cualquier `BDP_*`** | **No existe** |

---

## 2. Variables BDP confirmadas

Los valores del `.env` local son los **mismos** que se usarán en producción. No hay un BDP separado para Wandori — es la misma instancia BDP, el mismo POS 31, las mismas credenciales.

| Variable | Valor confirmado |
|---|---|
| `BDP_BASE_URL` | Ver `.env` (Tailscale, accesible desde el VPS) |
| `BDP_POS_ID` | `31` — CENTRAL 2026 (Series `00031TI`, IVA incluido) |
| `BDP_LOGIN` | Ver `.env` |
| `BDP_PASSWORD` | Ver `.env` |
| `BDP_INTEGRATOR_CODE` | Ver `.env` |
| `BDP_EMPLOYEE_ID` | `1` |
| `BDP_ITEMS_PROFILE_ID` | `1` |
| `BDP_DEFAULT_ARTICLE_CODE` | `1001` |
| `BDP_DEFAULT_ARTICLE_NAME` | `CAFE BOMBON` |

Estos valores ya están validados localmente contra el BDP real (111+ tests, preflight, Category C).

---

## 3. Estado de datos

Todo está confirmado. El email del usuario en la app es `$BDP_BOOTSTRAP_USER_EMAIL` (ya existe en la tabla `users`).

### 3.1 Mapeos (opcionales, configurables después desde la app)

| Variable ENV | Qué es | Valor por defecto |
|---|---|---|
| `BDP_TENDER_MAP_JSON` | Métodos de pago de la app → códigos BDP (ej: `{"efectivo":"1","tarjeta":"2"}`) | `{}` = usa default BDP |
| `BDP_ORDER_TYPE_MAP_JSON` | Canales de venta → tipos de pedido BDP (ej: `{"comedor":"0","barra":"0"}`) | `{}` = tipo 0 por defecto |
| `BDP_DEFAULT_CUSTOMER_CODE` | Cliente genérico BDP para ventas sin cliente asociado | Vacío (opcional) |

Se pueden configurar después desde la interfaz o como ENV. No bloquean el deploy.

---

## 4. Qué hace el bootstrap al arrancar

El `BdpConfigBootstrapService` corre **una sola vez** al iniciar el contenedor, solo si `BDP_BOOTSTRAP_USER_EMAIL` está definida:

1. Busca el usuario con ese email en la tabla `users`. Si no existe → error, no configura nada.
2. Carga los valores BDP en `configuracion_restaurante` para ese usuario.
3. **No sobreescribe** valores que ya existían (es idempotente).
4. Deja todo en modo seguro:
   - `bdp_sync_enabled = FALSE`
   - `bdp_poll_enabled = FALSE`
   - `bdp_auto_sync_customers = FALSE`
   - `bdp_sync_mode = 'read_only'`
   - Elimina cualquier `bdp_write_arming` previo.
5. Registra en `bdp_audit_log` que se aplicó el bootstrap.
6. Se marca como aplicado para no repetirse.

**Después del bootstrap, las envs BDP ya no son necesarias** (la config vive en la BD). Solo se necesitan de nuevo si se re-deploya con una BD limpia.

---

## 5. Secuencia de despliegue

### Paso 1 — Añadir envs BDP al contenedor

Usar `sync-env` o la API de Coolify para añadir las variables al servicio. Las envs:

Los valores se leen del `.env` del proyecto. Ver sección 2 para la lista de variables necesarias.

**NO configurar todavía:**
- `BDP_WRITE_ALLOWED_ORIGINS` (vacío = solo lectura, protege contra escrituras accidentales)
- `BDP_CHECK_ORDER_ALLOWED_ORIGINS` (vacío = no permite ni consultar órdenes)

### Paso 3 — Deploy

```powershell
$cm = "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs\target\release\coolify-manager.exe"
& $cm deploy --name glory-rest --update --skip-backup
```

Las 15 migraciones BDP se aplican automáticamente al arrancar el binario.

### Paso 4 — Verificar

```powershell
# Health check
& $cm health --name glory-rest

# Logs del bootstrap
& $cm logs --name glory-rest
# Buscar: "Bootstrap BDP dirigido" + "aplicado correctamente"

# Verificar modo seguro
& $cm exec --name glory-rest --target app --command "psql \$DATABASE_URL -c 'SELECT bdp_sync_enabled, bdp_poll_enabled, bdp_sync_mode FROM configuracion_restaurante LIMIT 1'"
```

Resultado esperado: `bdp_sync_enabled = false`, `bdp_poll_enabled = false`, `bdp_sync_mode = read_only`.

### Paso 5 — Smoke test de conectividad

Desde la app (interfaz web), ir a la sección BDP y verificar que la configuración técnica aparece cargada. Si el preflight muestra que el BDP responde, la integración está lista para las pruebas del cliente.

### Paso 6 — Activar allowlists (solo después de pruebas del cliente)

Una vez que el cliente haya verificado las 4 pruebas de escritura de la guía, añadir:

```
BDP_WRITE_ALLOWED_ORIGINS=<ip:puerto-del-bdp>
BDP_CHECK_ORDER_ALLOWED_ORIGINS=<ip:puerto-del-bdp>
```

Y habilitar desde la interfaz:
- `bdp_sync_enabled = TRUE`
- `bdp_poll_enabled = TRUE` (si se quiere polling automático)

---

## 6. Riesgos y mitigaciones

| Riesgo | Probabilidad | Mitigación |
|---|---|---|
| BDP no accesible desde VPS | Baja (ya confirmado) | Verificar con `exec --command "curl -s http://IP:8068/api/ServiceHealth"` antes del deploy |
| Email de usuario incorrecto | Media | El bootstrap falla sin daño si el email no existe en `users`. Corregir ENV y redeploy. |
| POS ID incorrecto | Media | El preflight detecta si el POS no existe. Corregir env y redeploy. |
| Migraciones fallan | Baja | El contenedor no arranca (fail-fast). Revisar logs, corregir, redeploy. |
| Bootstrap sobreescribe config existente | Ninguna | El bootstrap no sobreescribe valores ya confirmados. |
| Escritura accidental en BDP | Ninguna | `BDP_WRITE_ALLOWED_ORIGINS` vacío = todas las escrituras bloqueadas. |

---

## 7. Checklist pre-despliegue

- [x] IP/puerto del BDP confirmado (ver `.env`)
- [x] Credenciales WebLink confirmadas (ver `.env`)
- [x] POS confirmado: 31 (CENTRAL 2026)
- [x] Empleado ID: 1 — Perfil artículos: 1
- [x] Artículo por defecto: 1001 (CAFE BOMBON)
- [x] Email del usuario confirmado: `$BDP_BOOTSTRAP_USER_EMAIL`
- [ ] Verificar conectividad VPS → BDP (curl desde el contenedor)
- [ ] Añadir envs BDP al contenedor
- [ ] Deploy + health check
- [ ] Verificar bootstrap en logs
- [ ] Verificar modo seguro en BD
- [ ] Smoke test de conectividad BDP desde la app

---

## 8. Nota sobre la guía del cliente

El documento `guia-cliente-pruebas-integracion-bdp-2026-07-18.md` ya describe correctamente el flujo de pruebas. No requiere cambios técnicos — el POS 31 y toda la config BDP son los mismos en local y en producción.

**Estado:** todos los datos confirmados. Se puede desplegar cuando se decida.
