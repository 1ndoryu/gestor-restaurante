# Plan — Entorno de pruebas en la 89 conectado al BDP del portátil

Fecha: 2026-10-31 (ajustado el mismo día). Tarea: `31AA-1`; desbloquea `28AA-3`.
Sustituye a la primera versión (interruptor en producción con intercambio de conexión), descartada
por la usuaria: **producción no se toca**.

## Objetivo

La instancia de la 89 (`rest-dev.wandori.us`, puerto 3105, modo build) ya habla con el BDP del
portátil por el túnel `127.0.0.1:8070 → 89:8071`. Falta que (a) el cliente pueda entrar sin
complicaciones y (b) la pantalla diga siempre, sin ambigüedad, que es un entorno de pruebas
conectado al BDP del portátil. Producción queda con el mecanismo apagado.

## Por qué esto es mejor que el interruptor en producción

- La base de la 89 es otra: lo que se cree o importe durante las pruebas **no toca datos reales**
  (desaparece el riesgo de mezclar demo con producción y la limpieza posterior).
- No hace falta ruta de red a la 66 ni exponer el BDP demo a internet: el túnel a la 89 ya existe.
- Sin migración, sin intercambio de credenciales, sin cola que vigilar, sin despliegue a producción.

## Diseño

**Aviso por entorno, no por base de datos.** Una variable `ENTORNO_PRUEBA_ETIQUETA` (texto libre, p. ej.
«PRUEBAS · BDP demo del portátil de Wan»).
- Definida en el `.env` del laptop, viaja a la 89 por `envRemoto` (`"todo"` en la ficha, sin cambios).
- **Ausente en producción** (Coolify) = mecanismo apagado: sin franja, sin cambios de comportamiento.
- El backend la expone en `GET /api/entorno` (pública, sin autenticar, solo `{ etiqueta }` o
  `{ etiqueta: null }`), para que también salga en la pantalla de login.
- Frontend: `BannerEntorno` en `components/layout.tsx` (encima de `SiteHeader`) y en `Login.tsx`: franja
  fija, ámbar (distinta del rojo de errores), con la etiqueta y una segunda línea «Los datos de aquí son de
  prueba; no afectan al restaurante». Sin botón de cerrar. Si no hay etiqueta, no renderiza nada.
- El indicador BDP de la cabecera (`site-header.tsx`) muestra en el tooltip el host al que apunta.
- Si el BDP del portátil no responde (portátil apagado o túnel caído), el indicador ya degrada; el aviso
  añade «BDP sin respuesta» usando ese estado, para que el cliente sepa que no es un fallo suyo.

El código es el mismo para 89 y 66: cuando se despliegue producción por otro motivo, el cambio va inerte.

## Acceso del cliente (lo que hace falta y quién lo decide)

Hoy `rest-dev` está tras Cloudflare Access con **solo `andoryyu@gmail.com`**. La opción más simple para el
cliente, sin contraseñas ni VPN: **añadir su correo a la política Allow de Access** (entra con un código de
un solo uso que llega a su email, sesión de 24 h) y darle un usuario en la web.
- Cambiar la política de Access da acceso a un tercero: **reservado a la usuaria**. Necesito los correos
  (Guillermo y quien más) y su autorización explícita; sin ella, el plan se queda en «preparado».
- Usuario/contraseña de la web para el cliente en la base de la 89: lo creo yo con credenciales de prueba
  aleatorias en la base de pruebas; la usuaria se las entrega (no se imprimen en chat ni se versionan).
- No se quita Access: publicar sin él queda descartado (regla del dev público).

## Estabilidad (para que no falle delante del cliente)

- Reinicios siempre con `Arranque stop "RESTAURANTE"` + `Arranque up "RESTAURANTE" --sin-bd`; un `up` a secas
  rehace la base desde el laptop y borraría lo que el cliente haya probado.
- Comprobar que el túnel 8070→8071 y el conector `cloudflared` sobreviven a reinicios (hoy son sesiones
  tmux/proceso manual); si no, dejar el arranque documentado y un comprobador.
- Límite que se declara al cliente: el BDP vive en el portátil; si está apagado o sin red, el entorno
  abre pero sin datos del TPV.

## Orden de ejecución

1. `Arranque gate "RESTAURANTE" --inicio` (hay código).
2. Backend: lectura de la variable, `GET /api/entorno`, OpenAPI y cliente generado; test unitario
   (con y sin variable, texto vacío = apagado).
3. Frontend: `BannerEntorno` en layout y login, tooltip del host, estado «BDP sin respuesta». Revisión real
   en el navegador: responsive, tema claro/oscuro, contraste, con y sin variable.
4. Definir la variable en el `.env` del laptop y reiniciar en la 89 (`stop` + `up --sin-bd`); comprobar por
   `rest-dev` (Access activo) que la franja sale en login y en todas las pantallas, y que **sin** la variable
   no sale.
5. Recorrido de aceptación en la 89 con el BDP del portátil: mesas, catálogo, una comanda de prueba al
   BDP demo, lectura de estados.
6. Pruebas y type-check en la 89, `Arranque gate … --cierre`, doc breve
   (`Agente/documentacion/bdp/entorno-pruebas-2026-10-31.md`: cómo encender/apagar el aviso, cómo
   reiniciar sin perder datos, qué hacer si el portátil está apagado), commit, push, `Arranque cierre … --tarea 31AA-1`.
7. Con los correos y la autorización de la usuaria: añadirlos a Access y crear el usuario del cliente
   (cierra la parte técnica de `28AA-3`; el envío del enlace lo hace la usuaria).

## Hecho cuando

- `rest-dev.wandori.us` muestra la franja de pruebas en login y en cada pantalla; con la variable quitada no
  hay franja (probado ambas).
- Una comanda de prueba sale al BDP del portátil y la base de producción no cambia (producción no se tocó).
- Tras `stop` + `up --sin-bd`, los datos de prueba siguen ahí.
- Gate sin hallazgos nuevos, pruebas verdes en la 89, docs al día, commit en `main`.

## Fuera de este plan (reservado a la usuaria)

- Desplegar a producción (el cambio queda en `main`, inerte).
- Cambiar la política de Cloudflare Access y enviar el enlace al cliente.
- Cualquier cambio en la 66.
