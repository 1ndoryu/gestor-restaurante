# Plan — Trazabilidad completa y limpieza segura de tareas Sentinel

> **Fecha:** 2026-08-07
> **Alcance:** coordinador `sentinel task` del submódulo `tools/sentinel` y su operación desde el consumidor.
> **Objetivo:** que al terminar una tarea sea posible saber qué era, quién la tomó, cuándo ocurrió cada transición, qué plan la originó, qué cambió, qué gate pasó, qué recursos quedan y por qué algo no se puede limpiar.
> **Regla:** no borrar worktrees, ramas ni archivos de otro proyecto o de otra tarea; toda limpieza destructiva debe ser explícita, verificable y precedida por un diagnóstico.

## 1. Problema actual

Sentinel ya coordina el ciclo básico `claim → start → heartbeat → gate → integrate → cleanup → release`, mantiene metadata en `.sentinel/coordination/` y detecta algunos worktrees/ramas huérfanos. Sin embargo:

- la metadata activa se elimina al hacer cleanup y no queda un historial consultable;
- no se guardan de forma estructurada el plan relacionado, el propósito, los commits, los archivos cambiados ni cada resultado del gate;
- `task status` está centrado en la rama primaria y no ofrece una vista global de todas las identidades/tareas del repositorio;
- la detección compara principalmente Git contra metadata activa y puede omitir carpetas físicas antiguas cuyo registro Git ya fue podado;
- el operador debe reconstruir fechas, bloqueos y decisiones desde logs, commits y carpetas manualmente;
- no existe una separación clara entre «recurso activo», «histórico» y «huérfano pendiente de revisión».

## 2. Resultado deseado

Un operador debe poder ejecutar:

```text
sentinel task status --project-root . --all --json
```

y obtener:

1. tareas activas y su estado derivado;
2. historial de tareas ya integradas, liberadas o recuperadas;
3. línea temporal de transiciones con actor, fecha y motivo;
4. plan/documento relacionado y resumen declarado;
5. commits y archivos cambiados calculados desde Git;
6. todos los gates ejecutados, su resultado y modo;
7. ramas, worktrees, locks y carpetas físicas vinculadas;
8. huérfanos detectados, motivo, evidencia y acción segura recomendada;
9. advertencia explícita cuando una limpieza requiere revisión humana.

La salida humana debe resumir lo mismo sin ocultar los bloqueos; JSON debe ser estable y versionado para que una UI o un script pueda consumirlo.

## 3. Contrato de datos

### 3.1 TaskRecord compatible

Mantener `TaskRecord` schema v3 y añadir campos obligatorios al crear registros, pero aceptar registros v3 antiguos rellenando defaults al leerlos:

- `summary`: propósito breve de la tarea;
- `planReference`: ruta relativa al documento/plan de origen;
- `relatedTaskIds`: tareas relacionadas;
- `history`: eventos append-only del lifecycle;
- `gateRuns`: resultados estructurados de cada gate;
- `commits`: hashes de commits producidos;
- `changedFiles`: rutas relativas modificadas.

No se guardarán payloads completos del gate ni secretos. Los textos tendrán límites de longitud y las rutas serán relativas/normalizadas cuando procedan.

### 3.2 Eventos

Cada transición escribirá un evento con:

- `eventId`, `at` ISO-8601 y `actor`;
- acción (`CLAIM`, `START`, `HEARTBEAT`, `GATE`, `INTEGRATE`, `CLEANUP`, `RECOVER`, `RELEASE`);
- estado anterior y nuevo;
- motivo/resultado resumido;
- datos seguros: commit, exit code, cantidad de archivos o causa de bloqueo.

Los eventos se escriben atómicamente junto con la metadata bajo el lock de la tarea. Un heartbeat puede registrarse con una política de muestreo para no crear un archivo ilimitado; la primera versión registrará heartbeat explícito, con límite de historial.

### 3.3 Archivo histórico

Antes de eliminar metadata activa, `cleanup`, `release` y `recover` escribirán un archivo inmutable en:

```text
.sentinel/history/<project-identity>/<task-id>-<timestamp>-<event-id>.json
```

El archivo contiene el último `TaskRecord`, el estado terminal (`CLEANED`, `RELEASED` o `RECOVERED`), la fecha de archivado y el evento final. Nunca se sobrescribe: si se reutiliza un task-id, genera otra entrada.

## 4. Captura de relación, cambios y gate

### 4.1 Relación con el trabajo

Añadir a `claim` y al CLI:

```text
--summary "..."
--plan Agente/planes/mi-plan.md
--related-task OTRO-ID
```

`--plan` debe ser relativo, no absoluto, sin `..` y permanecer dentro del projectRoot; el resumen y los IDs deben estar acotados. La metadata debe mostrar claramente si la tarea no declaró plan.

### 4.2 Commits y archivos

Al integrar, calcular desde `baseHead..head`:

- `git log --format=%H` para commits;
- `git diff --name-only` para archivos relativos;
- conservar ambos en el registro aunque luego se elimine la rama.

Si no hay commit nuevo, integración sigue bloqueada como hoy. Si Git no permite calcular la evidencia, no se debe afirmar que la tarea está completa: registrar el bloqueo.

### 4.3 Gate

Después de cada `task gate`, registrar:

- timestamp, actor y modo (`local`, `full`, `ci`);
- exit code y estado (`PASS`, `FAIL`, `ERROR`);
- una referencia segura al reporte si existe, sin copiar el reporte entero.

El resultado debe persistir tanto si el gate pasa como si falla. Un gate fallido no cierra la tarea ni permite cleanup integrado, pero deja evidencia para explicar el pendiente.

## 5. Diagnóstico global

Extender `task status` con `--all`:

- sin `--all`: conserva el alcance actual por proyecto/primaryBranch para compatibilidad;
- con `--all`: agrega archivos de coordinación e historial de todas las identidades del mismo Git common dir;
- enumera ramas `task/<identity>/<id>` y worktrees registrados;
- inspecciona también carpetas físicas bajo `.sentinel/worktrees/`, incluso cuando Git ya las marca prunable;
- distingue worktree registrado, activo, huérfano Git, carpeta huérfana y rama huérfana;
- no confunde ramas de usuario como `task/028A-18-root` con el namespace coordinado si no cumplen el patrón de identidad.

Cada huérfano debe incluir `kind`, `path/branch`, `detectedAt`, `reason`, `taskId`/identity si se puede inferir y `cleanup: manual-review` o `cleanup: safe-prune`.

## 6. Cleanup y recuperación

### 6.1 Cierre normal

El flujo correcto queda:

```text
integrate → verificar ancestry → archivar historial → eliminar worktree → eliminar rama → eliminar metadata activa → release
```

Si una etapa falla, se conserva metadata y se reporta el recurso restante. Nunca se elimina metadata antes de que el recurso haya sido validado y retirado.

### 6.2 Tarea interrumpida

`recover` sigue exigiendo TTL vencido, PID muerto, HEAD consistente y worktree limpio. Antes de limpiar archivará el evento de recuperación. Un worktree sucio, una rama avanzada o una metadata manipulada producen bloqueo explícito, no borrado forzado.

### 6.3 Huérfanos

La primera mejora es diagnóstico y poda administrativa (`git worktree prune`) solo para registros Git prunable. La eliminación de una carpeta física huérfana requiere una segunda orden explícita, modo `--dry-run` por defecto y prueba de ownership por namespace; si no puede demostrarse ownership, se deja para revisión humana. Nunca se ejecutará `rm -rf` genérico sobre una raíz visible o un proyecto hermano.

## 7. Compatibilidad y migración

- aceptar TaskRecord v2 y v3 antiguos;
- completar campos nuevos con valores vacíos y registrar evento `MIGRATED` una sola vez;
- mantener `taskStatus(projectRoot, primaryBranch)` para consumidores TypeScript existentes;
- añadir opciones de CLI sin cambiar los formatos existentes salvo nuevos campos aditivos;
- versionar el archivo histórico por separado;
- actualizar README, CHANGELOG y skill de quality gate con el flujo de cierre.

## 8. Pruebas obligatorias

1. claim guarda resumen, plan, relaciones y evento inicial;
2. start/heartbeat agregan eventos y conservan fechas;
3. gate PASS y FAIL quedan registrados sin cambiar estado incorrectamente;
4. integrate captura commits y archivos cambiados;
5. cleanup archiva antes de retirar metadata y permite consultar `--all`;
6. recover archiva `RECOVERED` y nunca limpia PID vivo/worktree sucio;
7. reutilizar task-id crea historial independiente;
8. metadata v2/v3 antigua migra sin perder recursos;
9. `--all` detecta ramas/worktrees coordinados de otras identidades;
10. detecta carpetas físicas huérfanas después de `git worktree prune`;
11. no clasifica ramas `task/*` no coordinadas como huérfanas Sentinel;
12. rutas de plan, resumen e historial rechazan traversal, absolutos y valores excesivos;
13. cleanup no puede tocar otro worktree aunque se manipule metadata;
14. salida JSON es determinista y la salida humana explica bloqueos.

## 9. Orden de implementación

1. Extender contratos y normalización compatible.
2. Añadir helper de eventos e historial atómico.
3. Capturar plan/summary/relaciones desde claim/CLI.
4. Registrar gate y evidencia de integración.
5. Archivar en cleanup/recover/release.
6. Implementar `status --all` y detección física segura.
7. Añadir pruebas unitarias e integración.
8. Actualizar documentación y ejecutar compile, `check:core`, `smoke:lsp` y suite completa.

## 10. Implementación realizada en la extensión local (2026-08-08)

Se implementó en `tools/sentinel` —todavía como extensión local sin commit/publicación— lo siguiente:

- `TaskRecord` v3 conserva `summary`, `planReference`, `relatedTaskIds`, eventos acotados, gates, commits y archivos cambiados; la lectura sigue migrando registros v2 y acepta v3 antiguos con defaults.
- `claim` acepta `--summary`, `--plan` y `--related-task`, con validación de longitud, task-id y rutas relativas sin traversal.
- Cada claim, refresh, start, heartbeat, gate e integración deja un evento con fecha ISO, actor, estados, resultado y/o exit code.
- `task gate` registra PASS/FAIL y errores de ejecución como ERROR; `integrate` calcula evidencia Git de commits y archivos.
- `cleanup`, `release` y `recover` conservan un archivo inmutable bajo `.sentinel/history/<identity>/`; cleanup archiva solo después de retirar recursos correctamente y release no acepta metadata que aún conserve recursos.
- `task status --all --json` reúne tareas activas de todos los namespaces detectados, historial validado, ramas/worktrees coordinados, locks expirados y carpetas físicas huérfanas de raíces internas o externas conocidas; limita la lectura del historial y valida rutas/identidades antes de inspeccionar.
- La detección y cleanup mantienen validaciones de namespace, contención física, registro Git, HEAD, PID, suciedad y ascendencia; no hay poda automática de recursos ambiguos.

Validación local completada: `npm run compile`, `npm run check:core`, `npm run smoke:lsp` y suite Mocha: **519 passing / 1 pending**. El gate del consumidor, repin, lock, commit y publicación quedan deliberadamente fuera hasta revisar y aprobar esta ampliación.

## 11. Criterio de terminado

El trabajo se considera completo cuando un task-id terminado puede reconstruirse sin consultar logs externos, `status --all` no oculta recursos coordinados huérfanos, cleanup normal deja cero recursos activos, recuperación falla cerrado ante ambigüedad y las pruebas demuestran que ninguna operación destructiva escapa del namespace autorizado.

**Fuera de este bloque:** UI gráfica, sincronización con Linear/Jira y limpieza automática de worktrees de otros repositorios. Se dejan como fases posteriores porque requieren contratos externos y más superficie de permisos.

## 12. Verificación de cierre (2026-08-09)

Verificado contra la implementación real del consumidor `glory-rs-rest`, no solo contra la documentación:

- `npm run quality:lock -- --check` → `pass: match` (el lock del consumidor reproduce el manifest y el estado de los submódulos).
- `sentinel doctor --json` → `ready: true`, sin issues; `releaseEvidencePresent: true`, checkout limpio, lock y gitlink alineados en `00fe0c7e6b2ade865c7156546d0d858e34214f95`.
- `sentinel task status --all --json` → cero tareas activas, cero huérfanos (ramas, worktrees, físicos) y 3 entradas de historial inmutable (`048A-22` RELEASED, `048A-22` CLEANED y `066A-01` RELEASED) bajo `.sentinel/history/7a415955a7c3d121/`.
- El gate del consumidor (repin + lock + commit + publicación) quedó completado: gitlink en `tools/sentinel` al commit local publicado y el consumidor sincronizado con `origin/glory-rs-rest`.

Todas las metas del plan quedan cumplidas y verificadas; no queda trabajo dentro del alcance salvo publicar el commit documental correspondiente (requiere autorización explícita).

## Checklist de cierre

- [x] Trazabilidad implementada como extensión local y validada (Mocha 519 passing / 1 pending).
- [x] Repin, lock, commit y publicación del gate del consumidor verificados (2026-08-09): pin `00fe0c7`, lock match, `sentinel doctor` ready y zero recursos activos/huérfanos en `task status --all --json`.
