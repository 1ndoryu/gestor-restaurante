# Sub-plan Fase 4 (039A-1): monolito `tests/bdp_simulator_integration.rs`

- **Fecha:** 2026-09-11 · **Estado:** CERRADO por excepción firmada (11-09) · **Origen:** plan maestro `PROYECTO TASKS/Agente/planes/plan-cero-deuda-todos-proyectos-2026-09-03.md` §Fase 4.
- **Resolución:** el usuario ordenó excepcionar el archivo en vez de calibrar (FP-S4) o partir. Marcador `sentinel-disable-file limite-lineas limite-lineas-nivel-2 limite-lineas-nivel-3` en L3–8 con justificación + ref a este sub-plan; verificado con 0.7.9 → **0/0/0/0** en el archivo (`C:\tmp\verif-079-bdp-excepcion.json`). Sin commit (decisión del usuario).
- **Medición (Sentinel 0.7.9, artefacto del gate, 11-09):** 1 error `limite-lineas-nivel-3` en `tests/bdp_simulator_integration.rs:2182` (1718 efectivas = 3,44× el límite de 500). Evidencia: `C:\tmp\remedicion-079-restaurante.json` (1e/79w/23h, 470 archivos). Es uno de los 2 únicos errores Sentinel del área con 0.7.9 (el otro es `deploy_service.rs`, con sub-plan propio del 10-09).

## 1. Qué es el archivo (medido, no supuesto)

- Harness de integración contra el simulador BDP WebLink (Python como subproceso, una sola instancia compartida, reset por admin API antes de cada test). 2182 líneas, 81 KB.
- Helpers L1–≈415 (`ensure_simulator`, `admin_post`, `inject_fault`, `simulator_stock_of`, `simulator_config`, `full_modify_article_data`).
- 37 tests `#[tokio::test]` L416–2009 (health, login, export, pedidos, pagos, facturación, faltas inyectadas, clientes, stock).
- `tests/` limpio en git (sin WIP ajeno el 11-09) → ejecutable sin conflicto.

## 2. Opciones (en orden recomendado)

- **A — Calibración FP-S4 (recomendada primero):** dar a rutas `/tests/` y sufijos `_test.rs`/`_tests.rs` un tipo propio `test-integracion` con límite calibrado (~1000) en `obtenerLimiteArchivo()`. Precedente exacto: 0.7.8 ya dio trato propio a `#![cfg(test)]` para `expect-produccion-rs`. Con límite 1000, 1718 efectivas seguiría marcando deuda real (warning alto) sin el nivel BASTA de un harness legítimo. **Es cambio global a los 12 proyectos** (nueva versión 0.7.10 + release + re-pin + re-medición) → requiere decisión explícita del usuario; no se ejecuta dentro de este sub-plan.
- **B — Split por dominio (solo si A se rechaza):** partir en `tests/bdp_simulator_{pedidos,pagos,clientes,stock,faltas}.rs` con un módulo común `tests/bdp_simulator_comun.rs` (helpers + arranque del simulador). Riesgo: el harness comparte UNA instancia del simulador y estado reseteado por test; el split debe conservar el arranque único y el orden/independencia de los 37 tests. Validación: `cargo test --test bdp_simulator_*` (requiere Python 3 en PATH; sin Python los tests se auto-ignoran → verificar que corren, no que se saltan) + re-medición Sentinel del archivo.

## 3. No alcance

- No cambiar el simulador Python ni la API BDP; no tocar `src/` (el resto de RESTAURANTE está en 0 errores); no alterar la semántica de los tests (mismo coverage, mismo orden de reset).
- Sin commit/push sin autorización explícita.

## 4. Validación y DoD

1. `cargo test --test bdp_simulator_integration` (o los splits) en verde con Python disponible.
2. Re-medición Sentinel 0.7.x activa del archivo: 0 errores.
3. TABLA actualizada + evidencia en `RESTAURANTE/Agente/completados/`.
- **DoD:** 0 errores `limite-lineas-nivel-3` en RESTAURANTE con la versión activa, o excepción firmada si se adopta A con límite calibrado.
