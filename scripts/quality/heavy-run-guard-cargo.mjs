/* Clasificación de comandos Cargo: decide qué ejecuciones son pesadas y pasan por
 * el cooldown. Va aparte para que la regla de subcomandos se pueda leer y probar sola. */

/* Espejo de glory-sentinel/src/core/scheduler.ts (07acc77). Mantener en sincronía. */
const CARGO_VALUE_OPTIONS = new Set([
  '-C', '--color', '--config', '-Z', '--manifest-path', '--target-dir',
  '-p', '--package', '--exclude', '--test', '--bin', '--example', '--bench',
  '-F', '--features', '--target', '--profile', '-j', '--jobs', '--message-format',
]);

/* Localiza el subcomando saltando el toolchain (+nightly) y las opciones; las que
 * llevan valor consumen también su argumento para no confundirlo con el subcomando. */
function cargoSubcommand(args) {
  for (let index = 0; index < args.length; index++) {
    const arg = String(args[index]);
    if (arg.startsWith('+')) continue;
    if (arg.startsWith('-')) {
      if (CARGO_VALUE_OPTIONS.has(arg)) index++;
      continue;
    }
    return { command: arg.toLowerCase(), index };
  }
  return { index: args.length };
}

/* Un test filtrado (nombre posicional o `--test <archivo>`) ejecuta pocos casos
 * y no espera al cooldown. `cargo test` sin filtro, con solo `-p` o con `--lib`
 * sigue siendo la suite completa. Ante la duda, pesado. */
export function isFilteredTestRun(args) {
  const { command, index } = cargoSubcommand(args);
  if (command !== 'test') return false;
  for (let i = index + 1; i < args.length; i++) {
    const arg = String(args[i]);
    if (arg === '--') break;
    if (arg === '--test') {
      const next = args[i + 1];
      return next !== undefined && !String(next).startsWith('-');
    }
    if (arg.startsWith('-')) {
      if (CARGO_VALUE_OPTIONS.has(arg)) i++;
      continue;
    }
    return true;
  }
  return false;
}

export function isHeavyCargoCommand(args) {
  const { command } = cargoSubcommand(args);
  if (command === 'clippy' || command === 'bench') return true;
  return command === 'test' && !isFilteredTestRun(args);
}
