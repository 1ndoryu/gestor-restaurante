# Plan — Modo de prueba: producción conectada al BDP del portátil

Fecha: 2026-10-31. Tarea: `31AA-1`. Pedido por la usuaria a raíz de lo que Guillermo quiere probar (`28AA-3`).

## Objetivo

Un interruptor temporal en la web de producción que la conecta al BDP del portátil (el demo) en vez
del BDP del restaurante, con un aviso fijo arriba que dice sin ambigüedad **a qué BDP está
conectada**. Al apagarlo, vuelve exactamente a los datos reales de antes.

## Decidido por la usuaria (31/10)

- **Alcance: igual que el BDP real.** En modo prueba la web envía, importa y sondea como siempre,
  pero contra el BDP del portátil. Consecuencia aceptada: lo que se importe o se cree en ese rato
  (artículos, clientes, ventas, mesas) queda guardado en la base de producción.
- **Apagado solo a mano.** No hay caducidad automática; por eso el aviso es permanente y visible
  para todos los usuarios, no solo para quien lo activó.

## Diseño elegido: intercambio de conexión, no capa superpuesta

Al activar, los campos de conexión reales de `configuracion_restaurante` (`bdp_base_url`,
`bdp_login`, `bdp_password`, `bdp_integrator_code`, `bdp_pos_id`, `bdp_employee_id`,
`bdp_items_profile_id`) se **copian a una tabla de respaldo** y se sustituyen por los de prueba. Al
apagar, se restauran desde el respaldo.

**Por qué así y no un «override» al leer la configuración:** hay 52 llamadas a
`ConfiguracionService::obtener`, más lecturas directas por SQL que no pasarían por ningún override:
el poller (`bdp_order_poller/polling.rs`, `SELECT *`) y las consultas de `bdp_write_guard`
(`armado.rs`, `autorizacion.rs`), que atan cada permiso de escritura a `bdp_base_url`. Con el
intercambio, todo lector ve el mismo valor sin tocar ninguno, y los permisos de escritura de antes
dejan de valer contra el BDP de prueba (comportamiento deseado: hay que rearmar).

## Riesgos y cómo se cubren

| Riesgo | Cobertura |
|---|---|
| Cola `bdp_push_pendientes` con cambios del BDP real que saldrían al BDP del portátil (o al revés al volver) | Activar y apagar **se rechazan** si la cola o los avisos pendientes no están vacíos; el mensaje dice cuántos hay. |
| Los datos de prueba se mezclan con los reales | Aceptado. Se guarda `activado_en` y `apagado_en`; al apagar se informa cuántas filas nuevas hay por tabla desde `activado_en` (solo informe: borrar es decisión de la usuaria). |
| Se edita la conexión en Configuración mientras está en prueba y al apagar se pisa | Apagar restaura solo los 7 campos de conexión desde el respaldo; Configuración muestra un aviso encima de la tarjeta «Conexión BDP». |
| Se olvida encendido | Aviso rojo fijo, sin cerrar, en todas las pantallas y para todos los usuarios; el indicador BDP de la cabecera pasa a «PRUEBA». |
| Un trabajador lo activa | Activar y apagar exigen rol Admin (mismo criterio que el PATCH de configuración). |
| Credenciales de prueba en claro | Igual que las reales hoy (columnas de `configuracion_restaurante`); al apagar se borran de la configuración y no quedan en el respaldo. |
| Cache del modo operativo (TTL 60 s) | Activar y apagar llaman a `ServicioModoOperacion::invalidar`. |
| El BDP del portátil no es alcanzable desde la VPS 66 | Ver «Ruta de red». Antes de guardar, activar hace un login de prueba contra la URL nueva y no cambia nada si falla. |

## Ruta de red (hecho externo, no código)

La 66 no ve el portátil. En la 89 ya funciona un túnel (portátil `127.0.0.1:8070` → 89 `:8071`). Para
la 66 hay dos vías, y la elección es de la usuaria porque expone el BDP demo a internet o toca la 66:

1. **Cloudflare Tunnel** del portátil con un hostname propio detrás de Cloudflare Access (como el
   dev público de la 89). La web de la 66 enviaría la cabecera de servicio de Access (variables de
   entorno `BDP_PRUEBA_CF_ID` / `BDP_PRUEBA_CF_SECRET` en el servicio `glory-rest`, solo se envía
   si la URL de prueba coincide con el hostname configurado).
2. **Túnel SSH inverso** portátil → 66: requiere acceso a la 66 (reservado a la usuaria).

La función no depende de la vía: recibe una URL. Se construye y se prueba en la 89 con el túnel que ya
existe; la vía hacia la 66 se decide antes de desplegar.

## Piezas

### Backend (Rust)

1. Migración aditiva `bdp_modo_prueba` (`user_id` PK, `activo`, `etiqueta`, `respaldo` JSONB,
   `activado_en`, `activado_por`, `apagado_en`). Con `.down.sql`. Sin tocar columnas existentes.
2. `src/services/bdp_modo_prueba.rs`: `estado`, `activar`, `desactivar`. Transacción con
   `SELECT … FOR UPDATE` sobre la configuración; comprobación de colas vacías; login de prueba previo;
   invalidación de la caché de modo.
3. `src/handlers/bdp_modo_prueba.rs` + rutas: `GET /api/bdp/modo-prueba` (cualquier usuario
   autenticado; devuelve `activo`, `etiqueta`, `host` sin contraseña ni código integrador, `desde`),
   `POST /api/bdp/modo-prueba/activar` y `POST …/desactivar` (Admin). Documentados en OpenAPI para
   regenerar el cliente del frontend.
4. Cabecera de Access opcional en el transporte, solo si está definida la variable de entorno y el host
   coincide (vía 1 de red).
5. Auditoría: activar y apagar escriben en `bdp_audit_log` (quién, cuándo, host origen y destino).

### Frontend

1. `BannerModoPrueba` en `components/layout.tsx`, encima de `SiteHeader`: franja roja fija
   «MODO PRUEBA · conectado al BDP del portátil (host) · desde dd/mm hh:mm» y, si eres Admin, botón
   «Volver al BDP real». Datos con consulta que se refresca cada 60 s y al recuperar el foco.
2. Indicador BDP de la cabecera (`site-header.tsx`): muestra «PRUEBA» en modo prueba y, en modo normal, el
   host al que apunta, para que nunca haya duda de dónde está conectado.
3. Tarjeta «Modo de prueba» en Configuración → Conexión BDP (solo Admin): URL, login, contraseña, código
   integrador, terminal, empleado y perfil; botón «Activar modo de prueba» con diálogo de confirmación que
   cita el aviso de datos mezclados. Aviso sobre «Conexión BDP» mientras esté activo.
4. Estados vacío, error (cola no vacía, login fallido), carga y teclado; contraste revisado en claro y
   oscuro.

## Orden de ejecución

1. Registrar `31AA-1` en el roadmap (hecho con este plan) y `Arranque gate "RESTAURANTE" --inicio`.
2. Migración + servicio + tests del servicio (sin red): activar/apagar restauran los 7 campos; rechazo con
   cola no vacía; rechazo sin Admin; doble activación; apagado sin estar activo.
3. Handlers + OpenAPI + cliente generado.
4. Frontend: banner, indicador, tarjeta. Revisión real en el navegador con la web servida por la 89.
5. Prueba de extremo a extremo en la 89 (`Arranque stop` + `Arranque up --sin-bd`): config real = URL
   falsa, activar con `8071`, ver banner, leer mesas y catálogo, enviar una comanda de prueba, apagar y
   comprobar que la configuración queda idéntica. Después, el mismo recorrido con la cola no vacía.
6. Pruebas del backend y type-check en la 89; `Arranque gate "RESTAURANTE" --cierre`; documentación
   (`Agente/documentacion/bdp/modo-prueba-2026-10-31.md`, manual de uso y de limpieza) y ayuda en la
   propia tarjeta; commit y push a `main`; `Arranque cierre "RESTAURANTE" --tarea 31AA-1`.

## Hecho cuando

- Con el modo encendido, todas las pantallas muestran el aviso rojo con el host del BDP del portátil;
  apagado, desaparece y la configuración real es idéntica byte a byte a la de antes (probado).
- Una comanda de prueba sale al BDP del portátil y no al real (probado en la 89 con dos URL distintas).
- Con la cola pendiente, activar y apagar se rechazan con el número de pendientes.
- Gate sin hallazgos nuevos, pruebas verdes en la 89, documentación al día, commit en `main`.

## Fuera de este plan (reservado a la usuaria)

- **Desplegar a producción** (`coolify-manager-rs`, incluida la migración): solo con su autorización por
  escritura. Hasta entonces el cambio vive en `main` y en la 89.
- Elegir y montar la ruta de red hacia el portátil (arriba).
- Enviar el enlace a Guillermo (`28AA-3`).
- Borrar de producción los datos de demo que queden tras apagar el modo.
