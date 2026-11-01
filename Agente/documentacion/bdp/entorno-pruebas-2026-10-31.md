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

## Acceso del cliente (2026-11-01)

- Cloudflare Access: `guillermo.autoia@gmail.com` añadido a la política Allow de `rest-dev` (código por correo, 24 h).
- Usuario web: `demo@restaurante.com` (los datos del BDP, catálogo y ventas son **por usuario**: un usuario recién
  registrado sale vacío y con «BDP Desconectado»). Contraseña aleatoria, fuera del repo, en `DEMO_RESTAURANTE_PASSWORD`
  del `.env` del portátil; cambiarla con `PUT /api/auth/password`.
- Recorrido real con el BDP conectado: franja en todas las pantallas, plano de sala (6 mesas), catálogo, sincronización.

## Prueba completa para entrega (2026-11-01)

- Móvil 375 px y tema claro/oscuro, 12 rutas cada uno: sin desbordes y con la franja visible (rutas inexistentes redirigen a `/`).
- Venta con Mesa 5 y una línea por API: se crea (201) con `num_mesa` 5 y **no** se envía al BDP (`bdp_synced` falso, sin error).
  Es el guarda de escritura (`bdp_write_guard`): con el BDP en «Solo lectura» no se empuja nada hasta armar la escritura.
  Para que el cliente vea comandas en el TPV hay que armar la escritura del BDP demo; hoy el entorno solo lee.

## Pendiente de la usuaria

Enviar el enlace y las credenciales al cliente (`28AA-3`). Queda un usuario vacío `guillermo.autoia@gmail.com` en la base de la 89, sin uso.
