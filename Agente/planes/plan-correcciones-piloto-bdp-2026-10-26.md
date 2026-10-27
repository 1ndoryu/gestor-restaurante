# Plan — Correcciones antes del piloto BDP (2026-10-26)

**Fuente.** `Agente/usuario/correcciones-antes-del-piloto-2026-10-26.docx` (guía de revisión dirigida a Wan) sobre `Agente/completados/informe-fase-d-2026-10-22.md`.
**Objetivo.** Que el informe de cierre afirme solo lo demostrado: reabrir las fichas cerradas sin licencia, configuración o ejecución real; completar en local lo reproducible; automatizar los maestros; separar el modo automático por categoría. Producción sigue en solo lectura.

## Límites de este plan

- **Licencia.** Las pruebas con WebLinkRestAPI de pago no pueden correrse hasta que BDP active la suscripción 84 en el TPV local (decisión del cliente). El código se deja listo y la prueba queda `pendiente de licencia`.
- **Hardware.** Impresora de cocina y reinicio real de BDP.NET siguen siendo `pendiente de piloto`.
- **Terceros.** D32 (responsables) y D31 (terminal) no los cierra el agente.
- **Compilación y tests** en la 89 (`node scripts/run-cargo.mjs …`, `Arranque`); nada pesado en el laptop. No se escribe en el BDP de producción.

## Fases

| Fase | Tarea | Qué entrega | Depende de licencia |
|---|---|---|---|
| F1 | `26AA-1` | Estados honestos: informe, cobertura de fichas y anexo B8 con los cinco estados (cerrada, pendiente de licencia/configuración/prueba local/piloto, límite confirmado) | No |
| F2 | `26AA-2` | Lectura programada e independiente de formas de pago y empleados (cadencia por cuenta, activos/inactivos, última sincronización y error) | No |
| F3 | `26AA-3` | Permisos automáticos por categoría (comandas, artículos y clientes, propinas y puntos, pagos, facturas, stock, camarero), apagados de fábrica y auditados | No |
| F4 | `26AA-4` | D21 reformulada como cambio de estado de comanda pendiente de validación (avisos 0 y 1), sin atribuirla al Monitor de Cocina | Prueba viva: no |
| F5 | `26AA-5` | Pruebas locales: concurrencia D08 (peticiones paralelas), D29, D21a, stock completo (D26/D26c/D26e/D27), D18d y D05–D07 | Parte sí |
| F6 | `26AA-6` | Una ficha de evidencia por prueba e informe de cierre actualizado | No |

## Orden y criterios de hecho

1. F1 primero: es solo documentación y corrige el estado antes de seguir.
2. F2 y F3 en código, con test y gate (`Arranque gate "RESTAURANTE" --inicio` / `Arranque cierre`).
3. F4 y F5: lo que el BDP local en edición gratuita permita se ejecuta y se guarda; lo demás queda `pendiente de licencia` con la petición preparada.
4. F6 cierra con el informe nuevo. El modo automático no se propone mientras haya fichas abiertas.

## Checklist de cierre

- [x] F1 estados corregidos en informe, cobertura y anexo (informe alineado; anexo B8 remite a §1)
- [x] F2 código (`4fcb3a0`, con test) y prueba viva en el BDP local (24-m, punto 5)
- [x] F3 permisos por categoría, con migración, UI, servidor y cola (`dfa3716`, `cfa42de`, `4b43aaf`, `c82c713`; gate cerrado: 0 nuevos)
- [~] F4 D21 reformulada en el informe; tests del receptor 26/26 en la 89 (`bdp_avisos_worker`); comanda pendiente 72 creada en el BDP local, la consola de autocomanda no responde en la edición gratuita (pendiente de licencia o piloto)
- [~] F5 hechas en el BDP local: D08 (concurrencia real), D26, D26c, D26e, punto 9 y arreglo de idempotencia (`fed7ddd`); D21a, D22a y D26a cerradas el 27/10 (el aviso no llegaba por el 503 de cuentas múltiples, no por el receptor); pendientes: D21 (pasa a licencia o piloto), D27 (la venta no movió el stock), D29 (reinicio real, requiere UAC); D18d y D05–D07 esperan licencia
- [~] F6 informe actualizado con recuentos reales (26/10/2026); PDF regenerado el 27/10; faltan D27 y D29
