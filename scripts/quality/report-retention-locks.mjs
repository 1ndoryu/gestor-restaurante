/* Dueños de lock: un lock sigue activo si su PID vive o si arrancó dentro del
 * TTL; así el lock de un proceso caído deja de contar tras el TTL. */
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { ACTIVE_LOCK_TTL_MS } from './report-retention-config.mjs';

export async function readOwner(lockPath) {
  try { return JSON.parse(await readFile(path.join(lockPath, 'owner.json'), 'utf8')); }
  catch { return null; }
}

function pidAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try { process.kill(pid, 0); return true; }
  catch (error) { return error?.code === 'EPERM'; }
}

export function ownerIsActive(owner, now) {
  if (!owner) return false;
  const startedAt = Date.parse(owner.startedAt);
  return pidAlive(Number(owner.pid)) || (Number.isFinite(startedAt) && now - startedAt <= ACTIVE_LOCK_TTL_MS);
}
