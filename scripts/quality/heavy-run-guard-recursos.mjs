import crypto from 'node:crypto';
import { lstat, readFile, realpath, rename, unlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

/* Lectura y persistencia de recursos del guard: rutas, JSON de estado, proceso
 * vivo y marcadores del repo. Va aparte de heavy-run-guard.mjs para que el flujo
 * del lease no mezcle E/S de disco con la decisión. */

/* Defecto global 0 (decisión de la usuaria, 2026-10-12), espejo de glory-sentinel
 * scheduler.ts. Un proyecto lo activa con heavyRun.cooldownMinutes > 0. */
const DEFAULT_COOLDOWN_MS = 0;
const DEFAULT_TARGET_BASE = process.platform === 'win32'
  ? 'C:\\tmp\\glory-target'
  : path.join(os.tmpdir(), 'glory-target');

/* Ruta normalizada para comparar proyectos: Windows no distingue mayúsculas ni barras. */
export function normalizeRoot(value) {
  return path.resolve(value).replace(/\\/g, '/').toLowerCase();
}

/* Clave estable del proyecto en state.json: hash corto de la ruta normalizada. */
export function projectKey(projectRoot) {
  return crypto.createHash('sha256').update(normalizeRoot(projectRoot)).digest('hex').slice(0, 16);
}

export function resolveTargetBase() {
  return path.resolve(process.env.CARGO_TARGET_DIR_BASE || DEFAULT_TARGET_BASE);
}

export function resolveGuardRoot(targetBase = resolveTargetBase()) {
  return path.join(path.dirname(path.resolve(targetBase)), 'glory-quality-guard');
}

export function statePath(targetBase) {
  return path.join(resolveGuardRoot(targetBase), 'state.json');
}

export function activePath(targetBase) {
  return path.join(resolveGuardRoot(targetBase), 'active.json');
}

/* Un JSON ausente o corrupto no debe tumbar el guard: se usa el valor por defecto. */
export async function readJson(filePath, fallback) {
  try { return JSON.parse(await readFile(filePath, 'utf8')); }
  catch { return fallback; }
}

/* Escritura atómica: se escribe a un temporal y se renombra, así un corte no deja state.json a medias. */
export async function writeJsonAtomic(filePath, value) {
  const temporary = `${filePath}.${process.pid}.${Date.now()}.tmp`;
  await writeFile(temporary, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  try { await rename(temporary, filePath); }
  catch (error) {
    await unlink(temporary).catch(() => {});
    throw error;
  }
}

/* Signal 0 solo comprueba que el PID existe; sirve para detectar locks huérfanos. */
export function processAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try { process.kill(pid, 0); return true; }
  catch { return false; }
}

export function readCooldownMs(config = {}) {
  const minutes = Number(config.heavyRun?.cooldownMinutes);
  if (!Number.isFinite(minutes) || minutes < 0) return DEFAULT_COOLDOWN_MS;
  return minutes * 60 * 1000;
}

export async function readProjectConfig(projectRoot) {
  return readJson(path.join(projectRoot, 'quality.config.json'), {});
}

/* Sube por el árbol buscando quality.config.json y el guard como archivos regulares
 * (no symlinks ni junctions), para resolver el repo físico desde cualquier subcarpeta. */
export async function findQualityRoot(startPath = process.cwd()) {
  let candidate = path.resolve(startPath);
  try {
    candidate = await realpath(candidate);
  } catch (error) {
    if (error?.code === 'ENOENT') return path.resolve(startPath);
    throw error;
  }
  while (candidate) {
    const config = path.join(candidate, 'quality.config.json');
    const guard = path.join(candidate, 'scripts', 'quality', 'heavy-run-guard.mjs');
    try {
      const configMetadata = await lstat(config);
      const guardMetadata = await lstat(guard);
      if (!configMetadata.isFile() || !guardMetadata.isFile()) throw new Error('quality markers are not regular files');
      return candidate;
    } catch { /* Symlink/junction o marcador ausente: sube al padre. */ }
    const parent = path.dirname(candidate);
    if (parent === candidate) break;
    candidate = parent;
  }
  return path.resolve(startPath);
}
