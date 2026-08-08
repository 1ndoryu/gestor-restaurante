# Plan — Simplificación y confiabilidad de Sentinel

**Tarea de planificación:** `048A-22`
**Fecha:** 2026-08-08
**Estado:** plan listo; implementación pendiente por fases

## Objetivo

Convertir Sentinel en una herramienta que un agente pueda adoptar y operar sin reconstruir manualmente su
estado interno. Un proyecto debe pasar de checkout a “listo para trabajar” mediante un flujo corto,
determinista, rápido y reversible.

El resultado buscado es:

- una identidad inequívoca del runtime y sus capacidades;
- una sola transacción para instalar, repinear, bloquear y reparar;
- un único contrato de gate, desde el wrapper del proyecto hasta `task integrate`;
- worktrees que nazcan utilizables y se limpien de forma fiable en Windows, Linux y macOS;
- fallos tempranos, estructurados y accionables;
- tiempos razonables en una segunda ejecución sin sacrificar reproducibilidad.

Este plan no oculta el gate rojo actual del producto ni mezcla su deuda funcional con los defectos de
Sentinel.

## Evidencia confirmada el 2026-08-08

### P0 — Contratos que pueden corromper o aceptar un cierre inválido

1. **Misma versión, builds distintos.** El runtime global y el submódulo reportaban `0.6.4`, pero no
   compartían commit, schema ni capacidades; el global rechazaba metadata que el fijado acababa de crear.
2. **Demasiadas fuentes de verdad.** Gitlink, checkout del submódulo, `quality-tools.json`, lock, cache
   `.quality-tools`, `out/`, runtime global, shim y evidencia de publicación pueden divergir de forma
   independiente.
3. **`task gate` no resuelve su propio contrato.** Llama a `sentinel check`, que exige `--stages`, pero el
   comando coordinado no genera ni descubre el manifest. El operador debe conocer un detalle interno.
4. **Manifest incompatible desde el consumidor.** `stages.mjs` emitía `envAllowlist`, clave rechazada por el
   schema v1 de Sentinel; las suites de ambos lados pasaban porque no existía prueba cruzada.
5. **Semántica de salida contradictoria.** El transporte escribía un reporte válido y salía `1`; Sentinel
   trataba cualquier no-cero como herramienta rota y descartaba los findings. Se corrigió en el consumidor,
   pero falta convertirlo en protocolo versionado del core.
6. **`claim` y gate discrepan sobre la tarea.** Sentinel permitía reclamar un ID que el preflight documental
   rechazaba después, cuando ya existían lock, rama y worktree.
7. **`integrate` no exige un gate PASS vigente.** El CLI puede hacer fast-forward después de un FAIL. El
   protocolo documental lo prohíbe, pero la invariante no vive en código.
8. **Evidencia producida inválida.** `integrate` serializó varios commits y archivos como un único string con
   saltos de línea; su propio validador rechazó la metadata y bloqueó cleanup/release.
9. **Cleanup no es robusto a rutas largas.** `git worktree remove` desregistró el worktree, pero no pudo borrar
   físicamente una ruta de OneDrive por `Filename too long`; fue necesaria limpieza dirigida con ruta larga.

### P1 — Preparación, plataforma y confiabilidad

10. **Detección de shell falsa en Windows.** La presencia de `bash.exe` se interpretaba como disponibilidad
    aunque WSL no tuviera distribución y el probe saliera `1`. Corregido y publicado en Sentinel.
11. **Timeout de integración demasiado estrecho.** Un fixture Git real tardó ~9 segundos y la suite tenía un
    timeout global de 10 segundos. Se amplió y publicó, pero falta política basada en tipo de prueba.
12. **Worktree no equivale a entorno listo.** `task start` crea aislamiento Git, pero puede dejar submódulos,
    dependencias, outputs locales o inputs declarados sin preparar.
13. **Build generado obsoleto.** Cambiar el gitlink no invalida automáticamente `out/`; un build viejo puede
    ejecutarse desde un source nuevo y compartir la misma versión.
14. **`full (automatic)` con 0 archivos.** El gate post-commit seleccionó full aunque el scope reportó cero
    cambios, ejecutando todas las etapas sin una razón coherente.
15. **El lock de consumidor no comparte namespace con el worktree.** `task:take` se ejecutó en el repo raíz,
    pero `task:check` dentro del worktree volvió a pedir la toma. Dos mecanismos de ownership pueden discrepar
    aun cuando pertenecen a la misma tarea y agente.

### P2 — Rendimiento, diagnóstico y mantenimiento

16. **Preflight caro o tardío.** Algunas divergencias de commit/lock/schema aparecen después de suites de
    varios minutos; deben detectarse antes de instalar o probar.
17. **Setup repite trabajo estable.** Sentinel y VarSense recompilan o repiten suites aunque commit, SO,
    runtime y lock no hayan cambiado.
18. **Mensajes mezclan causa y consecuencia.** La salida compacta puede mostrar `tool-error` sin el reporte
    estructurado que ya existe, o recomendar un comando que reproduce el mismo contrato incompleto.
19. **Baseline de calidad no está gobernado.** Sentinel conserva errores del producto, VarSense conserva 103
    errores en el gate observado, frontend falla por deprecación TypeScript, docs por planes/referencia y Rust
    por tooling/código. Debe existir una política explícita de baseline, no suppressions improvisadas.
20. **Deuda de dependencias.** Los audits observados reportaron vulnerabilidades en Sentinel y VarSense. Deben
    resolverse como trabajo de seguridad con pruebas y lock, nunca mediante `npm audit fix` automático.
21. **Lint upstream rojo.** Sentinel compila y prueba, pero el lint conserva errores/warnings preexistentes;
    una release no debe esconder esa diferencia entre gates.

## Principios de diseño

1. **Una identidad canónica:** toda ejecución conoce versión, commit, hash de artefacto, protocolo, schemas y
   capacidades.
2. **Una transacción reversible:** repin, instalación, lock y verificación se aplican juntos o no se aplican.
3. **Fail fast:** política, identidad, plataforma, espacio y task ID se validan antes de suites pesadas.
4. **Un gate canónico:** el proyecto declara etapas; Sentinel agenda, normaliza, registra y decide.
5. **Estado derivable:** cache y runtime pueden reconstruirse desde config + lock; no son otra autoridad.
6. **Invariantes en código:** ownership, PASS vigente, ff-only y cleanup no dependen solo de documentación.
7. **Errores accionables:** cada fallo indica capa, causa, evidencia y un único siguiente comando seguro.

## Fase 0 — Congelar contratos y fixtures de regresión (P0)

Antes de refactorizar, crear fixtures que reproduzcan exactamente esta sesión:

- versión igual con commits/capacidades diferentes;
- gitlink, manifest, lock, cache y `out/` desalineados;
- `task gate` sin `--stages` y manifest con clave no soportada;
- reporte válido con exit `1`;
- claim de task ID ausente del índice documental;
- integrate después de FAIL;
- listas Git multilínea en `commits`/`changedFiles`;
- cleanup Windows con ruta >260 caracteres y worktree ya desregistrado;
- `bash.exe` presente con probe fallido;
- Git worktree lento bajo OneDrive;
- scope vacío que se vuelve full automático.

Los fixtures deben cubrir éxito y fallo, JSON estable, Windows/Linux/macOS cuando aplique y no depender de red.

**Definition of Done:** todos fallan contra la versión anterior por la razón esperada y quedan en CI como
contratos públicos.

## Fase 1 — Identidad única y resolución del runtime (P0)

Implementar `sentinel build-info --json` y `sentinel which --json` con:

- versión semántica y commit completo;
- hash del artefacto ejecutado;
- protocolo, schemas y capabilities;
- ruta real de source, build, runtime y shim;
- origen de selección: proyecto fijado, runtime global o fallback;
- estado limpio/sucio del source cuando sea relevante.

El doctor compara esta identidad contra manifest, gitlink y lock. Una misma versión con distinto commit/hash
es un mismatch bloqueante. `out/` se invalida por fingerprint de source/tsconfig/lockfile; nunca se reutiliza
solo porque existe.

**Definition of Done:** un agente puede responder “qué Sentinel estoy ejecutando” con un comando y doctor
detecta toda divergencia en menos de 2 segundos antes de correr suites.

## Fase 2 — Repin, setup y reparación transaccionales (P0)

Crear una operación idempotente, por ejemplo:

```text
sentinel tools repin sentinel <commit> --dry-run
sentinel tools repin sentinel <commit> --apply
sentinel repair --dry-run
sentinel repair --apply
```

La transacción debe:

1. verificar remoto/commit publicado y árbol limpio;
2. preparar submódulo y artefacto en staging aislado;
3. ejecutar la suite requerida una sola vez por fingerprint;
4. actualizar gitlink, manifest y lock como una unidad;
5. cambiar runtime/shim mediante rename atómico;
6. ejecutar doctor y smoke desde el consumidor;
7. revertir todos los punteros si falla cualquier paso.

`repair` debe poder arreglar un gate roto sin depender de que ese mismo gate pase primero. Nunca modifica
reglas, PATH global o otro proyecto sin `--apply` y evidencia explícita.

**Definition of Done:** repinear una herramienta requiere un comando de usuario; una interrupción deja el
estado anterior utilizable y el segundo run estable no recompila ni repite suites.

## Fase 3 — Unificar gate y cierre coordinado (P0)

- Versionar el protocolo de stage manifest y generar bindings/validadores compartidos para core y adapters.
- Hacer que `task gate` descubra el adapter, genere el manifest y ejecute el gate sin `--stages` manual.
- Definir en el protocolo si un proceso con reporte válido sale siempre `0` o declara `expectedExitCodes`;
  nunca inferir tool-error ignorando un JSON válido.
- Registrar en cada gate: HEAD, policy hash, lock hash, mode, scope y resultado.
- Exigir en `integrate` un PASS reciente que coincida con HEAD/policy/lock y no sea anterior al último commit.
- Validar task ID y fuente documental en `claim`, antes de crear ownership.
- Corregir la selección `full`: scope vacío no puede ser full automático sin una razón versionada.
- Hacer paridad contractual real: el test construye manifests con el adapter fijado y los valida/ejecuta con
  el core fijado.

**Definition of Done:** `claim → start → task gate → commit → integrate` no requiere conocer comandos internos;
un FAIL no puede integrarse y sus findings completos sobreviven hasta el reporte final.

## Fase 4 — Worktree listo y cleanup recuperable (P1)

- `task start` ejecuta una fase declarativa de provisión: submódulos, inputs ignorados autorizados, shims y
  dependencias necesarias; o falla antes de marcar `ACTIVE` con la lista exacta de faltantes.
- Ownership y locks complementarios viven en el Git common dir o en una API única de Sentinel, de modo que
  repo raíz y worktree observen el mismo task ID/agente/TTL sin copiar archivos locales.
- El env manifest admite archivos/directorios por estrategia explícita (`copy`, `link`, `install`, `build`),
  hashes, editable/no editable y redacción de secretos.
- Calcular presupuesto de longitud de ruta antes de crear el worktree. En Windows elegir automáticamente una
  raíz corta autorizada cuando el path previsto no sea seguro.
- `cleanup` tolera estado parcial: worktree desregistrado pero carpeta presente, rama ya ausente, metadata
  recuperable. Las acciones destructivas siguen limitadas a la identidad/ownership de la tarea.
- Corregir la serialización multilínea y validar la metadata antes de persistirla, no solo al leerla.
- `recover --dry-run` debe proponer la reparación exacta y conservar archivo de auditoría antes de mutar.

**Definition of Done:** un worktree `ACTIVE` puede ejecutar el gate; cleanup es idempotente y pasa fixtures de
rutas largas sin borrar recursos ajenos.

## Fase 5 — Rendimiento y observabilidad (P1)

- Separar `preflight`, `install`, `verify` y `gate`; mostrar tiempo por fase.
- Cachear por commit, lockfile, Node/runtime, SO/arquitectura y suite; nunca por versión sola.
- No recompilar Sentinel/VarSense ni repetir suites si el artefacto firmado/fingerprint ya fue validado.
- Ejecutar full/CI solo por regla de scope explicable, flag o cierre publicado; respetar cooldown y lease.
- Emitir progreso estructurado, timeout restante y ruta del log para operaciones largas.
- Usar códigos de salida distintos para política, identidad, plataforma, tool-error, findings y cancelación.

**Objetivos medibles:** preflight <2 s; doctor caliente <2 s; setup caliente <15 s; ningún retry ciego; cada
operación >10 s emite progreso y heartbeat.

## Fase 6 — UX, documentación y adopción (P2)

- `doctor` devuelve `ready`, razones ordenadas y un único `recommendedCommand` por bloqueo.
- Generar help y ejemplos desde el mismo schema de comandos; las skills enlazan el contrato y no lo duplican.
- La skill global conserva solo conducta universal. `AGENTS.md` genérico explica Sentinel; cada proyecto
  declara únicamente rama, adapter, comandos y excepciones propias.
- Publicar notas de migración cuando cambien schema/capabilities y subir versión semántica; no reutilizar
  `0.6.4` para contratos diferentes.
- Añadir un quickstart probado en un repo mínimo y en dos consumidores reales de stacks distintos.

**Definition of Done:** un agente nuevo puede diagnosticar, preparar, tomar y cerrar una tarea siguiendo la
salida del CLI, sin consultar documentación histórica.

## Fase 7 — Baseline, lint y seguridad de dependencias (P2/P3)

Crear tareas separadas para:

- llevar lint upstream a cero o declarar un baseline versionado con fecha/owner;
- triage de advisories por explotabilidad, alcance y upgrade seguro;
- actualizar locks con pruebas y rollback, sin `audit fix` automático;
- gobernar findings preexistentes del consumidor mediante baseline explícito que solo permita disminuir;
- reparar frontend/docs/Rust del consumidor sin mezclarlos con el core de Sentinel.

**Definition of Done:** CI distingue “regresión nueva” de “deuda aceptada con owner/fecha” y ninguna release
declara verde una suite que no ejecutó.

## Orden de entrega recomendado

1. PR A: fixtures P0 y `build-info/which`.
2. PR B: metadata válida, gate PASS obligatorio y cleanup idempotente.
3. PR C: protocolo de stages compartido y `task gate` autónomo.
4. PR D: repin/repair transaccional e invalidación de builds.
5. PR E: provisión de worktrees y rutas largas.
6. PR F: cache, progreso, códigos de salida y documentación generada.
7. Tareas independientes: baseline, lint y dependencias.

Cada PR debe poder publicarse y revertirse por separado, incluir fixture de regresión y actualizar un
consumidor de prueba. No mezclar breaking changes de schema sin migrador y compatibilidad temporal.

## Métricas de aceptación

- 0 builds con misma identidad declarada y capacidades distintas.
- 0 integraciones sin PASS coincidente con HEAD/policy/lock.
- 0 manifests aceptados por un lado y rechazados por el otro en la matriz soportada.
- 100% de cleanup/recover idempotente en fixtures; sin borrado fuera de la raíz autorizada.
- 1 comando para repin y 1 para repair, ambos con dry-run y rollback.
- Segundo setup estable <15 s; mismatch detectado <2 s.
- Todo FAIL conserva findings, archivos y líneas; todo tool-error conserva log y causa.
- Quickstart validado en Windows, Linux y macOS, incluido Windows con OneDrive/ruta larga.

## Rollback

- Mantener el artefacto y lock anteriores hasta completar smoke del nuevo.
- Escribir manifests/locks/metadata mediante temporal + rename atómico.
- Guardar un journal de la transacción con pasos aplicados y compensación.
- Los schemas nuevos incluyen lector/migrador del anterior; rollback no destruye metadata desconocida.
- `repair --dry-run` y `recover --dry-run` son obligatorios antes de mutaciones sobre estado ambiguo.
- Nunca limpiar worktrees/ramas/metadata de otra identidad o agente como efecto de una migración.

## Fuera de alcance de este plan

- resolver los findings funcionales actuales de `glory-rust-template`;
- borrar huérfanos legacy cuyo ownership no esté probado;
- deploy o cambios de producción;
- actualizar dependencias de forma automática sin triage.

## Checklist ejecutable

- [ ] Crear todos los fixtures P0 y hacerlos fallar contra el baseline anterior.
- [ ] Implementar identidad canónica y fail-fast de doctor.
- [ ] Hacer repin/repair transaccional con rollback probado.
- [ ] Unificar el protocolo de stages y volver autónomo `task gate`.
- [ ] Bloquear integrate sin PASS vigente y corregir metadata multilínea.
- [ ] Hacer `task start` listo para gate y cleanup idempotente en rutas largas.
- [ ] Añadir cache, progreso y métricas de tiempo.
- [ ] Migrar dos consumidores y validar la matriz de SO.
- [ ] Separar y ejecutar los planes de lint, baseline y dependencias.
- [ ] Actualizar documentación desde el contrato implementado y cerrar la tarea con evidencia.

## No hacer

- No añadir más wrappers como workaround permanente.
- No usar la versión semántica como sustituto de commit/hash/capabilities.
- No degradar un error de herramienta o contrato a warning.
- No publicar un source sin repinear y probar al menos un consumidor.
- No borrar estado ambiguo ni usar force antes de dry-run y ownership verificable.
