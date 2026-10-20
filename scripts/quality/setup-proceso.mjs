import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { access } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/* Procesos y entorno de setup, compartidos por sus módulos hermanos: un único
 * spawn con timeout y cierre de árbol en Windows, y un entorno npm aislado. */
const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const realNodeDir = path.dirname(process.execPath);

/* [por que] npm no está en la misma ruta según la instalación de Node: en Windows va
 * junto al ejecutable; en Linux con prefijo (p. ej. ~/.local/opt/node) va en
 * ../lib/node_modules. Se prueban ambas y, si no, la ruta que npm pasó en npm_execpath
 * (solo si es un npm-cli.js, nunca un ejecutable distinto). */
function resolveNpmCli() {
  const candidates = [
    path.join(realNodeDir, 'node_modules', 'npm', 'bin', 'npm-cli.js'),
    path.join(realNodeDir, '..', 'lib', 'node_modules', 'npm', 'bin', 'npm-cli.js'),
    process.env.npm_execpath,
  ];
  return candidates.find(candidate => candidate && /npm-cli\.js$/u.test(candidate) && existsSync(candidate)) ?? candidates[0];
}

const npmCliPath = resolveNpmCli();

function isolatedNpmEnvironment() {
  const pathKey = process.platform === 'win32' ? 'Path' : 'PATH';
  const currentPath = process.env[pathKey] ?? process.env.PATH ?? '';
  const safePath = currentPath
    .split(path.delimiter)
    .filter(entry => !/[\\/]Owner[\\/]bin(?:[\\/]|$)/iu.test(entry))
    .filter(entry => !/[\\/]GlorySentinel[\\/](?:shims|bin)(?:[\\/]|$)/iu.test(entry))
    .filter(entry => !/[\\/]scripts[\\/]quality(?:[\\/]|$)/iu.test(entry));
  const safePathValue = [realNodeDir, ...safePath].join(path.delimiter);
  const environment = {
    ...process.env,
    npm_execpath: npmCliPath,
    npm_config_script_shell: process.platform === 'win32' ? 'C:\\Windows\\System32\\cmd.exe' : '/bin/sh',
    PATH: safePathValue,
    ...(process.platform === 'win32' ? { Path: safePathValue } : {}),
  };
  for (const key of Object.keys(environment)) {
    if (/^GLORY_(?:REAL_|GUARD_|QUALITY_)/u.test(key)) delete environment[key];
  }
  delete environment.npm_config_userconfig;
  delete environment.NPM_CONFIG_USERCONFIG;
  delete environment.BASH_ENV;
  delete environment.ENV;
  return environment;
}

async function exists(target) {
  try {
    await access(target);
    return true;
  } catch {
    return false;
  }
}

function run(executable, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, {
      cwd: options.cwd ?? projectRoot,
      shell: false,
      stdio: options.capture
        ? [options.input ? 'pipe' : 'ignore', 'pipe', 'pipe']
        : [options.input ? 'pipe' : 'ignore', 'inherit', 'inherit'],
      windowsHide: true,
      ...(options.env ? { env: options.env } : {}),
    });
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    if (options.input !== undefined) child.stdin?.end(options.input);
    const timeout = setTimeout(() => {
      timedOut = true;
      if (process.platform === 'win32' && child.pid) {
        spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
          shell: false,
          stdio: 'ignore',
          windowsHide: true,
        });
      } else {
        child.kill('SIGTERM');
      }
    }, options.timeoutMs ?? 300_000);
    child.stdout?.on('data', chunk => { stdout += chunk; });
    child.stderr?.on('data', chunk => { stderr += chunk; });
    child.on('error', error => {
      clearTimeout(timeout);
      reject(error);
    });
    child.on('close', (code, signal) => {
      clearTimeout(timeout);
      if (timedOut) {
        reject(new Error(`${executable} excedió el timeout`));
        return;
      }
      if (signal || code !== 0) {
        if (options.allowFailure) {
          resolve({ code: code ?? 1, signal, stdout: stdout.trim(), stderr: stderr.trim() });
          return;
        }
        reject(new Error(`${executable} ${args.join(' ')} falló (${signal ?? code})\n${stderr.trim()}`));
        return;
      }
      resolve(stdout.trim());
    });
  });
}

function captureBinary(executable, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, {
      cwd: options.cwd ?? projectRoot,
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
      ...(options.env ? { env: options.env } : {}),
    });
    const stdout = [];
    let stderr = '';
    let timedOut = false;
    if (options.input !== undefined) child.stdin?.end(options.input);
    const timeout = setTimeout(() => {
      timedOut = true;
      if (process.platform === 'win32' && child.pid) {
        spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
          shell: false,
          stdio: 'ignore',
          windowsHide: true,
        });
      } else {
        child.kill('SIGTERM');
      }
    }, options.timeoutMs ?? 300_000);
    child.stdout.on('data', chunk => stdout.push(Buffer.from(chunk)));
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.on('error', error => {
      clearTimeout(timeout);
      reject(error);
    });
    child.on('close', (code, signal) => {
      clearTimeout(timeout);
      if (timedOut) {
        reject(new Error(`${executable} excedió el timeout`));
        return;
      }
      if (signal || code !== 0) {
        reject(new Error(`${executable} ${args.join(' ')} falló (${signal ?? code})\\n${stderr.trim()}`));
        return;
      }
      resolve(Buffer.concat(stdout));
    });
  });
}

export { captureBinary, exists, isolatedNpmEnvironment, npmCliPath, projectRoot, run };
