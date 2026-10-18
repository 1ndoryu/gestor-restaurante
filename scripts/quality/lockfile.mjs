// Fachada del lock de herramientas de calidad (sentinel.lock.json).
// El esquema vive en lockfile-validacion.mjs y el estado git en lockfile-git.mjs; aquí quedan rutas, lectura del lock y verificación de analyzers.
import { createHash } from 'node:crypto';
import { access, readFile, realpath } from 'node:fs/promises';
import path from 'node:path';
import { runProcess } from './runner.mjs';
import { resolveConfiguredProvisionPath, resolveConfiguredSourcePath } from './source-path.mjs';
import { fail, LOCK_SCHEMA_VERSION, validateInstallRoot, validateLock } from './lockfile-validacion.mjs';
import { gitArchiveSha256, gitDiffSha256, gitStatusPorcelain, parentGitlinkCommit, untrustedCheckoutChanges } from './lockfile-git.mjs';

const LOCK_FILE = 'sentinel.lock.json';

export async function resolveToolRoot(workspaceRoot, name, config, manifest) {
  const configuredSourcePath = resolveConfiguredSourcePath(config, `quality-tools.json.tools.${name}`, { baseDir: workspaceRoot });
  const configuredProvisionPath = resolveConfiguredProvisionPath(config, `quality-tools.json.tools.${name}`, { baseDir: workspaceRoot });
  if (configuredProvisionPath !== null) {
    try {
      return await realpath(configuredProvisionPath);
    } catch {
      fail(`quality-tools.json.tools.${name}.provisionPath no existe o no es resoluble; ejecuta npm run quality:setup`);
    }
  }
  if (configuredSourcePath !== null) {
    try {
      return await realpath(configuredSourcePath);
    } catch {
      fail(`quality-tools.json.tools.${name}.sourcePath externo no existe o no es resoluble`);
    }
  }
  validateInstallRoot(manifest.installRoot);
  const installRoot = await assertInsideWorkspace(
    workspaceRoot,
    path.resolve(workspaceRoot, manifest.installRoot),
    'quality-tools.installRoot',
  );
  return assertInsideWorkspace(workspaceRoot, path.join(installRoot, name), `quality-tools.${name}`);
}

export async function assertInsideWorkspace(workspaceRoot, target, label, { allowMissing = false } = {}) {
  let rootReal;
  let targetReal;
  try {
    rootReal = await realpath(workspaceRoot);
    targetReal = await realpath(target);
  } catch (error) {
    if (!allowMissing || error?.code !== 'ENOENT') fail(`${label}: ruta inexistente o no resoluble`);
    const parentReal = await realpath(path.dirname(target)).catch(() => null);
    if (!parentReal) fail(`${label}: ruta inexistente o no resoluble`);
    targetReal = path.join(parentReal, path.basename(target));
  }
  const relative = path.relative(rootReal, targetReal);
  if (relative === '..' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    fail(`${label}: ruta fuera del workspace`);
  }
  return targetReal;
}

async function validateResolvedSourcePaths(workspaceRoot, lock, manifest) {
  for (const [name, config] of Object.entries(manifest.tools)) {
    const configuredSourcePath = resolveConfiguredSourcePath(config, `quality-tools.json.tools.${name}`, { baseDir: workspaceRoot });
    if (configuredSourcePath === null) continue;
    const expectedRealpath = await realpath(configuredSourcePath).catch(() => null);
    if (!expectedRealpath) fail(`analyzers.${name}.sourcePath no existe o no es resoluble`);
    if (lock.analyzers[name].sourcePathRealpath !== undefined && lock.analyzers[name].sourcePathRealpath !== expectedRealpath) {
      fail(`analyzers.${name}.sourcePathRealpath no coincide con el checkout actual`);
    }
  }
}

export async function readLock(workspaceRoot, manifest, lockFile = LOCK_FILE) {
  if (typeof lockFile !== 'string' || path.isAbsolute(lockFile) || lockFile.replace(/\\/g, '/').split('/').includes('..')) {
    fail('runtime.lockFile debe ser una ruta relativa dentro del workspace');
  }
  const lockPath = path.join(workspaceRoot, lockFile);
  try {
    await assertInsideWorkspace(workspaceRoot, lockPath, 'runtime.lockFile');
    const lock = JSON.parse(await readFile(lockPath, 'utf8'));
    validateLock(lock, manifest);
    await validateResolvedSourcePaths(workspaceRoot, lock, manifest);
    return { lock, lockPath };
  } catch (error) {
    if (error?.code === 'ENOENT') throw new Error(`Falta ${lockFile}; ejecuta el generador/verificador de lock antes del gate`);
    throw error;
  }
}

export function runtimeLockHash(status, version, commit) {
  return createHash('sha256').update(`sentinel-runtime:${status}:${version}:${commit}`).digest('hex');
}

export function assertRuntimeLockHash(runtime) {
  const expected = runtimeLockHash(runtime.status, runtime.version, runtime.commit);
  if (runtime.identitySha256 !== expected) fail('runtime.identitySha256 no coincide con su identidad');
  if (runtime.status === 'installed' && !runtime.artifactSha256) fail('runtime instalado requiere artifactSha256');
}

async function patchFileSha256(workspaceRoot, patchPath) {
  if (typeof patchPath !== 'string' || path.isAbsolute(patchPath) || patchPath.replace(/\\/g, '/').split('/').includes('..')) {
    throw new Error('quality-tools.patch.path debe ser una ruta relativa dentro del workspace');
  }
  const patchAbsolute = path.join(workspaceRoot, patchPath);
  await assertInsideWorkspace(workspaceRoot, patchAbsolute, 'quality-tools.patch.path');
  return createHash('sha256').update(await readFile(patchAbsolute)).digest('hex');
}

export async function inspectInstalledAnalyzers(workspaceRoot, manifest) {
  const results = {};
  if (manifest.installRoot !== undefined) validateInstallRoot(manifest.installRoot);
  for (const [name, config] of Object.entries(manifest.tools)) {
    const toolRoot = await resolveToolRoot(workspaceRoot, name, config, manifest);
    const configuredSourcePath = resolveConfiguredSourcePath(config, `quality-tools.json.tools.${name}`, { baseDir: workspaceRoot });
    const sourceRoot = configuredSourcePath === null ? toolRoot : await realpath(configuredSourcePath);
    const cliPath = path.join(toolRoot, config.cli);
    try {
      await access(cliPath);
    } catch {
      throw new Error(`Falta el CLI provisionado de ${name}; ejecuta npm run quality:setup`);
    }
    const status = await gitStatusPorcelain(sourceRoot);
    const untrustedChanges = untrustedCheckoutChanges(status.text);
    if (configuredSourcePath !== null && config.patch !== undefined) {
      throw new Error(`${name}: sourcePath no puede combinarse con patch local`);
    }
    const patchSha256 = config.patch?.sha256 ?? null;
    if (patchSha256 !== null) {
      const declaredSha = await patchFileSha256(workspaceRoot, config.patch.path);
      if (declaredSha !== patchSha256) throw new Error(`${name}: SHA-256 del patch declarado no coincide con quality-tools.json`);
    }
    let actualPatchSha = createHash('sha256').digest('hex');
    if (untrustedChanges.length > 0) {
      actualPatchSha = await gitDiffSha256(sourceRoot);
      const patchPaths = patchSha256 ? await declaredPatchPaths(workspaceRoot, config.patch.path) : new Set();
      const changedPaths = checkoutPaths(status.text);
      const onlyDeclaredPatch = patchSha256 !== null
        && actualPatchSha === patchSha256
        && [...changedPaths].every(file => patchPaths.has(file));
      if (!onlyDeclaredPatch) throw new Error(`${name}: checkout modificado; no se puede confiar en sentinel.lock.json (${untrustedChanges.join(', ')})`);
    }
    if (patchSha256 === null && actualPatchSha !== createHash('sha256').digest('hex')) {
      throw new Error(`${name}: checkout modificado sin patch declarado`);
    }
    const version = await runProcess(process.execPath, [cliPath, '--version'], { cwd: workspaceRoot, timeoutMs: 10_000 });
    if (version.code !== 0) throw new Error(`${name}: no se pudo leer la versión instalada`);
    const revision = await runProcess('git', ['-C', sourceRoot, 'rev-parse', 'HEAD'], { cwd: workspaceRoot, timeoutMs: 10_000 });
    if (revision.code !== 0) throw new Error(`${name}: no se pudo leer el commit instalado`);
    const sha256 = await gitArchiveSha256(sourceRoot);
    if (patchSha256 !== null && actualPatchSha !== patchSha256) {
      throw new Error(`${name}: patch aplicado no coincide con quality-tools.json`);
    }
    if (configuredSourcePath !== null && revision.stdout.trim() !== config.commit) {
      throw new Error(`${name}: sourcePath externo no coincide con el commit fijado`);
    }
    if (configuredSourcePath !== null) {
      const gitlink = await parentGitlinkCommit(workspaceRoot, configuredSourcePath);
      if (!gitlink) {
        const stagedGitlink = await parentGitlinkCommit(workspaceRoot, configuredSourcePath, true);
        if (stagedGitlink) {
          throw new Error(`${name}: el gitlink está preparado en el índice (${stagedGitlink}), pero aún no está committeado en HEAD; committea el pin antes del lock/gate`);
        }
        throw new Error(`${name}: sourcePath interno no está representado por un gitlink en HEAD; inicializa el submódulo y committea el pin antes del lock/gate`);
      }
      if (gitlink !== revision.stdout.trim()) throw new Error(`${name}: gitlink del workspace no coincide con el checkout instalado`);
      if (gitlink !== config.commit) throw new Error(`${name}: gitlink no coincide con quality-tools.json`);
    }
    results[name] = {
      version: version.stdout.trim(),
      protocolVersion: Number(config.outputSchemaVersion),
      commit: revision.stdout.trim(),
      sha256,
      patchSha256,
      ...(config.capabilities === undefined ? {} : { capabilities: config.capabilities }),
      ...(config.sourcePathEnv === undefined ? {} : { sourcePathEnv: config.sourcePathEnv, sourcePathRealpath: toolRoot }),
      cliPath,
    };
  }
  return results;
}

function checkoutPaths(statusOutput) {
  return new Set(untrustedCheckoutChanges(statusOutput).map(line => line.slice(3).replace(/\\/g, '/')));
}

async function declaredPatchPaths(workspaceRoot, patchPath) {
  if (typeof patchPath !== 'string' || path.isAbsolute(patchPath) || patchPath.replace(/\\/g, '/').split('/').includes('..')) {
    throw new Error('quality-tools.patch.path debe ser una ruta relativa dentro del workspace');
  }
  const patch = await readFile(path.join(workspaceRoot, patchPath), 'utf8');
  const paths = new Set();
  for (const line of patch.split('\n')) {
    const match = /^diff --git a\/(.+) b\/(.+)$/u.exec(line);
    if (match) {
      paths.add(match[1]);
      paths.add(match[2]);
    }
  }
  if (paths.size === 0) throw new Error('quality-tools.patch no contiene rutas diff válidas');
  return paths;
}

export async function verifyInstalledAnalyzers(workspaceRoot, manifest, lock) {
  const inspected = await inspectInstalledAnalyzers(workspaceRoot, manifest);
  const results = {};
  for (const [name, installed] of Object.entries(inspected)) {
    const expected = lock.analyzers[name];
    if (!expected || installed.version !== expected.version) {
      throw new Error(`${name}: versión instalada no coincide con sentinel.lock.json`);
    }
    if (installed.commit !== expected.commit) {
      throw new Error(`${name}: commit instalado no coincide con sentinel.lock.json`);
    }
    if (installed.sha256 !== expected.sha256) {
      throw new Error(`${name}: SHA-256 del árbol instalado no coincide con sentinel.lock.json`);
    }
    if (installed.patchSha256 !== (expected.patchSha256 ?? null)) {
      throw new Error(`${name}: SHA-256 del patch instalado no coincide con sentinel.lock.json`);
    }
    if (installed.sourcePathEnv !== expected.sourcePathEnv) {
      throw new Error(`${name}: sourcePath externo instalado no coincide con sentinel.lock.json`);
    }
    if (expected.sourcePathRealpath !== undefined && installed.sourcePathRealpath !== expected.sourcePathRealpath) {
      throw new Error(`${name}: sourcePathRealpath instalado no coincide con sentinel.lock.json`);
    }
    results[name] = installed;
  }
  return results;
}

// Reexporta lo público movido a los módulos hermanos: los importadores siguen usando './lockfile.mjs'.
export { LOCK_FILE, LOCK_SCHEMA_VERSION, gitArchiveSha256, untrustedCheckoutChanges, validateInstallRoot, validateLock };
