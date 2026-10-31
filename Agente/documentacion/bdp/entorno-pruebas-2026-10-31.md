# Entorno de pruebas del cliente (VPS 89 + BDP del portátil)

Tarea `31AA-1`; plan en `Agente/planes/plan-modo-prueba-bdp-2026-10-31.md`.

`https://rest-dev.wandori.us` (puerto 3105 de la 89, modo build) usa el BDP demo del portátil por el túnel
8070→8071. Lleva una franja ámbar fija, sin botón de cerrar, en el login y en todas las pantallas:
«PRUEBAS · BDP demo del portátil de Wan — Los datos de aquí son de prueba; no afectan al restaurante».
Producción (VPS 66) no la tiene: sin la variable, el mecanismo está apagado.

## Cómo funciona

- Variable `ENTORNO_PRUEBA_ETIQUETA` en el `.env` del portátil; viaja a la 89 por `envRemoto`.
  **Va entre comillas** (lleva espacios y tildes): sin ellas `dotenvy` falla en silencio y la franja no sale.
- `GET /api/entorno` (pública) devuelve `{ "etiqueta": "..." }` o `{ "etiqueta": null }`; vacío o solo
  espacios cuenta como apagado. Código: `src/handlers/health.rs`.
- Frontend: `components/banner-entorno.tsx` + `hooks/useEntorno.ts`, montado en `components/layout.tsx` y
  `componentes/Login.tsx`.

## Encender, apagar, reiniciar

| Qué | Cómo |
|---|---|
| Apagar la franja | Comentar la línea en `RESTAURANTE/.env` y reiniciar |
| Encenderla | Descomentarla y reiniciar |
| Reiniciar sin perder datos | `Arranque stop "RESTAURANTE"` + `Arranque up "RESTAURANTE" --sin-bd` |

`Arranque up` a secas rehace la base de la 89 desde el portátil y borra lo que el cliente haya probado.

## Qué decirle al cliente

- Los datos de la web son de la base de la 89 (separada de producción); los pedidos de prueba van al BDP demo
  del portátil.
- Si el portátil está apagado o sin red, el entorno abre pero sin datos del TPV (el indicador de la cabecera
  pasa a desconectado).
- Tras reiniciar el portátil, lanzar `Arranque up "RESTAURANTE" --sin-bd` para volver a levantar el túnel
  8070→8071. El conector `cloudflared` corre en la 89 en un bucle de reintento dentro de tmux.

## Verificado (2026-10-31)

- Con la variable: franja en login y dashboard; catálogo del BDP cargado; datos intactos tras reiniciar
  (`stop` + `up --sin-bd`).
- Sin la variable: `/api/entorno` → `null` y sin franja.
- `rest-dev.wandori.us` redirige a Cloudflare Access (302); hoy solo `andoryyu@gmail.com`.

## Pendiente de la usuaria

Correos del cliente para la política Allow de Access y entrega de usuario/contraseña de la web (`28AA-3`).
