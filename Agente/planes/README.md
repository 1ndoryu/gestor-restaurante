# Planes activos

Cuando una tarea es compleja (>1 sesión o múltiples fases), se crea un plan aquí.
Al completarse, se mueve a `completados/`.

Un plan largo se puede partir, pero el plan sigue siendo **uno**: el índice
(`plan-<tema>-<fecha>.md`) se queda aquí con el checklist de cierre y las partes
van en una subcarpeta con su mismo nombre (`<tema>-<fecha>/`). Así el estado
vive en un solo sitio —el checklist del índice— y las partes son su cuerpo, no
planes sueltos que cada uno necesite su propio checklist.

## Checklist del índice

- [x] Cada plan activo conserva su checklist de cierre (regla `docs-plan-no-checklist` del gate): el índice lo lleva; las partes de un plan partido van en su subcarpeta y no cuentan como planes.
