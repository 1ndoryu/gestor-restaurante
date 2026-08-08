# Plan de corrección de auditoría Sentinel — 2026-08-08

**Tarea activa:** `048A-22`

## Objetivo

Dejar `glory-rust-template` en un estado reproducible y seguro para coordinar tareas, sin perder cambios locales del submódulo ni borrar recursos de otros checkouts. El trabajo se divide entre recuperación operativa, alcance del gate y mejoras de lifecycle.

## Orden obligatorio

1. **Inventario y congelación:** preservar metadata, ramas, worktrees y cambios locales; no ejecutar cleanup manual ni publicar el pin mientras el submódulo esté sucio.
2. **Corregir Sentinel:** mantener/publicar únicamente cambios revisados; migrar metadata legacy v1/v2 sin mutar durante `status`; validar profundamente eventos/gates; descubrir namespaces desde refs Git para no ocultar huérfanos.
3. **Alinear alcance:** añadir TSX a Sentinel/VarSense y excluir artefactos `.sentinel`, reportes, herramientas provisionadas y checkouts hermanos del análisis del consumidor.
4. **Alinear política:** la raíz interna sigue siendo el default; la raíz externa es una excepción declarada y validada solo porque el workspace visible la necesita. Debe quedar descrita en la skill, README, tests y configuración operativa.
5. **Recuperación controlada:** ejecutar primero diagnóstico/read-only. La metadata `028A-22` es legacy, apunta a una ruta rota y usa una rama antigua; no se elimina hasta verificar ownership. Los directorios físicos contienen datos y se conservan mientras haya ambigüedad.
6. **Validación:** compile, core check, smoke LSP, suite, lint, lock/doctor, task status JSON y gate consumidor. Los tests del working tree no sustituyen una validación del commit publicado.
7. **Publicación:** solo después de revisar diff y asignar ownership. Publicar Sentinel, repinear padre, regenerar lock y repetir doctor/gate. No hacer deploy.

## Criterios de terminado

- Sentinel fuente limpio, commit publicado y lock `match`.
- Metadata legacy visible como inválida/recuperable con diagnóstico claro, nunca borrada silenciosamente.
- `status --all` descubre ramas coordinadas aunque falte metadata y distingue worktrees físicos.
- `.tsx` cubierto por ambos analizadores declarativos.
- El reporte no incorpora `.sentinel/worktrees`, `.quality-tools`, `.quality-reports` ni `gate-hardening`.
- Existe una única política documentada para worktrees internos/externos.
- No quedan recursos de la tarea propia sin owner; los recursos de otros checkouts quedan intactos y documentados.
- `quality:lock --check`, doctor y gate del consumidor pasan o dejan un fallo concreto no relacionado con esta corrección.

## Checklist de cierre

- [x] Identificar la divergencia entre gitlink, manifiesto y lock del consumidor.
- [x] Corregir y publicar las regresiones reproducibles de Sentinel encontradas durante el setup.
- [x] Repinear el consumidor al commit publicado y regenerar el lock.
- [x] Ejecutar el gate coordinado y registrar el resultado real: el transporte ya alcanza las etapas; el gate full conserva deuda preexistente del producto y contratos cruzados pendientes, sin degradarlos a warning.
- [ ] Integrar, limpiar worktree/metadata y liberar ambos mecanismos de ownership.

## No hacer

- No usar `git reset`, `git clean`, `git stash`, `git worktree prune`, `rm` sobre carpetas ambiguas, force-push ni deploy.
- No publicar cambios del submódulo sin revisar que los seis archivos pertenezcan a esta ampliación.
- No convertir un error de herramienta en warning.
- No declarar que `SNT-16c` existe sin evidencia actual.
