# Prevención — Secretos BDP en documentación

**Fecha:** 2026-07-28
**Estado:** pendiente de automatización

## Riesgo

Planes y reportes Markdown pueden conservar contraseñas o códigos integradores copiados durante una prueba. Aunque después se redacte el archivo, el secreto permanece en el historial de Git.

## Regla propuesta para self-check / Glory Sentinel

- Analizar archivos Markdown y código versionado antes del commit.
- Rechazar valores literales junto a nombres como `BDP_PASSWORD`, `password`, `integrator_code`, `BDP_INTEGRATOR_CODE` o equivalentes.
- Admitir únicamente marcadores explícitos como `<redactado>`, `<configurado en entorno>`, `${VARIABLE}` o referencias al gestor de secretos.
- Mostrar archivo y línea, pero nunca repetir el valor detectado en la salida.
- Añadir una prueba con credencial ficticia y otra con marcador permitido.

## Respuesta operativa

Si el secreto llegó a un commit, redactar el árbol actual y rotarlo inmediatamente. Reescribir historial compartido requiere una decisión coordinada y no sustituye la rotación.
