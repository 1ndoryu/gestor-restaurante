import { appendFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { redact } from './redaction.mjs';

/* Excepción manual del guard y su auditoría. Va aparte porque es una política propia
 * (flag, env o token con motivo obligatorio) y su registro, separada del lease. */

/* [SNT-11] RE-ACTIVADO (2026-08-05, decisión explícita del usuario): la
 * excepción manual del guard vuelve a conceder saltos del cooldown de 180 min
 * usando `--allow-heavy --heavy-reason "<motivo>"` (o GLORY_HEAVY_RUN_TOKEN).
 * El cooldown NO se elimina: sigue bloqueando las ejecuciones pesadas normales
 * (sin excepción) y toda activación manual queda auditada en heavy-overrides.log
 * con granted:true/false, motivo, comando y PID. El motivo es requisito (un
 * intento sin motivo se rechaza). Únicamente CI (modo sancionado, no corre en
 * el equipo) sigue autorizado a full sin cooldown. */
export const HEAVY_MANUAL_OVERRIDE_ENABLED = true;

export function isHeavyOverride(options = {}) {
  if (options.ci) return true;
  if (!HEAVY_MANUAL_OVERRIDE_ENABLED) return false;
  return Boolean(
    options.allowHeavy
    || process.env.GLORY_QUALITY_ALLOW_HEAVY === '1'
    || process.env.GLORY_HEAVY_RUN_TOKEN,
  );
}

/* [028A-16] Fuente manual de la excepción del guard: flag, env o token. CI
 * es un modo sancionado (no una excepción manual) y no entra aquí. */
export function manualOverrideSource(options = {}) {
  if (options.allowHeavy) return 'flag';
  if (process.env.GLORY_QUALITY_ALLOW_HEAVY === '1') return 'env';
  if (process.env.GLORY_HEAVY_RUN_TOKEN) return 'token';
  return null;
}

/* [028A-16] El motivo de la excepción llega por flag o por env; si falta, la
 * excepción se rechaza y el intento queda registrado en el log de auditoría. */
export function overrideReason(options = {}) {
  const flag = typeof options.heavyReason === 'string' ? options.heavyReason.trim() : '';
  const env = typeof process.env.GLORY_HEAVY_RUN_REASON === 'string' ? process.env.GLORY_HEAVY_RUN_REASON.trim() : '';
  return flag || env;
}

/* [028A-16] Auditoría persistente de excepciones del guard: cada activación
 * de --allow-heavy / GLORY_QUALITY_ALLOW_HEAVY / GLORY_HEAVY_RUN_TOKEN queda
 * en .quality-reports/heavy-overrides.log con timestamp, source, comando,
 * cwd, PID y motivo. Nunca bloquea la decisión: un fallo de escritura solo se
 * reporta a stderr. */
export async function logHeavyOverride({ projectRoot, source, command, cwd, pid, reason, granted, taskId }) {
  try {
    const logDir = path.join(projectRoot, '.quality-reports');
    await mkdir(logDir, { recursive: true });
    /* [028A-16] El comando y el motivo se redactan igual que el resto del
     * pipeline: un comando con un secreto incrustado (p. ej. una URL con
     * credencial) no debe quedar en claro en el log de auditoría. */
    const entry = {
      version: 1,
      timestamp: new Date().toISOString(),
      source,
      command: redact(String(command ?? '')),
      cwd: cwd ?? process.cwd(),
      pid: pid ?? process.pid,
      reason: reason ? redact(String(reason)) : null,
      granted: Boolean(granted),
      taskId: taskId ?? null,
    };
    await appendFile(path.join(logDir, 'heavy-overrides.log'), `${JSON.stringify(entry)}\n`, 'utf8');
  } catch (error) {
    process.stderr.write(`[glory-quality] No se pudo escribir heavy-overrides.log: ${error.message}\n`);
  }
}
