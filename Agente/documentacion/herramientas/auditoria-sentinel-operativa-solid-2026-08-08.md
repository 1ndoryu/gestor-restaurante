# Auditoría consolidada de Glory Sentinel — operación y SOLID

**Fecha de verificación:** 2026-08-08
**Proyecto:** `glory-rust-template`
**Rama del consumidor:** `glory-rs-rest`
**Runtime auditado:** Glory Sentinel `0.6.4`, commit fijado `cfac119`
**Propósito:** consolidar la auditoría operativa recibida y la auditoría arquitectónica SOLID, separar hechos comprobados de interpretaciones y dejar un plan accionable.

> **Dictamen actual:** **NO APTO para cerrar o coordinar nuevas tareas sin recuperación operativa previa.** La razón inmediata no es una supuesta falta general de controles: el problema es que el checkout fijado de Sentinel está modificado localmente, el lock lo bloquea correctamente, existe metadata de una tarea antigua que apunta a worktrees rotos y el último reporte del consumidor no es PASS. La arquitectura SOLID muestra deuda real en los orquestadores, pero no requiere una reescritura inmediata para resolver el bloqueo operativo.

---

## 1. Resumen ejecutivo

### Bloqueos comprobados

1. **El submódulo Sentinel está sucio aunque su HEAD coincide con el pin.** `tools/sentinel` está en `cfac119`, pero tiene seis archivos tracked modificados. `lock-generator --check` y `sentinel-doctor --lock` terminan con código `2` y rechazan confiar en el lock.
2. **Hay metadata antigua ACTIVE y dos carpetas físicas con enlaces Git rotos.** `028A-22.json` usa schema v1, apunta a una ruta antigua inexistente y conserva la rama `task/157fb8a2b2a4e1dc/028A-22`; las dos carpetas bajo `.sentinel/worktrees/` tienen `.git` que referencia worktrees inexistentes.
3. **El último reporte de `glory-rs-rest` no es PASS.** El reporte `048A-11` tiene Sentinel en `fail`, VarSense en `error`, Rust y frontend en `error`, y docs en `fail`. Sentinel registró 11 errores, 999 warnings, 3 information y 22 hints; VarSense registró `Invalid string length`. Esto demuestra stages fallidos; no se debe llamar automáticamente a todo el reporte `SETUP ERROR` ni atribuirle exit 2 sin leer el campo de salida correspondiente.
4. **El alcance configurado excluye `.tsx` de ambos analizadores declarativos.** El repositorio contiene 111 archivos `.tsx`; `sentinel.config.json` incluye `frontend/**/*.ts`, no `frontend/**/*.tsx`, y `varsense.config.json` incluye `frontend/src/**/*.ts`, no `frontend/src/**/*.tsx`.
5. **La implementación actual permite una raíz externa de worktrees.** `taskCoordinator.ts` documenta y acepta `--worktrees-root` fuera del repositorio. Eso contradice la política operativa vigente, que exige worktrees dentro de `<repo>/.sentinel/worktrees/`. La discrepancia debe resolverse como contrato, no dejarse a criterio de cada tarea.

### Afirmaciones que no se confirmaron o quedaron matizadas

- **SNT-16c:** no existe evidencia actual en `.sentinel`, refs locales ni metadata inspeccionable; no se debe presentar como hecho vigente sin recuperar otra fuente histórica.
- **Hook de PowerShell hacia el checkout anterior:** no se confirmó. Los dos perfiles inspeccionados apuntan al shim instalado en `C:\Users\Owner\AppData\Local\GlorySentinel\shims\global-cargo-guard.ps1`, y el shim existe. Puede haber un problema de sesión distinta, pero no el error descrito en esta inspección.
- **“La rama `task/*` tiene exactamente tres ramas”**: sí se observaron tres ramas task locales, pero una de ellas (`task/gate-hardening`) corresponde a un checkout hermano y no debe atribuirse automáticamente a una tarea del proyecto actual.
- **“La coordinación quedó completamente rota”**: es demasiado amplio. El coordinador y los comandos de task existen y exponen status/recover; lo roto es el estado operativo heredado y la consistencia del release fijado.

### Arquitectura SOLID

La auditoría SOLID es técnicamente razonable como diagnóstico arquitectónico, pero algunas conclusiones son evaluaciones de diseño, no fallos funcionales demostrados. Los hechos más sólidos son:

- `src/cli/index.ts` tiene 1032 líneas y combina parsing, routing, I/O, análisis, tareas, leases, runtime, señales y códigos de salida.
- `src/core/taskCoordinator.ts` tiene 1064 líneas y depende directamente de filesystem, Git, procesos, locks, metadata, paths y lifecycle.
- varios módulos core dependen directamente de `fs`, `child_process`, `process.env` y APIs Node;
- existen estados globales mutables para roots, reglas, caches y procesos;
- `analyzeDocument` importa y selecciona analizadores concretos mediante ramas por lenguaje.

La prioridad correcta es **primero recuperar y estabilizar el contrato operativo; después extraer puertos/contextos en cambios pequeños**, con pruebas de regresión. No conviene iniciar una reescritura SOLID mientras el release y la coordinación están sucios.

---

## 2. Método y niveles de certeza

Cada afirmación se clasificó así:

- **Confirmado:** observado directamente en archivos, código, estado Git o comando reproducible.
- **Confirmado parcialmente:** el núcleo es cierto, pero la formulación original exagera el alcance o mezcla recursos de otros checkouts.
- **No confirmado:** no apareció evidencia en el checkout actual o la afirmación depende de una fuente que no está disponible.
- **Evaluación arquitectónica:** juicio técnico razonado a partir del código; no implica por sí solo un bug.
- **Pendiente:** requiere una prueba adicional, publicación, autorización o decisión de contrato.

No se ejecutaron comandos destructivos, `cleanup`, `prune`, `reset`, `checkout`, instalaciones ni operaciones contra producción durante esta verificación.

---

## 3. Verificación de la auditoría operativa

### 3.1 Matriz de hallazgos

| ID | Afirmación auditada | Estado | Evidencia actual | Consecuencia |
|---|---|---|---|---|
| SNT-AUD-01 | Metadata ACTIVE antigua y worktrees físicos rotos | **Confirmado** | `.sentinel/coordination/157fb8a2b2a4e1dc/028A-22.json`; dos carpetas bajo `.sentinel/worktrees/` con `.git` apuntando a `.git/worktrees/...` inexistentes | No ejecutar takeover ni borrar manualmente hasta recuperar ownership y decidir cleanup auditable |
| SNT-AUD-02 | Sentinel fijado en `cfac119` pero con seis cambios tracked | **Confirmado** | `git submodule status`; `git -C tools/sentinel status`; archivos: `CHANGELOG.md`, `README.md`, `src/cli/index.ts`, `src/core/taskCoordinator.ts`, `src/core/taskRecovery.ts`, `src/test/suite/taskCoordinator.test.ts` | El lock no puede representar de forma reproducible el runtime real |
| SNT-AUD-03 | El reporte contiene referencias contaminantes de worktrees/copies | **Confirmado parcialmente** | El reporte `048A-11` contiene rutas absolutas de `.sentinel/worktrees/...028A-22`; el config no excluye explícitamente `.sentinel/worktrees/**` ni `gate-hardening/**`. Esto no prueba por sí solo que se haya escaneado `gate-hardening` | Hay que fijar exclusiones y comprobar el alcance efectivo |
| SNT-AUD-04 | `.tsx` fuera de Sentinel y VarSense | **Confirmado** | 111 `.tsx`; Sentinel solo declara `frontend/**/*.ts`; VarSense solo `frontend/src/**/*.ts` | Type-check cubre archivos que los analizadores de reglas/tokens no cubren |
| SNT-AUD-05 | Release permite raíces externas mientras otra política/documentación exige raíces internas | **Confirmado como contradicción de contrato** | `resolveWorktreePath()` acepta `worktreesRoot` externo; CLI publica `--worktrees-root`; comentarios `[VISIBLE-WORKTREE]`; la skill global vigente describe por defecto `.sentinel/worktrees/` | Contrato ambiguo: no se debe usar adopción coordinada hasta decidir y probar un único modelo; la fuente canónica debe declarar si la raíz externa visible es una excepción autorizada |
| SNT-AUD-06 | Perfil PowerShell apunta al checkout anterior | **No confirmado en la inspección actual** | Ambos perfiles apuntan al shim instalado en `AppData/Local/GlorySentinel`; el shim existe y declara su runtime local | Mantener como incidencia pendiente solo si otra sesión reproduce el error; no corregir perfiles a ciegas |
| SNT-AUD-07 | No hay PASS vigente del consumidor | **Confirmado** | Reporte `048A-11`: Sentinel `fail`, VarSense `error`, Rust/frontend `error`, docs `fail` | No declarar la rama apta; repetir gate solo después de corregir setup/alcance/estado |
| SNT-AUD-08 | Gobernanza/documentación de raíz presuntamente incompleta | **No confirmado como ausencia de AGENTS** | El checkout sí contiene `AGENTS.MD`; en Windows no debe tratarse como inexistente por las mayúsculas. La afirmación adicional sobre `roadmap.md` requiere una búsqueda/lectura específica | Verificar referencias de Sentinel/VarSense en `roadmap.md` y conservar un único archivo de instrucciones |
| SNT-AUD-09 | Existe una toma SNT-16c expirada | **No confirmado** | No apareció `SNT-16c` en `.sentinel` ni en refs/metadata actuales | No recuperar ni liberar una tarea que no está identificada en la evidencia disponible |

### 3.2 Estado exacto de la coordinación heredada

La metadata encontrada contiene:

```text
schemaVersion: 1
 taskId: 028A-22
 state: ACTIVE
 agent: buffy-sentinel-desktop
 branch: task/157fb8a2b2a4e1dc/028A-22
 base/target: wandorius
 worktree: .../glory-rust-template/.sentinel/worktrees/glory-rust-template-157fb8a2b2a4e1dc-028A-22
 updatedAt: 2026-08-07T09:44:16.158Z
```

La ruta declarada no existe en el checkout actual. También se observaron dos carpetas físicas con enlaces Git rotos:

- `.sentinel/worktrees/glory-rust-template-157fb8a2b2a4e1dc-028A-22`
- `.sentinel/worktrees/wandorius-sentinel-desktop-coordinator`

Sus archivos `.git` referencian directorios desaparecidos bajo `glory-rust-template/.git/worktrees/`. Git, por su parte, lista actualmente tres worktrees registrados: el checkout principal, `opencode/playful-lagoon` y `wandorius-gate-hardening`. Los dos directorios físicos anteriores no aparecen como registros Git válidos.

**Decisión operativa:** congelar `claim`, `start`, `integrate` y `cleanup` hasta que el dueño confirme si `028A-22` y las ramas relacionadas son recuperables. La limpieza puede ser segura, pero debe ejecutarse con una operación auditable que valide namespace, rama, path y ausencia de cambios; no con `rm` manual indiscriminado.

### 3.3 Lock y release

La verificación ejecutada fue:

```text
node scripts/quality/lock-generator.mjs --check
→ exit 2
→ sentinel: checkout modificado; no se puede confiar en sentinel.lock.json

node scripts/quality/sentinel-doctor.mjs --lock
→ exit 2
→ sentinel: checkout modificado; no se puede confiar en sentinel.lock.json
```

Esto confirma que el control funciona según lo diseñado: el problema no es que `lock-check` deje pasar el drift, sino que existe drift local.

El consumidor declara:

- Sentinel: versión `0.6.4`, commit `cfac119af6479923ec9a7dc9ccc2408a17239e24`.
- VarSense: versión `2.2.0`, commit `e8360927ee92c4067f1f501dd77b951c8bc4f61d`.

Pero la carpeta fuente de Sentinel no está limpia. No debe regenerarse el lock para ocultar esto. Hay que elegir explícitamente entre:

1. publicar los seis cambios como una nueva release/commit y repinear el consumidor; o
2. retirar esos cambios con autorización del dueño, dejando el submódulo exactamente en `cfac119`.

### 3.4 Alcance real del último reporte

El reporte inspeccionado está en:

```text
.quality-reports/branches/glory-rs-rest--f100af0a041e6e8a/048A-11/
```

Estados registrados:

- `sentinel`: `fail`, estado `findings`;
- `varsense`: `error`, estado `tool-error`;
- `rust`: `error`;
- `frontend`: `error`;
- `docs`: `fail`;
- `custom`: `pass`.

Contadores Sentinel: **11 errores, 999 warnings, 3 information y 22 hints**; 691 archivos totales y 184 con violaciones. VarSense registra `Invalid string length`.

El `sentinel-analyzer-config.json` materializado por el reporte contiene:

```json
{
  "includePatterns": [
    "**/*.rs",
    "frontend/**/*.ts",
    "frontend/**/*.css",
    "scripts/**/*.mjs",
    "scripts/**/*.ps1"
  ],
  "excludePatterns": [
    "**/node_modules/**",
    "**/dist/**",
    "**/out/**",
    "**/target/**",
    "**/.quality-tools/**",
    "**/.quality-reports/**",
    "tools/sentinel/**",
    "frontend/src/api/generated/**",
    "frontend/src/utils/dom.ts"
  ]
}
```

La auditoría acierta al señalar que no están excluidos explícitamente `.sentinel/worktrees/**` ni `gate-hardening/**`. El reporte además conserva rutas de `028A-22`, evidencia de contaminación del reporte o del alcance efectivo; por sí sola no demuestra que cada ruta haya sido analizada. Antes de cambiar reglas, hay que corregir el alcance y repetir el gate para distinguir findings reales de contaminación.

### 3.5 Cobertura `.tsx`

Conteo verificado: **111 archivos `.tsx`**.

Configuración actual:

```text
sentinel.config.json  → frontend/**/*.ts
varsense.config.json  → frontend/src/**/*.ts
```

Los includes no contienen `tsx`. La corrección prevista debe ser explícita y acotada:

```text
Sentinel: frontend/**/*.ts y frontend/**/*.tsx
VarSense: frontend/src/**/*.ts y frontend/src/**/*.tsx
```

Después hay que comprobar que el runtime realmente acepta esos patrones y que el conteo/resultado del reporte aumenta de forma esperada. No basta con modificar JSON: el gate debe volver a ejecutarse.

### 3.6 Frontera física de worktrees

El código actual implementa dos modos:

- sin `--worktrees-root`: raíz interna por defecto bajo `.sentinel/worktrees/`;
- con `--worktrees-root`: raíz externa declarada y validada fuera del repositorio, para hacer visible el worktree.

La política global y la auditoría operativa actual dicen que la política obligatoria es interna. Esto genera una contradicción deliberada entre la necesidad de visibilidad del agente y la política de aislamiento.

**Recomendación:** no eliminar todavía el modo externo sin decidir el problema original del agente. Primero elegir uno de estos contratos:

- **Contrato estricto interno:** todo worktree vive bajo `.sentinel/worktrees/`; se añade un mecanismo de visibilidad del workspace/agente que no dependa de pasar rutas manualmente.
- **Contrato externo controlado:** se permite solo una raíz visible declarada por proyecto (por ejemplo `area-trabajo/task-worktrees`), con identidad Git, `realpath`, contención física, permisos de tarea y cleanup completo. En ese caso la política global debe actualizarse para decir “raíz autorizada”, no “siempre interna”.

Mientras la política diga “interno” y el release acepte externo, el estado es **no apto para adopción**.

### 3.7 Hook global de PowerShell

La afirmación recibida no coincide con la verificación actual:

- `Documents/WindowsPowerShell/Microsoft.PowerShell_profile.ps1` existe;
- `Documents/PowerShell/Microsoft.PowerShell_profile.ps1` existe;
- ambos cargan `C:\Users\Owner\AppData\Local\GlorySentinel\shims\global-cargo-guard.ps1`;
- el shim existe y apunta a `C:\Users\Owner\AppData\Local\GlorySentinel`.

Debe conservarse como **control a repetir desde una sesión de PowerShell nueva**, pero no se debe editar ni reinstalar el perfil basándose solo en la auditoría.

---

## 4. Verificación de la auditoría SOLID

### 4.1 Criterio de lectura

SOLID aquí se usa como guía para reducir motivos de cambio, dependencias ocultas y contratos débiles. No se interpreta como una obligación de convertir cada helper en una clase ni como prueba automática de defectos.

### 4.2 Hallazgos comprobables

| ID | Principio | Hallazgo | Estado | Evidencia |
|---|---|---|---|---|
| SOLID-01 | SRP/DIP | CLI demasiado amplia | **Evaluación respaldada** | `src/cli/index.ts` tiene 1032 líneas; contiene parser, ayuda, análisis, I/O, runtime, leases, task, señales y exit codes |
| SOLID-02 | SRP/DIP | TaskCoordinator concentra lifecycle e infraestructura | **Evaluación respaldada** | `src/core/taskCoordinator.ts` tiene 1064 líneas; importa `fs`, `path`, `os`, `crypto`, `child_process` y cubre metadata, locks, Git, worktrees, lifecycle y status |
| SOLID-03 | DIP | Estado global mutable | **Confirmado como hecho; impacto evaluativo** | `workspaceRoots.ts` mantiene `let workspaceRoots`; rule registry mantiene cache/proveedor; React mantiene roots/cache; tool runner mantiene procesos activos |
| SOLID-04 | OCP | Dispatcher de analizadores concretos | **Confirmado** | `analyzeDocument.ts` importa seis analizadores y selecciona PHP/TSX/Rust mediante ramas; además llama Glory/API y extras |
| SOLID-05 | OCP/SRP | Catálogo de reglas y comportamiento distribuidos | **Parcial** | `ruleRegistry.ts` es la fuente de metadatos/estado activo; `defaultRules.ts` contiene implementaciones/reglas estáticas. La duplicación exacta de IDs/metadatos no quedó demostrada por la búsqueda heurística |
| SOLID-06 | SRP/DIP | Runtime, shims y diagnóstico mezclan infraestructura/presentación | **Evaluación respaldada** | `runtimeInstall.ts` 549 líneas; `interceptorShims.ts` 697; `diagnose.ts` 533; todos combinan ejecución y resultados de operación |
| SOLID-07 | ISP | `ParsedCliArgs` y opciones de tareas son bolsas amplias | **Confirmado como forma; impacto evaluativo** | `ParsedCliArgs` agrupa comandos heterogéneos; `TaskCoordinatorOptions` contiene opciones de start, cleanup, recovery, manifests, paths y snapshots |
| SOLID-08 | LSP | Estados persistidos amplios y validación en runtime | **Evaluación respaldada** | `TaskRecord` representa varios estados; las operaciones validan transiciones manualmente. Es deuda de contrato, no fallo probado |
| SOLID-09 | DIP | Acceso directo a `process.env` y procesos | **Confirmado** | `gateRun`, `runtimeInstall`, `interceptorShims`, `lease`, `scope`, `toolRunner`, `taskCoordinator` y otros acceden directamente a Node/process |

### 4.3 Fortalezas que deben conservarse

- `check-core-no-vscode` protege la separación del core respecto de VS Code.
- `pathContainment`, `policyDecision`, `stageRunner` y adaptadores de diagnósticos tienen responsabilidades relativamente acotadas.
- El coordinador ya valida task IDs, ramas, contención de paths, snapshots de archivos ignorados y heads.
- El CLI local compilado expone y ejecuta `task status --all --json`; el runtime provisionado antiguo (`.quality-tools/sentinel`) todavía no reconoce `--all` hasta reprovisionar/publicar Sentinel.
- Los contratos `CoreTextDocument`/findings y adaptadores VS Code/LSP son una buena frontera.

### 4.4 Qué no está demostrado por la auditoría SOLID

No se debe afirmar sin pruebas adicionales que:

- existe una fuga real entre dos workspaces en producción;
- existe una carrera concurrente reproducible por el proveedor de reglas;
- todos los casts `as unknown as` producen sustituciones inválidas;
- `ruleRegistry.ts` y `defaultRules.ts` tienen IDs duplicados: la comprobación realizada no lo demostró;
- la extensión de un lenguaje siempre requiere modificar todos los dispatchers, porque existen extensiones adicionales aunque no un registro uniforme;
- una reestructuración de carpetas, por sí sola, resolvería los problemas.

Estas son hipótesis de riesgo que deben convertirse en fixtures y pruebas antes de refactorizar.

---

## 5. Plan priorizado de mejora

### Fase 0 — Recuperar la autoridad operativa

**Objetivo:** que el estado del proyecto vuelva a ser interpretable.

1. Congelar nuevas tareas y operaciones destructivas.
2. Obtener confirmación del dueño de los seis archivos modificados de Sentinel.
3. Publicar una release/commit nuevo o retirar los cambios autorizadamente; no regenerar el lock sobre checkout sucio.
4. Reprovisionar Sentinel, ejecutar `quality:lock --check` y `quality:doctor --lock`.
5. Inventariar `028A-22`, ramas `task/*`, carpetas físicas y worktrees registrados.
6. Ejecutar primero `recover --dry-run` solo si la metadata y el namespace corresponden a una tarea identificable; si no, crear un procedimiento de orphan-recovery con evidencia.
7. Limpiar únicamente recursos con ownership demostrado, registrando antes/después.

**Aceptación:** submódulo limpio y publicado; lock `match`; doctor PASS; no metadata ACTIVE apuntando a rutas rotas; no ramas/worktrees huérfanos atribuibles al proyecto actual.

### Fase 1 — Fijar el contrato de alcance

**Objetivo:** que el gate no analice copias operativas ni dependa de rutas ambiguas.

1. Añadir exclusiones explícitas para `.sentinel/worktrees/**`, `.quality-tools/**`, `.quality-reports/**` y el checkout hermano `gate-hardening/**` cuando esté dentro del alcance visible.
2. Confirmar que las exclusiones se aplican al analizador real, no solo a un JSON no consumido.
3. Añadir `.tsx` a Sentinel y VarSense.
4. Generar un scope manifest y verificar conteo de archivos, rutas incluidas/excluidas y ausencia de worktree paths.
5. Repetir el gate en modo local-light y luego full/CI según cooldown.

**Aceptación:** el reporte enumera su raíz, includes/excludes efectivos, no contiene `.sentinel/worktrees` ni `gate-hardening`, y cubre los 111 `.tsx` donde corresponda.

### Fase 2 — Resolver la política de worktrees

**Objetivo:** eliminar la contradicción entre visibilidad del agente y aislamiento.

1. Decidir entre raíz estrictamente interna o raíz externa autorizada.
2. Si se mantiene raíz externa: declarar una única raíz por proyecto, prohibir cualquier otra, validar `realpath`, identidad de common Git dir, contención, colisiones y cleanup; documentar la excepción en la política global.
3. Si se exige raíz interna: implementar visibilidad del worktree en la sesión/cliente sin hacer que el agente pase rutas absolutas manualmente.
4. Cubrir ambas decisiones con pruebas positivas y negativas: repo principal, otro repo, symlink/junction, path traversal, raíz inexistente, raíz dentro del repo, dos tareas concurrentes.
5. Añadir una prueba de edición parcial de un archivo tracked y de un `ignored-local` provisionado, verificando que no se reescribe el archivo completo.

**Aceptación:** un único contrato documentado, implementado y testeado; cleanup elimina metadata, rama, worktree y temporales de la tarea sin tocar recursos ajenos.

### Fase 3 — Contrato de entorno y archivos ignorados

**Objetivo:** separar versionado de accesibilidad y materialización.

1. Mantener `sentinel.env-manifest.json` como fuente declarativa.
2. Clasificar entradas como `tracked`, `generated`, `ignored-local`, `external` y `secret`.
3. No copiar secretos al worktree; usar entorno/secret store.
4. Entregar `missing-task-input` con ruta, categoría, origen y acción requerida.
5. Mantener baseline hash para detectar modificación/eliminación de ignorados no autorizados.
6. Probar lectura/búsqueda/edición parcial y gate desde el worktree real.

**Aceptación:** la tarea falla de manera explícita cuando falta un input, y nunca continúa contra el checkout principal por fallback silencioso.

### Fase 4 — Refactor incremental SOLID

#### 4.1 Contexto de análisis y catálogo

- Crear `AnalysisContext` explícito con roots, catálogo, índices, filesystem y logger.
- Crear `RuleCatalog` inmutable por invocación/workspace.
- Validar que cada implementación tenga exactamente un ID de catálogo.
- Mantener wrappers de compatibilidad para CLI/LSP/VS Code durante la migración.
- Añadir pruebas de dos workspaces y dos configuraciones de overrides sin contaminación.

#### 4.2 CLI

Separar gradualmente:

```text
src/cli/main.ts
src/cli/parseArgs.ts
src/cli/commands/checkCommand.ts
src/cli/commands/taskCommand.ts
src/cli/commands/runtimeCommand.ts
src/cli/commands/leaseCommand.ts
src/cli/renderers/cliOutput.ts
```

`main.ts` debe quedarse como composition root, señales y exit-code mapper; cada comando recibe opciones mínimas.

#### 4.3 Lifecycle de tareas

Extraer por puertos, sin cambiar primero el contrato público:

```text
TaskAggregate / transitions
TaskRepository
GitPort
LockPort
WorktreeService
EnvManifestPort
TaskLifecycleService
```

Primero escribir pruebas de contrato con fakes; después mover implementaciones Node/Git. Cada extracción debe conservar `taskCoordinator` como fachada compatible hasta que el consumidor esté migrado.

#### 4.4 Runtime, shells y diagnóstico

Separar `RuntimeArtifactStore`, activación/rollback, `ShellAdapter` por plataforma, integración de perfiles y `DiagnosticProbe`/renderer.

#### 4.5 Tipos y concurrencia

- Validar `unknown` en fronteras JSON/CLI.
- Sustituir `as unknown as` en fronteras de datos por parsers discriminados.
- Evitar mutar `process.env` en ejecuciones concurrentes; pasar `env` al proceso hijo.
- Definir owner de cada cache: proceso, workspace, sesión o documento.
- Añadir tests de cancelación, timeout, doble workspace y procesos huérfanos.

### Fase 5 — Gate y release

En cada fase:

```text
compile
→ check:core
→ smoke:lsp
→ test:unit
→ lint
→ quality:lock --check
→ quality:doctor --lock
→ task:check local-light
→ task:check full/CI cuando corresponda
```

No se declara una fase terminada si el gate del consumidor sigue en `SETUP ERROR`, `tool-error` o `FAIL` sin una explicación documentada y una decisión explícita de aceptación.

---

## 6. Matriz de aceptación final

| Área | Debe quedar demostrado |
|---|---|
| Release | Sentinel fuente limpio, commit publicado, manifest/lock/HEAD coincidentes |
| Coordinación | no hay ACTIVE con ruta inexistente; todo recurso tiene owner, rama, worktree y timestamps |
| Huérfanos | status global distingue activos, históricos, ramas y carpetas físicas; cleanup no borra recursos ajenos |
| Alcance | scope manifest reproducible; exclusiones explícitas; no se escanean copias operativas |
| TSX | Sentinel y VarSense cubren `.tsx` según contrato; reporte verifica conteo |
| Worktrees | una política única de raíz, validada contra Git/realpath y cubierta con pruebas negativas |
| Inputs ignorados | manifest, baseline hash, autorización parcial y `missing-task-input` reproducibles |
| Gate | doctor/lock PASS y consumidor con reporte PASS; errores de herramienta no se degradan a warnings |
| SOLID | contextos/puertos extraídos por etapas, sin reescritura masiva ni cambio de comportamiento silencioso |
| Editor/agente | el worktree autorizado es visible para búsqueda/edición; `str_replace` se usa para cambios parciales |
| PowerShell | perfil nuevo carga el shim correcto y no contiene referencias a checkouts desaparecidos |
| Documentación | esta auditoría enlazada desde el plan/roadmap canónico, con fecha y evidencia actualizada |

---

## 7. Comandos de verificación reproducibles

Ejecutar desde `glory-rust-template`, sin cleanup ni operaciones destructivas:

```bash
# Estado y pins
git status --short --branch
git -C tools/sentinel status --short --branch
git submodule status

# Lock/release
node scripts/quality/lock-generator.mjs --check
node scripts/quality/sentinel-doctor.mjs --lock

# Runtime
node .quality-tools/sentinel/out/cli/index.js --version
node .quality-tools/sentinel/out/cli/index.js task status --project-root . --all --json

# Alcance
git ls-files '**/*.tsx' | wc -l
node .quality-tools/sentinel/out/cli/index.js --help

# Validación interna de Sentinel
cd tools/sentinel
npm run compile
npm run check:core
npm run smoke:lsp
npm run test:unit
npm run lint
```

> El comando correcto para consultar la versión es:
>
> ```bash
> node .quality-tools/sentinel/out/cli/index.js --version
> ```

### Resultados obtenidos en esta auditoría

- `lock-generator --check`: **exit 2**, bloqueado por checkout Sentinel modificado.
- `sentinel-doctor --lock`: **exit 2**, misma causa.
- CLI fuente local: `task status --all --json` ejecutado; detecta 1 `legacyOrphan`, 1 rama huérfana y 2 worktrees físicos huérfanos, sin locks expirados. Runtime provisionado: **0.6.4**, pero sigue siendo anterior y no reconoce `--all`.
- Sentinel: `npm run compile`: **PASS en el working tree local modificado**; no demuestra el release limpio `cfac119`.
- Sentinel: `npm run check:core`: **PASS en el working tree local modificado**; no demuestra el release limpio `cfac119`.
- Sentinel: `npm run test:unit`: **522 passing / 1 pending en el working tree local modificado**; `test:task-coordinator` repite el mismo resultado; no demuestra el release limpio `cfac119`.
- `lint`: ejecutado; observa **9 errores y 12 warnings** en el checkout actual, principalmente regex/estilo, bucles constantes y variables no usadas. No se afirma que sean preexistentes sin una comparación contra `cfac119` limpio. Compile, check:core y smoke:lsp pasan.
- El consumidor `task:check` más reciente no es PASS; el reporte `048A-11` tiene stages `fail/error`.

Las acciones de cleanup, retirada de ramas/worktrees y recuperación que aparecen en este documento son **planificadas**, no comandos ejecutados durante esta auditoría; requieren autorización, ownership demostrado y un flujo auditable.

---

## 8. Decisión recomendada

1. **No publicar ni repinear todavía** los seis cambios locales de Sentinel.
2. Resolver ownership de esos cambios y de `028A-22` antes de cualquier cleanup.
3. Corregir el alcance `.tsx` y excluir artefactos operativos antes de interpretar nuevos findings.
4. Decidir formalmente la frontera interna/externa de worktrees; la solución externa puede ser válida para la visibilidad del agente, pero no mientras contradiga la política global.
5. Ejecutar un gate limpio del consumidor.
6. Solo después iniciar la Fase 4 de refactor SOLID, empezando por `AnalysisContext`/`RuleCatalog` y puertos del lifecycle, con cambios pequeños y reversibles.

**Conclusión:** la primera auditoría detectó bloqueos operativos reales, pero incluyó dos afirmaciones no confirmadas (SNT-16c y el hook antiguo) y mezcló recursos de otros checkouts. La auditoría SOLID identifica deuda arquitectónica real y útil, aunque debe leerse como priorización de diseño, no como una lista de bugs ya probados. El trabajo inmediato es recuperar la autoridad del checkout, hacer reproducible el alcance y cerrar el contrato de worktrees; la refactorización profunda viene después.
