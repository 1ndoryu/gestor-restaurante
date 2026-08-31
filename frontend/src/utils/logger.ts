/* Logger central del frontend.
 * [por que] La regla console-production marca el console directo en código de
 * producción. La instrumentación legítima (ErrorBoundary, diagnóstico de SSE)
 * se canaliza aquí, que es la whitelist declarada en `loggerModules` del
 * sentinel.config.json — no se borran logs útiles ni se deshabilita la regla.
 * El prefijo [restaurante] permite filtrar los logs propios en la consola. */

type Nivel = 'log' | 'warn' | 'error';

function emitir(nivel: Nivel, ...args: unknown[]): void {
  const prefijo = '[restaurante]';
  if (nivel === 'error') {
    console.error(prefijo, ...args);
  } else if (nivel === 'warn') {
    console.warn(prefijo, ...args);
  } else {
    console.log(prefijo, ...args);
  }
}

export const logger = {
  log: (...args: unknown[]) => emitir('log', ...args),
  warn: (...args: unknown[]) => emitir('warn', ...args),
  error: (...args: unknown[]) => emitir('error', ...args),
};
