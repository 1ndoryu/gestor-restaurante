# Plan 149A-3 — Permisos por rol y errores silenciosos (subplan de 149A-1)

> **Fecha:** 2026-09-14 · **Rama:** `main` · **ID:** `149A-3`
> **Origen:** reporte del usuario durante 149A-1/P1.2 ("algunos botones no funcionan… ¿cómo es que
> en modo trabajador ve configuraciones que no debería?… esto claramente necesita una revisión
> profunda, sería un subplan").
> **Padre:** `Agente/planes/plan-revision-integral-bdp-2026-09-14.md` (149A-1, P10 Permisos y P14.6).
> **Método:** igual que el padre — evidencia por caso y **confirmación visual del usuario por ítem**.
> **Requisito explícito del usuario:** probar **siempre con las dos cuentas** (propietario y trabajador).

## 1. Hallazgos (evidencia 2026-09-14)

| ID | Qué | Evidencia | Severidad |
|---|---|---|---|
| H-01 | **El menú no filtra por rol**: el trabajador ve Configuración, Trabajadores, Sincronización… | `app-sidebar.tsx` sin ninguna comprobación de rol; menú idéntico en las dos cuentas | Alta (UX/engaño) |
| H-02 | **403 sin aviso al usuario**: la pantalla queda en "Cargando..." indefinido | con trabajador: `GET /api/trabajadores` y `GET /api/bdp/push/pendientes` → 403 | Alta |
| H-03 | **403 con reintentos inútiles** | los mismos 403 se repiten **4 veces** (`/trabajadores`, `/bdp/sincronizacion`) | Media (ruido/carga) |
| H-04 | **Estado vacío que miente**: la pantalla dice "0 trabajadores · No hay trabajadores" | hay **3** trabajadores en BD; el 403 se pinta como "no hay nada" | **Crítica (datos falsos)** |
| H-05 | **El gate no lo detecta** | `sentinel.config.json` sin ninguna regla sobre estado de error visible / carga infinita | Alta (detección) |
| H-06 | **ESCALADA DE PRIVILEGIOS**: endpoints sensibles sin comprobación de rol | con trabajador: `PATCH /api/configuracion`, `PUT /api/configuracion/bdp/sync-mode` y `PATCH /api/configuracion/modo` responden **422 de validación, no 403** (sondas rechazadas; BD verificada sin cambios) | **Crítica** |
| H-07 | **Permisos por sección guardados pero nunca aplicados** | `permisos_trabajador (seccion, permitido)` existe, se lee/escribe (`repositories/trabajador.rs`) y se expone en `/api/trabajadores/secciones`, pero **ningún handler lo consulta** | Alta |
| H-08 | **El patrón de guarda existe pero se aplica a medias** | `AuthUser::require_role()` (`middleware/auth.rs:28`) se usa en `bdp_push.rs` (3) y `trabajadores.rs` (`require_owner`, 5), y **en ningún otro módulo** | Alta |

## 2. Inventario de endpoints sensibles y guarda actual (F0, código)

| Módulo | Endpoints | Guarda hoy | Debe ser |
|---|---|---|---|
| `configuracion.rs` | `GET/PATCH /api/configuracion`, `GET/PUT /api/configuracion/integraciones`, `PUT /api/configuracion/bdp/sync-mode`, `GET /api/configuracion/bdp/diagnostico`, `/sync-dry-run` | **ninguna** | Dueño (lectura de config: dueño; diagnóstico/dry-run: dueño) |
| `modo_operacion.rs` | `GET/PATCH /api/configuracion/modo` | **ninguna** | Dueño |
| `bdp_backup.rs` | `backup/completo|parcial|glory`, `snapshots` (list/get), **`DELETE snapshot`**, **`restaurar_glory`**, `audit`, `explorar`, `departamentos/perfil` | **ninguna** | Dueño (restaurar/borrar: **crítico**) |
| `bdp_article_map.rs` | `article-maps` (GET/POST), `import-catalog`, `sync-catalog`, `sync-prices`, `article-stock/ajustar` | **ninguna** | Dueño para sync/import/ajustar; lectura puede ser operativa |
| `admin.rs` | `/api/admin/reset`, `/api/admin/seed` | **ninguna** | Dueño o deshabilitado en producción (**crítico**) |
| `api_keys.rs` | `/api-keys`, `/api/api-keys/:id` | **ninguna** | Dueño |
| `bdp_push.rs` | `push/pendientes`, `push/flush`, `push/:id/reintentar` | `require_role(Admin)` ✓ | Correcto (revisar si el trabajador debe ver la cola: F1) |
| `trabajadores.rs` | `/api/trabajadores` (+secciones) | `require_owner` ✓ | Correcto |
| Resto (ventas, gastos, reservas, clientes, plano, compras, stock, menús, campañas…) | — | sin guarda | **Decisión F1**: operativas para el trabajador, y si no, permiso por sección |

## 3. Fases

- **F0 — Inventario completo (en curso).** Matriz **rol × página × endpoint × respuesta × qué
  muestra la pantalla**, con las **dos cuentas**, registrando red (XHR) y estado de UI por ruta.
  Hecho: barrido de las 24 rutas como **trabajador** (18 ok; 2 con 403×4; `/configuracion` accesible)
  y los 3 endpoints de escalada. **Falta:** barrido equivalente como **dueño** + inventario de
  acciones dentro de cada página (botones/modales).
- **F1 — Decisión de modelo (requiere al usuario).** Por sección: ¿oculta / deshabilitada con
  explicación / visible con aviso? ¿Se usa `permisos_trabajador` (recomendado, ya existe) o se
  simplifica a "dueño vs trabajador"? ¿Qué puede hacer un trabajador hoy por diseño?
  **DIFERIDA por el usuario el 2026-09-16 (retomar más tarde); F3, F5 y F6 quedan en espera tras ella.**
- **F2 — UI honesta ante 401/403.** Estado de error explícito ("No tienes permiso para ver esta
  sección") en lugar de "Cargando..." infinito o de un vacío falso; **sin reintentos en 4xx**
  (React Query) y sin pintar "0 elementos" cuando la consulta falló.
- **F3 — Menú por rol.** Ocultar/deshabilitar entradas según F1, coherente con F2 y F4.
- **F4 — Guardas en backend.** Aplicar `auth.require_role(&[UserRole::Admin])` (o el helper de
  sección que salga de F1) en **configuración, modo, sync-mode, integraciones, backup
  (restaurar/borrar), admin/reset|seed, api-keys, y sync/import de catálogo**; respuesta **403
  clara y consistente**, sin filtrar datos. Pruebas: trabajador 403 / dueño OK por endpoint.
- **F5 — Brecha del gate.** Proponer a Sentinel una regla o medición que detecte el patrón
  ("consulta en error sin UI de error", "cargando infinito", "vacío falso"), con caso mínimo
  reproducible. **No se modifica el gate ni `sentinel.config.json` sin autorización** (afecta a 12
  proyectos).
- **F6 — Verificación con ambas cuentas.** Recorrido completo como dueño y como trabajador, con
  confirmación visual del usuario por ítem (mismo protocolo que 149A-1), y evidencia en
  `Agente/completados/`.

## 4. Riesgos y no alcance

- **Riesgo de romper el uso real**: si el trabajador necesita hoy alguna de esas pantallas, cerrarla
  en seco lo bloquea → por eso F1 primero y F4 con pruebas de las dos cuentas.
- **No alcance:** rediseñar el modelo de datos de permisos, RBAC completo, o tocar el gate sin
  autorización. Cero escrituras al BDP.

## 5. DoD

- 0 endpoints sensibles alcanzables por un trabajador (F4) con prueba por endpoint.
- 0 pantallas en "Cargando..." o con vacío falso ante 401/403 (F2), verificado con ambas cuentas.
- 0 entradas de menú sin permiso y sin explicación (F3).
- Decisión F1 documentada y aplicada; propuesta F5 elevada con su caso mínimo.
- Evidencia y cierre en `Agente/completados/` con el visto bueno visual del usuario.

## 5b. Evidencia F4 Tanda A (probado con las DOS cuentas)

| Endpoint | Trabajador | Dueño | Cero daño verificado |
|---|---|---|---|
| `PATCH /api/configuracion` (payload inválido) | **403** | 422 (validación) | BD sin cambios |
| `PUT /api/configuracion/bdp/sync-mode` (inválido) | **403** | 422 | `bdp_sync_mode` sigue `read_only` |
| `PATCH /api/configuracion/modo` (inválido) | **403** | 422 | modo sigue `standalone` |
| `DELETE /api/bdp/backup/snapshots/:id` | **403** | 404 (no encontrado) | snapshots=5 |
| `POST /api/bdp/backup/restaurar/:id` (confirmación incorrecta a propósito) | **403** | 422 | ventas=50, snapshots=5 |
| `GET /api/configuracion/bdp/diagnostico` | **403** | 200 | — |
| `POST /api/admin/reset` | **403** | 400 (puerta demo) | users=1, ventas=50 |
| `POST /api/admin/seed` | **403** | 400 (puerta demo) | igual |
| `GET /api/api-keys` | **403** | 200 | — |

Ninguna sonda modificó nada (verificado en BD tras cada tanda). Se mantienen abiertos a propósito
`GET /api/configuracion` y `GET /api/configuracion/modo` (la cabecera y todas las pantallas los usan).

## 6. Camino elegido (decisión del agente, 2026-09-14)

**Orden recomendado y elegido:**

1. **F4 primero, en dos tandas** (es el riesgo real y es pequeño y testeable):
   - **Tanda A (crítica):** `configuracion.rs` (PATCH/GET config, integraciones, sync-mode),
     `modo_operacion.rs`, `bdp_backup.rs` (restaurar/borrar snapshots), `admin.rs`, `api_keys.rs`.
     Motivo: cualquiera de ellos permite **habilitar escrituras reales al BDP**, **restaurar/borrar
     datos locales** o **resetear el sistema**.
   - **Tanda B:** `bdp_article_map.rs` (sync-catalog/precios/import/ajustar stock) y revisión de
     `bdp_push.rs` según F1.
2. **F2 después** (honestidad: 403 visible, sin reintentos, sin vacíos falsos) — es lo que el usuario
   ve y lo que hoy miente.
3. **F3** (menú por rol) cuando F1 esté decidido.
4. **F0** se completa en paralelo (barrido del dueño) porque alimenta F1.
5. **F5** al final: elevar la regla al gate, que es trabajo de otra herramienta.

**Por qué este orden:** cerrar primero lo que puede causar daño irreversible (escrituras al BDP real,
restauración de snapshots, reset) y que además es un cambio pequeño con patrón ya existente
(`require_role`); lo cosmético/UX puede esperar sin riesgo.

## 5c. Evidencia F2 — UI honesta ante 401/403 (2026-09-14)

Pieza nueva y única: `frontend/src/components/ui/estado-error.tsx` (`EstadoError` + `estadoDelError`
+ `esErrorDePermiso`). Se declara **una sola vez** cómo se ve un fallo y quién puede reintentar.

| Cambio | Archivo | Qué se probó |
|---|---|---|
| `retry` no reintenta 4xx (solo 1 intento en red/5xx) | `frontend/src/App.tsx` | `GET /api/trabajadores` y `/api/bdp/push/pendientes`: **1 sola llamada 403** (antes ×4) |
| `isError`/`error` expuestos | `hooks/useTrabajadores.ts` | — |
| Estado de error en vez de conteo falso | `componentes/ListaTrabajadores.tsx` | trabajador: "No tienes permiso para ver la gestión de trabajadores (código 403)", sin "0 trabajadores"; dueño: "3 trabajadores" + tabla |
| Estado de error y sin resumen/acciones | `componentes/bdp/BdpSincronizacion.tsx` | trabajador: solo el 403, **sin** "0 filas" ni "Sincronizar ahora"; dueño: "15 filas · 3 sincronizadas" + botón |
| Sección completa honesta | `componentes/ConfigChatbot.tsx` | trabajador pestaña Chatbot: 403 sin formulario de creación; dueño: sección normal |
| Aviso en vez de "Cargando secciones..." infinito | `componentes/ListaTrabajadores.tsx` | si falla el listado de secciones |

`type-check` del frontend: **0 errores en `src/` propio** (los 22 del submódulo `glory-rs` siguen
aparte). Verificado **en vivo** con las dos cuentas en la pestaña Preview (trabajador
`sara.lopez@demo.com` / dueño `demo@restaurante.com`).

## 7. Estado

- [x] Subplan con inventario de endpoints y hallazgos H-01…H-08 (2026-09-14)
- [ ] F0 — parcial (barrido de trabajador hecho; falta dueño)
- [ ] F1 — requiere al usuario
- [x] **F2 — HECHA y verificada con las dos cuentas (2026-09-14)**: `EstadoError` compartido,
      `retry` que no repite 4xx, y 3 pantallas que mentían (trabajadores, sincronización,
      Chatbot) ya dicen "no tienes permiso". Evidencia en §5c.
- [ ] F3 — pendiente
- [x] **F4 Tanda A — HECHA (2026-09-14)**: guardas `require_role(&[UserRole::Admin])` en
      `configuracion.rs` (PATCH config, PUT sync-mode, GET/PUT integraciones, diagnóstico, dry-run),
      `modo_operacion.rs` (PATCH), `bdp_backup.rs` (snapshot completo/parcial, borrar snapshot,
      restaurar), `admin.rs` (seed/reset) y `api_keys.rs` (crear/listar/revocar).
- [x] **F4 Tanda B — HECHA (2026-09-16, commit `c40de93`)**: `verificar_permiso(CatalogoEdicion)`
      en `importar_catalogo`, `sync_catalog`, `sync_prices` (default solo Admin, delegable por el
      dueño; lecturas —listar, stock, conteos, definiciones de menú/pack— siguen operativas) y
      `require_role(Admin)` en `sync_tables` (escribe ZonaSala+Mesa, sin AccionPermiso de plano;
      patrón Tanda A como `bdp_push`). Tests nuevos `tests/permisos_catalogo_edicion.rs` 3/3
      (default fail-closed, delegación `admin_trabajador`, patrón sync-tables). Sondas vivas :3100
      con las dos cuentas (sara.lopez@demo.com / demo@restaurante.com): trabajador **403** en los 4
      endpoints; dueño `422` en sync-tables sin confirmación (pasa guards, cero escrituras) y `200`
      en lectura. Cero daño (los 403 abortan antes de modo/BDP). `bdp_push.rs` ya era Admin-only:
      su revisión queda supeditada a F1 (si el trabajador debe ver la cola).
- [x] **Hallazgo nuevo (bloqueante, preexistente) — RESUELTO (2026-09-14)**: `cargo test --lib` no
      compilaba — 28 errores en `src/services/bdp_sync.rs` porque el split del 2026-09-12 movió
      `Venta`, `VentaLinea`, `BdpArticleMapRepository` y `chrono::Utc` a los submódulos y el módulo
      `tests` seguía con `use super::*`. Se añadieron los 4 imports al módulo de tests: **`cargo test
      --lib` → 176 passed / 0 failed**. No lo causaba este cambio; quedó reparado porque bloqueaba el
      gate.
- [ ] F5 — pendiente · [ ] F6 — pendiente
