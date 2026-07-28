# Prevención — Inmutabilidad de migraciones aplicadas

**Fecha:** 2026-07-28
**Estado:** pendiente de automatización

## Riesgo

Modificar una migración SQL que ya fue aplicada cambia su checksum. SQLx responde con `VersionMismatch` y detiene el backend antes de servir tráfico.

## Regla propuesta

- Registrar o consultar las versiones presentes en `_sqlx_migrations` del entorno local.
- Antes del commit, comparar el contenido de cada versión aplicada con el commit base.
- Rechazar modificaciones o eliminaciones de una migración aplicada; permitir únicamente migraciones nuevas.
- No ofrecer como solución actualizar manualmente el checksum. La corrección debe restaurar el archivo original y mover el cambio a una versión posterior.
- Añadir una prueba que modifique una migración ficticia aplicada y confirme que el self-check falla.
