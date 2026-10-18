import { projectRoot } from './preflight.mjs';
import { defaultAgent, isStale, sanitizeAgentName, sanitizeTaskId } from './task-takeover-modelo.mjs';
import { listTakeovers } from './task-takeover-lectura.mjs';
import { releaseTask, takeTask } from './task-takeover-reglas.mjs';

/* Interfaz de línea de comandos (take, release, status). Solo traduce argumentos
 * y resultados a texto; la lógica vive en reglas y lectura. El punto de entrada
 * sigue siendo task-takeover.mjs (npm run task:*). */

function formatEntryLine({ taskId, entry, stale, corrupt }) {
  if (corrupt || !entry) return `- ${taskId} · registro ilegible`;
  const state = stale ? 'EXPIRADA (olvidada)' : 'activa';
  return `- ${taskId} · ${entry.id} · por ${entry.takenBy} · desde ${entry.takenAt} · expira ${entry.expiresAt} · ${state}`;
}

export async function main() {
  const argv = process.argv.slice(2);
  const command = argv[0];
  const args = {};
  for (let index = 1; index < argv.length; index++) {
    const arg = argv[index];
    if (arg === '--task') args.task = argv[++index];
    else if (arg === '--by') args.by = argv[++index];
    else if (arg === '--force') args.force = true;
    else if (arg === '--root') args.root = argv[++index];
  }
  const root = args.root ?? projectRoot;

  if (command === 'take' || command === 'release') {
    if (!args.task) {
      process.stderr.write(`[task-takeover] ${command} requiere --task <ID> [--by <agente>] [--force]\n`);
      process.exitCode = 2;
      return;
    }
    /* [018A-97] ID inválido (traversal, controles): error limpio, no un
     * stack trace sin controlar desde takeTask. */
    try {
      sanitizeTaskId(args.task);
    } catch (error) {
      process.stderr.write(`[task-takeover] ${error.message}\n`);
      process.exitCode = 2;
      return;
    }
    const agent = sanitizeAgentName(args.by ?? defaultAgent());
    const result = command === 'take'
      ? await takeTask(root, args.task, { by: agent, force: args.force })
      : await releaseTask(root, args.task, { by: agent });
    const { entry } = result;
    if (result.status === 'conflict' && entry) {
      const stale = isStale(entry);
      const hint = command === 'release'
        ? ' — solo el autor libera su toma activa (o un marcado expirado)'
        : '';
      process.stderr.write(`[task-takeover] ${command.toUpperCase()} RECHAZADO — ${args.task} tomada por ${entry.takenBy} (${entry.id}) desde ${entry.takenAt}${stale ? ' (expirada)' : ''}${hint}\n`);
      process.stderr.write('[task-takeover] Next: npm run task:status\n');
      process.exitCode = 1;
      return;
    }
    if (result.status === 'conflict' && !entry) {
      process.stderr.write(`[task-takeover] ${command.toUpperCase()} RECHAZADO — carrera de escritura, reintenta\n`);
      process.exitCode = 1;
      return;
    }
    if (result.status === 'not-taken') {
      process.stdout.write(`[task-takeover] NO TOMADA ${args.task} — no había marcado que liberar\n`);
      return;
    }
    const verb = {
      taken: 'TOMADA', refreshed: 'RENOVADA',
      'taken-over-stale': 'RE-TOMADA (marcado olvidado liberado)',
      'taken-over-corrupt': 'RE-TOMADA (registro ilegible retirado)',
      released: 'LIBERADA', 'released-stale': 'LIBERADA (marcado olvidado)',
      'released-corrupt': 'LIBERADA (registro ilegible retirado)',
    }[result.status] ?? result.status.toUpperCase();
    const who = entry ? ` · ${entry.id} · por ${entry.takenBy}` : ` · por ${agent}`;
    process.stdout.write(`[task-takeover] ${verb} ${args.task}${who}\n`);
    return;
  }

  if (command === 'status') {
    const entries = await listTakeovers(root);
    if (entries.length === 0) {
      process.stdout.write('[task-takeover] Ninguna tarea tomada.\n');
      return;
    }
    process.stdout.write('[task-takeover] Tomás de tarea (TTL 6 h; expiradas = olvidadas):\n');
    for (const item of entries) process.stdout.write(`${formatEntryLine(item)}\n`);
    if (args.task) {
      const match = entries.find(item => item.taskId === args.task);
      if (!match) process.stdout.write(`- ${args.task} · NO tomada\n`);
    }
    return;
  }

  process.stderr.write(`[task-takeover] uso:\n`);
  process.stderr.write(`  take    --task <ID> [--by <agente>] [--force]   (--force: re-tomar un marcado expirado)\n`);
  process.stderr.write(`  release --task <ID> [--by <agente>]            (solo el autor, o un marcado expirado)\n`);
  process.stderr.write(`  status  [--task <ID>] [--root <ruta>]\n`);
  process.exitCode = 2;
}
