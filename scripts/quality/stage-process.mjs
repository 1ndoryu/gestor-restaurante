#!/usr/bin/env node
/* [SNT-14] Adaptador de proceso para `sentinel check --stages`: ejecuta una
 * etapa del proyecto y escribe únicamente el contrato JSON que Sentinel
 * consume. Scheduler, locks, reportería final y exit semantics pertenecen al
 * core; este wrapper solo resuelve el adapter y produce una salida versionada. */
import path from 'node:path';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { readFileSync } from 'node:fs';
import { stageDefinitions } from './stage-definitions.mjs';
import { preflight, projectRoot } from './preflight.mjs';
import { detectScope, manifestToScope } from './scope.mjs';
import { readAdapterManifest, adapterStageNames, manifestStageNames, adapterEnvironmentAllowlist, resolveWorkspacePath, assertTaskId } from './adapter-manifest.mjs';
import { DEFAULT_ENV_ALLOWLIST } from './runner.mjs';
import { sentinelTransportExitCode } from './sentinel-stage-contract.mjs';

function parseArgs(argv) {
  const parsed = { reportPath: null, taskId: null, stage: null, full: false, ci: false, profile: null, scopeManifest: null };
  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    if (arg === '--stage') parsed.stage = argv[++index] ?? null;
    else if (arg === '--report') parsed.reportPath = argv[++index] ?? null;
    else if (arg === '--task-id') parsed.taskId = argv[++index] ?? null;
    else if (arg === '--scope-manifest') parsed.scopeManifest = argv[++index] ?? null;
    else if (arg === '--full') parsed.full = true;
    else if (arg === '--ci') parsed.ci = true;
    else if (arg === '--profile') parsed.profile = argv[++index] ?? null;
  }
  return parsed;
}

/* [10AA-3] Sentinel filtra el entorno al lanzar las etapas (su `toolRunner.ts` tiene su
 * propia lista blanca y no incluye `CM_HEAVY_HELD`). Sin esa marca, el `cm-heavy` de la
 * etapa cree que viene de fuera, espera 600 s un candado que ya tiene su propio abuelo y
 * el gate sale rojo como si el código del proyecto fallara.
 * Mirar el descriptor heredado no sirve: Node lo cierra y reutiliza el número; y preguntar
 * quién tiene el candado tampoco, porque `/proc/locks` nombra al `flock(1)` que lo pidió, que
 * ya ha muerto. Lo que sí vale es el buzón que deja `cm-heavy` con su PID: si ese PID está en
 * nuestra cadena de padres, el candado es de un ancestro. Se pone la marca y `safeEnvironment`
 * (que ya la permite) la propaga a los pasos.
 * Solo Linux: sin `/proc` no hay nada que averiguar y se deja el entorno como venga. */
function marcarSiElCandadoEsDeUnAncestro(rutaDueno = '/run/lock/cm-heavy.owner') {
  if (process.platform !== 'linux' || process.env.CM_HEAVY_HELD) return false;
  let dueno;
  try {
    dueno = Number(readFileSync(rutaDueno, 'utf8').trim());
  } catch {
    return false; // sin buzón: nadie ha pasado por cm-heavy
  }
  if (!Number.isInteger(dueno) || dueno <= 1) return false;
  try {
    /* El número solo vale si ese proceso sigue dentro de un scope transitorio: cm-heavy
     * `exec`a el scope conservando el PID, así que el buzón lo delata. Si el número se
     * recicló tras un build viejo, el nuevo dueño no estará en uno. */
    if (!/\/run-r[0-9a-f]+\.scope/.test(readFileSync(`/proc/${dueno}/cgroup`, 'utf8'))) {
      return false;
    }
  } catch {
    return false;
  }
  let actual = process.pid;
  for (let saltos = 0; saltos < 64 && actual > 1; saltos += 1) {
    if (actual === dueno) {
      process.env.CM_HEAVY_HELD = '1';
      return true;
    }
    try {
      /* En `/proc/<pid>/stat` el nombre va entre paréntesis y puede traer espacios:
       * el ppid (campo 4) se busca detrás del último `)`. */
      const stat = readFileSync(`/proc/${actual}/stat`, 'utf8');
      actual = Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1]);
    } catch {
      return false;
    }
  }
  return false;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.stage || !args.reportPath || !args.taskId) {
    process.stderr.write('[stage-process] requiere --stage, --report y task-id seguro\n');
    process.exitCode = 2;
    return;
  }
  assertTaskId(args.taskId);
  /* Antes de nada: recuperar la marca de `cm-heavy` que Sentinel filtra al lanzarnos. */
  marcarSiElCandadoEsDeUnAncestro();
  const adapter = await readAdapterManifest(projectRoot);
  const profiles = args.profile ? args.profile.split(',').map(item => item.trim()).filter(Boolean) : [];
  const allowedStages = profiles.length > 0 || args.full || args.ci
    ? adapterStageNames(adapter, profiles, args.full || args.ci)
    : manifestStageNames(adapter);
  if (!allowedStages.includes(args.stage)) {
    process.stderr.write(`[stage-process] etapa no declarada por el adapter: ${args.stage}\n`);
    process.exitCode = 2;
    return;
  }
  const reportPath = resolveWorkspacePath(projectRoot, args.reportPath, '--report', { allowReportRoot: true });
  const reportRoot = path.dirname(reportPath);
  const scopeArgs = { full: args.full, ci: args.ci, profiles };
  const context = await preflight({ taskId: args.taskId, cwd: projectRoot, ...scopeArgs });
  context.reportRoot = reportRoot;
  context.logsRoot = path.join(reportRoot, 'logs');
  context.adapterEnvironmentAllowlist = adapterEnvironmentAllowlist(adapter, DEFAULT_ENV_ALLOWLIST);
  await mkdir(context.logsRoot, { recursive: true });
  const scope = args.scopeManifest
    ? manifestToScope(JSON.parse(await readFile(resolveWorkspacePath(projectRoot, args.scopeManifest, '--scope-manifest', { allowReportRoot: true }), 'utf8')), reportRoot)
    : await detectScope(context, scopeArgs);
  if (args.scopeManifest) await writeFile(scope.changedFilesPath, `${scope.files.join('\n')}\n`, 'utf8');
  const definition = stageDefinitions(context, scope, args.taskId, adapter).find(item => item.name === args.stage);
  if (!definition) {
    process.stderr.write(`[stage-process] etapa no implementada por el adapter: ${args.stage}\n`);
    process.exitCode = 2;
    return;
  }
  const result = await definition.run();
  const report = {
    schemaVersion: String(adapter.adapter.output.schemaVersion),
    stage: result.stage ?? args.stage,
    entries: [{ findings: (result.findings ?? []).map(item => ({
      ruleId: String(item.ruleId ?? 'unknown'),
      severity: String(item.severity ?? 'warning'),
      ...(item.file ? { file: String(item.file).replace(/\\/g, '/') } : {}),
      ...(Number.isInteger(item.line) ? { line: item.line } : {}),
      message: String(item.message ?? 'Hallazgo sin mensaje'),
      ...(item.help ? { help: String(item.help) } : {}),
    })) }],
    summary: result.summary ?? '',
    durationMs: result.durationMs ?? 0,
  };
  await mkdir(reportRoot, { recursive: true });
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.exitCode = sentinelTransportExitCode();
}

try {
  await main();
} catch (error) {
  process.stderr.write(`[stage-process] SETUP ERROR — ${error.message}\n`);
  process.exitCode = 2;
}
