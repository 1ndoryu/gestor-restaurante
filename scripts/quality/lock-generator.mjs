import { access, copyFile, lstat, mkdir, readFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { writeAtomic } from './atomic-file.mjs';
import {
  LOCK_FILE,
  LOCK_SCHEMA_VERSION,
  assertInsideWorkspace,
  inspectInstalledAnalyzers,
  runtimeLockHash,
  validateLock,
} from './lockfile.mjs';

/* [por que] Destino por defecto de `sentinel install` (README §Instalación):
 * %LOCALAPPDATA%\GlorySentinel en Windows; si no hay LOCALAPPDATA, ~/.glory-sentinel. */
function runtimeRoot() {
  if (process.platform === 'win32' && process.env.LOCALAPPDATA) return path.join(process.env.LOCALAPPDATA, 'GlorySentinel');
  return path.join(os.homedir(), '.glory-sentinel');
}

/* [por que] El bloque runtime describe el Sentinel instalado globalmente (current.json
 * + manifest de la versión), no un valor fijo. El commit sale del analizador fijado y
 * solo se acepta si la versión instalada es la fijada: así el lock no declara un
 * runtime distinto del pin. Si algo no cuadra, falla sin escribir. */
async function inspectRuntime(analyzers) {
  const root = runtimeRoot();
  const pointer = await readJson(path.join(root, 'current.json')).catch(() => null);
  if (!pointer?.version || !pointer?.artifactSha256) {
    throw new Error('no hay runtime global de Sentinel; instala el checkout fijado con `sentinel install --source-root . --with-shims --with-path`');
  }
  const manifest = await readJson(path.join(root, 'versions', pointer.version, 'manifest.json')).catch(() => null);
  if (manifest?.artifactSha256 !== pointer.artifactSha256) {
    throw new Error(`runtime ${pointer.version}: manifest y current.json no coinciden en artifactSha256`);
  }
  const sentinel = analyzers.sentinel;
  if (!sentinel) throw new Error('falta analyzers.sentinel en quality-tools.json; no se puede fijar el runtime');
  if (sentinel.version !== pointer.version) {
    throw new Error(`runtime global ${pointer.version} != analizador fijado ${sentinel.version}; reinstala el runtime`);
  }
  return { status: 'installed', version: pointer.version, commit: sentinel.commit, artifactSha256: pointer.artifactSha256 };
}

function relativeWorkspacePath(value, label) {
  if (typeof value !== 'string' || value.length === 0 || path.isAbsolute(value) || value.replace(/\\\\/g, '/').split('/').includes('..')) {
    throw new Error(`${label} debe ser una ruta relativa dentro del workspace`);
  }
  return value;
}

function readJson(filePath) {
  return readFile(filePath, 'utf8').then(value => JSON.parse(value));
}

function runtimeEntry(runtime) {
  return {
    ...runtime,
    identitySha256: runtimeLockHash(runtime.status, runtime.version, runtime.commit),
  };
}

export function parseLockArgs(argv) {
  const options = { mode: 'check', cwd: process.cwd(), json: false };
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === '--check') options.mode = 'check';
    else if (value === '--write') options.mode = 'write';
    else if (value === '--json') options.json = true;
    else if (value === '--cwd') {
      options.cwd = argv[index + 1];
      if (!options.cwd || options.cwd.startsWith('--')) throw new Error('Falta valor para --cwd');
      index += 1;
    } else if (value.startsWith('--')) throw new Error(`Opción desconocida: ${value}`);
    else throw new Error(`Argumento inesperado: ${value}`);
  }
  return options;
}

function comparableLock(lock) {
  return {
    schemaVersion: lock.schemaVersion,
    runtime: lock.runtime,
    analyzers: lock.analyzers,
  };
}

function stableSerialize(value) {
  if (Array.isArray(value)) return `[${value.map(stableSerialize).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${stableSerialize(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

export function buildLock(analyzers, runtime, generatedAt = new Date().toISOString()) {
  return {
    schemaVersion: LOCK_SCHEMA_VERSION,
    generatedAt,
    runtime: runtimeEntry(runtime),
    analyzers: Object.fromEntries(Object.entries(analyzers).map(([name, value]) => [name, {
      version: value.version,
      protocolVersion: value.protocolVersion,
      commit: value.commit,
      sha256: value.sha256,
      patchSha256: value.patchSha256 ?? null,
      ...(value.capabilities === undefined ? {} : { capabilities: value.capabilities }),
      ...(value.sourcePathEnv === undefined ? {} : { sourcePathEnv: value.sourcePathEnv }),
    }])),
  };
}

export async function generateLock(workspaceRoot, { manifestPath = 'quality-tools.json' } = {}) {
  const root = await assertInsideWorkspace(workspaceRoot, workspaceRoot, 'workspace');
  const manifestPathAbsolute = await assertInsideWorkspace(
    root,
    path.join(root, relativeWorkspacePath(manifestPath, 'manifestPath')),
    'quality-tools.json',
  );
  const manifest = await readJson(manifestPathAbsolute);
  const analyzers = await inspectInstalledAnalyzers(root, manifest);
  const runtime = await inspectRuntime(analyzers);
  const lock = buildLock(analyzers, runtime);
  validateLock(lock, manifest);
  return { root, manifest, lock };
}

export async function checkLock(workspaceRoot, options = {}) {
  const { root, manifest, lock: expected } = await generateLock(workspaceRoot, options);
  const lockFile = relativeWorkspacePath(options.lockFile ?? LOCK_FILE, 'lockFile');
  const lockPath = await assertInsideWorkspace(root, path.join(root, lockFile), 'sentinel.lock.json', { allowMissing: true });
  let actual;
  try {
    actual = JSON.parse(await readFile(lockPath, 'utf8'));
  } catch (error) {
    if (error?.code === 'ENOENT') {
      return { ok: false, reason: 'missing', lockPath, expected, manifest };
    }
    throw error;
  }
  validateLock(actual, manifest);
  const ok = stableSerialize(comparableLock(actual)) === stableSerialize(comparableLock(expected));
  return { ok, reason: ok ? 'match' : 'mismatch', lockPath, actual, expected, manifest };
}

export async function writeLock(workspaceRoot, options = {}) {
  const result = await generateLock(workspaceRoot, options);
  const lockFile = relativeWorkspacePath(options.lockFile ?? LOCK_FILE, 'lockFile');
  const lockPath = await assertInsideWorkspace(result.root, path.join(result.root, lockFile), 'sentinel.lock.json', { allowMissing: true });
  await mkdir(path.dirname(lockPath), { recursive: true });
  const backupPath = `${lockPath}.bak`;
  try {
    const metadata = await lstat(lockPath);
    if (metadata.isSymbolicLink()) throw new Error('sentinel.lock.json: no se puede reemplazar un symlink');
    const backupMetadata = await lstat(backupPath).catch(error => {
      if (error?.code === 'ENOENT') return null;
      throw error;
    });
    if (backupMetadata?.isSymbolicLink()) throw new Error('sentinel.lock.json.bak: no se puede reemplazar un symlink');
    await copyFile(lockPath, backupPath);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  await writeAtomic(lockPath, `${JSON.stringify(result.lock, null, 2)}\n`);
  return { ...result, lockPath, backupPath, backupCreated: await fileExists(backupPath) };
}

async function fileExists(filePath) {
  try { await access(filePath); return true; } catch { return false; }
}

export async function main(argv = process.argv.slice(2)) {
  const options = parseLockArgs(argv);
  const workspaceRoot = path.resolve(options.cwd);
  const result = options.mode === 'write'
    ? await writeLock(workspaceRoot)
    : await checkLock(workspaceRoot);
  const output = {
    schemaVersion: 1,
    command: 'quality lock',
    mode: options.mode,
    status: result.ok === undefined ? 'written' : result.ok ? 'pass' : 'error',
    reason: result.reason ?? 'written',
    lockPath: result.lockPath,
    backupCreated: result.backupCreated ?? false,
  };
  if (options.json) process.stdout.write(`${JSON.stringify(output, null, 2)}\n`);
  else process.stdout.write(`[quality:lock] ${output.status}: ${output.reason}\n`);
  if (options.mode === 'check' && !result.ok) process.exitCode = 1;
  return result;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch(error => {
    process.stderr.write(`[quality:lock] ERROR: ${error.message}\n`);
    process.exitCode = 2;
  });
}
