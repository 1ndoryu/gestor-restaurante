const STAGE_KEYS = new Set([
  'name',
  'executable',
  'args',
  'expectedSchemaVersion',
  'timeoutMs',
  'reportPath',
]);

export function sentinelStageDeclaration(values) {
  const declaration = Object.fromEntries(
    Object.entries(values).filter(([key, value]) => STAGE_KEYS.has(key) && value !== undefined),
  );
  return declaration;
}

/* Un proceso de transporte que escribió un reporte estructurado válido debe
 * terminar en cero. Sentinel decide PASS/FAIL leyendo sus findings; los
 * errores de setup anteriores a la escritura conservan código no-cero. */
export function sentinelTransportExitCode() {
  return 0;
}
