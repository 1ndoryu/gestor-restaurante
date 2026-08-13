# Tareas completadas — F9 (bloque 128A-1) — Pruebas con/sin BDP, simulador y regresión

## F9 — Verificación integral del bloque F1–F8 (con y sin BDP)

- **Qué:** fase de verificación del plan (F9): suite standalone completa (sin BDP), suites del
  simulador BDP WebLink (con BDP simulado) y regresión del gate `task:check` con reporte
  reproducible. Sin cambios de código: solo evidencia y documentación.
- **Evidencia (comandos y resultados):**
  - **Standalone completo (sin BDP):** `node scripts/run-with-db.mjs test` → **PASS (exit 0)**.
    Suite completa en verde: `bdp_f8_permisos` 13/13, `bdp_f7_menus_locales` 15/15,
    `bdp_venta_lineas` 9/9, `bdp_write_guard` 2/2, `haddock_db` 10/10, doc-tests 0, etc.
    (los tests de simulador quedan `ignored` en la corrida normal y se ejecutan explícitamente,
    ver abajo).
  - **Simulador BDP — suite Python:** `python -m unittest discover -s tools/bdp-weblink-simulator
    -p "test_*.py" -v` → **92/92 OK** (auth/login y rechazos, pagos parciales, fault injection
    http_status/remote_error/invalid_json/delay, reset y state, seguridad loopback-only,
    rutas desconocidas 404, etc.).
  - **Simulador BDP — integración Rust:** `node scripts/run-with-db.mjs test --test
    bdp_simulator_integration -- --include-ignored` → **24/24 PASS** (ciclo completo
    crear→pagar→facturar, idempotencia de order/invoice, pagos parciales y sobrepago,
    cancelación, faults HTTP 500 / remote error / JSON inválido / disconnect+reconcile, login
    cacheado, export de artículos/clientes, health/version/tenders).
  - **Regresión del gate:** `npm run task:check -- 128A-1 --full --allow-heavy --heavy-reason
    "F9 pruebas con/sin BDP + simulador + regresión"` → **PASS** (sentinel, varsense, rust con
    4 comandos, frontend type-check, docs). Reporte reproducible:
    `.quality-reports/branches/glory-rs-rest--f100af0a041e6e8a/128A-1/latest.md` (commit
    `3fc17534`, rama `glory-rs-rest`, política enforce).
  - **Con/sin BDP (recorrido):** sin credenciales → modo efectivo `standalone`, bloque F1–F8
    probado con suites locales; con credenciales + simulador → modo `bdp`, verificado por
    `bdp_simulator_integration` y la suite Python del simulador. Sin escrituras ni llamadas al
    BDP real.
- **Archivos:** ninguno de código (fase de verificación). Actualiza
  `Agente/planes/completados/plan-independencia-bdp-2026-08-12.md` y este registro de completados.
- **Sentinel:** gate PASS con 0 errores (364 warnings y 34 info preexistentes, sin regresión
  nueva).
- **GLORY:** no aplica; rama `glory-rs-rest`.
