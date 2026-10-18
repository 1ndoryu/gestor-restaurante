import { readFile, readdir } from 'node:fs/promises';
import { CORRUPT_TAKEOVER, TAKEOVER_SCHEMA_VERSION, defaultAgent, isStale, takeoverEntryPath, takeoverRegistryRoot } from './task-takeover-modelo.mjs';

/* Lectura del estado de las tomas. Un marcado ilegible se devuelve como
 * CORRUPT_TAKEOVER para no confundirlo con "sin tomar" (ausente = null). */

export async function readTakeover(root, taskId) {
  let raw;
  try {
    raw = await readFile(takeoverEntryPath(root, taskId), 'utf8');
  } catch (error) {
    /* Marcado inexistente (o leído a medias por otro proceso): sin tomar. */
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
  try {
    const entry = JSON.parse(raw);
    if (entry?.schemaVersion !== TAKEOVER_SCHEMA_VERSION || typeof entry.takenAtMs !== 'number') {
      /* Archivo presente pero ilegible/ajeno: se distingue del "sin tomar"
       * para que status lo liste y el re-toma lo retire (antes un marcado
       * corrupto bloqueaba la tarea para siempre con un falso conflicto). */
      return CORRUPT_TAKEOVER;
    }
    return entry;
  } catch (error) {
    if (error instanceof SyntaxError) return CORRUPT_TAKEOVER;
    throw error;
  }
}

export async function listTakeovers(root, nowMs = Date.now()) {
  const registry = takeoverRegistryRoot(root);
  let names = [];
  try {
    names = await readdir(registry);
  } catch (error) {
    if (error?.code === 'ENOENT') return [];
    throw error;
  }
  const entries = [];
  for (const name of names) {
    if (!name.endsWith('.json')) continue;
    const taskId = name.slice(0, -'.json'.length);
    try {
      const entry = await readTakeover(root, taskId);
      if (entry === CORRUPT_TAKEOVER) {
        /* Marcado ilegible: se lista como corrupto, no se aborta el listado. */
        entries.push({ taskId, entry: null, stale: false, corrupt: true });
      } else if (entry) {
        entries.push({ taskId, entry, stale: isStale(entry, nowMs) });
      }
    } catch {
      entries.push({ taskId, entry: null, stale: false, corrupt: true });
    }
  }
  return entries.sort((left, right) => (left.entry?.takenAtMs ?? 0) - (right.entry?.takenAtMs ?? 0));
}

/* Tomas activas de OTROS agentes (para banners de visibilidad temprana en
 * cualquier comando de trabajo, no solo el gate de la tarea objetivo). */
export async function listActiveForeignTakeovers(root, agent = defaultAgent(), nowMs = Date.now()) {
  const entries = await listTakeovers(root, nowMs);
  return entries.filter(item => item.entry
    && item.entry !== CORRUPT_TAKEOVER
    && !item.stale
    && item.entry.takenBy !== agent);
}

