#!/usr/bin/env node
/* [028A-17] Coordinación de tomas de tarea entre agentes en el checkout
 * compartido. Cada vez que un agente EMPIEZA una tarea la marca con
 * `npm run task:take -- --task <ID> --by <agente>`; al terminar la libera con
 * `npm run task:release -- --task <ID>`. El identificador de cada toma
 * codifica el instante exacto en que se tomó (formato `T-<epochMs>-<hex8>`),
 * de modo que basta leerlo para saber cuándo se tomó sin abrir el registro.
 *
 * Reglas:
 * - Una tarea tomada por OTRO agente activo no se puede tomar de nuevo.
 * - Un marcado que supera TAKEOVER_TTL_MS (6 h) sin liberarse se considera
 *   olvidado: cualquier agente puede re-tomarlo (con aviso) o liberarlo.
 * - task:release SOLO libera la tarea del agente que la tomó (o un marcado
 *   ya expirado); un agente NUNCA puede liberar la tarea activa de otro,
 *   ni siquiera con --force (--force solo afecta a task:take).
 * - El registro vive en `.quality-reports/task-takeover/<taskId>.json`
 *   (ignorado por git): es coordinación local del checkout, no un contrato.
 * - Un marcado ilegible (JSON roto o esquema desconocido) no pertenece a
 *   nadie: `status` lo lista como corrupto y `take`/`release` lo retiran
 *   para poder re-tomar la tarea (antes bloqueaba el re-toma para siempre
 *   con un falso "carrera de escritura").
 * - `task:check` solo informa/recuerda; nunca toma ni libera por sorpresa.
 *
 * Seguridad: el taskId se valida contra un patrón seguro (nada de traversal),
 * la creación de un marcado usa `wx` (falla si ya existe) para que dos tomas
 * concurrentes no se pisen, y los campos del registro son solo cadenas/números. */
import { pathToFileURL } from 'node:url';
import { main } from './task-takeover-cli.mjs';

/* Fachada: reexporta los módulos task-takeover-*.mjs para que los importadores
 * (task-check.mjs, tests) sigan usando esta misma ruta y nombres. */
export { TAKEOVER_TTL_MS, TAKEOVER_SCHEMA_VERSION, CORRUPT_TAKEOVER, sanitizeTaskId, takeoverRegistryRoot, takeoverEntryPath, createTakeoverId, decodeTakeoverId, isStale, defaultAgent, sanitizeAgentName } from './task-takeover-modelo.mjs';
export { readTakeover, listTakeovers, listActiveForeignTakeovers } from './task-takeover-lectura.mjs';
export { takeTask, touchTakeover, foreignTakeoverDecision, releaseTask, takeoverReminders } from './task-takeover-reglas.mjs';

const isEntryPoint = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isEntryPoint) await main();
