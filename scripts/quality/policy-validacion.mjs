import path from 'node:path';

// Esquema v2 de sentinel.config.json. Vive fuera de policy.mjs para que cada módulo quede bajo el límite de líneas efectivas de Sentinel.
// Sus primitivas (isRecord, validateName, MAX_STRING_LENGTH) también las usa policy-legacy.mjs al validar la configuración v1.
const MAX_STRING_LENGTH = 160;
const NAME_PATTERN = /^[A-Za-z0-9:_*.-]+$/u;
const MODES = new Set(['enforce', 'observe', 'pass-through']);
const ROOT_KEYS = new Set(['schemaVersion', 'mode', 'project', 'gate', 'guard', 'runtime', 'analyzers']);
const PROJECT_KEYS = new Set(['primaryBranch']);
const ANALYZER_KEYS = new Set(['enabled', 'profile', 'config']);
const RUNTIME_KEYS = new Set(['minimumVersion', 'protocolVersion', 'lockFile']);
const GATE_KEYS = new Set(['command', 'taskIdRequired']);
const GUARD_KEYS = new Set(['directCommands']);
const DIRECT_COMMAND_KEYS = new Set(['npmScripts', 'npxTools', 'cargoSubcommands', 'tools']);

function fail(message) {
  throw new Error(`sentinel.config.json: ${message}`);
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function validateKeys(value, allowed, label) {
  const unknown = Object.keys(value).filter(key => !allowed.has(key));
  if (unknown.length > 0) fail(`${label}: claves desconocidas: ${unknown.join(', ')}`);
}

function validateName(value, label) {
  if (typeof value !== 'string' || value.length === 0 || value.length > MAX_STRING_LENGTH || !NAME_PATTERN.test(value)) {
    fail(`${label}: nombre inválido`);
  }
}

function validateStringList(value, label) {
  if (!Array.isArray(value) || value.length > 128) fail(`${label}: debe ser una lista de nombres`);
  for (const item of value) validateName(item, label);
}

function validateRelativePath(value, label) {
  if (typeof value !== 'string' || value.length === 0 || value.length > MAX_STRING_LENGTH || path.isAbsolute(value)) {
    fail(`${label}: debe ser una ruta relativa`);
  }
  const normalized = value.replace(/\\/g, '/');
  if (normalized.split('/').includes('..')) fail(`${label}: no puede salir del workspace`);
}

function validateProject(value) {
  if (!isRecord(value)) fail('project debe ser un objeto');
  validateKeys(value, PROJECT_KEYS, 'project');
  if (typeof value.primaryBranch !== 'string' || value.primaryBranch.length === 0 || value.primaryBranch.length > 127 || /[\u0000-\u001f\u007f\s]/u.test(value.primaryBranch)) {
    fail('project.primaryBranch: nombre inválido');
  }
  const branch = value.primaryBranch;
  const invalidBranch = branch.includes('..')
    || branch.includes('@{')
    || /[~^:?*\[\]\\]/u.test(branch)
    || branch.includes('//')
    || branch.endsWith('/')
    || branch.endsWith('.')
    || branch.split('/').some(component => component.length === 0 || component.startsWith('.') || component.endsWith('.') || component.toLowerCase().endsWith('.lock'));
  if (invalidBranch) fail('project.primaryBranch debe ser un nombre de rama Git válido');
}

function validateAnalyzer(value, label) {
  if (!isRecord(value)) fail(`${label}: debe ser un objeto`);
  validateKeys(value, ANALYZER_KEYS, label);
  if (typeof value.enabled !== 'boolean') fail(`${label}.enabled inválido`);
  if (value.profile !== undefined) validateName(value.profile, `${label}.profile`);
  if (value.config !== undefined) {
    if (typeof value.config === 'string') validateRelativePath(value.config, `${label}.config`);
    else if (!isRecord(value.config)) fail(`${label}.config inválido`);
  }
}

export function validatePolicy(policy) {
  if (!isRecord(policy)) fail('la raíz debe ser un objeto');
  validateKeys(policy, ROOT_KEYS, 'raíz');
  if (policy.schemaVersion !== 2) fail('schemaVersion debe ser 2');
  if (typeof policy.mode !== 'string' || !MODES.has(policy.mode)) fail('mode inválido');
  if (policy.project !== undefined) validateProject(policy.project);

  if (!isRecord(policy.gate)) fail('gate debe ser un objeto');
  validateKeys(policy.gate, GATE_KEYS, 'gate');
  if (!Array.isArray(policy.gate.command) || policy.gate.command.length < 2 || policy.gate.command.length > 8) {
    fail('gate.command inválido');
  }
  for (const item of policy.gate.command) validateName(item, 'gate.command');
  if (typeof policy.gate.taskIdRequired !== 'boolean') fail('gate.taskIdRequired inválido');

  if (!isRecord(policy.guard)) fail('guard debe ser un objeto');
  validateKeys(policy.guard, GUARD_KEYS, 'guard');
  if (!isRecord(policy.guard.directCommands)) fail('guard.directCommands debe ser un objeto');
  validateKeys(policy.guard.directCommands, DIRECT_COMMAND_KEYS, 'guard.directCommands');
  for (const key of DIRECT_COMMAND_KEYS) validateStringList(policy.guard.directCommands[key], `guard.directCommands.${key}`);

  if (!isRecord(policy.runtime)) fail('runtime debe ser un objeto');
  validateKeys(policy.runtime, RUNTIME_KEYS, 'runtime');
  validateName(policy.runtime.minimumVersion, 'runtime.minimumVersion');
  if (!Number.isInteger(policy.runtime.protocolVersion) || policy.runtime.protocolVersion < 1 || policy.runtime.protocolVersion > 100) {
    fail('runtime.protocolVersion inválido');
  }
  validateRelativePath(policy.runtime.lockFile, 'runtime.lockFile');

  if (!isRecord(policy.analyzers)) fail('analyzers debe ser un objeto');
  validateKeys(policy.analyzers, new Set(['sentinel', 'varsense']), 'analyzers');
  validateAnalyzer(policy.analyzers.sentinel, 'analyzers.sentinel');
  validateAnalyzer(policy.analyzers.varsense, 'analyzers.varsense');
  return policy;
}

export { isRecord, validateName, MAX_STRING_LENGTH };
