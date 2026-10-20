# Fase D BDP local — 01 Objetivo, alcance y reglas

> Parte del [plan Fase D sobre BDP local](../plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 1. Objetivo

Dejar la aplicación apuntando al BDP local (TPV 36.2 licenciado en el portátil) en lugar del BDP real, y con ese TPV: (1) cerrar la F0 empírica de la fase D (Avisos de Acciones), que bloquea F2 y F4; (2) ejecutar en local las pruebas de aceptación D01–D32 que promete el alcance (`Agente/documentacion/bdp/alcance-fase-d-pdf-2026-10-10.html` y `Agente/usuario/alcance-fase-d-2026-10-02.md`; fichas en `Agente/planes/anexo-b8-aceptacion-2026-10-08.md`, rev. 10/10), cada una en su fase (§10). Lo que el PDF promete y este plan no puede probar en local queda marcado como diferido, con su motivo.

**Regla de alineación (2026-10-18):** este plan **solo cumple y prueba lo que pide el alcance** (`alcance-fase-d-2026-10-02.md`: matriz D01–D32, «Qué se logrará», «Fuera del alcance»). Nada se añade por iniciativa propia. Cada paso L* existe para una ficha D*; lo que el alcance no pide no entra. Lo que el entorno local no puede reproducir se **declara con su motivo y no se persigue**:

| No replicable en local | Motivo | Qué se hace |
|---|---|---|
| Aviso `cambiar-estado` (D21/D22, estado de cocina) | Lo emite el Monitor de Cocina, app aparte (sin licencia) en otro PC de la red; no está en este equipo ni se localiza instalador | Se prueba el receptor con carga sintética (acción `cambiar-estado`, `Estado` 0–3) y queda «pendiente de real» en la ficha |
| L3.a2 / D03a / D09a (envío y efecto en cocina) | Sin impresora de cocina en el TPV local; la tecla Cocina no genera aviso | «Ensayada sin testigo de impresora; pendiente de real» |
| L3.b / D05–D07 (modificar por la API) | La API WebLink no tiene operación de modificación (§9.9) | «No soportado: pendiente de BDP»; no se simula |
| L6–L10 con pagos, facturas, `AddPoints` | Dependen de la suscripción confirmada | Condicionados; si no hay suscripción, se declaran no probados |
| D30 piloto | Permiso aparte (F4) | Fuera |

## 2. Alcance y no alcance

**Dentro**
- Conmutar la configuración BDP de real a local (en base de datos, no en `.env`).
- Revisar la cola de envíos pendientes antes de conmutar (L0.3).
- Pruebas de lectura y de escritura reversible contra el TPV local (comandas `PRUEBA-*`).
- F0 empírica de los 7 avisos en el TPV local.
- Tras la F0: F1–F4 de 199A-1, según el diseño del plan 2026-09-19, precedidas de la base T2 (L5.0, hecha).
- Pruebas de aceptación D01–D32 en local (L6–L10), con la evidencia que exige cada ficha del anexo B8.
- **Toda acción manual en el TPV/BDP local se automatiza** (regla obligatoria, §11). Un agente la ejecutará; la usuaria solo da permisos por lote.

**Fuera**
- BDP real (`IP_BDP:8068`, PC remoto por Tailscale) y cualquier host de producción: ni lectura ni escritura.
- `bdp_sync_mode = automatic`. Este plan opera en `read_only`, salvo escrituras por lote autorizado (L3).
- Pagos y facturas (`Payment/Add`, `InvoiceOrder`) hasta confirmar suscripción: no se prueban en L3; pasan a L6 con esa precondición.
- Producción y piloto (D30, F4): fuera, permiso aparte. Antes de F4, el despliegue por etapas del PDF (observación → maestros → lectura → escritura → pagos/factura → stock) y el rollback por escrito.
- Envío a Guillermo y cambios en PDF o alcance: permiso expreso. Commit e integración: solo en el cierre (§7).
- Preguntas al distribuidor: decisión 2026-09-20 (`plan-fase-d-avisos-bdp-2026-09-19.md:21`). Lo no documentado se resuelve por prueba en el TPV.


## 4. Reglas de ejecución

- Ninguna escritura al TPV local sin permiso expreso por lote.
- Cada escritura deja fila en `bdp_audit_log` y comprobación visual en el TPV.
- Secretos (login, password, código integrador, IP real): nunca en repo, plan, logs ni chat. El volcado de L1.1 contiene credenciales: va a `C:\tmp\fase-d-local\` (fuera del repo) y se borra al cerrar.
- Compilar solo con los wrappers de AGENTS §0.2 (`CARGO_TARGET_DIR` en `C:\tmp`). Medir `C:\tmp` antes de compilar; si roza 20 GB, limpiar.
- Stage explícito por archivo (AGENTS §5). En `roadmap.md`, por hunk: tiene ediciones ajenas. Nunca `git add .`. No se tocan ni se stashean los cambios ajenos.
- Subagentes: máximo 3 en paralelo; respuestas de 15 líneas como máximo, con `archivo:línea`. El principal comprueba el dato crítico antes de decidir con él y revisa el diff del `implementador` antes de cerrar.
- Diseño y decisiones técnicas reversibles: las decide el agente (AGENTS §0.3). La usuaria decide solo lo listado en §9.
- **Entorno de prueba (dicho por la usuaria, 2026-10-18):** el TPV y la BD locales son de prueba. Los datos que se creen, modifiquen o borren (comandas, tickets, líneas, turnos) no importan. No hace falta preservar ni consultar el estado previo de un ticket de prueba; sí hace falta no tocar el BDP real ni producción.
- **Antes de cada acción en el TPV, revisar el manual o la ayuda de esa función** (`AyudaHOS.chm`, §11.3b). Sin esa revisión anotada, no se pulsa nada.
- **Acciones en el TPV:** solo por lote automatizado (`scripts/pc/lote.mjs`, cuando exista) o por `tpv.mjs`; nunca clics sueltos ni pasos «los hace la usuaria» (§11). Sin permiso por lote en el chat, ninguna acción que escriba, cierre, cobre, facture o borre.
