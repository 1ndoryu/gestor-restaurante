#!/usr/bin/env node

/* [branch-db.mjs] Helper reutilizable: detecta la rama git actual y devuelve
 * el DATABASE_URL correspondiente, creando la BD si no existe.
 * Convencion: {base_db}_{branch_slug}. main/master usan el nombre base sin sufijo. */

import { execSync, execFileSync } from 'node:child_process';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(__dirname, '..');

/** Lee el nombre del paquete desde Cargo.toml (sin hardcodes). */
function readPackageName() {
  const cargoToml = path.join(projectRoot, 'Cargo.toml');
  if (!existsSync(cargoToml)) return 'glory';
  const toml = readFileSync(cargoToml, 'utf8');
  const m = toml.match(/^name\s*=\s*"([^"]+)"/m);
  return (m ? m[1] : 'glory').replace(/-/g, '_');
}

const cargoTargetBase =
  process.env.CARGO_TARGET_DIR_BASE ||
  (process.platform === 'win32' ? 'C:\\tmp\\glory-target\\RESTAURANTE' : path.join(tmpdir(), 'glory-target'));

function parseEnvFile(content) {
  const vars = {};
  for (const line of content.split(/\r?\n/)) {
    const m = line.match(/^([A-Z_][A-Z0-9_]*)=(.*)$/);
    if (m) vars[m[1]] = m[2].trim();
  }
  return vars;
}

function findPsql() {
  if (process.platform === 'win32') {
    const pgRoot = 'C:\\Program Files\\PostgreSQL';
    if (existsSync(pgRoot)) {
      try {
        const versions = readdirSync(pgRoot)
          .filter((v) => /^\d+$/.test(v))
          .sort((a, b) => parseInt(b) - parseInt(a));
        for (const v of versions) {
          const candidate = path.join(pgRoot, v, 'bin', 'psql.exe');
          if (existsSync(candidate)) return candidate;
        }
      } catch { /* sin postgres instalado */ }
    }
  }
  return 'psql';
}

export function getBranchDbUrl({ verbose = true } = {}) {
  /* 1. Rama */
  let branch = 'main';
  try {
    branch = execSync('git rev-parse --abbrev-ref HEAD', {
      encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'],
    }).trim();
  } catch {
    if (verbose) console.warn('[db] No se pudo detectar la rama, usando "main"');
  }

  const branchSlug = branch.replace(/[^a-zA-Z0-9]/g, '_').toLowerCase();

  /* 2. Parsear .env */
  const envPath = path.resolve('.env');
  const envVars = existsSync(envPath) ? parseEnvFile(readFileSync(envPath, 'utf8')) : {};
  const baseDbUrl = envVars.DATABASE_URL;
  if (!baseDbUrl) throw new Error('DATABASE_URL no encontrado en .env');

  /* 3. Parsear URL — solo necesitamos user/pass/host/port */
  const m = baseDbUrl.match(/^postgres(?:ql)?:\/\/([^:]+):([^@]+)@([^:/]+):(\d+)\/(.+)$/);
  if (!m) throw new Error(`No se pudo parsear DATABASE_URL: ${baseDbUrl}`);
  const [, dbUser, dbPass, dbHost, dbPort] = m;

  /* 4. Nombre de BD y cargo target derivados de Cargo.toml + rama (sin hardcodes) */
  const pkgName = readPackageName();
  const isDefault = branch === 'main' || branch === 'master';
  const dbName = isDefault ? pkgName : `${pkgName}_${branchSlug}`;
  const dbUrl = `postgres://${dbUser}:${dbPass}@${dbHost}:${dbPort}/${dbName}`;
  /* GLORY_CARGO_TARGET_DIR es el override manual; CARGO_TARGET_DIR esta fijado a nivel
   * global en config.toml y se ignora aqui para que el subdirectorio por proyecto+rama
   * siempre se calcule correctamente. */
  const cargoTargetDir =
    process.env.GLORY_CARGO_TARGET_DIR ||
    path.join(cargoTargetBase, `${pkgName}_${branchSlug}`);

  if (verbose) {
    console.log(`\x1b[36m[db] Rama:          \x1b[0m ${branch}`);
    console.log(`\x1b[36m[db] Base de datos: \x1b[0m ${dbName}`);
    console.log(`\x1b[36m[db] Cargo target:  \x1b[0m ${cargoTargetDir}`);
  }

  /* 5. Crear BD si no existe */
  const psqlBin = findPsql();
  const psqlEnv = { ...process.env, PGPASSWORD: dbPass };

  function psql(sql, db = 'postgres') {
    return execFileSync(psqlBin, ['-U', dbUser, '-h', dbHost, '-p', dbPort, '-d', db, '-t', '-A', '-c', sql], {
      env: psqlEnv, encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'],
    });
  }

  const exists = psql(`SELECT 1 FROM pg_database WHERE datname = '${dbName}'`).trim();
  if (exists === '1') {
    if (verbose) console.log(`\x1b[32m[db] BD existente: ${dbName}\x1b[0m`);
  } else {
    psql(`CREATE DATABASE "${dbName}"`);
    if (verbose) console.log(`\x1b[32m[db] BD creada: ${dbName}\x1b[0m`);
  }

  return { dbUrl, dbName, branch, cargoTargetDir };
}
