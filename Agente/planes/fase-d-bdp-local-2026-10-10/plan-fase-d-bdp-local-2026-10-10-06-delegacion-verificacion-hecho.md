# Fase D BDP local — 06 Delegación, verificación y definición de hecho

> Parte del [plan Fase D sobre BDP local](../plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 6. Delegación

| Tarea | Agente | Paralelo |
|---|---|---|
| L0.1 comprobación de proceso y puerto (comandos) | principal | 1 |
| L0.3 consulta de estado y cola | principal | 1 |
| L1.4 revisión de logs | verificador | 1 |
| L2.1 y L4.1 scripts | implementador | 1 |
| L2.1 ejecución, L3 comprobación por lote | principal + verificador | 1 |
| L4.3 matriz desde log | investigador | 1 |
| L5 tests y builds | verificador | 1 |
| L5.0 migraciones de T2 (según especificación del principal) | implementador | 1 | **hecho 2026-10-17** |
| L6–L9 scripts de ficha (un patrón por familia) | implementador | 1 |
| L6–L10 ejecución y lectura de resultados contra el TPV | principal + verificador | 1 |
| L9 matriz de avisos duplicados y fuera de orden desde log | investigador | 1 |

La configuración (L1.2), las decisiones y la lectura de resultados de escritura las hace el principal o la usuaria.

## 7. Verificación y Definition of Done

- **L1:** `GET /api/configuracion` muestra URL local, `bdp_sync_enabled = false` y `read_only`. Cero apariciones de `IP_BDP` en logs.
- **L2:** lectura con evidencia.
- **L3:** lotes con `bdp_audit_log` y estado en TPV verificados.
- **L4:** matriz de 7 eventos.
- **L5.0 (HECHA 2026-10-17):** las 5 tablas de T2 que faltaban creadas por migración, con tests de idempotencia (inbox, mapeo, cursor, agotados, libro de stock). *Outbox* no se crea: ya es `bdp_push_pendientes`.
- **L6–L10:** ninguna ficha D se da por superada sin la evidencia que su propia ficha exige (anexo B8): estado en TPV vía `GetOrder`/`GetStock`, filas en `bdp_audit_log`, `bdp_id_map` y la cola. Fichas que dependan del BDP real (licencia 84, almacén real, D26b/f/g) quedan «ensayadas en local, pendientes de real», nunca «superadas». El estado de cada una vive en §10.
- **Calidad (AGENTS §6):** `npm run gate:check` en RESTAURANTE y re-análisis `POST http://localhost:8787/api/gate/analizar` con `{"clave":"RESTAURANTE","forzar":true}`. Sin hallazgos nuevos atribuibles al cambio.
- **Documentación:** roadmap (10AA-3 cerrada; 01AA-1 y 199A-1 alineadas), `Agente/completados/tareas-<fecha>.md`, matriz de avisos. El plan se mueve a `Agente/planes/completados/` con sus enlaces actualizados (AGENTS §7.2).
- **Limpieza:** receptor parado (PID verificado), `C:\tmp\fase-d-local\` borrado, procesos propios parados.
- **Commit e integración (CLAUDE.md "Ramas"):** commits por bloque si la política del repositorio lo exige (AGENTS §5), stage explícito, cada commit con la línea `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Rama `fase-d-bdp-local` integrada en `main` con fast-forward o merge, sin force-push. El push a `main` es publicación a remoto compartido: se pide confirmación antes (AGENTS §0.3). Antes de borrar ramas local y remota: `git bundle create` y `git bundle verify`. Si la integración necesita una decisión de la usuaria (conflicto, rama no fast-forward), anotar en el roadmap con su motivo y preguntar.
