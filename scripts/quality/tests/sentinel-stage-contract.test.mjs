import assert from 'node:assert/strict';
import test from 'node:test';
import { sentinelStageDeclaration, sentinelTransportExitCode } from '../sentinel-stage-contract.mjs';

test('el manifest v1 solo emite claves admitidas por Sentinel', () => {
  const declaration = sentinelStageDeclaration({
    name: 'docs',
    executable: 'node',
    args: ['stage-process.mjs'],
    expectedSchemaVersion: '1',
    timeoutMs: 1_000,
    reportPath: '.quality-reports/docs.json',
    envAllowlist: ['PATH'],
  });

  assert.deepEqual(Object.keys(declaration), [
    'name',
    'executable',
    'args',
    'expectedSchemaVersion',
    'timeoutMs',
    'reportPath',
  ]);
  assert.equal('envAllowlist' in declaration, false);
});

test('un reporte estructurado válido deja la decisión a Sentinel', () => {
  assert.equal(sentinelTransportExitCode('pass'), 0);
  assert.equal(sentinelTransportExitCode('fail'), 0);
  assert.equal(sentinelTransportExitCode('error'), 0);
});
