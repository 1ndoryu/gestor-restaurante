import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { inspectInstalledAnalyzers } from './lockfile.mjs';
import { resolveConfiguredSourcePath } from './source-path.mjs';
import { exists, npmCliPath, projectRoot, run } from './setup-proceso.mjs';
import { applyDeclaredPatch } from './setup-parches.mjs';
import { ensureSourcePathReady } from './setup-fuente.mjs';

const manifestPath = path.join(projectRoot, 'quality-tools.json');

async function readManifest() {
  const parsed = JSON.parse(await readFile(manifestPath, 'utf8'));
  if (parsed.schemaVersion !== 1 || !parsed.installRoot || !parsed.tools) {
    throw new Error('quality-tools.json no cumple schemaVersion 1');
  }
  return parsed;
}

async function installTool(name, config, installRoot) {
  const configuredSourcePath = resolveConfiguredSourcePath(config, `quality-tools.json.tools.${name}`, { baseDir: projectRoot });
  if (configuredSourcePath !== null) {
    if (config.patch) throw new Error(`${name}: sourcePath no puede combinarse con patch local`);
    const { currentCommit, installedVersion } = await ensureSourcePathReady(name, config);
    process.stdout.write(`[quality:setup] ${name}: sourcePath y provisión verificados, no se modifica el submódulo\\n`);
    return {
      commit: currentCommit,
      version: installedVersion,
      patchSha256: null,
      sourcePathEnv: config.sourcePathEnv,
      cli: config.cli,
    };
  }
  const toolRoot = path.join(installRoot, name);
  const gitRoot = path.join(toolRoot, '.git');
  const markerPath = path.join(toolRoot, '.quality-install.json');
  process.stdout.write(`[quality:setup] ${name}: preparando ${config.version}\n`);

  if (!await exists(toolRoot)) {
    await run('git', ['clone', '--filter=blob:none', '--no-checkout', config.repository, toolRoot]);
    await run('git', ['checkout', '--detach', config.commit], { cwd: toolRoot });
  } else {
    if (!await exists(gitRoot)) {
      throw new Error(`${toolRoot} existe pero no es un checkout administrado; muévelo y repite quality:setup`);
    }
    const currentCommit = await run('git', ['rev-parse', 'HEAD'], { cwd: toolRoot, capture: true });
    if (currentCommit !== config.commit) {
      throw new Error(`${name} está en ${currentCommit}; se esperaba ${config.commit}. Reinstala .quality-tools/${name}`);
    }
  }

  const patchSha256 = await applyDeclaredPatch(name, config, toolRoot);
  const cliPath = path.join(toolRoot, config.cli);
  if (await exists(markerPath) && await exists(cliPath)) {
    const marker = JSON.parse(await readFile(markerPath, 'utf8'));
    const installedVersion = await run(process.execPath, [cliPath, '--version'], { capture: true });
    if (
      marker.commit === config.commit
      && marker.patchSha256 === patchSha256
      && marker.testScript === (config.testScript ?? null)
      && installedVersion === config.version
    ) {
      process.stdout.write(`[quality:setup] ${name}: PASS (cache)\n`);
      return {
        commit: config.commit,
        version: installedVersion,
        patchSha256,
        cli: path.relative(projectRoot, cliPath),
      };
    }
  }

  await run(process.execPath, [npmCliPath, 'ci', '--ignore-scripts'], { cwd: toolRoot });
  await run(process.execPath, [npmCliPath, 'run', config.buildScript], { cwd: toolRoot });
  if (config.testScript) {
    await run(process.execPath, [npmCliPath, 'run', config.testScript], { cwd: toolRoot });
  }
  const installedVersion = await run(process.execPath, [cliPath, '--version'], { capture: true });
  if (installedVersion !== config.version) {
    throw new Error(`${name} reporta ${installedVersion}; se esperaba ${config.version}`);
  }
  const markerTemporaryPath = `${markerPath}.tmp`;
  await writeFile(
    markerTemporaryPath,
    `${JSON.stringify({
      commit: config.commit,
      version: installedVersion,
      patchSha256,
      testScript: config.testScript ?? null,
    })}\n`,
    'utf8',
  );
  await rename(markerTemporaryPath, markerPath);
  process.stdout.write(`[quality:setup] ${name}: PASS\n`);
  return {
    commit: config.commit,
    version: installedVersion,
    patchSha256,
    cli: path.relative(projectRoot, cliPath),
  };
}

/* [10AA-3] `--tool a,b` limita el setup a esas herramientas. Hace falta porque el
 * setup monta las herramientas en orden y aborta en la primera que no se puede: con
 * sentinel 0.7.27 fijado, su `test:unit` falla en un checkout limpio (`rules.md`
 * desfasado del registro, defecto del propio tag), así que `quality:setup` a secas
 * nunca llegaba a montar varsense. Sin filtro, el comportamiento es el de siempre. */
function herramientasPedidas(manifest) {
  const i = process.argv.indexOf('--tool');
  if (i < 0) return manifest.tools;
  const pedidas = String(process.argv[i + 1] ?? '').split(',').map(nombre => nombre.trim()).filter(Boolean);
  if (pedidas.length === 0) throw new Error('--tool necesita una lista de herramientas: --tool sentinel,varsense');
  const desconocidas = pedidas.filter(nombre => manifest.tools[nombre] === undefined);
  if (desconocidas.length > 0) throw new Error(`--tool: no están en quality-tools.json: ${desconocidas.join(', ')}`);
  return Object.fromEntries(pedidas.map(nombre => [nombre, manifest.tools[nombre]]));
}

async function main() {
  const manifest = await readManifest();
  const tools = herramientasPedidas(manifest);
  const externalOnly = Object.values(tools).every(config => resolveConfiguredSourcePath(config, 'quality-tools.json.tools', { baseDir: projectRoot }) !== null);
  if (externalOnly) {
    /* [028A-8] Un sourcePath interno (submódulo) debe estar inicializado y
     * compilado antes de inspeccionar, para que un clon limpio sea reproducible. */
    for (const [name, config] of Object.entries(tools)) {
      if (resolveConfiguredSourcePath(config, `quality-tools.json.tools.${name}`, { baseDir: projectRoot }) === null) continue;
      await ensureSourcePathReady(name, config);
    }
    const inspected = await inspectInstalledAnalyzers(projectRoot, { ...manifest, tools });
    for (const [name, tool] of Object.entries(inspected)) {
      process.stdout.write(`[quality:setup] ${name}: sourcePath verificado (${tool.commit}), no se modifica .quality-tools\\n`);
    }
    process.stdout.write('[quality:setup] Checkouts listos. Próximo: npm run task:check -- <ID>\\n');
    return;
  }

  const installRoot = path.resolve(projectRoot, manifest.installRoot);
  await mkdir(installRoot, { recursive: true });
  const installed = {};

  for (const [name, config] of Object.entries(tools)) {
    installed[name] = await installTool(name, config, installRoot);
  }

  const statePath = path.join(installRoot, 'install-state.json');
  const temporaryPath = `${statePath}.tmp`;
  await writeFile(temporaryPath, `${JSON.stringify({ schemaVersion: 1, installed }, null, 2)}\n`, 'utf8');
  await rename(temporaryPath, statePath);
  process.stdout.write('[quality:setup] Herramientas listas. Próximo: npm run task:check -- <ID>\n');
}

main().catch(error => {
  process.stderr.write(`[quality:setup] ERROR: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 2;
});
