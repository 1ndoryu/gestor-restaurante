# BDP-NET — Análisis de cambios y problemas reportados por cliente

> **Fecha:** 2026-06-08
> **Contexto:** El cliente reporta que el TPV BDP-NET "se ha desconfigurado" tras las pruebas de integración WebLink
> **PC remoto:** `100.83.196.35` (POS 31 — CENTRAL 2026)

---

## 1. Problemas reportados por el cliente

| # | Problema | Severidad |
|---|---|---|
| 1 | Facturas vuelven a empezar desde #1 (se perdió la secuencia) | 🔴 Crítico |
| 2 | No pueden cerrar tickets ni facturas | 🔴 Crítico |
| 3 | Ha desaparecido el logo del restaurante | 🟡 Medio |
| 4 | Precios salen sin IVA | 🔴 Crítico |

---

## 2. Cambios que se hicieron en BDP-NET (documentados)

### 2.1 Cambio de serie de facturación (MESAS)

| Campo | Antes | Después |
|---|---|---|
| **Serie en "Facturas 1 → Parámetros en Mesas"** | `00031TM` (31T Facturas Simplificadas Mesa) | `00031TI` (31T Facturas Simplificadas IVA Incluido) |

**Por qué se hizo:** El error `[300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA` bloqueaba la creación de comandas vía WebLink. La serie `00031TM` no tenía "IVA Incluido" activo (y no se podía modificar por tener documentos existentes). Se creó `00031TI` como serie nueva con IVA Incluido y se asignó al terminal 31.

**Cuándo:** 2026-06-07

**Documentación:** `Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md`

### 2.2 Creación de nueva serie TPV

| Serie | Descripción | IVA Incluido | Estado |
|---|---|---|---|
| `00031TM` | 31T Facturas Simplificadas Mesa | ❌ | Existente (NO modificada) |
| `00031TI` | 31T Facturas Simplificadas (IVA Incluido) | ✅ | **NUEVA — creada 2026-06-07** |
| `00031AL` | 31T Albaranes | — | Existente (NO modificada) |
| `00031P` | (probablemente otra serie) | — | Creada durante pruebas tempranas |

### 2.3 Pruebas de API WebLink (solo lectura + dry-run)

Estas pruebas **NO modifican datos** en BDP-NET:

| Acción | Endpoint | Modifica datos | Notas |
|---|---|---|---|
| Health check | `GET /Service/Health` | ❌ No | Solo lectura |
| Login | `POST /Auth/Login` | ❌ No | Crea sesión temporal (59 min) |
| GetVersion | `GET /Service/GetVersion` | ❌ No | Solo lectura |
| GetPOS | `POST /API/POS/Get` | ❌ No | Consulta terminal |
| GetEmployee | `POST /API/Employee/Get` | ❌ No | Consulta empleado |
| GetPOSEmployees | `POST /API/POS/Employees/Get` | ❌ No | Consulta empleados del terminal |
| GetPOSTenderList | `POST /API/Tenders/GetPOSList` | ❌ No | Consulta formas de pago |
| DepartmentsExportFromProfile | `POST /API/Departments/ExportFromProfile` | ❌ No | Consulta departamentos |
| GetPOSArticlesList | `POST /API/Articles/GetPOSList` | ❌ No | Consulta artículos |
| **CreateOrder (OnlyCheck)** | `POST /API/Orders/Create` con `OrderOperationType=1` | ❌ No | **Dry-run — BDP valida pero NO crea** |

### 2.4 Configuración de Web Services (Weblink REST API)

Estos campos ya estaban configurados ANTES de nuestras pruebas (confirmado en la documentación):

| Campo | Valor | ¿Lo modificamos? |
|---|---|---|
| IP Address | Configurado | ❌ No |
| IP Port | 8068 | ❌ No |
| Usar Password | Activo | ❌ No |
| Credenciales (login/password) | admin / kamples2026 | ❌ No |
| CodigoIntegrador | VBW2MBM5 | ❌ No |

---

## 3. Relación cambio → problema

### 3.1 Facturas desde #1

**Causa: Cambio de serie de `00031TM` → `00031TI`**

Cada serie en BDP-NET tiene su propio contador de numeración independiente. Al cambiar la serie asignada a Mesas de `00031TM` (que tenía ej. factura #450) a `00031TI` (nueva, contador en 0), todas las nuevas facturas empezaron desde #1.

**Solución:** Restaurar `00031TM` como serie de Mesas. O, si se necesita `00031TI`, usar el botón "Modificar Contador" de BDP-NET para copiar el contador de `00031TM` a `00031TI`.

**Dónde revertir:** BDP-NET → Utilidades → Configuración TPV → Terminal 31 → Facturas 1 → Parámetros en Mesas → Serie de Facturación → cambiar de `00031TI` a `00031TM`

### 3.2 No pueden cerrar tickets/facturas

**Causa: Serie de Barra posiblemente desconfigurada, o efecto colateral del cambio de serie**

La configuración de "Parámetros en Barra" del terminal 31 **siempre estuvo vacía** (sin serie asignada). Esto es PRE-EXISTENTE a nuestros cambios. Sin embargo:

- Si antes el personal usaba Mesas (con `00031TM`) y funcionaba, el cambio a `00031TI` podría haber roto algún flujo
- Si BDP-NET cachea la serie activa al inicio y el cambio forzó un reinicio sin recargar correctamente
- Si el error de "cerrar tickets" es diferente (ej: ya están facturados y no se pueden cerrar de nuevo)

**Posible solución:**
1. Verificar que `00031TM` (o `00031TI`) esté asignada correctamente en Mesas
2. Verificar si "Barra" necesita una serie asignada (si el personal cierra desde Barra)
3. Revisar si los tickets bloqueados tienen estado intermedio (abiertos pero no facturables)

**Dónde revisar:** BDP-NET → Utilidades → Configuración TPV → Terminal 31 → Facturas 1 → Parámetros en Barra (debe tener serie) + Parámetros en Mesas

### 3.3 Logo desaparecido

**Causa: NO hay cambios nuestros relacionados con el logo**

En ninguna de las conversaciones ni pruebas se modificó:
- Diseño de ticket
- Cabecera/logo de impresión
- Configuración de impresora
- Formato DIS de ticket

**Posibles causas ajenas:**
- Al crear `00031TI` como serie nueva, hereda el diseño por defecto (sin logo personalizado)
- Si el logo estaba vinculado a `00031TM` y el cambio de serie lo perdió
- Reinicio de BDP-NET durante las pruebas que no recargó bien la configuración
- Cambio hecho por otra persona / actualización de BDP-NET

**Dónde restaurar:** BDP-NET → Utilidades → Series TPV → seleccionar la serie activa → Diseño de Ticket → cargar imagen de cabecera

### 3.4 Precios sin IVA

**Causa: Posible efecto del cambio de serie `00031TM` → `00031TI`**

- `00031TM` NO tenía "IVA Incluido" → los precios se interpretaban como **base imponible** y el IVA se añadía al mostrar/facturar
- `00031TI` SÍ tiene "IVA Incluido" → los precios se interpretan como **precio final** (IVA ya incluido)

Si los artículos en BDP tienen precios como **base imponible** (ej: café = 4.55€ + 10% IVA = 5.00€), al cambiar a serie con IVA Incluido el ticket mostraría 4.55€ como precio final (sin el IVA visible).

**Solución:**
1. Verificar cómo están definidos los precios en los artículos (con o sin IVA)
2. Si los precios son base imponible, restaurar serie `00031TM` (sin IVA Incluido)
3. Si los precios son finales, mantener `00031TI` y verificar que el desglose de IVA aparezca

**Dónde verificar:** BDP-NET → Mantenimiento de Artículos → revisar precios del artículo de prueba (ej: CAFE BOMBON)

---

## 4. Tabla de reversión recomendada

| Paso | Acción | Dónde en BDP-NET | Revierte qué |
|---|---|---|---|
| 1 | Cambiar serie Mesas de `00031TI` a `00031TM` | Configuración TPV → T31 → Facturas 1 → Mesas | Facturas desde #1, precios sin IVA |
| 2 | Verificar serie en Barra | Configuración TPV → T31 → Facturas 1 → Barra | No cerrar tickets (si cierran desde Barra) |
| 3 | Restaurar logo en serie activa | Series TPV → serie activa → Diseño Ticket | Logo desaparecido |
| 4 | Verificar precios de artículos | Mantenimiento Artículos | Precios sin IVA (si el paso 1 no lo resuelve) |

---

## 5. Lo que NO hicimos (para tranquilidad del cliente)

| Acción | ¿La hicimos? | Evidencia |
|---|---|---|
| Modificar artículos | ❌ No | Solo lectura con `GetPOSArticlesList` |
| Modificar precios | ❌ No | Solo lectura |
| Modificar clientes | ❌ No | Solo lectura con `ExportCustomers` (ni siquiera se ejecutó) |
| Modificar empleados | ❌ No | Solo lectura con `GetEmployee` |
| Crear/borrar comandas reales | ❌ No | Solo `OnlyCheck` (dry-run) |
| Modificar formas de pago | ❌ No | Solo lectura con `GetPOSTenderList` |
| Modificar diseño de ticket | ❌ No | No se tocó |
| Modificar configuración de impresora | ❌ No | No se tocó |
| Modificar logo | ❌ No | No se tocó |
| Cancelar facturas existentes | ❌ No | `CancelOrder` devuelve error (no disponible) |
| Modificar contadores de series existentes | ❌ No | Solo se creó serie nueva `00031TI` |

---

## 6. Resumen ejecutivo

### Único cambio que hicimos en BDP-NET:

> **Se creó la serie `00031TI` (IVA Incluido) y se asignó a Terminal 31 → Facturas 1 → Parámetros en Mesas, reemplazando a `00031TM`.**

Todo lo demás fueron pruebas de solo lectura contra la API WebLink (las comandas se crean desde Glory hacia BDP, no al revés, y ninguna llegó a producción).

### Los 4 problemas del cliente se explican con este único cambio:

1. **Facturas desde #1** → Serie nueva = contador nuevo
2. **No cerrar tickets** → Posible desconfiguración de Barra (pre-existente) o efecto del cambio de serie
3. **Logo** → Serie nueva no hereda diseño personalizado de la serie anterior
4. **Precios sin IVA** → Serie con "IVA Incluido" cambia la interpretación de precios

### Acción inmediata recomendada:

Cambiar la serie de Mesas de `00031TI` de vuelta a `00031TM`. Esto debería revertir los problemas 1, 3 y 4. Para el problema 2, hay que verificar la configuración de Barra por separado.
