#!/usr/bin/env node

/* [144A-1] Centraliza la ejecucion de Cargo para los scripts npm.
 * Permite usar la instalacion estandar de rustup aunque el PATH de la terminal
 * todavia no se haya refrescado y evita el error opaco de cmd.exe en Windows.
 * Pendiente: si el proyecto necesita bootstrap automatico de Rust, resolverlo
 * fuera del repo para no instalar toolchains sin consentimiento explicito. */

import { spawn, spawnSync } from 'node:child_process';
import { accessSync, constants, readFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const taskId = '144A-1';
const cargoArgs = process.argv.slice(2);
const executableName = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const cleanScriptPath = path.join(repoRoot, 'glory-rs', 'scripts', 'clean-cargo-target.ps1');
const minFreeMB = Number.parseInt(process.env.GLORY_CARGO_MIN_FREE_MB ?? '6144', 10);
const maxTargetMB = Number.parseInt(process.env.GLORY_CARGO_MAX_TARGET_MB ?? '4096', 10);
const cargoCommandsThatCompile = new Set(['build', 'check', 'clippy', 'run', 'test']);

function fileExists(filePath) {
  try {
    accessSync(filePath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

function unique(values) {
  return [...new Set(values.filter(Boolean))];
}

function buildCandidates() {
  const homeDir = os.homedir();
  const pathEntries = (process.env.PATH ?? '').split(path.delimiter);

  return unique([
    process.env.CARGO,
    ...pathEntries.map((entry) => path.join(entry, executableName)),
    homeDir ? path.join(homeDir, '.cargo', 'bin', executableName) : '',
  ]);
}

function resolveCargoPath() {
  return buildCandidates().find(fileExists);
}

function parseConfiguredTargetDir() {
  const configPath = path.join(repoRoot, '.cargo', 'config.toml');
  if (!fileExists(configPath)) {
    return '';
  }

  const config = readFileSync(configPath, 'utf8');
  const match = config.match(/^target-dir\s*=\s*["']([^"']+)["']/m);
  return match?.[1] ?? '';
}

function resolveTargetDirs() {
  const configured = parseConfiguredTargetDir();
  const fallback = process.platform === 'win32' ? 'C:\\tmp\\glory-target' : path.join(os.tmpdir(), 'glory-target');
  return unique([process.env.CARGO_TARGET_DIR, configured, fallback]).map((targetDir) => path.resolve(targetDir));
}

function shouldPreflightTargetSpace() {
  return cargoCommandsThatCompile.has(cargoArgs[0] ?? '');
}

function getFreeMBForPath(targetDir) {
  if (process.platform !== 'win32') {
    return Number.POSITIVE_INFINITY;
  }

  const root = path.parse(targetDir).root.replace(/[:\\/]+$/, '');
  const result = spawnSync('powershell', [
    '-NoProfile',
    '-ExecutionPolicy',
    'Bypass',
    '-Command',
    `(Get-PSDrive -Name '${root}').Free`,
  ], { encoding: 'utf8' });

  if (result.status !== 0) {
    return Number.POSITIVE_INFINITY;
  }

  const freeBytes = Number.parseInt(result.stdout.trim(), 10);
  return Number.isFinite(freeBytes) ? Math.round(freeBytes / 1024 / 1024) : Number.POSITIVE_INFINITY;
}

function runCargoTargetCleanup(targetDirs) {
  if (process.platform !== 'win32' || !fileExists(cleanScriptPath)) {
    return;
  }

  const freeMB = Math.min(...targetDirs.map(getFreeMBForPath));
  const reason = freeMB <= minFreeMB
    ? `Espacio libre bajo (${freeMB} MB)`
    : `Verificando target de Cargo (${freeMB} MB libres)`;
  console.error(`[run-cargo] ${reason}. Ejecutando limpieza preventiva...`);

  const result = spawnSync('powershell', [
    '-NoProfile',
    '-ExecutionPolicy',
    'Bypass',
    '-File',
    cleanScriptPath,
    '-TargetDirs',
    ...targetDirs,
    '-MaxTotalMB',
    String(maxTargetMB),
  ], { encoding: 'utf8', stdio: 'inherit' });

  if (result.status !== 0) {
    console.error('[run-cargo] La limpieza preventiva de target fallo; se continua para mostrar el error real de Cargo.');
  }

  const freeAfterCleanupMB = Math.min(...targetDirs.map(getFreeMBForPath));
  if (freeAfterCleanupMB <= minFreeMB) {
    console.error(`[run-cargo] Espacio libre insuficiente tras limpieza (${freeAfterCleanupMB} MB). Libera espacio o cierra procesos Rust que bloqueen target.`);
    process.exit(1);
  }
}

function printMissingCargoMessage() {
  const cargoHome = path.join(os.homedir(), '.cargo', 'bin');
  const installCommand =
    process.platform === 'win32'
      ? 'winget install --id Rustlang.Rustup --exact --accept-source-agreements --accept-package-agreements'
      : 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh';

  console.error(`[${taskId}] No se encontro Cargo. El backend Rust no puede iniciarse sin la toolchain.`);
  console.error(`Instala rustup y vuelve a abrir la terminal: ${installCommand}`);
  console.error(`Ruta esperada por defecto: ${cargoHome}`);
}

if (cargoArgs.length === 0) {
  console.error('Uso: node scripts/run-cargo.mjs <subcomando cargo> [...args]');
  process.exit(1);
}

const cargoPath = resolveCargoPath();

if (!cargoPath) {
  printMissingCargoMessage();
  process.exit(1);
}

const childEnv = {
  ...process.env,
  PATH: unique([path.dirname(cargoPath), process.env.PATH ?? '']).join(path.delimiter),
};

if (shouldPreflightTargetSpace()) {
  runCargoTargetCleanup(resolveTargetDirs());
}

const childProcess = spawn(cargoPath, cargoArgs, {
  stdio: 'inherit',
  env: childEnv,
});

childProcess.on('error', (error) => {
  console.error(`[run-cargo] Error al ejecutar Cargo: ${error.message}`);
  process.exit(1);
});

childProcess.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});