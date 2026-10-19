import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { runProcess, DEFAULT_ENV_ALLOWLIST } from '../runner.mjs';
import { conciseFailure, npmInvocation, writeStageLog } from './common.mjs';

/* [por que] El frontend se valida en la VPS 89, igual que Rust (AGENTS §1): el portátil
 * no compila ni consume recursos del taller. Se reutiliza el taller remoto de
 * workspace-manager (sync Mutagen + ssh); en la propia 89 se ejecuta en local. */
const RCARGO_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..', 'workspace-manager', 'scripts', 'remoto');

async function cargarTaller() {
  return import(pathToFileURL(path.join(RCARGO_DIR, 'rcargo.mjs')).href);
}

async function runCommand(context, name, executable, args, timeoutMs) {
  const execution = await runProcess(executable, args, {
    cwd: context.projectRoot,
    timeoutMs,
    envAllowlist: context.adapterEnvironmentAllowlist ?? DEFAULT_ENV_ALLOWLIST,
  });
  return { name, execution };
}

/* [por que] Sincroniza la réplica antes de ejecutar: sin el flush, la 89 podría validar un
 * árbol antiguo. Un fallo de Mutagen se devuelve como código 2 (infraestructura, no código roto). */
async function runEnVps(context, taller, comando, timeoutMs) {
  const config = taller.cargarConfig();
  const proyecto = taller.detectarProyecto(context.projectRoot);
  const rutas = taller.rutasRemotas(config, proyecto, taller.detectarRama(context.projectRoot));
  const sync = taller.asegurarSesion({ config, proyecto, rutaLocal: context.projectRoot, rutas, cwd: context.projectRoot });
  if (sync !== 0) return { code: 2, stdout: '', stderr: `mutagen: sincronización fallida (código ${sync})`, timedOut: false, signal: null, durationMs: 0 };
  const interno = `cd ${taller.shQuote(rutas.dirRemoto)} && ${comando}`;
  const ssh = taller.argsSsh(config, ['bash', '-lc', taller.shQuote(interno)]);
  const { BIN_SSH } = await import(pathToFileURL(path.join(RCARGO_DIR, '..', 'shared', 'binarios.mjs')).href);
  const execution = await runProcess(BIN_SSH, ssh, { cwd: context.projectRoot, timeoutMs, envAllowlist: context.adapterEnvironmentAllowlist ?? DEFAULT_ENV_ALLOWLIST });
  return execution;
}

async function runStep(context, name, script) {
  const timeoutMs = name === 'test-full' || name === 'build' ? context.qualityConfig.timeoutsMs.frontendTest : context.qualityConfig.timeoutsMs.frontend;
  const taller = await cargarTaller();
  if (taller.esVps()) {
    const npm = npmInvocation(['--prefix', 'frontend', 'run', script]);
    return runCommand(context, name, npm.executable, npm.args, timeoutMs);
  }
  return { name, execution: await runEnVps(context, taller, `npm --prefix frontend run ${script}`, timeoutMs) };
}

async function runBudget(context) {
  const timeoutMs = context.qualityConfig.timeoutsMs.frontendTest;
  const taller = await cargarTaller();
  if (taller.esVps()) {
    const budgetScript = path.join(context.projectRoot, 'scripts', 'quality', 'performance-budget.mjs');
    return runCommand(context, 'performance-budget', process.execPath, [budgetScript], timeoutMs);
  }
  return { name: 'performance-budget', execution: await runEnVps(context, taller, 'node scripts/quality/performance-budget.mjs', timeoutMs) };
}

export async function runFrontend(context) {
  const steps = [await runStep(context, 'type-check', 'type-check')];
  if (context.ci && steps[0].execution.code === 0) {
    steps.push(await runStep(context, 'test-full', 'test:full'));
    if (steps.at(-1).execution.code === 0) steps.push(await runStep(context, 'build', 'build'));
    if (steps.at(-1).execution.code === 0) steps.push(await runBudget(context));
  }
  const log = steps.map(step => `## ${step.name}\n${step.execution.stdout}\n${step.execution.stderr}`).join('\n');
  const logPath = await writeStageLog(context, 'frontend', log);
  const failed = steps.filter(step => step.execution.code !== 0 || step.execution.timedOut);
  return { stage: 'frontend', status: failed.some(step => step.execution.timedOut || step.execution.code === 2 || step.execution.signal) ? 'error' : failed.length > 0 ? 'fail' : 'pass', durationMs: steps.reduce((total, step) => total + step.execution.durationMs, 0), findings: failed.map(step => ({ ruleId: step.execution.timedOut ? 'quality-timeout' : `frontend-${step.name}`, severity: 'error', message: conciseFailure(`${step.execution.stdout}\n${step.execution.stderr}`, `${step.name} falló`) })), summary: failed.length > 0 ? `${failed.length} validaciones frontend fallaron` : context.ci ? 'type-check + suite + build + budgets pasaron' : 'type-check pasó', logPath };
}
