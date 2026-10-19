# Fase D BDP local — 02 Hechos verificados y pendientes

> Parte del [plan Fase D sobre BDP local](plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 3. Hechos verificados y pendientes

**Verificado en código, configuración o documentación**

- La configuración BDP vive en la tabla `configuracion_restaurante` (`migrations/20260506000000_bdp_weblink_config.up.sql:7-14`).
- Las variables de `.env` solo arrancan la configuración. Tras `bdp_env_bootstrap_applied_at` se ignoran (`src/services/bdp_config_bootstrap.rs:165-173`; la marca se fija en l.210). Por eso el cambio de host se hace por API o por la pantalla de Configuración, no editando `.env`.
- `PATCH /api/configuracion` con `bdp_base_url`, login, password, código integrador o pos desarma la sincronización y fuerza `bdp_sync_mode = read_only` en el mismo UPDATE (`src/handlers/configuracion.rs:197-215`). Rutas: `GET/PATCH /api/configuracion` (l.111, l.130).
- Kill switch de escritura (`src/services/bdp_weblink.rs:795-831`): loopback (`localhost`, `127.0.0.1`, `::1`) siempre se permite. Cualquier otro host exige coincidencia exacta con `BDP_WRITE_ALLOWED_ORIGINS`, sin credenciales, query ni path distinto de `/`. Solo cubre escrituras; las lecturas no tienen este freno.
- Modo efectivo: `modo_efectivo_desde_config` (`src/services/modo_operacion.rs`). En `auto` depende de `bdp_sync_enabled`.
- Cola de envíos: existen las tablas `bdp_push_pendientes`, `bdp_write_arming` y `bdp_poll_schedule` (migraciones). Si hay filas pendientes, al cambiar la URL podrían salir hacia el TPV local. Por eso L0.3 va antes de L1.
- Puerto del BDP real: `IP_BDP:8068` (`Agente/documentacion/api/bdp-300035-resumen-completo-2026-06-01.md:6`). Puerto por defecto en código: `localhost:8068` (`src/services/bdp_sync.rs:115`). Eso no prueba la instancia local.
- `RESTAURANTE` ya figura en `workspace-manager/scripts/dev/registro.json` (L0.0, hecho el 2026-10-10). Sin entrada no hay mando `Arranque` (AGENTS §9.2). **Actualizado 2026-10-17:** el proyecto está dado de alta y bajo mando (`Arranque status "RESTAURANTE"` → `bajo-mando`, `dev89`, tmux 1/1, sonda verde).
- Frontend: `frontend/vite.config.ts:19` declara `5173`. El `:5182` de las verificaciones del 09-16 es un arranque manual, no configuración.
- Scripts existentes: `scripts/run-cargo.mjs`, `scripts/run-with-db.mjs`, `scripts/branch-db.mjs`. Gate canónico: `npm run gate:check` (`package.json:37`, `scripts/quality/gate-check.mjs`).
- **Base T2 (resuelto el 2026-10-17, L5.0):** el 2026-10-10 se verificó que no existía (búsqueda en `migrations/` y `src/`: ningún resultado para `bdp_sync_cursor`, `bdp_id_map`, `bdp_stock_ledger`, `bdp_avisos_inbox` ni outbox; testigo en el mismo barrido: `bdp_push_pendientes` y `bdp_write_arming` sí aparecen, 66 coincidencias en 17 archivos). Ya está creada: `migrations/20261017120000_bdp_base_t2.{up,down}.sql` con **5** tablas, no 6 — el *outbox* que el PDF nombra ya es `bdp_push_pendientes` y no se duplica (§8).
- Caja cerrada (2026-10-10, L3.a): `POST /api/ventas` crea la venta; su envío a BDP devuelve `301400` (caja no abierta, POS 1). No hay endpoint WebLink de apertura de caja (el manual local solo documenta comandos `ws*` sin caja); se abre en TpvNet («Apertura de Caja»).
- Gaps de código vistos en L2/L3: modificar una venta no tiene ruta BDP; `/bdp/article-stock` lee caché (no `GetStock` en vivo); comentarios desfasados («C3=b: nunca se llama CancelOrder») en `src/services/venta.rs:291` y `src/handlers/ventas.rs:225`, cuando `anular` ya encola `venta/cancelar` (`src/services/venta.rs:349-367`); `clippy -D warnings` falla por avisos preexistentes en `bdp_push.rs`, `bdp_weblink.rs` y `modo_operacion.rs`.
- Contradicción: `roadmap.md` 01AA-1 dice "P0 siguiente", pero `plan-alcance-local-bdp-2026-10-01.md:144` dice "P0 COMPLETO".
- Contradicción: `roadmap.md` 199A-1 menciona "12 preguntas a BDP", frente a la decisión de 2026-09-20 "sin preguntas a BDP".
- Git (2026-10-10): `main` sin ramas locales; `git status` mostraba 32 entradas ajenas. **Actualizado 2026-10-17:** árbol limpio salvo `quality-tools.json` modificado (subida de pin de Sentinel a 0.7.26 y de VarSense, sin commitear; no se toca aquí).

**No verificado (condiciona o bloquea)**

- Host y puerto del WebLink del TPV local. No consta en el repositorio. L0.1 lo averigua sin tocar la app.
- Licencia activa en vivo (solo consta en documentación; el TPV local figura como DEMO y D01 espera la licencia 84, que no es replicable en local).
- Serie `PRUEBA`: creada según bitácora, no confirmada por el distribuidor.
- Suscripción para `Payment/Add` e `InvoiceOrder`.
- Si el aviso de traspaso trae origen y destino.
- `CancelOrder` con efecto en cocina: `roadmap.md` l.118 dice "verificado 2026-09-18"; el alcance dice "pendiente de prueba local".
- `GET /api/configuracion/bdp/diagnostico` (l.534): no leído. Solo se usa si se confirma que es de solo lectura.

