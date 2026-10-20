import { cp, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { resolveConfiguredProvisionPath, resolveConfiguredSourcePath } from './source-path.mjs';
import { captureBinary, exists, isolatedNpmEnvironment, npmCliPath, projectRoot, run } from './setup-proceso.mjs';

/* Fuente y staging de sourcePath: compila y prueba en un árbol aislado para
 * certificar el commit fijado sin ensuciar el submódulo versionado. */
/* [259A-5] La suite de Sentinel 0.7.14 (700 tests, ~5 min medidos) supera el
 * presupuesto fijo de 300 s del runner y el setup abortaba sin provisionar.
 * Espejo de WANDORIUS [109A-9]: el presupuesto es un suelo del runner, no un
 * techo del test; se declara explícito para ci/compile/suite en staging. */
const STAGE_TIMEOUT_MS = 900_000;

async function writeReleaseEvidence(name, config, commit) {
  const evidenceRoot = path.join(projectRoot, '.sentinel', 'release-evidence');
  await mkdir(evidenceRoot, { recursive: true });
  await writeFile(path.join(evidenceRoot, `${name}.json`), `${JSON.stringify({
    schemaVersion: 1,
    tool: name,
    commit,
    compile: 'passed',
    suite: config.testScript ? 'passed' : 'not-configured',
    cleanStaging: true,
    at: new Date().toISOString(),
  }, null, 2)}\n`, 'utf8');
}

async function stageSourcePathBuild(name, config, toolRoot) {
  const provisionRoot = resolveConfiguredProvisionPath(config, `quality-tools.json.tools.${name}`, { baseDir: projectRoot });
  if (provisionRoot === null) throw new Error(`${name}: sourcePath requiere provisionPath para conservar artefactos fuera del checkout`);
  const sourceReal = path.resolve(toolRoot);
  const provisionReal = path.resolve(provisionRoot);
  const relativeProvision = path.relative(sourceReal, provisionReal);
  if (relativeProvision === '' || (!relativeProvision.startsWith('..') && !path.isAbsolute(relativeProvision))) {
    throw new Error(`${name}: provisionPath no puede estar dentro del sourcePath versionado`);
  }
  const currentCommit = await run('git', ['rev-parse', 'HEAD'], { cwd: toolRoot, capture: true });
  if (currentCommit !== config.commit) {
    throw new Error(`${name}: sourcePath está en ${currentCommit}; se esperaba ${config.commit}`);
  }
  const beforeStatus = await run('git', ['status', '--porcelain=v1', '--untracked-files=all'], { cwd: toolRoot, capture: true });
  const stagingRoot = await mkdtemp(path.join(os.tmpdir(), `glory-quality-${name}-`));
  try {
    /* [SNT-16f] El staging se materializa desde `git archive HEAD`: solo el
     * árbol commiteado (sin cambios sin commitear ni artefactos locales), de
     * modo que compile + suite certifican exactamente el commit fijado.
     * [10AA-3] `-c core.autocrlf=false`: `git archive` aplica la conversión de
     * fin de línea de la máquina que archiva, y en el portátil y en la 89 está
     * en `true`. El staging salía con CRLF y la suite de sentinel moría en
     * `check:reglas` («rules.md está desfasado del registro») aunque el commit
     * fuera correcto: el blob del commit es LF. El adaptador compartido ya lo
     * hacía; esto era el mismo fallo por la otra puerta. */
    const archive = await captureBinary('git', ['-c', 'core.autocrlf=false', '-c', 'core.eol=lf', 'archive', '--format=tar', 'HEAD'], { cwd: toolRoot });
    /* Windows bsdtar interpreta `C:\...` como host remoto; rutas en forward-slash
     * y `-C` fuerzan extracción local portable; `--force-local` no se usa
     * porque bsdtar de Windows 3.8.4 lo rechaza. */
    /* [048A-12] bsdtar de Windows no soporta `--force-local` y trata una
     * ruta `C:/...` como host remoto. El tar entra por stdin y el cwd ya es
     * staging: no existe ninguna ruta que bsdtar pueda reinterpretar. */
    await run('tar', ['-xf', '-'], { cwd: stagingRoot, capture: true, input: archive });
    const env = isolatedNpmEnvironment();
    env.GLORY_QUALITY_SETUP = '1';
    await run(process.execPath, [npmCliPath, 'ci', '--ignore-scripts'], { cwd: stagingRoot, env, timeoutMs: STAGE_TIMEOUT_MS });
    await run(process.execPath, [npmCliPath, 'run', config.buildScript], { cwd: stagingRoot, env, timeoutMs: STAGE_TIMEOUT_MS });
    if (config.testScript) {
      const testArgs = [npmCliPath, 'run', config.testScript];
      await run(process.execPath, testArgs, { cwd: stagingRoot, env, timeoutMs: STAGE_TIMEOUT_MS });
    }

    /* [SNT-16f] El checkout versionado no recibe node_modules ni artefactos.
     * La provisión utilizable vive en el cache declarado, fuera del submódulo;
     * el gate posterior inspecciona ese cache aislado. */
    await rm(provisionRoot, { recursive: true, force: true });
    await mkdir(provisionRoot, { recursive: true });
    const artifactRoots = new Set(['package.json', 'package-lock.json', 'node_modules', String(config.cli).split(/[\\/]/u)[0]]);
    for (const artifact of artifactRoots) {
      const sourceArtifact = path.join(stagingRoot, artifact);
      if (await exists(sourceArtifact)) await cp(sourceArtifact, path.join(provisionRoot, artifact), { recursive: true });
    }
    const afterStatus = await run('git', ['status', '--porcelain=v1', '--untracked-files=all'], { cwd: toolRoot, capture: true });
    if (afterStatus !== beforeStatus) {
      throw new Error(`${name}: el provisioning aislado modificó archivos versionados del submódulo`);
    }
    const commit = await run('git', ['rev-parse', 'HEAD'], { cwd: toolRoot, capture: true });
    await writeReleaseEvidence(name, config, commit);
  } finally {
    await rm(stagingRoot, { recursive: true, force: true });
    /* El archivo tar nunca se materializa: el staging no deja temporales. */
  }
}

async function ensureSourcePathReady(name, config) {
  const configuredSourcePath = resolveConfiguredSourcePath(config, `quality-tools.json.tools.${name}`, { baseDir: projectRoot });
  const toolRoot = path.resolve(configuredSourcePath);
  const relativeFromRoot = path.relative(projectRoot, toolRoot);
  const isInsideWorkspace = relativeFromRoot !== ''
    && !relativeFromRoot.startsWith('..')
    && !path.isAbsolute(relativeFromRoot);
  if (!await exists(path.join(toolRoot, '.git'))) {
    if (!isInsideWorkspace) throw new Error(`${name}: sourcePath no contiene un checkout Git válido`);
    /* [028A-8] Clon limpio: el submódulo interno puede no estar inicializado. */
    process.stdout.write(`[quality:setup] ${name}: inicializando submódulo ${relativeFromRoot}\n`);
    await run('git', ['submodule', 'update', '--init', '--', relativeFromRoot.replace(/\\/g, '/')], { cwd: projectRoot });
  }
  if (isInsideWorkspace) {

    /* [SNT-16f] Cada setup interno recompila y prueba en staging aislado. */
    process.stdout.write(`[quality:setup] ${name}: compile + suite en staging aislado ${relativeFromRoot}\n`);
    await stageSourcePathBuild(name, config, toolRoot);
  } else if (!await exists(path.join(toolRoot, config.cli))) {
    throw new Error(`${name}: sourcePath no contiene un CLI válido; compílalo manualmente en ${toolRoot}`);
  }
  const currentCommit = await run('git', ['rev-parse', 'HEAD'], { cwd: toolRoot, capture: true });
  if (currentCommit !== config.commit) {
    throw new Error(`${name}: sourcePath está en ${currentCommit}; se esperaba ${config.commit}`);
  }
  const provisionRoot = resolveConfiguredProvisionPath(config, `quality-tools.json.tools.${name}`, { baseDir: projectRoot });
  const installedRoot = provisionRoot ?? toolRoot;
  const installedVersion = await run(process.execPath, [path.join(installedRoot, config.cli), '--version'], { capture: true });
  if (installedVersion !== config.version) {
    throw new Error(`${name}: provisión reporta ${installedVersion}; se esperaba ${config.version}`);
  }
  return { toolRoot, provisionRoot, currentCommit, installedVersion };
}

export { ensureSourcePathReady };
