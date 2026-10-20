# Plan — Fase D sobre BDP local (2026-10-10)

**ID:** 10AA-3 · **Estado:** **PARCIAL (2026-10-19): fase D no completa.** Detalle por fase en el checklist; cabecera original literal en la [parte 07](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-07-bitacora-parte-1.md).
**Relacionado:** `plan-fase-d-avisos-bdp-2026-09-19.md` (diseño F0–F4), `plan-alcance-local-bdp-2026-10-01.md` (P0–P4 con TPV local), `Agente/usuario/alcance-fase-d-2026-10-02.md` (alcance rev. 10/10).

Este documento es el **índice**: checklist de ejecución y mapa de las partes. Cada tema vive en una sola parte; las demás enlazan.

## Checklist de ejecución

Marcas según la evidencia más reciente de cada parte (fecha entre paréntesis). Pendiente de confirmar: ver «Discrepancias».

- [x] **L0** — Preparación (L0.1–L0.4). → [parte 03](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-03-fases-L0-L3.md)
- [x] **L1** — Conmutar la app a BDP local (2026-10-10). → [parte 03](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-03-fases-L0-L3.md)
- [x] **L2** — Lectura contra BDP local (L2.1, L2.2 y L2.3 hechos; L2.2 cerrada 2026-10-17, ver discrepancia 1). → [parte 03](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-03-fases-L0-L3.md)
- [ ] **L3** — Escritura controlada `PRUEBA-*`: L3.a y L3.c hechas (2026-10-17); **L3.b bloqueada por código** (§9.9); **L3.a2 sin impresora de cocina**. → [parte 03](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-03-fases-L0-L3.md)
- [ ] **L4** — F0 empírica de avisos en TPV local: L4.1 hecha; L4.2 con 6 de 7 eventos con evidencia (2026-10-18), `cambiar-estado` declarado no replicable en local; L4.3 y L4.4 hechas (`Agente/documentacion/bdp/avisos-local-matriz-2026-10-18.md`). → [parte 04](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-04-fases-L4-L5.md)
- [ ] **L5** — Fase D F1–F4 (código) y base T2: L5.0 hecha (2026-10-17, 5 tablas); **F1 hecha** (2026-10-17); **F2 hecha** (2026-10-19, `Agente/documentacion/bdp/avisos-worker-matriz-f2-2026-10-19.md`); **F3 pendiente** (respuesta del distribuidor); **F4 fuera de esta fase** (permiso aparte). → [parte 04](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-04-fases-L4-L5.md)
- [ ] **L6** — Pagos, propina, puntos y factura (D11a–D20). Decidido (usuaria, 2026-10-19): se prueba en el BDP local aunque la suscripción no esté confirmada; lo que falle se registra. L6.d parcial (2026-10-19): OrderEndType configurable (default 0, commit 7385681) y D18 OK en la app; D18a–c OK; D18d sin verificar en BDP; D19 y D19t OK (2026-10-19, commit 3ec3ebb); D20 bloqueado (aviso `finalizar` real, D28 sin implementar); D11a/b y D11c sin cerrar; D16 pendiente (TPV de la usuaria). → [parte 05](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-05-fases-L6-L10.md)
- [ ] **L7** — Stock (D26–D27): ensayo en almacén 1 hecho (2026-10-19, +1 sincronizado, 1001 = 0 en BDP y local); pendiente de real y del `Store` por `warehouse_id`. → [parte 05](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-05-fases-L6-L10.md)
- [ ] **L8** — Maestros y documentos (D24–D25, D28). → [parte 05](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-05-fases-L6-L10.md)
- [ ] **L9** — Avisos duplicados, caídas y reinicios (D04, D08, D10, D21–D23, D29, D31). → [parte 05](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-05-fases-L6-L10.md)
- [ ] **L10** — Excepciones, accesos y piloto (D01, D02, D30, D32). → [parte 05](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-05-fases-L6-L10.md)
- [ ] **Cierre de fase D** — definición de hecho y verificación. → [parte 06](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-06-delegacion-verificacion-hecho.md)

**Abiertas con su desbloqueo** (cabecera 2026-10-19): F3 (respuesta del distribuidor); L6–L10 (suscripción y licencia 84); L3.a2, D03a, D09a (impresora de cocina); L3.b (ruta BDP para modificar venta, no iniciada; propuesta como sesión aparte); `cambiar-estado` (instalador del Monitor de Cocina); descripción de salones y plano de mesas (herramienta de control del TPV); L2.2 (conteos con la usuaria, ver discrepancia 1); F4 (permiso aparte).

## Mapa de partes

Las partes viven en `Agente/planes/fase-d-bdp-local-2026-10-10/`, junto a este índice. El contenido original se ha repartido sin cambiar su texto, salvo los encabezados de cada parte.

| Parte | Contenido | Secciones del original | Líneas del original |
|---|---|---|---|
| [01](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-01-objetivo-alcance-reglas.md) | Objetivo, alcance, no alcance y reglas de ejecución | §1, §2, §4 | 25–57, 89–100 |
| [02](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-02-hechos-pendientes.md) | Hechos verificados y pendientes | §3 | 58–88 |
| [03](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-03-fases-L0-L3.md) | Fases L0–L3 | §5 (L0–L3) | 102–157 |
| [04](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-04-fases-L4-L5.md) | Fases L4–L5 (avisos y código) | §5 (L4–L5) | 159–182 |
| [05](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-05-fases-L6-L10.md) | Fases L6–L10 (pagos, stock, maestros, avisos, excepciones) | §5 (L6–L10) | 184–239 |
| [06](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-06-delegacion-verificacion-hecho.md) | Delegación, verificación y definición de hecho | §6, §7 | 241–270 |
| [07](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-07-bitacora-parte-1.md) | Cabecera original, checklist original y bitácora parte 1 | cabecera, checklist, §8 | 3–5, 10–23, 276–305 |
| [08](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-08-bitacora-parte-2.md) | Bitácora parte 2, mapa de código L6–L8 y próximo paso original | §8 | 274, 306–327 |
| [09](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-09-decisiones.md) | Decisiones pendientes (por recomendación) | §9 | 329–355 |
| [10](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-10-cobertura-fichas.md) | Cobertura del PDF, fichas D01–D32 | §10 | 357–388 |
| [11](fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-11-automatizacion-tpv.md) | Automatización obligatoria de acciones manuales | §11 | 390–447 |

Equivalencias para leer las referencias «§N» del texto: §1, §2, §4 → parte 01; §3 → parte 02; §5 → partes 03–05; §6, §7 → parte 06; §8 → partes 07–08; §9 → parte 09; §10 → parte 10; §11 → parte 11.

Los encabezados y líneas vacías del original se recrean en cada parte. La línea 7 (Relacionado) está en este índice; las líneas 3–5 y 10–23 (cabecera y checklist originales) se conservan literales en la parte 07.

## Discrepancias detectadas (sin resolver)

No se han corregido en esta reorganización; quedan para decidir y actualizar la fuente única:

1. **L2.2:** la cabecera de 2026-10-19 la lista como abierta («conteos con la usuaria»), pero el estado anterior y la cronología de §8 la dan por cerrada el 2026-10-17. El checklist la marca hecha.
2. **Próximo paso:** la cabecera y la línea 326 del original (en la parte 08) dicen «L4.2 … luego L4.3–L4.4 y F2», pero F2 consta hecha (2026-10-19) y L4.3–L4.4 constan hechas (2026-10-18). El próximo paso vigente debe fijarse en este índice.
3. **L5:** el checklist original dice «F2–F4 pendientes», la cabecera de 2026-10-19 dice F2 hecha.
