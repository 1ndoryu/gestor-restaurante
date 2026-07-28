# Incidente de red Traefik en deploy de glory-rest — 2026-07-28

## Resumen

Los despliegues Rust construyen y arrancan correctamente la aplicación, pero el flujo de rollback de `coolify-manager-rs` restaura un compose donde `app` solo pertenece a la red privada del stack. La etiqueta `traefik.docker.network=coolify` queda activa sin que el contenedor pertenezca a esa red, por lo que la aplicación responde internamente y el acceso público devuelve `503`.

## Evidencia

- El endpoint interno `/api/health` respondió `200`.
- Los logs mostraron migraciones correctas y el servidor escuchando en `0.0.0.0:3000`.
- El contenedor restaurado tenía únicamente la IP de la red privada.
- Al conectar `app-b8s0cks444o0sogo8kg8wcgw` a `coolify` mediante `coolify-manager-rs host-exec`, el health pasó inmediatamente a `http_ok=true`, `app_ok=true`, `fatal_logs=false`.
- El asset desplegado contenía el marcador de la versión nueva, aunque el gestor informó rollback a la versión anterior.

## Corrección requerida en coolify-manager-rs

1. Declarar la red externa `coolify` en el compose persistido, no depender solo de una conexión posterior al `docker compose up`.
2. Preservar o reaplicar esa red después de todo rollback y recreate.
3. No restaurar el compose anterior dentro de la fase de health antes de decidir si existe un fallo real.
4. Verificar el commit o un marcador de versión del contenedor activo antes de informar qué versión quedó desplegada.
5. Añadir una prueba de regresión: app interna `200`, pertenencia a `coolify`, HTTP público `200` y rollback sin pérdida de red.

## Estado actual

Producción quedó saludable con la versión `79d18956`. La conexión a `coolify` fue reaplicada mediante el propio gestor. La corrección permanente de la herramienta sigue pendiente; un recreate futuro puede requerir la misma recuperación hasta que se implemente 287A-8.
