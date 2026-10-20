# Fase D BDP local — 04 Fases L4–L5

> Parte del [plan Fase D sobre BDP local](plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 5. Fases (continuación)

### L4 — F0 empírica de avisos en TPV local

Precondición: L3.a–L3.c OK. La usuaria configura en el TPV local la URL de destino de los avisos (paso manual en el TPV) y el receptor es alcanzable desde el TPV.

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L4.1 | Receptor mínimo que registra avisos en log (sin inbox): escucha solo en `127.0.0.1`, log en `C:\tmp`, parada con PID verificado | Implementador (script según especificación) | Registra un aviso de prueba |
| L4.2 | Provocar los eventos con `PRUEBA-*`: crear, modificar, borrar y finalizar (**hechos**), traspasar y traspasar línea (**hechos**, sobre SALON 1). `cambiar-estado`: **no replicable en local** (Monitor de Cocina) | Automatizado con `scripts/pc/tpv.mjs`; vigilancia con `scripts/bdp-local/vigilar-avisos.mjs` | 6 de 7 eventos con evidencia; el séptimo, declarado |
| L4.3 | Matriz: payload real, valores de `Estado` 0–3, orden, duplicados, timestamps, origen y destino | Investigador extrae del log | `Agente/documentacion/bdp/avisos-local-matriz-<fecha>.md` con evidencia por fila |
| L4.4 | Valor seguro por defecto para lo no documentado (p. ej. `Estado` desconocido: registrar y no actuar) | Principal | Decisión anotada en la matriz |

**Gate L4:** los eventos reproducibles con evidencia (matriz `avisos-local-matriz-2026-10-18.md`); `cambiar-estado` queda como limitación declarada.

### L5 — Fase D F1–F4 (código)

Solo después de L4. El diseño es el del plan 2026-09-19; aquí no se re-diseña.

- **L5.0 Base T2 (roadmap 09AA-2): HECHA el 2026-10-17** (detalle y evidencia en §8). El PDF pide 6 tablas antes de las pruebas de aceptación (inbox, outbox, mapeo de IDs, cursor, pendientes agotados, libro de stock); se crearon **5**, porque *outbox* ya es `bdp_push_pendientes` y no se duplica. Incluye el bloqueo por artículo y almacén y el libro S/V que D26g declara como trabajo a implementar. L5.0 iba antes de F1: F1 ya no tiene ese bloqueo.
- **F1 Receptor e inbox: HECHA el 2026-10-17.** `GET|POST /api/bdp/avisos/{accion}` (las siete acciones, con alias), inbox `bdp_avisos_inbox` con clave única `{acción}|{comanda}|{timestamp canónico}`; secreto compartido `BDP_AVISOS_SECRETO` (tiempo constante, 503 si no está), límite de 16 KiB y 120/min, payload censurado, buzón con sesión de Admin (`GET /api/bdp/avisos`, `POST /api/bdp/avisos/{id}/reintentar`). Contrato y configuración del TPV: `Agente/documentacion/bdp/avisos-receptor-199A-1-2026-10-17.md`. Evidencia: `tests/bdp_avisos.rs` (11), `tests/bdp_avisos_limite.rs` (1), 7 unitarios en el servicio, clippy limpio en lo nuevo (§9.19), suite completa **475 pasan / 0 fallan**, gate de cierre **0 nuevos** (base 226) y **sonda HTTP real** contra el servidor de la 89 con el secreto del `.env` (404 sin acción, 401 sin secreto, 202 el aviso bueno, 200 el repetido, 422 sin comanda; el `Secreto` quedó censurado en el buzón). Cerrada con `Arranque cierre "RESTAURANTE" --tarea 199A-1` en verde; commits `14dd2c1` (receptor), `3c53d42` (pruebas) y `78b52f3` (docs), en `main`.
- **F2 worker de avisos: HECHA el 2026-10-18 (código y pruebas; commit `7646d19`). Migración `down` verificada el 2026-10-18 en una BD temporal de la 89 (up → down → up; columna presente, ausente, presente; BD temporal borrada).** `src/services/bdp_avisos/worker.rs`: `procesar_lote` reclama `recibido` con `FOR UPDATE SKIP LOCKED` y consulta `GetOrder` detrás del trait `ConsultaPedido`. Decisiones: `procesado` con foto en `pedido_bdp` (migración `20261018120000_bdp_avisos_pedido`); `301000` en `borrar` = `procesado`; `301000` en otra acción, o comanda no numérica = `requiere_revision`; fallo transitorio = `recibido` con `ultimo_error`, y a `MAX_INTENTOS` (5) = `requiere_revision`. Disparo: `POST /api/bdp/avisos/procesar` (Admin, `?limite=` ≤ 50). Evidencia: `tests/bdp_avisos_worker.rs` (8), `tests/bdp_avisos.rs` (11) y `tests/bdp_avisos_limite.rs` (1) pasan en la 89; clippy sin avisos nuevos en los archivos tocados (el de `worker.rs:33` corregido). **Sin bucle periódico**: el lote se dispara a mano o desde un planificador; un bucle automático queda para F3 si la usuaria lo quiere. Clippy de la lib falla por deuda heredada (`bdp_push.rs`, `bdp_weblink.rs`, `modo_operacion.rs`, `bdp_backup/conexion.rs`), no por este bloque.
- **F3** reconciliación, perfiles y XML para distribuidor. Pendiente de respuesta del distribuidor.
- **F4** TPV real y piloto de 7 días. Fuera de este plan; requiere permiso aparte.

Tests de L5 según AGENTS §8.1: por archivo tocado en cada bloque; suite completa una vez, al cierre.
