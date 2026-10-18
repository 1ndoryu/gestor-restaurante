import os from 'node:os';
import path from 'node:path';
import { randomBytes } from 'node:crypto';
import { projectRoot } from './preflight.mjs';

/* Modelo base de la toma de tarea: constantes, validación del taskId, rutas del
 * registro, identificadores y saneado del agente. Vive aparte para que lectura,
 * escritura y reglas compartan esta base sin importarse entre sí (sin ciclos). */

export const TAKEOVER_TTL_MS = 6 * 60 * 60 * 1000;
export const TAKEOVER_SCHEMA_VERSION = 1;

/* Centinela para marcados presentes pero ilegibles (JSON roto o esquema
 * desconocido): no pertenecen a nadie; `status` los lista como corruptos y
 * cualquier agente puede retirarlos con take/release. [018A-97] */
export const CORRUPT_TAKEOVER = Symbol('corrupt-takeover');

const SAFE_TASK_ID = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/u;

export function sanitizeTaskId(taskId) {
  if (typeof taskId !== 'string' || !SAFE_TASK_ID.test(taskId)) {
    throw new Error(`taskId inválido para toma de tarea: ${String(taskId)}`);
  }
  return taskId;
}

export function takeoverRegistryRoot(root = projectRoot) {
  return path.join(root, '.quality-reports', 'task-takeover');
}

export function takeoverEntryPath(root, taskId) {
  return path.join(takeoverRegistryRoot(root), `${sanitizeTaskId(taskId)}.json`);
}

/* Identificador de toma: `T-<epochMs>-<hex8>`. El epoch codifica el instante
 * exacto de la toma; basta decodificarlo para saber cuándo se tomó. */
export function createTakeoverId(takenAtMs) {
  return `T-${Math.trunc(takenAtMs)}-${randomBytes(4).toString('hex')}`;
}

export function decodeTakeoverId(id) {
  if (typeof id !== 'string') return null;
  const match = /^T-(\d{13})-([0-9a-f]{8})$/u.exec(id);
  if (!match) return null;
  const takenAtMs = Number(match[1]);
  if (!Number.isSafeInteger(takenAtMs)) return null;
  return { takenAtMs, randomHex: match[2] };
}

export function isStale(entry, nowMs = Date.now()) {
  return typeof entry?.takenAtMs === 'number'
    && nowMs - entry.takenAtMs > TAKEOVER_TTL_MS;
}

export function defaultAgent(env = process.env) {
  return env.GLORY_AGENT_ID?.trim() || os.hostname();
}

/* El nombre de agente entra por CLI (--by) o env y se vuelca en el reporte
 * Markdown: se recorta y se eliminan caracteres de control para que un valor
 * con `\n` no inyecte contenido en el reporte. */
export function sanitizeAgentName(value) {
  const cleaned = String(value ?? '').replace(/[\u0000-\u001f\u007f]/gu, '').trim();
  return cleaned.slice(0, 64) || 'unknown';
}

