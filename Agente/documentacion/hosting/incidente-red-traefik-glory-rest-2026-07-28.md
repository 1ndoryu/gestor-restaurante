# Incidente de red Traefik en deploy de glory-rest — 2026-07-28

## Resumen

Los despliegues Rust construyen y arrancan correctamente la aplicación, pero `coolify-manager-rs` reemplaza el healthcheck seguro por otro que usa la primera dirección de `hostname -i`. Cuando `app` pertenece a la red `coolify`, la primera dirección es IPv6 y se inserta sin corchetes en la URL. `curl` termina con código 3, Docker marca el contenedor como `unhealthy` y Traefik devuelve `503`, aunque `/api/health` responde `200` por `localhost`.

El rollback agrava el incidente: puede restaurar un compose donde `app` solo pertenece a la red privada del stack y puede recrear el contenedor antes de determinar qué versión quedó activa.

## Evidencia

- El endpoint interno `/api/health` respondió `200`.
- El healthcheck efectivo era `set -- $(hostname -i); curl ... http://$1:3000/api/health`.
- Con la red `coolify`, `$1` fue `fd93:f87:6a54::9`; `curl` devolvió código 3 por una URL IPv6 inválida.
- `http://localhost:3000/api/health` respondió `200` dentro del mismo contenedor.
- Los logs mostraron migraciones correctas y el servidor escuchando en `0.0.0.0:3000`.
- El contenedor restaurado tenía únicamente la IP de la red privada.
- Al conectar `app-b8s0cks444o0sogo8kg8wcgw` a `coolify` mediante `coolify-manager-rs host-exec`, el health pasó inmediatamente a `http_ok=true`, `app_ok=true`, `fatal_logs=false`.
- El asset desplegado contenía el marcador de la versión nueva, aunque el gestor informó rollback a la versión anterior.

## Corrección requerida en coolify-manager-rs

1. Mantener `curl ... http://localhost:3000/api/health`; no derivar la URL de la primera dirección de `hostname -i`.
2. Declarar la red externa `coolify` en el compose persistido, no depender solo de una conexión posterior al `docker compose up`.
3. Preservar o reaplicar esa red después de todo rollback y recreate.
4. No restaurar el compose anterior dentro de la fase de health antes de decidir si existe un fallo real.
5. Verificar el commit o un marcador de versión del contenedor activo antes de informar qué versión quedó desplegada.
6. Añadir una prueba de regresión con red IPv4+IPv6: health Docker, app interna y HTTP público en `200`, además de rollback sin pérdida de red.

## Estado actual

Producción quedó saludable con la versión `79d18956`. El compose activo tiene respaldo `docker-compose.yml.bak-287A-8`; se corrigió el healthcheck a `localhost`, se recreó únicamente `app` con la imagen existente y se reaplicó la conexión `coolify`. PostgreSQL permaneció ejecutándose y saludable. La corrección permanente de la herramienta sigue pendiente; un sync/recreate futuro puede reintroducir el healthcheck defectuoso hasta que se implemente 287A-8.
