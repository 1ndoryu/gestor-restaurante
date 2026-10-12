#!/usr/bin/env node

/* [run-with-db.mjs] Wrapper generico: ejecuta cualquier subcomando de Cargo
 * con DATABASE_URL apuntando a la BD de la rama actual.
 * Uso: node scripts/run-with-db.mjs run --bin seed
 *      node scripts/run-with-db.mjs run --bin migrate */

import { spawn } from 'node:child_process';
import path from 'node:path';
import { getBranchDbUrl } from './branch-db.mjs';

const cargoArgs = process.argv.slice(2);
if (cargoArgs.length === 0) {
  console.error('Uso: node scripts/run-with-db.mjs <subcomando cargo> [...args]');
  process.exit(1);
}

console.log('');
const { dbUrl, cargoTargetDir } = getBranchDbUrl();
console.log('');

// `check` valida las queries sqlx contra la BD de la rama, que puede estar
// desfasada (tablas o migraciones pendientes). El cache `.sqlx/` versionado
// basta para compilar; los tests con BD siguen online. SQLX_OFFLINE=false
// fuerza la comprobación online.
const sqlxOffline = cargoArgs[0] === 'check' && process.env.SQLX_OFFLINE === undefined
  ? { SQLX_OFFLINE: 'true' }
  : {};

const child = spawn('node', ['scripts/run-cargo.mjs', ...cargoArgs], {
  stdio: 'inherit',
  env: { ...process.env, ...sqlxOffline, DATABASE_URL: dbUrl, CARGO_TARGET_DIR: cargoTargetDir },
  shell: false,
});

child.on('error', (err) => { console.error('[run-with-db] Error:', err.message); process.exit(1); });
child.on('exit', (code) => process.exit(code ?? 0));
