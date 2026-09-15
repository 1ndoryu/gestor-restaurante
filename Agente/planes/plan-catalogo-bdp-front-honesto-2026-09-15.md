# Subplan — Catálogo BDP: front honesto (código visible + estado push) — 2026-09-15

Origen: lecciones de W-Q2.3 (149A-2): el alta de departamento funciona en backend
pero el front miente por omisión — toast "Departamento creado" sin código, sin
estado del push a BDP, error genérico. Navegador diferido por el usuario (se
verificará después con este mismo plan como checklist).

## Objetivo

Que crear un departamento/familia desde Catálogo muestre **qué código se asignó**,
**en qué estado está su push a BDP** y **errores que distingan** "ocupado local"
de "ocupado en BDP", reutilizando la receta ya existente de Sincronización.

## Alcance / no alcance

- Sí: `frontend/src/componentes/bdp/BdpCatalogo.tsx`, `frontend/src/api/bdp.ts`
  (solo hooks/lecturas ya existentes), y —solo en F2— un endpoint backend de
  **lectura** (pre-chequeo código libre; cero escrituras BDP).
- No: worker/cola (`services/bdp_push.rs`), `siguiente_code` por defecto,
  escrituras reales contra BDP en la verificación (simulador), navegador
  (diferido explícito, ver § Verificación visual).

## Dependencias

- Existe y se reutiliza: `GET /api/bdp/push/pendientes`, `POST
  /api/bdp/push/:id/reintentar`, `POST /api/bdp/push/flush`
  (`src/handlers/bdp_push.rs:10-11`); hooks `useListarPushFilas`,
  `useReintentarPushFila`, `useFlushBdpPush` (`frontend/src/api/bdp.ts:932-968`).
- Receta visual a copiar: `frontend/src/componentes/bdp/BdpSincronizacion.tsx`
  (badges `sincronizado`, tabla con `ultimo_error`, toasts honestos) y
  `BdpInventario.tsx:116-117` (toast que ya no miente sobre "encolado").
- El `POST /api/bdp/catalogo` devuelve la fila creada con `code`
  (`src/handlers/bdp_catalogo.rs:52-90`); el push es async vía
  `bdp_push_pendientes` (`entidad_id` = id de la clasificación).

## Fases verificables

- **F1 — Mínimo honesto (obligatorio).**
  1. Toast de alta con código: `"Departamento 901 creado · encolado para BDP"`
     (usa `resp.code`, no solo `nombre`).
  2. Columna/badge de estado push por fila en la tabla de clasificaciones
     (lookup `entidad_id` en `useListarPushFilas`; estados: `pendiente`,
     `sincronizado`, `error` + `ultimo_error` como tooltip/título).
  3. Error honesto: 409 local → "código ocupado"; fila en `error` →
     "falló el envío a BDP: <ultimo_error>. Revisa Sincronización" (sin inventar
     "código ocupado en BDP" si no se sabe).
  Verificación: `npm run type-check` + build Vite OK; alta contra simulador y
  comprobar toast + badge (evidencia: captura o log).
- **F2 — Pre-chequeo código libre (recomendado).**
  Nuevo `GET /api/bdp/catalogo/codigo-libre?tipo=` (solo lectura BDP vía
  `ExportFromProfile`: compara `siguiente_code` local con `MAX_CODE` del perfil;
  si BDP offline, responde `solo_local:true` y el front no bloquea).
  Front: si el código que se asignará está ocupado en BDP, aviso previo
  "el código N está ocupado en BDP; se usará el M" (M = primer libre según
  perfil) o bloqueo con mensaje si no hay libre < 1000.
  Motivación: colisiones reales 1 (=CAFES) y 104 (fuera del perfil) de W-Q2.3.
  Verificación: test backend del endpoint con simulador (perfil con MAX_CODE
  conocido) + type-check/build.
- **F3 — Código manual opcional (diferible).**
  Input código 1–999 en el formulario + validación (backend ya tiene `CHECK` y
  `uq_clasificacion_code`). Solo si tras F2 sigue habiendo fricción real.
  Decisión al cierre de F2, no antes.

## Estado

- Plan creado 2026-09-15. F1/F2/F3 sin empezar. Navegador diferido.

## Próximo paso

F1.1 (toast con código) → F1.2 (badge estado push) → F1.3 (error honesto),
validando type-check + build al cierre de cada uno (edición por módulo).

## Gate y Definition of Done

- Gate: el que declare el proyecto al ejecutar (mínimo type-check + build Vite
  + suites tocadas en verde); ninguna escritura real BDP en la verificación.
- DoD: toast muestra código; cada fila muestra su estado push real; ningún
  mensaje afirma "enviado a BDP" sin fila `sincronizado`; doc actualizada
  (este plan → `Agente/completados/tareas-YYYY-MM-DD.md` al cerrar);
  verificación en navegador hecha con el checklist de abajo.

## Verificación visual (diferida — checklist para entonces)

1. Catálogo → Departamentos muestra `901 PRUEBA-149A2-DEP-20260915`.
2. Crear familia de prueba: toast con código + badge `pendiente`/`sincronizado`.
3. Forzar error (p.ej. BDP offline): mensaje honesto, sin falso éxito.
4. Responsive ≥320px (regla del proyecto) en la tabla con la columna nueva.
