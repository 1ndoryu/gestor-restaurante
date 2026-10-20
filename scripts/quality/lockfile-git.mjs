// Estado git del checkout de cada analyzer: hashes del árbol y del diff, estado y gitlink del workspace.
// Va aparte porque concentra los procesos git; los de archivo, estado y diff tienen timeout de 30 s.
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { realpath } from 'node:fs/promises';
import path from 'node:path';
import { runProcess } from './runner.mjs';

// Metadata que el instalador escribe en el checkout; se ignora al decidir si está modificado.
const INSTALL_METADATA_PATH = '.quality-install.json';

// Ignora solo esa metadata: cualquier otro cambio vuelve el checkout no confiable.
export function untrustedCheckoutChanges(statusOutput) {
  const records = String(statusOutput).includes('\0')
    ? String(statusOutput).split('\0')
    : String(statusOutput).split(/\r?\n/);
  return records
    .map(line => line.trimEnd())
    .filter(Boolean)
    .filter(line => {
      const pathName = line.slice(3).replace(/\\/g, '/');
      return pathName !== INSTALL_METADATA_PATH;
    });
}

/* Hash del árbol versionado (git archive): un checkout con otro contenido no puede pasar por el lock.
 * [10AA-3] `core.autocrlf=false` + `core.eol=lf`: `git archive` convierte los finales de línea del
 * árbol que archiva, y con `autocrlf=true` (portátil y 89) el mismo commit daba un tar con CRLF y por
 * tanto otro sha256. El lock del área se genera con el generador del workspace-manager, que ya fija
 * LF, así que aquí salía «SHA-256 del árbol instalado no coincide» en cualquier máquina con autocrlf.
 * El hash tiene que depender del commit, no del `core.autocrlf` de quien mira. */
export async function gitArchiveSha256(toolRoot) {
  return new Promise((resolve, reject) => {
    const child = spawn('git', ['-c', 'core.autocrlf=false', '-c', 'core.eol=lf', '-C', toolRoot, 'archive', '--format=tar', 'HEAD'], {
      cwd: toolRoot,
      shell: false,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const hash = createHash('sha256');
    let stderr = '';
    const timeout = setTimeout(() => {
      if (process.platform === 'win32' && child.pid) {
        spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], { shell: false, stdio: 'ignore', windowsHide: true });
      } else child.kill('SIGTERM');
      reject(new Error('git archive excedió el timeout'));
    }, 30_000);
    child.stdout.on('data', chunk => hash.update(chunk));
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.on('error', error => { clearTimeout(timeout); reject(error); });
    child.on('close', (code, signal) => {
      clearTimeout(timeout);
      if (signal || code !== 0) reject(new Error(`git archive falló (${signal ?? code}): ${stderr.trim()}`));
      else resolve(hash.digest('hex'));
    });
  });
}

// Estado porcelain en formato -z: rutas con espacios o saltos de línea se leen sin ambigüedad.
export async function gitStatusPorcelain(toolRoot) {
  return new Promise((resolve, reject) => {
    const child = spawn('git', ['-C', toolRoot, 'status', '--porcelain=v1', '-z', '--untracked-files=all'], {
      cwd: toolRoot,
      shell: false,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const chunks = [];
    let stderr = '';
    const timeout = setTimeout(() => {
      if (process.platform === 'win32' && child.pid) {
        spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], { shell: false, stdio: 'ignore', windowsHide: true });
      } else child.kill('SIGTERM');
      reject(new Error('git status excedió el timeout'));
    }, 30_000);
    child.stdout.on('data', chunk => chunks.push(Buffer.from(chunk)));
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.on('error', error => { clearTimeout(timeout); reject(error); });
    child.on('close', (code, signal) => {
      clearTimeout(timeout);
      if (signal || code !== 0) reject(new Error(`git status falló (${signal ?? code}): ${stderr.trim()}`));
      else resolve({ text: Buffer.concat(chunks).toString('utf8') });
    });
  });
}

// Commit del gitlink en HEAD o en el índice: detecta pins preparados pero no committeados.
export async function parentGitlinkCommit(workspaceRoot, configuredSourcePath, staged = false) {
  const rootReal = await realpath(workspaceRoot);
  const sourceReal = await realpath(configuredSourcePath);
  const relative = path.relative(rootReal, sourceReal).replace(/\\/g, '/');
  if (!relative || relative.startsWith('../') || path.isAbsolute(relative)) return null;
  const args = staged
    ? ['-C', rootReal, 'ls-files', '--stage', '--', relative]
    : ['-C', rootReal, 'ls-tree', 'HEAD', '--', relative];
  const result = await runProcess('git', args, { cwd: rootReal, timeoutMs: 10_000 });
  if (result.code !== 0) throw new Error(`no se pudo leer el gitlink de ${relative}`);
  const match = staged
    ? /^160000\s+([a-f0-9]{40})\s+0\s+/mu.exec(result.stdout)
    : /^160000\s+commit\s+([a-f0-9]{40})\s+/mu.exec(result.stdout);
  return match?.[1] ?? null;
}

// Hash del diff binario: permite aceptar el patch local declarado y rechazar cualquier otro cambio.
export async function gitDiffSha256(toolRoot) {
  return new Promise((resolve, reject) => {
    const child = spawn('git', ['-C', toolRoot, 'diff', '--binary', '--no-ext-diff'], {
      cwd: toolRoot,
      shell: false,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const hash = createHash('sha256');
    let stderr = '';
    const timeout = setTimeout(() => {
      if (process.platform === 'win32' && child.pid) {
        spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], { shell: false, stdio: 'ignore', windowsHide: true });
      } else child.kill('SIGTERM');
      reject(new Error('git diff excedió el timeout'));
    }, 30_000);
    child.stdout.on('data', chunk => hash.update(chunk));
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.on('error', error => { clearTimeout(timeout); reject(error); });
    child.on('close', (code, signal) => {
      clearTimeout(timeout);
      if (signal || code !== 0) reject(new Error(`git diff falló (${signal ?? code}): ${stderr.trim()}`));
      else resolve(hash.digest('hex'));
    });
  });
}
