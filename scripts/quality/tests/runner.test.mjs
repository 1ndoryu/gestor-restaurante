import assert from 'node:assert/strict';
import test from 'node:test';
import { runProcess, safeEnvironment } from '../runner.mjs';

test('runner distingue éxito y timeout', async () => {
  const success = await runProcess(process.execPath, ['-e', 'process.stdout.write("ok")'], { timeoutMs: 2_000 });
  assert.equal(success.code, 0);
  assert.equal(success.stdout, 'ok');

  const timeout = await runProcess(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], { timeoutMs: 100 });
  assert.equal(timeout.code, 2);
  assert.equal(timeout.timedOut, true);
});

test('runner conserva el estado cancelled solo ante transición durante la ejecución', async () => {
  const notCancelled = await runProcess(process.execPath, ['-e', 'setTimeout(() => process.exit(0), 50)'], { timeoutMs: 2_000, isCancelled: () => false });
  assert.equal(notCancelled.code, 0);
  assert.equal(notCancelled.cancelled, false);

  let cancelled = false;
  setTimeout(() => { cancelled = true; }, 30);
  const cancelledResult = await runProcess(process.execPath, ['-e', 'setTimeout(() => process.exit(0), 200)'], { timeoutMs: 2_000, isCancelled: () => cancelled });
  assert.equal(cancelledResult.cancelled, true);
  assert.equal(cancelledResult.code, 130);

  const alreadyCancelled = await runProcess(process.execPath, ['-e', 'process.exit(0)'], { timeoutMs: 2_000, isCancelled: () => true });
  assert.equal(alreadyCancelled.cancelled, true);
  assert.equal(alreadyCancelled.code, 130);
});

test('runner limita la captura de salida ruidosa', async () => {
  const noisy = await runProcess(process.execPath, ['-e', 'process.stdout.write("x".repeat(100000))'], { timeoutMs: 2_000 });
  assert.equal(noisy.code, 0);
  assert.match(noisy.stdout, /quality output truncated at 65536 bytes/);
  assert.ok(noisy.stdout.length < 70_000);
});

test('safeEnvironment no permite extras fuera del allowlist efectivo', () => {
  const env = safeEnvironment({ ALLOWED: 'yes', SECRET_SHOULD_NOT_PASS: 'no' }, ['ALLOWED']);
  assert.equal(env.ALLOWED, 'yes');
  assert.equal(Object.hasOwn(env, 'SECRET_SHOULD_NOT_PASS'), false);
});

/* [10AA-3] La marca de reentrada de cm-heavy tiene que llegar a los comandos del gate:
 * el gate envuelve el ssh en cm-heavy y su etapa de Rust vuelve a entrar en cm-heavy, así
 * que si el filtro la borra, el hijo espera 600 s al candado de su propio abuelo y el gate
 * falla por «otro build ocupa el lock», que parece un fallo del proyecto. */
test('safeEnvironment deja pasar la marca de reentrada de cm-heavy', () => {
  const previo = process.env.CM_HEAVY_HELD;
  process.env.CM_HEAVY_HELD = '1';
  try {
    assert.equal(safeEnvironment().CM_HEAVY_HELD, '1');
  } finally {
    if (previo === undefined) delete process.env.CM_HEAVY_HELD;
    else process.env.CM_HEAVY_HELD = previo;
  }
});
