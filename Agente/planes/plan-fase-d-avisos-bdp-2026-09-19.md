# Plan fase D — Receptor Avisos de Acciones BDP → web (199A-1, activo 2026-09-19)

> Origen: `Agente/documentacion/bdp/guia-bidireccional-avisos-2026-09-19.md` + guía externa
> (fases A–E, 21 pasos). Fase C (web→BDP) = lo ya construido. **Esto solo cubre la fase D**
> (BDP→web). Activación **por dominios**; BDP queda como contingencia.
> No urgente: el restaurante ya opera desde la web sin esta fase.

## Objetivo

Absorber los cambios excepcionales hechos en el TPV (crear/modificar/cambiar-estado/
borrar/finalizar/traspasar) en la misma venta local, sin doble escritura ni facturario propio.

## No alcance

Escritura ciega simultánea web↔BDP · facturario propio · rectificación/anulación fiscal
(sin cobertura documental) · compras/gastos/reservas/marketing/empleados (sin cobertura) ·
evento de pago (no existe: los pagos solo se confirman por `GetOrder`/reconciliación).

## Dependencias externas (bloquean F2/F4)

> Decisión 2026-09-20: **sin preguntas a BDP**. Todo lo no documentado se resuelve
> por prueba empírica en TPV v36 real + valor seguro por defecto (ver F0).

- Verificar en TPV v36 real: qué eventos llegan, qué valores de `Estado` (0–3), formato real.
- Distribuidor: importar XML del punto de exportación en el TPV del restaurante.
- Red: el TPV debe alcanzar la URL del receptor (alcance/red a definir con el restaurante).
- Franja sin servicio para pruebas reales + anulación de residuos en TPV.

## Fases

### F0 — Decisiones + pruebas empíricas (sin código contra BDP salvo sonda)

- [ ] Decisión cliente: sí/no a fase D, por qué dominio se empieza (ventas primero).
- [ ] En TPV v36 real, probar sin preguntar: semántica de update (fusiona/reemplaza),
  `OnlyLastMovements` true/false, valores de `Estado`, unicidad observable de
  `MarketplaceOrderId`/`PaymentId` (sin asumirla: la unicidad la impone la BD local).
- [ ] Lo no resoluble por prueba queda con valor seguro: update sin confirmar →
  fallback cancelar+crear sustituta (antes de cobrar) y edición bloqueada (tras pago);
  rectificativas/anulación fiscal → procedimiento manual en BDP; listado de abiertas →
  se asume ausente (la exportación lo detecta al finalizar); frecuencia → conservadora.
- [ ] Definir alcance/red del receptor con el restaurante.
- DoD: matriz empírica archivada + dominio inicial elegido.

### F1 — Receptor + inbox idempotente

- [ ] `POST /api/bdp/avisos/:accion` (7 acciones; aceptar también GET si se configura así
  en el TPV). Secreto compartido por punto de exportación, rate-limit, límite de tamaño,
  payload censurado en logs (nunca secretos).
- [ ] Inbox `bdp_avisos_inbox` con idempotencia (evento + id comanda + timestamp) y
  reintento manual; aviso rechazado = 4xx sin guardar.
- [ ] Tests: duplicado, perdido, fuera de orden, firma inválida, cuerpo gigante.
- DoD: `cargo test bdp_avisos` verde + sonda con el demo del ZIP si es viable.

### F2 — Worker 7 eventos → misma venta

- [ ] Por evento: `GetOrder`/exportación correspondiente y actualización de **la misma venta**
  (`bdp_order_id` primero; si no, `market_id + marketplace_order_id`).
- [ ] Timeouts por operación según guía (Status 3 → no reintentar factura; Status 2 →
  cancelación confirmada). Sin evento de pago: los pagos solo vía lectura/reconciliación.
- [ ] Estados visibles al usuario (Local / Pendiente de BDP / Confirmado / Requiere
  revisión / Facturado en BDP…).
- [ ] Matriz guía: A05, A07, A09, A13, A17–A20 + regresión cola/poller existentes.
- DoD: matriz verde en simulador + revisión de código.

### F3 — Reconciliación + perfiles + XML distribuidor

- [ ] Reconciliación periódica (extiende `reconcile_ambiguous_orders` del poller).
- [ ] Perfiles de exportación por dominio.
- [ ] Generar el XML de configuración del punto de exportación para el distribuidor.
- DoD: doc operativa + XML probado en importación.

### F4 — TPV real + piloto + cierre

- [ ] Verificación en TPV v36 real (matriz F2 + 7 eventos reales, residuos anulados).
- [ ] Piloto 7 días con el restaurante + informe.
- [ ] Gate canónico + commit/push + deploy vía coolify-manager-rs con autorización.
- DoD: último gate PASS asociado al commit + piloto sin duplicados.

## Seguridad (hereda 149A-2: auditoría + simulación antes de escribir)

Un solo escritor + absorción de eventos (nunca dos maestros). Receptor autenticado por
secreto, fail-closed, todo auditado en `bdp_audit_log`. Sin `eval`/SQL interpolado/shell.

## Estimación

~2.200–3.400 líneas nuevas (~3–4 % del código actual), 7–12 laborables
(5–8 código+tests, 2–4 reales dependientes de terceros).

## Estado

> 2026-09-20: **ninguna prueba contra el TPV real sin autorización explícita del
> usuario** (incluye F0 empírico, F2 y F4). Sin autorización, solo trabajo local
> (código + tests contra simulador).

F0 pendiente de decisión del cliente. Siguiente paso: pruebas empíricas en TPV v36 (sin preguntas a BDP).
