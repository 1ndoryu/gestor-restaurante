/* Seguridad de rutas: confina cada escritura o borrado a .quality-reports y
 * rechaza symlinks, para que una ruta manipulada no salga del workspace. */
import { lstat, mkdir, realpath, rm } from 'node:fs/promises';
import path from 'node:path';

export async function assertReportsRoot(workspaceRoot, reportsRoot) {
  const qualityRoot = path.resolve(workspaceRoot, '.quality-reports');
  const expected = path.join(qualityRoot, 'branches');
  const actual = path.resolve(reportsRoot);
  if (actual !== expected) throw new Error('reportRoot fuera del namespace permitido');

  for (const parent of [qualityRoot, actual]) {
    const existing = await lstat(parent).catch(error => {
      if (error?.code === 'ENOENT') return null;
      throw error;
    });
    if (existing?.isSymbolicLink()) throw new Error('reportRoot no puede ser symlink');
  }
  await mkdir(actual, { recursive: true });
  const resolved = await realpath(actual);
  if (path.resolve(resolved) !== expected) throw new Error('reportRoot resuelve fuera del workspace');
  return actual;
}

export async function assertBeforeRemove(workspaceRoot, target) {
  const qualityRoot = path.resolve(workspaceRoot, '.quality-reports');
  const resolved = await realpath(target);
  const relative = path.relative(qualityRoot, resolved);
  if (relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error('target de retención fuera de .quality-reports');
  }
  const metadata = await lstat(target);
  if (metadata.isSymbolicLink()) throw new Error('target de retención no puede ser symlink');
}

export async function removeTreeSafely(projectRoot, target) {
  try {
    await assertBeforeRemove(projectRoot, target);
    await rm(target, { recursive: true, force: true });
    return true;
  } catch (error) {
    if (error?.code === 'ENOENT') return false;
    throw error;
  }
}
