# Contexto de reinicio — alcance multi-carpeta

**Fecha:** 2026-08-07
**Proyecto actual:** `C:\Users\Owner\OneDrive\Documentos\glory-rust-template`
**Rama:** `glory-rs-rest`
**Objetivo de la conversación:** ampliar el alcance del workspace a `C:\Users\Owner\OneDrive\Documentos\area-trabajo` para poder trabajar en proyectos/carpetas hermanas sin sobrescribir archivos completos.

## Problema

La herramienta `str_replace` funciona dentro del proyecto actual, pero al intentar editar un archivo fuera de `glory-rust-template` falló diciendo que el archivo no existía, aunque el archivo sí estaba creado.

No se quiere usar `write_file` para sobrescribir archivos completos: es inseguro para cambios parciales porque puede borrar contenido no incluido accidentalmente.

## Prueba realizada

Se usó como carpeta externa `C:\Users\Owner\OneDrive\Documentos\test`:

- `write_file` fuera del proyecto: funcionó.
- `read_files` fuera del proyecto: funcionó.
- `str_replace` fuera del proyecto: falló con “The file does not exist”.
- `write_file` pudo modificar el archivo externo, pero esa opción se considera inadecuada para edición parcial.
- El archivo de prueba se eliminó después. No quedó ningún archivo de prueba nuevo.
- Los archivos preexistentes de `test` (`pagina.html`, `prueba.md`, `test-archivo.txt`) no se modificaron.

## Interpretación acordada

El usuario no pide mover Sentinel ni convertir la carpeta padre en un repositorio Git. Solo quiere que el agente tenga alcance de edición sobre varias carpetas hermanas, manteniendo cada proyecto separado.

La hipótesis pendiente de verificar es:

> Si se abre/configura `C:\Users\Owner\OneDrive\Documentos\area-trabajo` como raíz de alcance del workspace, `str_replace` debería poder editar archivos de los proyectos contenidos allí sin usar sobrescrituras completas.

Esto debe probarse con un archivo temporal pequeño dentro de una carpeta hermana, usando `str_replace`, y limpiarlo después.

## Interfaz y plan de Sentinel

El trabajo no es solo ampliar el alcance de edición: también existe un plan de mejora de la interfaz/operación de Sentinel y del quality gate. El agente nuevo debe revisar como referencias:

- `gate-hardening/roadmap-sentinel.md`
- `gate-hardening/Agente/planes/plan-sentinel-orquestacion-tareas-worktrees-2026-08-06.md`
- `gate-hardening/Agente/planes/plan-gobernanza-workspace-2026-08-02.md`
- `gate-hardening/Agente/documentacion/herramientas/sentinel-varsense-editor-agnostico-2026-05-08.md`
- `gate-hardening/Agente/documentacion/herramientas/inventario-scripts-adapters-sentinel-2026-08-06.md`

La interfaz/flujo debe dejar claro qué raíz de proyecto se analiza, qué worktree pertenece a cada tarea, qué herramientas están disponibles y qué archivos son generados, ignorados o externos.

## Problema de archivos ignorados y agentes

Hay que ajustar la forma de trabajo para que una carpeta ignorada no deje al agente sin capacidad de leer o editar lo que necesita. La ignorancia de Git es correcta para no versionar artefactos (`.sentinel/`, `.quality-tools/`, `dist/`, clientes generados, caches), pero no debe convertirse automáticamente en una limitación de las herramientas del agente.

Hipótesis importante para verificar: cuando Sentinel crea una rama/worktree para una tarea, el agente trabaja dentro de ese worktree y las herramientas de búsqueda/edición pueden aplicar las reglas de exclusión del workspace o de Git. Por eso los agentes podrían heredar la misma limitación: no encontrar o no poder modificar archivos ignorados aunque sean necesarios para ejecutar o diagnosticar la tarea.

El plan del agente nuevo debe incluir una revisión de este contrato:

1. Separar **no versionar** un archivo de **no poder leerlo/editarlo**.
2. Permitir explícitamente archivos ignorados necesarios para una tarea, especialmente configuración, fixtures, artefactos generados y archivos de tooling.
3. Mantener el aislamiento: el agente solo puede modificar archivos dentro del worktree/proyecto autorizado, nunca usar la ampliación de alcance para saltarse Sentinel.
4. Hacer que el worktree de la tarea sea **visible por defecto** dentro del workspace (ver `Corrección de la hipótesis`), de modo que `str_replace`, búsquedas y validaciones lo encuentren sin depender del checkout inicial ni del estado de `.gitignore`; la resolución interna sigue siendo contra el worktree autorizado.
5. Añadir una prueba reproducible en un worktree de tarea: archivo ignorado dentro del worktree, lectura y edición parcial segura, y confirmación de que no se altera otro proyecto.
6. Documentar qué herramientas respetan `.gitignore`/carpetas ocultas y cuáles necesitan un modo de tarea que permita archivos autorizados.

La solución buscada es ajustar el alcance, la ubicación y el contrato de las herramientas, no sobrescribir archivos completos ni desactivar globalmente las protecciones.

## Hallazgo de la investigación: límite real del modelo de worktrees

La preocupación queda confirmada por el código y la documentación de Sentinel `0.6.4` (`tools/sentinel` en el commit `83eafbfef7a469c309059c9bcc0bb1a648e391b7`): el modelo actual aísla Git correctamente, pero **no provisiona automáticamente el entorno local completo para el agente**.

### Qué hace realmente Sentinel

- `task start` ejecuta `git worktree add -b <rama> <ruta> <base>` desde el checkout de origen (`tools/sentinel/src/core/taskCoordinator.ts`). La ruta por defecto queda dentro de `<repo>/.sentinel/worktrees/`; las rutas externas se rechazan.
- El worktree nace de un commit/base de Git. Por tanto, contiene los archivos versionados de esa base, pero no hay código en el coordinador que copie al nuevo worktree los archivos ignorados o untracked del checkout de origen.
- La implementación solo copia dependencias en `runtimeInstall.ts` al instalar el runtime global de Sentinel; eso no es una provisión de worktrees de tareas y no copia `.env`, configuraciones locales del consumidor, caches, `dist`, `frontend/node_modules` ni otros artefactos del proyecto.
- `task gate` no trabaja contra la carpeta abierta originalmente por el agente: el CLI recibe `--project-root <worktree>`, `verifyTaskWorktree` compara el path real con la metadata registrada y exige la rama/worktree autorizados. Después el gate usa esa misma ruta como `workspace`.
- El coordinador permite `--path`, pero únicamente dentro de `.sentinel/worktrees`; no existe un `--mount`, `--copy-ignored`, `--env-file`, `--dependencies-from` ni sincronización equivalente.
- `task start` también exige que el checkout de origen esté limpio. La metadata de `.sentinel/` se excluye específicamente de esa comprobación porque es estado propio de Sentinel, no porque los demás archivos ignorados se compartan con la tarea.
- `task gate` y el analizador resuelven su configuración y sus índices desde el workspace que reciben. Hacer visible el worktree físico no basta: el agente debe abrir/usar ese árbol como contexto de tarea, y la herramienta debe validar internamente que coincide con la tarea autorizada. Si el editor solo conoce `area-trabajo` y no reconoce el worktree como carpeta de trabajo, la limitación original puede persistir aunque la carpeta exista.

### Qué no debe confundirse

Hay cuatro capas distintas:

1. **Git/versionado:** `.gitignore` decide qué no entra en el índice; no es un mecanismo de permisos de lectura/escritura.
2. **Materialización del worktree:** `git worktree add` no materializa los archivos ignorados/untracked locales del checkout de origen.
3. **Herramientas del agente/editor:** búsquedas y edición pueden aplicar exclusiones del workspace o de Git, aunque el archivo sí exista físicamente. La ampliación de `area-trabajo` resuelve el alcance de Freebuff, no modifica por sí sola estas exclusiones.
4. **Aislamiento de Sentinel:** aunque el workspace de Freebuff abarque varias carpetas hermanas, el agente de una tarea solo debe leer/editar y validar el worktree registrado. No se puede usar la raíz padre para alcanzar el checkout principal u otro proyecto.

La limitación, por tanto, no es exactamente que “Git ignore impide editar”. Es que el worktree de la tarea puede no contener el archivo local que se necesita, y además las herramientas pueden ocultar archivos que sí existan por sus propias reglas de exclusión. La solución no es desactivar globalmente `.gitignore`, copiar todo el checkout ni usar `write_file` para reemplazar archivos completos.

### Corrección de la hipótesis: el worktree debe ser físicamente visible

La hipótesis anterior de pasar una ruta explícita a cada herramienta **no resuelve el problema operativo que preocupa al usuario**: las herramientas del agente no siempre aceptan o propagan una raíz arbitraria. Si el worktree solo vive en `glory-rust-template/.sentinel/worktrees/`, puede quedar fuera del alcance visible del workspace; entonces búsquedas y edición parcial pueden fallar y el agente puede intentar reescribir el archivo completo.

La solución que debe investigarse e implementarse es distinta:

- Crear físicamente el worktree temporal bajo la raíz visible del workspace, por ejemplo `area-trabajo/task-worktrees/<project-identity>/<task-id>/`, no mediante junction/symlink. Se prefiere un nombre visible que no empiece por `.` si las búsquedas excluyen carpetas ocultas por defecto.
- Mantener `glory-rust-template` como raíz Git y autoridad de Sentinel; la carpeta visible es solo la ubicación temporal del checkout de tarea, no un repositorio Git padre que mezcle proyectos.
- Distinguir una **raíz externa autorizada** (`area-trabajo`, validada con `realpath`, identidad del proyecto y contención) de una ruta externa arbitraria, que debe seguir bloqueada. Sentinel `0.6.4` rechaza actualmente ambas porque solo permite `<repo>/.sentinel/worktrees`; eso tendría que cambiar de forma explícita y cubierta por pruebas.
- Si el worktree está fuera del árbol Git de `glory-rust-template`, el `.gitignore` de ese proyecto no lo controla y normalmente no hace falta añadirlo a dicho `.gitignore`. Si la ubicación visible cae dentro de algún repositorio, usar una exclusión local (`.git/info/exclude`) o una regla específica, sin hacerla pasar por una exclusión de las herramientas.
- No confiar en `.git/info/exclude` como mecanismo de visibilidad: solo controla qué Git muestra como no versionado. La prueba debe verificar por separado que Freebuff/búsqueda ve el worktree y que Git no lo integra ni lo reporta como cambio del checkout principal.
- El worktree debe conservar su propio `.gitignore` para artefactos internos del proyecto, pero las herramientas deben tener un modo de tarea que permita los archivos ignorados declarados/provisionados, sin convertir automáticamente “no versionado” en “inaccesible”. También hay que comprobar si Freebuff excluye carpetas ocultas o ignoradas por reglas propias.
- Los enlaces simbólicos/junctions no son equivalentes: pueden ser rechazados o no seguidos por búsquedas y agentes, y además complican la comprobación de contención física.
- Al terminar, Sentinel debe ejecutar `git worktree remove`, eliminar la carpeta visible temporal, retirar metadata/ramas y comprobar que no quedan recursos ni cambios en ningún proyecto.

**Límite importante:** Sentinel `0.6.4` no permite esta arquitectura todavía: `resolveWorktreePath()` exige que la ruta esté dentro de `<repo>/.sentinel/worktrees/`, y sus pruebas rechazan paths externos. Por tanto, no basta con cambiar `.gitignore`; habrá que cambiar el contrato de ubicación autorizada, la limpieza y las pruebas de contención sin perder el aislamiento.

### Contrato corregido que debe implementar/documentar el flujo

> **Estado actualizado (2026-08-07):** los puntos 5, 6, 8 y 9 están implementados en la extensión publicada `cfac119`; el padre ya está repineado a ese commit. El manifiesto (`sentinel.env-manifest.json` / `--env-manifest`) admite `editable: true` únicamente para `ignored-local`; por defecto las entradas provisionadas quedan protegidas por hash. Sentinel captura una línea base hash de los archivos ignorados preexistentes y bloquea modificaciones/eliminaciones no autorizadas y nuevos paths ignorados en `gate`, `integrate` y `cleanup`. También rechaza manifiestos fuera del `projectRoot`, symlinks de escape, fuentes externas y fuentes de directorio no soportadas, y migra registros `TaskRecord` v2 a schema v3 al leerlos. La autorización visible sigue siendo declarativa por manifiesto; la UI de autorización por tarea (punto 7) continúa pendiente. `0.6.4` base no tiene estas capacidades.

1. **Contexto explícito:** Sentinel debe publicar para cada tarea `projectRoot` (raíz Git del consumidor), `worktreeRoot` (ruta exacta del worktree), `taskId`, agente, rama, base/HEAD y estado de autorización. La UI debe mostrar esos valores antes de habilitar edición o gate.
2. **Configuración disponible en la raíz:** `sentinel.config.json`, `quality-tools.json`, `sentinel.lock.json` y cualquier otra política que necesite `task gate` deben estar presentes dentro del worktree o ser entradas declaradas y provisionadas antes del gate. Si una de esas piezas es local/ignorada y no se materializa, el gate no debe apuntar silenciosamente al checkout principal: debe fallar con una dependencia ausente explícita.
3. **Raíz visible por defecto:** las herramientas del agente deben descubrir el worktree desde el workspace visible sin que cada llamada reciba una ruta absoluta. La sesión debe registrar internamente qué worktree está activo, pero la visibilidad del árbol no puede depender de que el agente conozca esa ruta.
4. **Raíz única por operación interna:** aunque la herramienta descubra el worktree por el workspace, `read`, búsqueda, `str_replace`, ejecución de comandos, análisis, VarSense y `task gate` deben resolverse internamente contra el worktree autorizado. La raíz explícita pertenece al contrato interno entre Sentinel y las herramientas; el agente no debe tener que proporcionar manualmente una ruta absoluta. Nunca deben inferirla desde el checkout inicial, el cwd accidental o la raíz padre `area-trabajo`.
5. **Manifiesto de entorno:** una tarea debe declarar qué necesita además del contenido versionado: configuración local, fixtures, archivos generados, dependencias, variables de entorno y servicios externos. Cada elemento debe clasificarse como `tracked`, `generated`, `ignored-local`, `external` o `secret`.
6. **Provisionamiento reproducible:** los archivos `ignored-local` necesarios no deben copiarse silenciosamente desde el checkout principal. Deben materializarse mediante una fuente declarada y segura (generación, fixture versionado, plantilla saneada o copia explícita aprobada), dentro del worktree, antes del gate. Los secretos deben entrar por un mecanismo de entorno/secret store, nunca por copia al árbol.
7. **Visibilidad controlada:** un archivo ignorado que ya exista dentro del worktree puede leerse y editarse parcialmente si la tarea lo autoriza; su exclusión de Git no debe convertirse automáticamente en exclusión de las herramientas. La autorización debe seguir limitada al worktree y a los patrones permitidos de la tarea.
8. **Integración sin contaminación:** los artefactos generados y archivos ignorados no se integran por accidente. Antes de `integrate`, Sentinel debe distinguir cambios versionables de outputs locales y rechazar paths fuera del worktree o symlink/junction escapes.
9. **Fallo claro:** si falta una dependencia local, el agente debe recibir `missing-task-input` con la ruta, categoría, origen esperado y acción requerida. No debe continuar fingiendo que el worktree es equivalente al checkout original.
10. **Prueba obligatoria:** crear un worktree de tarea; provisionar un archivo ignorado declarado —incluida, cuando aplique, la configuración local que deba consumir el gate—; leerlo y modificar una sola línea con edición parcial; ejecutar búsqueda y gate desde esa raíz; comprobar que el archivo no aparece en otro proyecto ni se integra por accidente; limpiar todo al finalizar.

La lista anterior define diez requisitos propuestos; no son IDs existentes de Sentinel.

### Matriz mínima de pruebas para cerrar la limitación

1. Crear un worktree físico temporal bajo la raíz visible `area-trabajo`, sin symlink/junction.
2. Confirmar que la búsqueda normal del workspace encuentra un archivo versionado del worktree sin pasarle una ruta directa.
3. Editar una sola línea con `str_replace`; comprobar que el diff contiene únicamente ese cambio y que una guardia de la sesión rechaza `write_file` sobre un archivo existente, reservándolo para archivos nuevos o una reescritura expresamente autorizada.
4. Crear/provisionar dentro del worktree un archivo ignorado declarado y repetir lectura, búsqueda y edición parcial.
5. Confirmar que `.gitignore` del proyecto no impide la visibilidad de la carpeta de tarea, pero que Git tampoco intenta añadirla al checkout principal; la exclusión local debe comprobarse con `git status` y `git check-ignore` por separado.
6. Ejecutar `task gate` contra el worktree autorizado y confirmar que no acepta el checkout principal, otro proyecto, una ruta externa ni un symlink/junction.
7. Simular cleanup/recovery y verificar que se eliminan worktree visible, metadata, rama y exclusiones temporales sin tocar archivos preexistentes.
8. Repetir con dos proyectos y el mismo task-id para confirmar aislamiento.
9. Probar nombres de carpeta con espacios y caracteres no ASCII (Windows), y dos tareas concurrentes del mismo proyecto sin colisión de rutas ni de metadata.
10. Simular una interrupción (proceso muerto sin cleanup) y comprobar que `task recover` valida heads/PID y libera únicamente los recursos de esa tarea.
11. Nota OneDrive: la raíz vive bajo `C:\Users\Owner\OneDrive\...` pero la sincronización no está activada, por lo que no interfiere con el worktree temporal. No hace falta configurar exclusiones de sincronización; basta con que se respete la ruta física real.

### Estado de verificación

**Confirmado por inspección:** Sentinel `0.6.4` aísla ramas/worktrees, mantiene los worktrees dentro del repositorio, verifica el worktree registrado para `task gate` y no copia automáticamente archivos ignorados/untracked del checkout de origen. También se confirmó que `.sentinel/`, `.quality-tools/` y `frontend/node_modules/` están ignorados en este consumidor. No se encontró soporte actual para un worktree físicamente visible bajo `area-trabajo` ni para hacer que las herramientas ignoren selectivamente `.gitignore`.

**Prueba operativa EJECUTADA (2026-08-07, workspace raíz `area-trabajo`):** ✅ resultados:
1. `str_replace` sobre un archivo en `area-trabajo` fuera de `glory-rust-template`: **funcionó** (edición de una sola línea; hipótesis confirmada — la ampliación del workspace sí habilita la edición multi-carpeta).
2. `sentinel task start --worktrees-root ../task-worktrees` creó el worktree físicamente en `area-trabajo/task-worktrees/glory-rust-template-<identity>-PROBE-01`.
3. Dentro del worktree, con ruta relativa al workspace (sin ruta absoluta): `read_files` **funcionó**, `str_replace` de una línea **funcionó**, y una búsqueda normal **descubrió el worktree** (encontró la marca `PROBE-01`).
4. `write_file` sobre un archivo existente del worktree: **SOBRESCRIBIÓ sin bloquearse** (el bloqueo propuesto no existe en esta build; sigue aplicando la regla operativa: `str_replace` para cambios parciales).
5. Archivo gitignored (`.env`) dentro del worktree: `str_replace` **falló** con “The file does not exist” — limitación confirmada; requiere provisión declarada (parte B del contrato).
6. Checkout principal: **limpio** durante toda la prueba; `git worktree list` muestra el worktree visible pero Git no lo integra ni lo reporta como cambio.
7. Cleanup: la ruta de abandono diseñada exige tarea expirada (TTL 6h) — se hizo teardown manual seguro del probe propio (worktree + rama + metadata + carpeta vacía). Nota de diseño: `cleanup --force`/`release` no cubren una tarea ACTIVE sin commits; considerar un flujo de abandono explícito.

**Decisión provisional:** no apuntar las herramientas al checkout principal ni usar reescrituras completas como solución. La dirección preferida es un worktree físico temporal visible dentro de `area-trabajo`, con una raíz de worktrees autorizada y validada por Sentinel, exclusión local de Git separada de la visibilidad de las herramientas, provisión declarada de archivos ignorados y cleanup verificable. Antes de cambiar Sentinel hay que probar que la ubicación visible realmente permite que las herramientas encuentren y editen parcialmente un archivo sin ruta directa.

### Implementación histórica en Sentinel (extensión local `8502710`, ya publicada; referencia histórica)

Se implementó la capacidad en el checkout local de `tools/sentinel` (marca `[VISIBLE-WORKTREE]`) y esa primera extensión ya fue publicada; la ampliación descrita en **Estado actual de esta continuación B** todavía es local:

- **Nuevo flag:** `sentinel task start ... --worktrees-root <dir>` declara una raíz externa autorizada para worktrees temporalmente visibles (p. ej. `area-trabajo/task-worktrees`).
- **Validaciones:** la raíz debe existir y resolverse a un path físico real (canonical); NO puede ser el repositorio ni una subcarpeta de él; el worktree debe quedar dentro de esa raíz; sin `--worktrees-root` sigue el comportamiento interno anterior (`<repo>/.sentinel/worktrees`) y se siguen rechazando rutas arbitrarias.
- **Metadata:** `TaskRecord` (schema v3) conserva `worktreesRoot`, snapshots de `ignored-local` y la línea base hash para que `verify`/`integrate`/`cleanup`/`recover` validen contención y no oculten contaminación local; los registros v2 se migran de forma segura al leerse.
- **Gate en worktree externo:** `repositoryRoot()` acepta worktrees vinculados cuyo top level es hermano de la raíz Git (la identidad sigue anclada al common dir); `verifyTaskWorktree`/`task gate` funcionan desde la raíz visible.
- **Pruebas históricas de la primera extensión:** `taskCoordinator.test.ts` añadió caso positivo (worktree creado en la raíz externa + repo limpio) y caso negativo (raíz dentro del repo rechazada). La validación actual de la ampliación está en el bloque de estado al final: **517 passing / 1 pending**, `check:core` y `smoke:lsp` OK.
- **Docs:** `README.md` y `CHANGELOG.md` del submodulo actualizados (marcados como `[Unreleased]` — extensión local, no se declaró release `0.6.5`).
- **Compatibilidad de metadata:** `TaskRecord` usa schema v3. Los registros v2 existentes se leen y se migran automáticamente añadiendo snapshots vacíos (`ignoredInputs: []`, `ignoredBaseline: null`), conservando los recursos y evitando romper tareas activas antiguas. Registros con schemas desconocidos siguen siendo `invalidMetadata` y no se limpian silenciosamente.

**Cierre histórico del consumidor:** el cambio quedó committeado en el submodulo (`8502710a`) y el gate del consumidor fue repineado y validado:
1. `quality-tools.json`: `tools.sentinel.commit` → `8502710a` (versión manifest sigue `0.6.4`; el CLI provisionado reporta `0.6.4`).
2. `quality:setup` re-provisionó el CLI en `.quality-tools/sentinel` desde el nuevo commit (staging aislado + suite) — el binario del gate ya expone `--worktrees-root`.
3. `sentinel.lock.json` regenerado y verificado: doctor `--lock` → `pass`/`match`; suite `quality:test` → **230 pass / 0 fail** (1 skip esperado).
4. Commits del padre: repin (`59d6bb24`) + cierre de gate y docs (siguiente commit).

**Contrato de entorno inicial (extensión `9126811`, ya publicada):** `sentinel.env-manifest.json` (o `--env-manifest <path>`) declara entradas `tracked`, `generated`, `ignored-local`, `external` y `secret`; `task start` provisiona las `ignored-local` desde su fuente declarada (contenida en el projectRoot) dentro del worktree antes de ACTIVE. Fuente faltante → `missing-task-input` con ruta, categoría, origen y acción (y rollback del worktree sin huérfanos). `secret` no admite source; `external`/`generated` no se copian; el manifiesto no puede pisar contenido tracked; los provisionados son ignorados por Git (worktree limpio para gate/integrate). `worktree remove` usa `--force` en rollback/cleanup por los provisionados. Validación: 511 pass / 1 pending en el submódulo, e2e con CLI (provisión + missing-task-input + sin huérfanos), `check:core` OK, `smoke:lsp` OK, suite del gate 230/230.

**Publicación previa:** `tools/sentinel` `9126811` fue empujado a `origin/main`; la ampliación actual de autorización y protección de baseline quedó publicada en `cfac119` y el padre la referencia mediante el repin `b1737fba`; el lock se regeneró y verificó.


## Sentinel

Sentinel debe seguir operando con `glory-rust-template` como raíz Git del proyecto actual:

- Configuración: `glory-rust-template/sentinel.config.json`.
- Rama primaria: `glory-rs-rest`.
- `sentinel task` funciona con `--project-root .` dentro de este proyecto.
- No se debe mover Sentinel a `Documentos` ni mezclar las raíces Git de proyectos diferentes.
- La prueba de `sentinel task` con `--project-root ..` falló porque `Documentos` no es un repositorio Git y no tiene configuración Sentinel; esto no invalida la hipótesis sobre el alcance de las herramientas de edición.

## Estado de Git conocido al terminar

El checkout actual ya tenía cambios ajenos a esta conversación; no deben sobrescribirse, descartarse ni commitearse:

- Modificados: `.gitignore`, `glory-rs` (gitlink), `package.json`, `roadmap.md`.
- Untracked: `Agente/planes/plan-deploy-produccion-intuitividad-2026-08-07.md`, `gate-hardening/`.
- La tarea de prueba Sentinel `SCOPE-PROBE-CURRENT` fue liberada y no quedó activa.
- No se hizo commit, push ni deploy.

> **Actualización histórica (mismo día):** los bloques de calidad y Sentinel cerraron con commits locales propios de aquella fase (`8502710` en el submodulo; `59d6bb24` + `b105188b` en el padre). El estado de publicación de esa fase se detalla más abajo; no usar esta frase como estado actual del checkout.

## Siguiente paso recomendado

1. Abrir/reiniciar el workspace con alcance en `C:\Users\Owner\OneDrive\Documentos\area-trabajo`.
2. Crear un archivo temporal nuevo dentro de `area-trabajo` mediante una herramienta segura (solo para validar alcance de edición).
3. Leerlo y modificar solo una línea con `str_replace`; verificar el contenido y eliminar únicamente el archivo temporal.
4. Crear un worktree físico temporal bajo `area-trabajo/task-worktrees/...` (sin symlink/junction) y repetir el paso 3 dentro de él.
5. Comprobar que una búsqueda normal del workspace descubre ese worktree sin pasarle una ruta directa, y que `write_file` sobre archivos existentes está bloqueado.
6. Probar dentro del worktree un archivo ignorado declarado: leerlo y editarlo parcialmente sin `write_file` completo.
7. Confirmar que el checkout principal y otros proyectos quedan intactos y que Git no intenta integrar el worktree; después, cleanup completo (worktree, rama, metadata).
8. Si algo falla, documentar exactamente qué capa bloquea (sandbox, alcance del workspace, `.gitignore`, carpetas ocultas, búsqueda o worktree) y ajustar el contrato de herramientas; no resolverlo sobrescribiendo archivos completos.

## Corrección de una conclusión anterior

La afirmación de que ampliar el alcance obligaría a mover Sentinel es incorrecta. El alcance de edición del workspace y la raíz Git usada por Sentinel son conceptos separados. El objetivo es ampliar solo el primero.

---

# Quality gate — estado y problemas pendientes por resolver (2026-08-07)

> Contexto de retomar el bloque. Rama `glory-rs-rest`. Objetivo: cerrar el repin de Glory Sentinel 0.6.4 en el checkout activo, ajustar el plan `Agente/planes/plan-deploy-produccion-intuitividad-2026-08-07.md` (gitignore) y dejar el gate `task:check` como prerequisito de calidad. **Nunca escribir al BDP** (regla `bdp_sync_mode read_only`).

## Qué se portó y validó (ya hecho)

- **Repin Sentinel v0.6.4** portado del worktree del otro agente al checkout activo: `scripts/quality/`, `quality-tools.json`, `quality-adapter.json` NUEVO (blob canónico `a1d5e8d60ef...`) y `sentinel.lock.json`.
- Submódulo `tools/sentinel` fijado en commit `83eafbf...` (tag v0.6.4) → commit `4cce909a`.
- `VarSense` v2.2.0 (commit `e8360927...`) en `tools/varsense`.
- `.gitignore` + `package.json` (scripts `task:check`, `quality:*`, `task:*`) + `roadmap.md` (fila 1e) → commit `a311145e`.
- Setup pasa (`sourcePath` verificado), doctor pasa (`policy ok`), lock check pasa (`pass: match`), `stage-process.mjs` pasa con árbol limpio.

## Problemas pendientes por investigar/resolver (la closure del bloque)

### 1. Test `observe-integration` NO determinista (rompe con árbol sucio) — ✅ CORREGIDO
- **Archivo:** `scripts/quality/tests/observe-integration.test.mjs:43`.
- **Síntoma**: ejecuta `stage-process.mjs --stage custom` esperando exit `[0,1]`; falla con `exit inesperado 2; etapa no implementada por el adapter: custom`.
- **Causa raíz**: `scripts/quality/scope.mjs` usa `git diff HEAD` + **untracked** (`git ls-files --others`) para decidir el scope. La etapa `custom` solo se implementa con scope full o con el perfil `frontend`. Cualquier archivo untracked (p.ej. `Agente/contexto-reinicio-alcance-2026-08-07.md`) hace `files.length > 0` → `automaticFull=false` → sin perfil frontend → `custom` no implementada.
- **Fix aplicado**: el test ahora ejecuta `stage-process.mjs --stage custom ... --profile frontend`, lo que lo hace hermético frente al estado del árbol (el perfil frontend declara la etapa `custom` en `quality-adapter.json`). Se descartó el enfoque de ignorar untracked en el scope porque eso alteraría el comportamiento de producción.
- **Validación**: `observe-integration` 4 tests PASS (1 skip esperado por CLI); suite completa 230/230 PASS.

### 2. Fixtures del bench anclados al workspace del otro agente (no a esta rama) — ✅ CORREGIDO
- **Archivo**: `scripts/quality/bench-fixtures.mjs` y `tests/bench-fixtures.test.mjs:9`.
- **Síntoma**: `validateFixtureFiles(FIXTURES.small)` devuelve `['frontend/src/features/runtime/workspace/public-resource-locator.ts', 'frontend/src/styles/layout.css']`; el test espera `[]`.
- **Causa raíz**: el fixture usa rutas de un frontend tipo escritorio del entorno del otro agente. En `glory-rs-rest` (restaurante) NO existen: el repo real usa `frontend/src/componentes/`, `frontend/src/api/`, `frontend/src/estilos/`, `frontend/src/index.css`.
- **Fix aplicado**: `FIXTURES` reescritos con archivos reales verificados de la rama:
  - `small`: `frontend/src/api/bdp.ts` + `frontend/src/index.css`.
  - `medium`: 12 archivos reales (`api/bdp.ts`, `api/axios-instance.ts`, `componentes/FormularioReserva.tsx`, `componentes/ListaClientes.tsx`, `components/bdp-menu-explorer.tsx`, `components/bdp-required-setting.tsx`, `hooks/useBdpStockFilters.ts`, `stores/authStore.ts`, `lib/utils.ts`, `estilos/PlanoSala.css`, `index.css`, `vite.config.ts`) con borrado simulado (`estilos/PlanoOcupacion.css`) y rename simulado (`componentes/CalendarioReservas.tsx` → `ReservasCalendario.tsx`).
- **Validación**: `bench-fixtures` 4 tests PASS; suite completa 230/230 PASS.

### 3. Pendientes del gate→ estado al cierre del bloque
- ✅ **Repin histórico del consumidor cerrado** (extensión de Sentinel `8502710`): `quality-tools.json` repineado, CLI re-provisionado en `.quality-tools/sentinel`, `sentinel.lock.json` regenerado; doctor `--lock` → `pass`/`match`; commits `59d6bb24` + `b105188b` en `glory-rs-rest`.
- ✅ **`npm run quality:test` completo: 230 PASS / 0 FAIL / 1 skip esperado** (ejecutado el 2026-08-07 y revalidado tras el repin).
- ✅ Verificación del doctor: `doctor.mjs` ya no existe (migrado); el equivalente actual es `node scripts/quality/sentinel-doctor.mjs --lock` → `pass: match`, y el doctor de política → ok.
- ✅ **Gate final `task:check` ejecutado** (2026-08-07, task `048A-11`, alcance full automático, 631s): el gate completo corrió todas las etapas (sentinel, varsense, rust, frontend, docs, custom) y **falló por problemas preexistentes de la rama, ninguno introducido por este bloque**: scanner Sentinel reporta errores de fixtures de otros worktrees (`/worktrees/...028A-22`, `gate-hardening/`) y del código `src/` (broadcast-mutex, incidente 096A); TS de `ListaReservas.tsx`; clippy de `glory-backend`; 6 planes sin checklist; varsense `Invalid string length`; cargo test supera timeout (4m15s). Reporte completo en `.quality-reports/branches/glory-rs-rest--f100af0a041e6e8a/048A-11/`. El árbol quedó limpio y sin locks.
- ✅ Plan de deploy actualizado: `Agente/planes/plan-deploy-produccion-intuitividad-2026-08-07.md` Fase 2 documenta que el gate está configurado y usa `task:check` (sigue en gitignore).
- ✅ **Repin global del binario Sentinel a 0.6.4 hecho:** el `sentinel update` del binario 0.5.0 tiene un bug (descarga el mismo artefacto `945d41e6…` para cualquier versión y falla el rename del shim con EPERM); se reparó manualmente copiando el build provisionado del gate a `versions/0.6.4/` (0.6.4 + extensión). `sentinel --version` → **0.6.4** con `--worktrees-root` disponible. Queda el bug del updater reportado para upstream.
- ✅ **Push histórico hecho:** `tools/sentinel` → `origin/main` (`8502710`) primero; luego `glory-rs-rest` → `origin` (`ec57799e`). Este registro corresponde al cierre anterior, no a los cambios locales actuales.

### 4. Limitación de edición confirmada en vivo: `str_replace` no edita archivos gitignored

Durante la corrección de los bugs anteriores se reprodujo la limitación exacta que motivó esta conversación:

- `scripts/quality/**` está ignorado por Git (`.gitignore`: `/scripts/*` con excepciones puntuales).
- `read_files` y las búsquedas no devuelven esos archivos (BLOCKED / sin resultados).
- `str_replace` falla con "The file does not exist" aunque el archivo exista en disco.
- `write_file` SÍ puede crearlos/sobrescribirlos (riesgo de reescritura completa) — por eso se usó edición vía terminal (python heredoc con reemplazo exacto y assert de unicidad) para cambios parciales seguros.

**Regla operativa adoptada:** cuando un archivo necesario esté gitignored y `str_replace` no pueda tocarlo, usar edición quirúrgica vía terminal con verificación previa (assert de que el patrón aparece exactamente una vez), nunca `write_file` completo para cambios parciales. Esta regla se documenta también en la skill de conducta global.

### Estado actual de esta continuación B

- ✅ Implementación local de autorización y protección: `editable: true` solo para `ignored-local`; baseline hash de ignorados preexistentes; bloqueo de modificaciones/eliminaciones no autorizadas y nuevos paths ignorados; validación física de symlinks; manifiesto contenido dentro del `projectRoot`; migración v2→v3.
- ✅ Pruebas actuales del submódulo: `npm run test:unit` → **517 passing / 1 pending**, `check:core` OK y `smoke:lsp` OK.
- ✅ La ampliación está validada y publicada en `tools/sentinel@cfac119`; el padre quedó repineado, reprovisionado y con `sentinel.lock.json` actualizado/verificado. Falta publicar el commit documental/lock actual del padre.
- ⏳ Sigue pendiente la UI/autorización interactiva por tarea (punto 7); hoy la autorización es declarativa en el manifiesto.

## Límites del bloque (no olvidar)
- Arbol limpio antes del gate: `git status --porcelain` vacío. Si hay untracked que ensucia el scope, ignorarlo o moverlo en `.gitignore`.
- `glory-rs` debe quedar fijado en `15c5ad6b` (detach/local) hasta no decidir si subir la sección.
- No hacer deploy: si procede, solo vía `coolify-manager-rs` (`deploy --name glory-rest --update`). Nunca SSH directo.
- No reiniciar VS Code (regla 20).
