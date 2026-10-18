import { createHash } from 'node:crypto';
import { lstat, readFile, realpath } from 'node:fs/promises';
import path from 'node:path';
import { BLOCKED_CARGO_COMMANDS, BLOCKED_NPM_SCRIPTS, BLOCKED_TOOLS, DEFAULT_GATE_COMMAND } from './policy-defaults.mjs';
import { policyDecision } from './policy-decision.mjs';
import { isRecord, validatePolicy } from './policy-validacion.mjs';
// La validación v1 vive en policy-legacy.mjs; se importa aquí solo para la migración.
import { validateLegacyContracts } from './policy-legacy.mjs';

// Reexporta el validador v2 para que los importadores existentes sigan funcionando sin cambios.
export { validatePolicy };

const POLICY_FILE = 'sentinel.config.json';

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

export function defaultGuardPolicy() {
  return {
    npmScripts: [...BLOCKED_NPM_SCRIPTS],
    npxTools: [...BLOCKED_TOOLS],
    cargoSubcommands: [...BLOCKED_CARGO_COMMANDS],
    tools: ['rustfmt'],
  };
}

export function migrateLegacyConfig({ sentinelConfig, qualityConfig, varsenseConfig, toolManifest }) {
  if (!isRecord(sentinelConfig) || !isRecord(qualityConfig) || !isRecord(varsenseConfig) || !isRecord(toolManifest)) {
    throw new Error('No se puede migrar una configuración legacy incompleta');
  }
  validateLegacyContracts({ sentinelConfig, qualityConfig, varsenseConfig, toolManifest });
  const sentinelTool = toolManifest.tools?.sentinel;
  const protocolVersion = Number.isInteger(Number(sentinelTool?.outputSchemaVersion))
    ? Number(sentinelTool.outputSchemaVersion)
    : 1;
  const migrated = {
    schemaVersion: 2,
    mode: 'enforce',
    gate: { command: [...DEFAULT_GATE_COMMAND], taskIdRequired: true },
    guard: { directCommands: defaultGuardPolicy() },
    runtime: {
      minimumVersion: String(sentinelTool?.version ?? '0.4.0'),
      protocolVersion,
      lockFile: 'sentinel.lock.json',
    },
    analyzers: {
      sentinel: { enabled: true, profile: 'project-default', config: sentinelConfig },
      varsense: { enabled: true, profile: 'project-default', config: 'varsense.config.json' },
    },
  };
  validatePolicy(migrated);
  return {
    policy: migrated,
    mapped: {
      analyzers: {
        sentinel: { config: cloneJson(sentinelConfig) },
        varsense: { config: cloneJson(varsenseConfig) },
      },
      scheduler: {
        maxConcurrentStages: qualityConfig.maxConcurrentStages,
        timeoutsMs: cloneJson(qualityConfig.timeoutsMs ?? {}),
        heavyRun: cloneJson(qualityConfig.heavyRun ?? {}),
      },
      reporting: {
        maxFindings: qualityConfig.maxFindings,
        maxReminders: qualityConfig.maxReminders,
        maxTerminalLines: qualityConfig.maxTerminalLines,
        reportRetention: cloneJson(qualityConfig.reportRetention ?? {}),
      },
      scope: {
        fullPatterns: cloneJson(qualityConfig.fullPatterns ?? []),
        profiles: cloneJson(qualityConfig.profiles ?? {}),
      },
      tools: {
        schemaVersion: toolManifest.schemaVersion,
        installRoot: toolManifest.installRoot,
        tools: cloneJson(toolManifest.tools),
      },
    },
    legacy: {
      sentinelConfig: cloneJson(sentinelConfig),
      qualityConfig: cloneJson(qualityConfig),
      varsenseConfig: cloneJson(varsenseConfig),
      toolManifest: cloneJson(toolManifest),
    },
  };
}

function hashText(value) {
  return createHash('sha256').update(value).digest('hex');
}

function policyReason(discovered) {
  return discovered.status === 'policy'
    ? 'política v2 válida'
    : discovered.warning ?? discovered.error ?? discovered.status;
}

export function policyIdentity(discovered, runtimeVersion = null) {
  return {
    projectRoot: discovered.projectRoot,
    policyPath: discovered.policyPath,
    policyHash: discovered.policyHash ?? hashText(discovered.status),
    runtimeVersion,
    decision: policyDecision(discovered),
    reason: policyReason(discovered),
    recommendedCommand: discovered.status === 'legacy-v1'
      ? 'npm run quality:doctor -- --migrate --dry-run'
      : discovered.status === 'invalid-policy'
        ? 'npm run quality:doctor -- --json'
        : discovered.status === 'no-policy'
          ? 'sentinel check <task-id>'
          : 'npm run task:check -- <task-id>',
  };
}

export async function discoverPolicy(startPath) {
  let candidate = path.resolve(startPath);
  try {
    candidate = await realpath(candidate);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  while (true) {
    const policyPath = path.join(candidate, POLICY_FILE);
    const metadata = await lstat(policyPath).catch(error => {
      if (error?.code === 'ENOENT') return null;
      throw error;
    });
    if (metadata?.isSymbolicLink()) {
      return { projectRoot: candidate, policyPath, symlink: true };
    }
    if (metadata?.isFile()) return { projectRoot: candidate, policyPath };
    const parent = path.dirname(candidate);
    if (parent === candidate) return { projectRoot: null, policyPath: null };
    candidate = parent;
  }
}

export async function loadPolicy(startPath) {
  const discovered = await discoverPolicy(startPath);
  if (!discovered.policyPath) {
    return { status: 'no-policy', ...discovered, policyHash: hashText('no-policy') };
  }
  if (discovered.symlink) {
    return {
      status: 'invalid-policy',
      ...discovered,
      policyHash: hashText('symlink-policy'),
      error: 'sentinel.config.json no puede ser symlink o junction',
    };
  }
  const raw = await readFile(discovered.policyPath, 'utf8');
  const policyHash = hashText(raw);
  let parsed;
  try { parsed = JSON.parse(raw); }
  catch (error) {
    return { status: 'invalid-policy', ...discovered, policyHash, error: `JSON inválido: ${error.message}` };
  }
  if (parsed.schemaVersion === undefined) {
    return { status: 'legacy-v1', ...discovered, policyHash, warning: 'sentinel.config.json usa el formato de analizador v1; no se activa como política v2' };
  }
  try {
    validatePolicy(parsed);
    return { status: 'policy', ...discovered, policyHash, policy: parsed };
  } catch (error) {
    return { status: 'invalid-policy', ...discovered, policyHash, error: error.message };
  }
}
