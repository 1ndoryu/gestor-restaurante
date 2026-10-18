import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { captureBinary, projectRoot, run } from './setup-proceso.mjs';

/* Parches locales declarados en quality-tools.json. El SHA-256 fijado impide
 * que un árbol modificado a mano se dé por bueno sin avisar. */
async function assertAppliedPatch(name, toolRoot, expectedSha256) {
  const currentPatch = await captureBinary('git', ['diff', '--binary', '--no-ext-diff'], {
    cwd: toolRoot,
  });
  const currentSha256 = createHash('sha256').update(currentPatch).digest('hex');
  if (currentSha256 !== expectedSha256) {
    throw new Error(
      `${name}: el árbol parcheado difiere del patch declarado; revisa cambios locales antes de continuar`,
    );
  }
}

async function applyDeclaredPatch(name, config, toolRoot) {
  if (!config.patch) return null;

  const patchPath = path.resolve(projectRoot, config.patch.path);
  const relativePatchPath = path.relative(projectRoot, patchPath);
  if (relativePatchPath.startsWith('..') || path.isAbsolute(relativePatchPath)) {
    throw new Error(`${name}: la ruta del patch debe permanecer dentro del workspace`);
  }
  if (!/^[a-f0-9]{64}$/i.test(config.patch.sha256)) {
    throw new Error(`${name}: el manifest contiene un SHA-256 inválido`);
  }
  const patchBytes = await readFile(patchPath);
  const patchSha256 = createHash('sha256').update(patchBytes).digest('hex');
  if (patchSha256 !== config.patch.sha256) {
    throw new Error(`${name}: SHA-256 del patch no coincide; revisa el patch y actualiza el manifest`);
  }

  const check = await run('git', ['apply', '--check', patchPath], {
    cwd: toolRoot,
    capture: true,
    allowFailure: true,
  });
  if (typeof check === 'string' || check.code === 0) {
    await run('git', ['apply', patchPath], { cwd: toolRoot });
    await assertAppliedPatch(name, toolRoot, patchSha256);
    process.stdout.write(`[quality:setup] ${name}: patch aplicado (${patchSha256.slice(0, 12)})\n`);
    return patchSha256;
  }

  const reverseCheck = await run('git', ['apply', '--reverse', '--check', patchPath], {
    cwd: toolRoot,
    capture: true,
    allowFailure: true,
  });
  if (typeof reverseCheck === 'string' || reverseCheck.code === 0) {
    await assertAppliedPatch(name, toolRoot, patchSha256);
    process.stdout.write(`[quality:setup] ${name}: patch ya aplicado (${patchSha256.slice(0, 12)})\n`);
    return patchSha256;
  }

  throw new Error(
    `${name}: no se pudo aplicar ni reconocer el patch ${config.patch.path}\n${check.stderr || reverseCheck.stderr}`,
  );
}

export { applyDeclaredPatch };
