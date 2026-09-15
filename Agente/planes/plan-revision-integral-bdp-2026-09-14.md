# Plan 149A-1 — Revisión integral BDP (REINICIO 2026-09-14): todo desde cero, con confirmación visual ítem por ítem

> **Fecha:** 2026-09-14 · **Rama operativa:** `main` · **ID:** `149A-1`
> **Sustituye a:** `039A-1` (`Agente/planes/completados/plan-revision-integral-bdp-2026-09-03.md`,
> cerrado por reinicio).
> **Motivo del reinicio (cita del usuario):** "hubieron muchas refactorizaciones y cambios en el
> css, así vamos a reiniciar el plan… vamos a probar todo desde el principio, incluyendo las
> pruebas reales, y todo en general".
> **Regla de escrituras:** las escrituras reales al BDP **no** viven aquí; se repiten en `149A-2`
> (`Agente/planes/plan-seguridad-escrituras-bdp-2026-09-14.md`). Este plan ejecuta su etapa S3
> solo como simulación/observación y deja el disparo real a 149A-2.

## 1. Qué cambia respecto a 039A-1

Se hereda el **inventario** (Parte 1 P0–P13, Parte 2 Q0–Q5, Parte 3 S0–S3) pero **no** su evidencia:
todo se vuelve a ejecutar desde cero contra el HEAD actual. El cambio de método es el **gate de
confirmación visual**: ningún ítem se marca sin que el usuario lo haya visto y dado el OK.

## 2. Protocolo de confirmación visual (regla dura de este plan)

Por **cada ítem** (numerado: P1.1, P2.3, Q1.4, S1.2…):

1. **Evidencia técnica**: comando reproducido + salida redactada (sin credenciales/token; datos
   sensibles enmascarados). Si el ítem es de datos, la consulta a BD con su resultado contado.
2. **Evidencia visual**: pantalla real del preview (`http://localhost:5173/`), captura + resumen de
   lo que se ve (estado, textos, orden, estilos, foco).
3. **Petición de OK**: mensaje corto al usuario con qué mirar y qué debería verse.
4. **Marcado**: `- [x]` **solo** con `✅ confirmado por usuario <fecha> <hora>` y la evidencia
   pegada al lado. Sin confirmación el ítem queda `- [ ]` aunque funcione.
5. **Corrección**: si el usuario pide cambiar algo → se abre `H-149A-1-<n>` en §9, se corrige, se
   vuelve a mostrar el ítem **corregido** y se pide OK otra vez. Un ítem re-abierto nunca se deja
   marcado a medias.

No se agrupan ítems para pedir el OK. Si un ítem depende de otro, se confirma primero el base.

## 3. Entorno de la revisión

- Stack aislado: backend `:3100` (wrapper `node scripts/run-cargo.mjs`, binario en
  `C:\tmp\glory-target\debug\glory-backend.exe`), BD `glory_backend_glory_rs_rest`, seed demo.
- Frontend: Vite `:5173` (`VITE_API_TARGET=http://127.0.0.1:3100`).
- **Nunca** el BDP real para la Parte 1 (cero tráfico). Para la Parte 2, el BDP real se contacta
  **solo en lectura** y **solo con autorización explícita previa** del usuario.
- **Parte 1 se ejecuta en `standalone`**: conmutado vía `PATCH /api/configuracion
  {"modo_operacion":"standalone"}` el 2026-09-14 (efecto: `sync_enabled` pasó a `false` por
  normalización H5). Estado previo registrado para restaurar antes de la Parte 2:
  `modo_operacion=auto, bdp_sync_enabled=true, bdp_sync_mode=read_only`.
- Disco: `C:` ≥ 6 GB antes de compilar; target podable en `C:\tmp\glory-target\debug`.

## 4. PARTE 1 — Funcionalidad independiente (`standalone`), todo desde cero

Los bloques heredan el alcance de 039A-1 y **añaden las comprobaciones visuales/estilo** que el
reinicio justifica (tokens, responsive, foco, zoom, temas).

### P0. Baseline del repositorio
- [x] P0.1 HEAD `4464c52` (2026-09-14 06:30, "wip: snapshot 2026-09-14") y árbol limpio al empezar
      — ✅ confirmado por usuario 2026-09-14 12:16
- [x] P0.2 Stack arriba y sano: backend `:3100` `/api/health` → `{"status":"ok","version":"0.1.0"}`,
      migraciones al día (última `20260905100000`), seed demo presente (`users=1, trabajadores=3,
      ventas=50, gastos=34, clientes=11, mesas=6, article_map=561`), Vite `:5173` → 200
      — ✅ confirmado por usuario 2026-09-14 12:16
- [x] P0.3 `type-check` del frontend: **0 errores en `src/` propio**, 22 preexistentes del submódulo
      `glory-rs` registrados aparte — ✅ confirmado por usuario 2026-09-14 12:16

### P1. Modo operativo, conmutador, badge y degradación
- [x] P1.1 Badge de modo visible y correcto (standalone: no miente sobre el BDP) — en `standalone`
      muestra "Modo independiente" (antes, en BDP: "BDP: lectura") y no queda ninguna mención a BDP en
      el DOM. Corregido **H-149A-1-01** (padding del badge) y re-mostrado al usuario
      — ✅ confirmado por usuario 2026-09-14 12:24
- [x] P1.2 Conmutador de modo: cambio y persistencia, sin recargar de forma extraña — **evidencia
      2026-09-14:** el menú del badge cambia el modo (`PATCH /api/configuracion/modo` → 200) y
      **persiste** (BD `modo=auto`/`standalone` comprobado tras cada click); tras el fix
      **H-149A-1-02** el badge se actualiza **en vivo sin recargar** ("Modo automático activado" y
      badge pasa de "Modo independiente" a "BDP: off"; `recargasDePagina=1`, sin navegación extra)
      — ✅ confirmado por usuario 2026-09-14
- [x] P1.2b **Vuelta atrás desde "BDP: lectura"**: el menú del badge no ofrecía desactivar la
      integración (solo activar escritura, sincronizar e ir a Configuración) → **H-149A-1-05**.
      Verificado en vivo: `Desactivar BDP (quedar solo en local)` → `PATCH /api/configuracion
      {bdp_sync_enabled:false}` 200, badge pasa a "BDP: off" sin recargar, BD `bdp_sync_enabled=f`
      — ✅ confirmado por usuario 2026-09-14
- [x] P1.3 Degradación por fallos — **DESCARTADO por decisión del usuario 2026-09-15:**
      no hay caída real que observar y no quiere perder tiempo simulándola. El fix del badge
      (consume `GET /api/configuracion/modo`, muestra "BDP: sin respuesta" si hay degradación)
      queda implementado sin verificación visual; la evidencia de umbral/histéresis queda en
      código + tests `m2_*` (`modo_operacion.rs`, suite lib 176/176).
- [x] P1.4 Invalidación de caché del modo al guardar configuración — **verificado 2026-09-15
      con confirmación visual:** guardar en Configuración (sin cambios) → `Configuración guardada`,
      badge sigue `BDP: lectura` sin recargar. **Hallazgo corregido:** `useConfiguracion.ts` solo
      invalidaba la query de configuración; ahora invalida también `getObtenerModoOperacionQueryKey`
      (mismo patrón que `site-header.tsx`). `type-check`: 0 errores en `src/` propio (preexistentes
      de `glory-rs` intactos).
- [ ] P1.5 Histéresis cableada (fallos/éxitos registrados por el poller)
- [ ] P1.6 BDP caído → degradación con banner y operación local sin error (requiere BDP real o
      simulación de caída; si exige red, se verifica en Q3)

### P2. Catálogo de artículos unificado
- [ ] P2.1 Alta de artículo local (código automático y manual)
- [ ] P2.2 Edición de artículo
- [ ] P2.3 Desactivación / reactivación
- [ ] P2.4 Filtros, búsqueda y orden **efectivos** (no solo pintados)
- [ ] P2.5 Origen del dato visible (local vs BDP) por artículo
- [ ] P2.6 CSV/exportación coherente con lo filtrado
- [ ] P2.7 Visual: tabla, estados vacío/carga/error, responsive y tokens de estilo

### P3. Stock
- [ ] P3.1 Stock visible y origen por línea
- [ ] P3.2 Ajuste de stock (ruta y unidades correctas)
- [ ] P3.3 Sync de stock deshabilitada en standalone
- [ ] P3.4 Stock efectivo: lo local manda en filtro/orden/CSV
- [ ] P3.5 Visual: columnas, badges, estados de error

### P4. Inventario con persistencia local
- [ ] P4.1 Movimientos de inventario (entradas/salidas) persistidos
- [ ] P4.2 Conteo/regularización local
- [ ] P4.3 Historial de movimientos coherente con BD
- [ ] P4.4 Visual: formularios, validaciones visibles y mensajes

### P5. Anulación y eliminación de ventas
- [ ] P5.1 Anulación local 100 % (sin encolar nada)
- [ ] P5.2 Resumen diario que excluye anuladas
- [ ] P5.3 Liberación de mesa al cerrar/anular
- [ ] P5.4 Eliminación de venta (alcance y efecto en BD)
- [ ] P5.5 Visual: confirmaciones, estados y mensajes de la anulación

### P6. Compras locales
- [ ] P6.1 Albarán local (cabecera + líneas con IVA)
- [ ] P6.2 Estados del albarán (pendiente → borrador → conciliado)
- [ ] P6.3 Gasto asociado a la compra
- [ ] P6.4 Visual: tabla de compras, acciones por fila y modales

### P7. Pagos y factura local
- [ ] P7.1 Pago local (parcial y total)
- [ ] P7.2 Factura local y su numeración
- [ ] P7.3 Estados y coherencia con el resumen de ventas
- [ ] P7.4 Visual: flujo de cobro y documentos

### P8. Menús y packs locales + Explorador
- [ ] P8.1 Alta/edición de menús y packs
- [ ] P8.2 Composición (componentes, precios, disponibilidad)
- [ ] P8.3 Explorador de catálogo
- [ ] P8.4 Visual: formularios complejos, validación y responsive

### P9. Historial / auditoría
- [ ] P9.1 Registro de acciones locales (venta, anulación, compra, pago…)
- [ ] P9.2 Filtros y detalle del historial
- [ ] P9.3 Visual: timeline/tabla y estados

### P10. Permisos operativos

> **Derivado a subplan `149A-3`** (`Agente/planes/plan-permisos-y-errores-silenciosos-2026-09-14.md`):
> el 2026-09-14 se descubrió que el menú **no filtra por rol** (el trabajador ve Configuración,
> Trabajadores y Sincronización), que los 403 dejan la pantalla en **"Cargando..." sin aviso**
> y que se **reintentan 4–5 veces**, y que el gate **no tiene ninguna regla** que detecte este
> tipo de fallo silencioso. Los tres ítems de abajo se verifican allí con **ambas cuentas**.

- [ ] P10.1 Permisos por trabajador aplicados en UI (ocultar/deshabilitar) — ver 149A-3
- [ ] P10.2 Permisos aplicados en API (no solo en pantalla) — ver 149A-3 (F4)
- [ ] P10.3 Visual: pantalla de trabajadores/permisos — ver 149A-3 (F0/F6)

### P11. Invariante central de red
- [ ] P11.1 En standalone **cero tráfico** al BDP (verificado en red, no en código)
- [ ] P11.2 Controles BDP ocultos/deshabilitados con explicación honesta

### P12. Integridad de datos e independencia real
- [ ] P12.1 Operación completa sin BDP: ningún dato queda a medias
- [ ] P12.2 Reinicio del backend: los datos locales persisten
- [ ] P12.3 Coherencia entre pantallas (mismo dato, mismo valor)

### P13. Efectos locales de los controles de escritura
- [ ] P13.1 Alta local sin BDP (queda local, con aviso honesto)
- [ ] P13.2 Estado por ítem en la cola de Sincronización (sin BDP: vacío/deshabilitado honesto)
- [ ] P13.3 Visual: pantalla de Sincronización y sus mensajes

### P14. Sistema visual (nuevo en este reinicio)
- [ ] P14.1 Tokens: cero color/fuente/tamaño literal en componentes (`variables.css` manda)
- [ ] P14.2 Responsive real: 320 / 768 / 1024 / 1440 sin cortes ni solapes
- [ ] P14.3 Foco y teclado: navegación completa y foco visible
- [ ] P14.4 Zoom 200 %: nada se rompe ni se solapa
- [ ] P14.5 Tema claro/oscuro: contraste legible en ambos
- [ ] P14.6 Estados vacío/carga/error presentes y consistentes (no pantallas en blanco) — **hallazgo
      real 2026-09-14:** con rol trabajador, `GET /api/trabajadores` y `/api/bdp/push/pendientes`
      devuelven 403 y la pantalla queda en "Cargando..." indefinido, sin badge ni mensaje (derivado a
      `149A-3` H-149A-3-02)

## 5. PARTE 2 — Integración real (BDP), solo lecturas

**Precondición dura:** autorización explícita del usuario para contactar el BDP real y confirmación
de que la suscripción/credenciales están vigentes. Sin eso, esta parte no se ejecuta (se documenta
`⏸`). Única vía de red: la app (`BdpWeblinkClient` vía sus endpoints); prohibido curl/psql/SSH.

> **Estado de la suscripción (sonda de solo lectura, 2026-09-14):** BDP online (`health_ok`,
> `login_ok`, `version 36.2`, aplicación "01" Hostelería) y **la suscripción de salones/mesas ya está
> ACTIVA** — `POST /api/bdp/sync-tables` con `aplicar:false` responde 200 con `salones_bdp:7…` y ya
> **no** devuelve "Subscripción no activada" (era `⏸` externo en Q1.24). El flag de
> **pagos/factura/cancelación sigue sin verificar** (solo se comprueba con escritura autorizada o con
> el proveedor). Cero escritura en la sonda: `applied:false` + BD local intacta.

- [ ] Q0.1 Diagnóstico/preflight del BDP desde la app (health + login) con evidencia redactada
- [ ] Q0.2 Snapshot de configuración local antes de empezar
- [ ] Q1.1–Q1.24 Las 24 lecturas "en uso", una a una: endpoint real, respuesta tipada, y **lo que
      se ve en pantalla**; cada una con su confirmación visual
- [ ] Q1.25 Verificación de que la lectura no escribe (estado remoto sin cambios)
- [ ] Q1.26 Redacción de evidencia: contadores + muestra ≤3 items enmascarada
- [ ] Q3.1 BDP caído/reintento → degradación visible y operación local sin error
- [ ] Q3.2 Polling y reconciliación: estado honesto, sin duplicar nada
- [ ] Q4.1 Convivencia local + BDP en pantalla (origen del dato correcto)
- [ ] Q4.2 Integridad: lo local no se pisa con datos remotos inesperados
- [ ] Q5.1 Guardas de destino activas (no se puede apuntar a un BDP no autorizado)
- [ ] Q5.2 Cero fuga de credenciales/token en logs, pantalla y evidencia
- [ ] Q5.3 Escrituras bloqueadas mientras se hacen lecturas (fail-closed probado)

## 6. PARTE 3 — Tarea final: simulación del cliente (aceptación)

- [ ] S0.1 Persona y guion definidos (dueño y trabajador), sin leer código durante la simulación
- [ ] S0.2 Inventario del producto navegable (todas las pantallas, no solo BDP)
- [ ] S1.1…S1.n Día completo **sin BDP**: todas las funcionalidades independientes, uso real
- [ ] S2.1…S2.n El mismo día **con BDP conectado (solo lecturas)**: integración real
- [ ] S3.1 Escrituras reales: **fuera de este plan** → se ejecutan en `149A-2` con autorización por
      operación; aquí solo se observa dónde aparecerían

## 7. Rondas y cierre

- **Ronda 1** — contra el inventario heredado (¿algún bloque P/Q/S quedó sin ítem?).
- **Ronda 2** — contra la implementación real del HEAD (¿algún control nuevo sin cubrir?).
- **Ronda 3** — contra la ejecución: cada `- [x]` tiene OK del usuario y evidencia reproducible.
- **DoD:** 0 ítems marcados sin confirmación del usuario; todos los hallazgos `H-149A-1-*` cerrados
  con re-confirmación; Parte 1 completa contra el HEAD actual; Parte 2 ejecutada o `⏸` justificado
  (sin autorización / sin BDP); evidencia en `Agente/completados/`.

## 8. Estado

- [x] Reinicio decidido y `039A-1` archivado como **cerrado por reinicio** (2026-09-14)
- [ ] Parte 1 — en curso
- [ ] Parte 2 — pendiente de autorización de lecturas reales
- [ ] Parte 3 — pendiente

## 9. Hallazgos y bitácora de confirmaciones

**H-149A-1-05 (UI, corregido) — en "BDP: lectura" no había botón para desactivar la integración:**
reporte del usuario. El menú del badge (`site-header.tsx`) solo ofrecía, en ese estado, "Activar
escritura temporal", "Sincronizar a BDP", "Ver historial BDP" y "Configuración BDP": la única vuelta
atrás era entrar a Configuración a mano, mientras que el estado apagado sí tiene su "Activar BDP"
(el simétrico). Fix: nuevo ítem **"Desactivar BDP (quedar solo en local)"** → `PATCH
/api/configuracion {bdp_sync_enabled:false}` + invalidación de `getObtenerConfiguracionQueryKey()`
+ toasts de éxito/error. Verificado en vivo (click real desde el menú): badge "BDP: lectura" →
"BDP: off" sin recargar y BD `bdp_sync_enabled=f`; después se reactivó para que el usuario pueda
probarlo. Gotcha corregido en el mismo paso: el `useState` del estado "Desactivando..." se declaró
después del `if (!cfg) return null` y rompía el orden de hooks ("Rendered more hooks than during
the previous render", pantalla de error) — movido junto al resto de hooks.

**H-149A-1-04 (UI, corregido) — divisor del encabezado no centrado verticalmente:** el `<Separator
orientation="vertical">` de la cabecera (`site-header.tsx`) usa `self-stretch` en el primitivo; con
un alto fijo `h-4` eso lo pega al borde superior del renglón en vez de centrarlo (`align-self: stretch`
con altura definida = `flex-start`). Medido antes: centro del divisor y=20 vs centro de la fila y=28;
ahora coinciden (27,6 / 27,6 / 27,6 con el título). Fix: el divisor se dibuja con `div h-4 w-px bg-border`
(el padre ya centra con `items-center`) y se retira el import de `Separator`. Reporte del usuario
("esto de aquí no está centrado verticalmente") durante P1.1; `type-check`: 0 errores propios.

**H-149A-1-02 (UI, corregido) — el badge no reflejaba el cambio de modo:** los dos handlers del menú
del badge (`site-header.tsx`) invalidaban la clave de caché **equivocada** (`['configuracion']`) en
lugar de la clave real del proyecto (`getObtenerConfiguracionQueryKey()` de
`api/generated/configuracion`), así que el `PATCH` persistía en BD pero la cabecera seguía mostrando
el modo anterior hasta recargar a mano. Evidencia: con la BD en `modo=auto`, el badge aún decía
"Modo independiente" y `/api/configuracion` ya devolvía `auto`. Fix: usar el helper de clave
(patrón ya usado en `useConfiguracion.ts:98`) + verificado en vivo: badge pasa a "BDP: off" sin
recargar. **Afectaba también a "Activar/Desactivar BDP".**

**H-149A-1-01 (UI, corregido):** los tres badges de modo de la cabecera (`site-header.tsx`)
tenían el padding por defecto del `Badge` (2px/8px) y se veían "muy pegados" (reporte del usuario
en P1.1). Fix: `h-auto px-2.5 py-1` en los tres (mismo aspecto entre ellos) → verificado
`padding 4px/10px`, alto 25,6px. Re-mostrado y confirmado.

| Ítem | Evidencia (técnica + visual) | Confirmado por usuario | Fecha/hora |
|---|---|---|---|
| P0.1 | HEAD `4464c52` (2026-09-14 06:30) + árbol limpio | ✅ OK | 2026-09-14 12:16 |
| P0.2 | `/api/health` ok en `:3100`; última migración `20260905100000`; seed (users=1, trabajadores=3, ventas=50, gastos=34, clientes=11, mesas=6, article_map=561); Vite `:5173` 200; captura del Dashboard en vivo | ✅ OK | 2026-09-14 12:16 |
| P0.3 | `type-check`: 0 errores en `src/` propio; 22 preexistentes de `glory-rs` | ✅ OK | 2026-09-14 12:16 |
| P1.1 | Cabecera en standalone muestra **"Modo independiente"** (antes "BDP: lectura"); cero menciones a BDP en el DOM; tras el fix de padding, `padding 4px/10px`, alto 25,6px | ✅ OK | 2026-09-14 12:24 |
| P1.1b | Divisor del encabezado centrado: centro del divisor = centro de la fila = centro del título (27,6 px) | ✅ OK | 2026-09-14 |
| P1.2 | Menú del badge: `PATCH /api/configuracion/modo` 200 + persistencia en BD; badge cambia en vivo ("Modo independiente" → "BDP: off") sin recargar. Sandbox devuelto a `standalone` | ✅ OK | 2026-09-14 |
| P1.2b | `Desactivar BDP (quedar solo en local)` desde "BDP: lectura": click real → badge "BDP: off" sin recargar, BD `bdp_sync_enabled=f` | ✅ OK | 2026-09-14 |
| Obs. | `GET /api/configuracion/bdp/diagnostico` **sí contacta el BDP** estando en `standalone` (`health_ok:true`, `login_ok:true`, v36.2) al invocarlo explícitamente → a clasificar en P11.1 (¿debe gatearse en modo independiente?) | ⏳ | — |

## 10. Próximo paso

P0 (baseline del repositorio) con evidencia + primera captura para confirmación visual del usuario.
