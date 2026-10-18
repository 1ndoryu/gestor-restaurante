import { mkdir, unlink } from 'node:fs/promises';
import path from 'node:path';
import { writeAtomic } from './atomic-file.mjs';
import { CORRUPT_TAKEOVER, defaultAgent, isStale, sanitizeAgentName, sanitizeTaskId, takeoverEntryPath } from './task-takeover-modelo.mjs';
import { readTakeover } from './task-takeover-lectura.mjs';
import { buildEntry, writeEntry } from './task-takeover-escritura.mjs';

/* Reglas de toma de control: tomar, renovar (heartbeat), liberar, decidir el
 * bloqueo del gate y generar recordatorios. Leen con lectura.mjs y escriben con
 * escritura.mjs; no interpretan la CLI. */

/* Toma la tarea. Devuelve:
 * - { status: 'taken', entry }            marcado nuevo
 * - { status: 'refreshed', entry }        mismo agente re-tomando (renueva)
 * - { status: 'taken-over-stale', entry } marcado viejo olvidado, re-tomado
 * - { status: 'conflict', entry }         tomada por otro agente activo */
export async function takeTask(root, taskId, { by = defaultAgent(), force = false, nowMs = Date.now() } = {}) {
  sanitizeTaskId(taskId);
  const agent = sanitizeAgentName(by);
  const target = takeoverEntryPath(root, taskId);
  await mkdir(path.dirname(target), { recursive: true });
  const existing = await readTakeover(root, taskId);
  if (existing === CORRUPT_TAKEOVER) {
    /* Marcado ilegible: no es de nadie; se retira y se toma de cero. */
    await unlink(target).catch(() => {});
    return { status: 'taken-over-corrupt', entry: await writeEntry(target, taskId, agent, nowMs) };
  }
  if (existing && existing.takenBy === agent) {
    /* Re-toma del mismo agente (con o sin --force): renueva el marcado.
     * El archivo ya existe, así que se reemplaza atómicamente, no con `wx`. */
    const entry = buildEntry(taskId, agent, nowMs);
    await writeAtomic(target, `${JSON.stringify(entry, null, 2)}\n`);
    return { status: 'refreshed', entry };
  }
  if (existing && !isStale(existing, nowMs)) {
    return { status: 'conflict', entry: existing };
  }
  if (existing && force) {
    /* Re-toma forzada de un marcado expirado: se reemplaza atómicamente
     * (el archivo ya existe, `wx` fallaría con EEXIST). */
    const entry = buildEntry(taskId, agent, nowMs);
    await writeAtomic(target, `${JSON.stringify(entry, null, 2)}\n`);
    return { status: 'taken-over-stale', entry };
  }
  if (existing) {
    /* Marcado olvidado (>6h): re-toma con aviso. Compare-and-delete: solo se
     * retira si sigue siendo el mismo marcado que observamos; si otro agente
     * lo re-tomó entre la lectura y este punto, se devuelve conflicto y no se
     * pisa la toma fresca ajena. */
    const current = await readTakeover(root, taskId);
    if (current && current.id !== existing.id) {
      return { status: 'conflict', entry: current };
    }
    await unlink(target).catch(() => {});
  }
  try {
    return { status: existing ? 'taken-over-stale' : 'taken', entry: await writeEntry(target, taskId, agent, nowMs) };
  } catch (error) {
    if (error?.code === 'EEXIST') {
      const other = await readTakeover(root, taskId);
      if (other === CORRUPT_TAKEOVER) {
        /* El marcado se corrompió entre la lectura y la escritura (o era
         * ilegible): retirarlo y reintentar una vez con escritura atómica. */
        await unlink(target).catch(() => {});
        return { status: 'taken-over-corrupt', entry: await writeEntry(target, taskId, agent, nowMs) };
      }
      if (other && !isStale(other, nowMs)) return { status: 'conflict', entry: other };
      return { status: 'conflict', entry: other ?? null };
    }
    throw error;
  }
}

/* Heartbeat: renueva la expiración de la toma propia (el agente que la tomó
 * sigue trabajando). Devuelve:
 * - { status: 'touched', entry }    renovada (nuevo expiresAt)
 * - { status: 'not-taken', entry: null }
 * - { status: 'corrupt', entry }    ilegible (no se renueva)
 * - { status: 'foreign', entry }    activa de otro agente (no se toca)
 *
 * El TTL es un recordatorio de “olvidada”, no un plazo real de trabajo: un
 * trabajo largo no debe expirar a mitad solo porque superó 6 h. Cada gate
 * del propio agente renueva su toma, de modo que un marcado activo con
 * actividad reciente NUNCA puede ser re-tomado como “olvidado” por otro. */
export async function touchTakeover(root, taskId, { by = defaultAgent(), nowMs = Date.now() } = {}) {
  sanitizeTaskId(taskId);
  const agent = sanitizeAgentName(by);
  const existing = await readTakeover(root, taskId);
  if (!existing) return { status: 'not-taken', entry: null };
  if (existing === CORRUPT_TAKEOVER) return { status: 'corrupt', entry: existing };
  if (existing.takenBy !== agent) return { status: 'foreign', entry: existing };
  const target = takeoverEntryPath(root, taskId);
  /* [018A-97] Compare-and-write: entre la lectura y la escritura otro agente
   * pudo re-tomar la tarea (marcado expirado con --force o taken-over-stale).
   * Sin esta re-verificación, el heartbeat pisaría la toma fresca ajena. Se
   * vuelve a leer justo antes de escribir y se aborta si el marcado cambió de
   * identidad (id) — el id es único por toma, así que compararlo por valor
   * detecta el re-toma aunque el objeto JSON sea nuevo (mismo patrón que
   * takeTask sobre stale). */
  const current = await readTakeover(root, taskId);
  if (!current || current === CORRUPT_TAKEOVER || current.id !== existing.id) {
    return { status: 'foreign', entry: current ?? null };
  }
  const entry = buildEntry(taskId, agent, nowMs);
  await writeAtomic(target, `${JSON.stringify(entry, null, 2)}\n`);
  return { status: 'touched', entry };
}

/* Decisión de cumplimiento para el gate: ¿puede este agente cerrar la tarea?
 * - 'active-foreign': tomada por otro agente activo → BLOQUEA
 * - 'own' / 'none' / 'corrupt' / 'stale-foreign' → no bloquea */
export function foreignTakeoverDecision({ entry, agent = defaultAgent(), nowMs = Date.now() } = {}) {
  if (!entry || entry === CORRUPT_TAKEOVER) return { blocked: false, reason: 'none' };
  if (entry.takenBy === agent) return { blocked: false, reason: 'own' };
  if (isStale(entry, nowMs)) return { blocked: false, reason: 'stale-foreign' };
  return { blocked: true, reason: 'active-foreign' };
}

/* Libera la tarea. Devuelve:
 * - { status: 'released', entry }       liberada por su autor
 * - { status: 'released-stale', entry } marcado expirado liberado por otro
 * - { status: 'not-taken', entry: null } no estaba tomada
 * - { status: 'conflict', entry }       activa y de otro agente (imposible
 *   liberar, ni con --force: el autor debe liberarla o esperar la expiración)
 *
 * Sin parámetro force: el propietario libera su toma; un marcado expirado
 * (>6 h, olvidado) puede liberarlo cualquier agente; un marcado ACTIVO de
 * otro agente SIEMPRE devuelve conflicto. Así un agente no puede desmarcar
 * la tarea que otro está trabajando. */
export async function releaseTask(root, taskId, { by = defaultAgent(), nowMs = Date.now() } = {}) {
  sanitizeTaskId(taskId);
  const agent = sanitizeAgentName(by);
  const target = takeoverEntryPath(root, taskId);
  const existing = await readTakeover(root, taskId);
  if (!existing) return { status: 'not-taken', entry: null };
  if (existing === CORRUPT_TAKEOVER) {
    /* Marcado ilegible: no es de nadie; cualquier agente puede retirarlo. */
    await unlink(target).catch(() => {});
    return { status: 'released-corrupt', entry: null };
  }
  if (existing.takenBy !== agent && !isStale(existing, nowMs)) {
    return { status: 'conflict', entry: existing };
  }
  await unlink(target).catch(error => {
    if (error?.code !== 'ENOENT') throw error;
  });
  return { status: existing.takenBy === agent ? 'released' : 'released-stale', entry: existing };
}

/* Recordatorios para el gate: el cierre debe recordar LIBERAR la tarea si el
 * agente la tomó, avisar si la tomó otro, y sugerir marcarla si no está. */
export function takeoverReminders({ taskId, entry, agent = defaultAgent(), nowMs = Date.now() } = {}) {
  if (!entry) {
    return [`Marca la tarea antes de trabajarla: npm run task:take -- --task ${taskId} --by <agente>`];
  }
  if (entry === CORRUPT_TAKEOVER) {
    return [`El marcado de ${taskId} es ilegible: re-tómala con npm run task:take -- --task ${taskId} --by <agente>`];
  }
  const stale = isStale(entry, nowMs);
  if (entry.takenBy === agent) {
    return [`Libera la tarea al terminar: npm run task:release -- --task ${taskId}`];
  }
  if (stale) {
    return [
      `El marcado de ${entry.takenBy} en ${taskId} expiró (olvidó liberarla): puedes re-tomarla con npm run task:take -- --task ${taskId} --by <agente> --force, o liberarla con npm run task:release -- --task ${taskId} (un marcado expirado lo libera cualquier agente; uno activo solo su autor)`,
    ];
  }
  return [
    `TAREA TOMADA por ${entry.takenBy} (${entry.id}) desde ${entry.takenAt} — expira ${entry.expiresAt}; no la trabajes en paralelo sin coordinar (npm run task:status)`,
  ];
}
