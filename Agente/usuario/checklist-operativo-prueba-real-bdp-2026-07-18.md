# Checklist operativo — futura prueba real BDP

> **NO EJECUTAR.** Este documento prepara una autorización futura; no la concede.
> **Decisión vigente del 18 de julio de 2026:** nuestro equipo no realizará pruebas reales. El cliente usará `guia-cliente-pruebas-integracion-bdp-2026-07-18.md`, limitada a acciones sin escritura en BDP. Este archivo queda solo como referencia de riesgos.

## Aborto inmediato

Abortar y volver a `read_only` si ocurre cualquiera de estos casos:

- host, empresa, POS, empleado, perfil, tender o serie no coincide con lo autorizado;
- snapshot incompleto, vencido o con una sección nula;
- auditoría no puede abrirse;
- `GetOrder` no devuelve estado, total o pagos esperados;
- timeout, HTTP anómalo, JSON inválido o desconexión;
- respuesta sin identificador remoto esperado;
- aparece una operación simultánea o un estado `ambiguo` previo;
- el payload revisado difiere en un solo campo del autorizado.

## Paquete que se presentará para autorización

- endpoint y una única entidad exacta;
- URL base exacta y valor exacto que se añadirá a `BDP_WRITE_ALLOWED_ORIGINS`;
- payload completo redactando solo secretos, no campos operativos;
- snapshot y lecturas preflight con fecha/hora;
- efecto esperado en BDP y Glory;
- riesgo residual e irreversibilidad;
- consulta de reconciliación posterior;
- pasos manuales de remediación acordados con el restaurante;
- comando de retorno inmediato a `read_only`.

## Secuencia por operación

1. Confirmar que no existe estado `ambiguo` para la entidad/idempotency key.
2. Obtener y revisar lecturas preflight.
3. Crear snapshot completo; recordar que no es rollback BDP.
4. Abrir auditoría antes de armar escritura.
5. Autorizar exactamente un alcance y un destino.
6. Ejecutar una única llamada sin retry automático.
7. Reconciliar por identificador estable aunque la respuesta parezca exitosa.
8. Cerrar auditoría como `exito`, `error` o `ambiguo`.
9. Volver a `read_only` y retirar la allowlist de escritura.
10. Comparar snapshot/estado y detenerse antes de cualquier operación siguiente.

Pago y factura nunca se autorizan juntos. Una autorización de orden no autoriza cliente, pago ni factura.
