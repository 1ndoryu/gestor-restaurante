/* Árbol de reportes: mide tamaño, fechas y locks de cada rama o tarea sin
 * seguir symlinks (se rechazan o se omiten), para podar con datos reales. */
import { lstat, readdir } from 'node:fs/promises';
import path from 'node:path';
import { ACTIVE_LOCK_TTL_MS, TEMP_PATTERN } from './report-retention-config.mjs';
import { ownerIsActive, readOwner } from './report-retention-locks.mjs';

export async function inspectTree(root, now, { isLockTree = false } = {}) {
  const rootMetadata = await lstat(root).catch(error => {
    if (error?.code === 'ENOENT') return null;
    throw error;
  });
  if (rootMetadata?.isSymbolicLink()) throw new Error(`namespace de retención symlink rechazado: ${root}`);
  let bytes = 0;
  let newestMs = 0;
  let hasRecentWrite = false;
  let activeLock = false;
  let staleLock = false;
  let protectedTemp = false;
  let exists = true;
  let entries = [];
  try { entries = await readdir(root, { withFileTypes: true }); }
  catch (error) {
    if (error?.code === 'ENOENT') return { bytes, newestMs, hasRecentWrite, activeLock, staleLock, protectedTemp, exists: false };
    throw error;
  }
  if (isLockTree) {
    const owner = await readOwner(root);
    const startedAt = Date.parse(owner?.startedAt);
    activeLock = ownerIsActive(owner, now);
    staleLock = Boolean(owner && !activeLock && Number.isFinite(startedAt) && now - startedAt > ACTIVE_LOCK_TTL_MS);
  }
  for (const entry of entries) {
    const target = path.join(root, entry.name);
    const metadata = await lstat(target);
    if (metadata.isSymbolicLink()) continue;
    /* `mtime` representa la última escritura del reporte. `ctime` cambia al
     * crear/chmod en Windows y no debe mantener vivo un resultado antiguo. */
    const modifiedMs = metadata.mtimeMs;
    newestMs = Math.max(newestMs, modifiedMs);
    if (now - modifiedMs <= ACTIVE_LOCK_TTL_MS) hasRecentWrite = true;
    if (TEMP_PATTERN.test(entry.name) && now - modifiedMs <= ACTIVE_LOCK_TTL_MS) protectedTemp = true;
    if (entry.isDirectory()) {
      if (isLockTree && ownerIsActive(await readOwner(target), now)) activeLock = true;
      const child = await inspectTree(target, now, { isLockTree });
      bytes += child.bytes;
      newestMs = Math.max(newestMs, child.newestMs);
      hasRecentWrite ||= child.hasRecentWrite;
      activeLock ||= child.activeLock;
      staleLock ||= child.staleLock;
      protectedTemp ||= child.protectedTemp;
    } else if (entry.isFile()) {
      bytes += metadata.size;
    }
  }
  return { bytes, newestMs, hasRecentWrite, activeLock, staleLock, protectedTemp, exists };
}

export async function branchEntries(reportsRoot) {
  const entries = await readdir(reportsRoot, { withFileTypes: true });
  const result = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const target = path.join(reportsRoot, entry.name);
    const metadata = await lstat(target);
    if (metadata.isSymbolicLink()) throw new Error(`namespace de rama symlink rechazado: ${target}`);
    result.push({ name: entry.name, path: target });
  }
  return result;
}

export async function taskEntries(branchPath) {
  const entries = await readdir(branchPath, { withFileTypes: true });
  const result = [];
  for (const entry of entries) {
    if (!entry.isDirectory() || entry.name === 'cache' || entry.name === 'locks') continue;
    const target = path.join(branchPath, entry.name);
    const metadata = await lstat(target);
    if (metadata.isSymbolicLink()) throw new Error(`namespace de tarea symlink rechazado: ${target}`);
    result.push({ name: entry.name, path: target });
  }
  return result;
}
