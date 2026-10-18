import { writeFile } from 'node:fs/promises';
import { TAKEOVER_TTL_MS, TAKEOVER_SCHEMA_VERSION, createTakeoverId, sanitizeAgentName } from './task-takeover-modelo.mjs';

/* Escritura de registros de toma: construye la entrada (id, expiración) y la
 * crea de forma exclusiva para que dos tomas concurrentes no se pisen. */

export function buildEntry(taskId, by, nowMs) {
  const takenAtMs = Math.trunc(nowMs);
  return {
    schemaVersion: TAKEOVER_SCHEMA_VERSION,
    taskId,
    id: createTakeoverId(takenAtMs),
    takenAt: new Date(takenAtMs).toISOString(),
    takenAtMs,
    takenBy: sanitizeAgentName(by),
    expiresAt: new Date(takenAtMs + TAKEOVER_TTL_MS).toISOString(),
    expiresAtMs: takenAtMs + TAKEOVER_TTL_MS,
  };
}

export async function writeEntry(target, taskId, by, nowMs) {
  const entry = buildEntry(taskId, by, nowMs);
  /* `wx` garantiza que dos agentes tomando a la vez no se pisen: si el
   * archivo ya existe, la creación falla y el caller decide (conflicto). */
  await writeFile(target, `${JSON.stringify(entry, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' });
  return entry;
}

