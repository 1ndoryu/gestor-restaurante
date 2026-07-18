# Simulador local BDP WebLink

Este servicio reproduce únicamente el contrato HTTP/JSON que Glory utiliza. No contiene BDP, no copia ejecutables ni licencias y no demuestra por sí solo el comportamiento del producto real.

## Límites de seguridad

- Solo admite `127.0.0.1`, `localhost` o `::1`; rechaza `0.0.0.0` y direcciones de red.
- Los fixtures son ficticios (`example.invalid`) y el reset elimina todo el estado simulado.
- Las rutas administrativas exigen `X-Simulator-Key` y el historial redacta credenciales y datos personales.
- La consola imprime claramente `SIMULADOR (NO BDP)`.

## Inicio manual

```powershell
python tools/bdp-weblink-simulator/server.py --admin-key clave-local-de-al-menos-16-caracteres
```

URL permitida para Glory durante pruebas locales: `http://127.0.0.1:18765`.

## Control de escenarios

- `POST /__simulator/reset`: restaura fixtures.
- `GET /__simulator/state`: estado actual.
- `GET /__simulator/history`: llamadas recibidas.
- `POST /__simulator/fault`: programa un fallo para la próxima llamada a una ruta.

Todas requieren `X-Simulator-Key`. Ejemplo de respuesta perdida después de aplicar una orden:

```json
{"Path":"/API/Orders/Create","apply_then_disconnect":true}
```

También se aceptan `http_status`, `remote_error`, `invalid_json` y `delay_ms`.

## Tests locales

```powershell
python -m unittest discover -s tools/bdp-weblink-simulator -p "test_*.py" -v
```

Los tests solo abren un puerto efímero en `127.0.0.1` y nunca leen la configuración de Glory ni una URL BDP.
