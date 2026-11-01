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

- Desborde horizontal a 880 px de escritorio (13 de 23 rutas): `<main data-slot="sidebar-inset">` no tenía `min-w-0` y su
  contenido mínimo ensanchaba la página a 1030 px. Corregido en `components/ui/sidebar.tsx`; tras el despliegue, 0 desbordes en las 23 rutas.
- IVA 8 % en la venta nueva: no es un fallo; `demo@restaurante.com` tenía `iva_por_defecto` 8.00 en su configuración y
  `useFormularioVenta` lo antepone al 10 % del formulario. Puesto a 10 por la UI de Configuración (2026-11-01).
- Desborde a 375 px en cabecera (`site-header.tsx`), barras de acciones (Clientes, Plano de sala, Campañas, Plantillas WA)
  y campos de hora de «Horarios de turnos» (recortaban «12:00 a. m.» en locales de 12 h): `min-w-0`/`truncate`, `flex-wrap`
  y rejilla de una columna hasta `2xl` con `w-36`. Verificado a 375 y 880 px.
- Flujos creados a mano con éxito: venta, gasto, reserva, cliente, búsqueda vacía, validaciones y teclado (Tab/Escape).
- **Túneles:** `Arranque up` solo gestiona el túnel directo 3105 (lo retiene el supervisor del workspace-manager y `up`
  acaba en «tunel ssh no levanto…», aunque el backend de la 89 sirve el build nuevo). El túnel inverso 89:8071→portátil:8070
  del BDP **no** lo gestiona el wrapper: es un `ssh -N -f -R 127.0.0.1:8071:127.0.0.1:8070 dev89` manual; hay que
  recrearlo tras reiniciar el portátil o cortarse el ssh.
- **No confirmado:** escritura real de una venta en el BDP (exige armar `unidirectional` con confirmaciones y snapshot fresco).

## Pendiente de la usuaria

Enviar el enlace y las credenciales al cliente (`28AA-3`). Queda un usuario vacío `guillermo.autoia@gmail.com` en la base de la 89, sin uso.
