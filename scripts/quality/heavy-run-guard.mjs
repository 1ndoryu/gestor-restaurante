import { mkdir, unlink, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import path from 'node:path';
import { isFilteredTestRun, isHeavyCargoCommand } from './heavy-run-guard-cargo.mjs';
import {
  activePath, findQualityRoot, normalizeRoot, processAlive, projectKey, readCooldownMs, readJson,
  readProjectConfig, resolveGuardRoot, resolveTargetBase, statePath, writeJsonAtomic,
} from './heavy-run-guard-recursos.mjs';
import {
  HEAVY_MANUAL_OVERRIDE_ENABLED, isHeavyOverride, logHeavyOverride, manualOverrideSource, overrideReason,
} from './heavy-run-guard-excepcion.mjs';

/* Reexporta la API pública: task-check, target-maintenance y los tests importan
 * de este archivo, así que el reparto en módulos hermanos no debe cambiar esa vía. */
export {
  HEAVY_MANUAL_OVERRIDE_ENABLED, isFilteredTestRun, isHeavyCargoCommand, isHeavyOverride,
  findQualityRoot, logHeavyOverride, manualOverrideSource, overrideReason, resolveGuardRoot, resolveTargetBase,
};

export async function inspectHeavyRun({ projectRoot, targetBase = resolveTargetBase(), mode = 'full', allowHeavy = false, heavyReason, now = Date.now() }) {
  const config = await readProjectConfig(projectRoot);
  const cooldownMs = readCooldownMs(config);
  const state = await readJson(statePath(targetBase), { version: 1, projects: {} });
  const entry = state.projects?.[projectKey(projectRoot)];
  const lastHeavyAt = Number(entry?.lastHeavyAt || 0);
  const elapsed = lastHeavyAt > 0 ? now - lastHeavyAt : Number.POSITIVE_INFINITY;
  const remainingMs = Math.max(0, cooldownMs - elapsed);
  const manualSource = manualOverrideSource({ allowHeavy });
  const reason = overrideReason({ heavyReason });
  /* [028A-16] Una excepción manual sin motivo se rechaza antes de consultar el
   * cooldown: el motivo es requisito de la excepción, no del modo CI. */
  if (manualSource && !reason) {
    return {
      allowed: false,
      reason: 'heavy-reason-required',
      message: 'La excepción del guard requiere motivo: usa --heavy-reason "<motivo>" (o GLORY_HEAVY_RUN_REASON).',
      cooldownMs,
      remainingMs: 0,
      source: manualSource,
      override: true,
    };
  }
  const override = isHeavyOverride({ allowHeavy, ci: mode === 'ci' });
  if (!override && remainingMs > 0) {
    return {
      allowed: false,
      reason: 'cooldown',
      cooldownMs,
      remainingMs,
      nextAllowedAt: new Date(now + remainingMs).toISOString(),
      lastHeavyAt: new Date(lastHeavyAt).toISOString(),
    };
  }
  return { allowed: true, cooldownMs, remainingMs: 0, override, source: manualSource, reason };
}

async function clearStaleActiveLock(filePath, targetBase) {
  const active = await readJson(filePath, null);
  if (!active) return null;
  if (processAlive(Number(active.pid))) return active;
  await unlink(filePath).catch(() => {});
  return null;
}

export async function acquireHeavyRun({
  projectRoot,
  targetBase = resolveTargetBase(),
  mode = 'full',
  taskId = null,
  command = 'quality-full',
  allowHeavy = false,
  heavyReason,
}) {
  const decision = await inspectHeavyRun({ projectRoot, targetBase, mode, allowHeavy, heavyReason });
  /* [028A-16] Toda activación manual de la excepción queda en el log de
   * auditoría, concedida o rechazada; el propio acquire lo registra para que
   * el flag/env/token sea trazable aunque la entrada llegue por otro camino
   * (run-with-db, cargo.cmd). */
  const manualSource = manualOverrideSource({ allowHeavy });
  if (manualSource) {
    await logHeavyOverride({
      projectRoot,
      source: manualSource,
      command,
      reason: overrideReason({ heavyReason }),
      granted: decision.allowed,
      taskId,
    });
  }
  if (!decision.allowed) return decision;

  const guardRoot = resolveGuardRoot(targetBase);
  await mkdir(guardRoot, { recursive: true });
  const lockPath = activePath(targetBase);
  const token = crypto.randomUUID();
  const lock = {
    version: 1,
    token,
    pid: process.pid,
    projectRoot: normalizeRoot(projectRoot),
    taskId,
    command,
    startedAt: new Date().toISOString(),
  };
  const existing = await clearStaleActiveLock(lockPath, targetBase);
  if (existing) {
    return {
      allowed: false,
      reason: 'active',
      message: `Ya existe una ejecución pesada activa (PID ${existing.pid}).`,
    };
  }
  try { await writeFile(lockPath, `${JSON.stringify(lock, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' }); }
  catch (error) {
    if (error.code === 'EEXIST') return { allowed: false, reason: 'active', message: 'Otra ejecución pesada tomó el guard.' };
    throw error;
  }

  let released = false;
  return {
    ...decision,
    allowed: true,
    token,
    async release({ status = 'completed' } = {}) {
      if (released) return;
      released = true;
      const stateFile = statePath(targetBase);
      const state = await readJson(stateFile, { version: 1, projects: {} });
      state.version = 1;
      state.projects ??= {};
      state.projects[projectKey(projectRoot)] = {
        projectRoot: normalizeRoot(projectRoot),
        lastHeavyAt: Date.now(),
        lastStatus: status,
        taskId,
        command,
      };
      await writeJsonAtomic(stateFile, state);
      const current = await readJson(lockPath, null);
      if (current?.token === token) await unlink(lockPath).catch(() => {});
    },
  };
}

export function formatHeavyGuardMessage(decision) {
  if (decision.reason === 'cooldown') {
    const minutes = Math.ceil(decision.remainingMs / 60_000);
    /* [SNT-11] La excepción manual sigue activa (HEAVY_MANUAL_OVERRIDE_ENABLED):
     * el mensaje la nombra y dice que queda auditada. */
    return `Full diferido por cooldown: faltan aproximadamente ${minutes} min. Próxima ejecución: ${decision.nextAllowedAt}. Un test filtrado no tiene cooldown: prueba 'cargo test <filtro>' o 'cargo test --test <archivo>'. La suite completa espera al cooldown, o usa --allow-heavy --heavy-reason "<motivo>" (queda auditado).`;
  }
  return decision.message || 'Full diferido porque ya hay otra ejecución pesada activa.';
}

async function executeCargo(argv) {
  const separator = argv.indexOf('--');
  const options = argv.slice(0, separator === -1 ? argv.length : separator);
  const cargoArgs = separator === -1 ? [] : argv.slice(separator + 1);
  const projectIndex = options.indexOf('--project-root');
  const cargoIndex = options.indexOf('--cargo-path');
  const reasonIndex = options.indexOf('--heavy-reason');
  const requestedRoot = projectIndex >= 0 ? path.resolve(options[projectIndex + 1]) : process.cwd();
  const projectRoot = await findQualityRoot(requestedRoot);
  const cargoPath = cargoIndex >= 0 ? options[cargoIndex + 1] : (process.platform === 'win32' ? 'cargo.exe' : 'cargo');
  const heavyReason = reasonIndex >= 0 ? options[reasonIndex + 1] : undefined;
  if (!isHeavyCargoCommand(cargoArgs)) {
    const light = spawn(cargoPath, cargoArgs, { cwd: projectRoot, env: process.env, stdio: 'inherit', shell: false, windowsHide: true });
    light.on('error', error => {
      console.error(`[glory-quality] Cargo no pudo iniciar: ${error.message}`);
      process.exitCode = 2;
    });
    light.on('exit', (code, signal) => { process.exitCode = signal ? 2 : code ?? 2; });
    return;
  }
  const lease = await acquireHeavyRun({
    projectRoot,
    mode: 'raw-cargo',
    command: `cargo ${cargoArgs.join(' ')}`,
    allowHeavy: options.includes('--allow-heavy'),
    heavyReason,
  });
  if (!lease.allowed) {
    console.error(`[glory-quality] BLOQUEADO: ${formatHeavyGuardMessage(lease)}`);
    process.exitCode = 75;
    return;
  }
  const child = spawn(cargoPath, cargoArgs, { cwd: projectRoot, env: process.env, stdio: 'inherit', shell: false, windowsHide: true });
  child.on('error', async error => {
    await lease.release({ status: 'error' });
    console.error(`[glory-quality] Cargo no pudo iniciar: ${error.message}`);
    process.exitCode = 2;
  });
  child.on('exit', async (code, signal) => {
    await lease.release({ status: signal ? 'signal' : code === 0 ? 'pass' : 'fail' });
    process.exitCode = signal ? 2 : code ?? 2;
  });
}

const argv = process.argv.slice(2);
if (argv.includes('--execute-cargo')) await executeCargo(argv);
else if (argv.includes('--status')) {
  const targetBase = resolveTargetBase();
  const state = await readJson(statePath(targetBase), { version: 1, projects: {} });
  const active = await readJson(activePath(targetBase), null);
  console.log(JSON.stringify({ targetBase, state, active }, null, 2));
}
