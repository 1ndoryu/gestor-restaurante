# Diagnóstico BDP de producción — 27/11/2026

`21AB-4b`. Intento de correr los seis checks de configuración contra el BDP del restaurante
desde producción (`https://restaurante.wandori.us`). Se hizo en solo lectura: login de
administrador y `GET /api/configuracion/bdp/diagnostico`. Ninguna escritura en producción ni
en el TPV, y ninguna credencial impresa.

## Resultado: no se llegó al TPV

```
login            200
diagnostico      200
health_ok        false
detalle          No se pudo contactar BDP Health:
                 http://100.83.196.35:8068/Service/Health: operation timed out
```

`GET https://restaurante.wandori.us/api/health` → **200**: producción (contenedor, base,
proxy) está sana. Lo que no responde es el PC del restaurante, que producción alcanza por
Tailscale (`100.83.196.35`, puerto 8068 de WebLink REST).

## Qué sí queda demostrado y qué no

- **Sí:** producción tiene la ruta a Tailscale y la usa (la etapa 0 del plan de Guillermo
  quedó resuelta el 03/11/2026). El diagnóstico llega a ejecutarse y sabe informar del fallo
  de Health en vez de dar valores inventados.
- **Sí:** el portátil y la 89 no pueden llegar al TPV del restaurante por su cuenta:
  `ping 100.83.196.35` → 100 % de pérdida y la 89 no tiene binario `tailscale`.
- **No:** no hay ningún dato del restaurante medido. Los seis checks siguen sin valor real
  hasta que el TPV vuelva a contestar.

## Por qué no se puede repetir hoy

El TPV del restaurante no está escuchando en 8068 ahora mismo. No es algo que podamos
arreglar desde aquí: hace falta que el equipo del restaurante confirme que ese PC está
encendido, que WebLink REST está corriendo y que sigue en la misma red Tailscale.
Guillermo es quien puede comprobarlo.

## Cómo repetirlo cuando esté accesible

Con las claves `PROD_ADMIN_EMAIL` / `PROD_ADMIN_PASSWORD` del `.env` de RESTAURANTE (nunca
en chat ni en commits), una lectura de login más la llamada al endpoint devuelve los seis
checks. La salida se guarda aquí, en `Agente/documentacion/bdp/`, como el equivalente del
`diagnostico-tpv-demo-2026-11-21.md` del demo.

## Límite conocido del propio diagnóstico

El check «Mapeo de order types» (`bdp_sync_preflight/checks.rs:190`) solo comprueba que cada
`Type` sea un entero ≥ 0: BDP no publica ninguna ruta que liste los tipos de comanda del
servidor. Que `comedor → 1` y `barra → 0` sean los correctos para *este* restaurante solo se
confirma con una comanda real de cada canal en su TPV.
