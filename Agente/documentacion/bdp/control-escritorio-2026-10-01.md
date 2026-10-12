# Control del escritorio (cua-driver) — notas de uso 2026-10-01

> Herramienta en BETA (`pc-control`). Lo aprendido operando el TPV local
> (plan `Agente/planes/plan-alcance-local-bdp-2026-10-01.md`). Incluye feedback
> para mejorar la skill/herramienta al final.

## Setup que funciona

- Binario: `%APPDATA%\ai.opencode.desktop.propio\bin\cua-driver.exe`.
- El daemon de la app (`\\.\pipe\opencode-pc`, integridad Medium) NO sirve para
  el TPV: `TpvNet.exe` corre elevated (High) y UIPI descarta sus PostMessage.
- Solución: segundo daemon elevado en otro pipe (requiere aceptar UAC una vez):
  `Start-Process cua-driver 'serve --socket \\.\pipe\opencode-pc-admin' -Verb RunAs`.
- Llamadas por CLI: `cua-driver --socket <pipe> call <tool>` con el JSON **por
  stdin** (la shell aquí desmonta las comillas si va como argumento).
- Sesión común en todas las llamadas: `"session": "bdp-p0"`.

## Lo que ve el daemon elevado (y el normal no)

- `get_window_state` del `MENÚ GENERAL` con daemon normal: **0 elementos**
  (parecía WinForms sin UIA). Con daemon elevado: **371 MenuItems con nombre**
  (`MenuStripBDP1` completo). Mapa extraído en `mapa-menu-tpv-2026-10-01.md`.
- Items P0: `Series` [3], `Formas de Pago` [6], `Series Tpv` [61], `Almacenes`
  [98], `Motivos de Traspaso` [99], `Parámetros de Exportación` [256],
  `Configuración Avisos de Acciones` [358].
- El sufijo del `element_token` (`s00000001:N`) coincide con el índice `[N]`
  del `tree_markdown`. Invocar `Series` [3] por token **abrió la ventana
  SERIES** sin tocar el foco (vía background).

## Reglas de un solo uso (dieron errores reales)

- `element_token`: muere con el siguiente snapshot de la misma ventana.
- `capture_id`: se consume al usarlo (`capture_not_found` si se reutiliza).
  Ritmo: snapshot → usar su `capture_id` en la llamada inmediata siguiente.
- No combinar snapshot+clic+sleep en un solo comando largo: si algo se cuelga
  no se sabe qué fue. Una llamada por comando, timeouts explícitos.
- Sin `python3` en esta máquina (el alias abre la Store y cuelga): parsear el
  JSON con PowerShell (`ConvertFrom-Json`), PNG con
  `[IO.File]::WriteAllBytes(..., [Convert]::FromBase64String(...))`.
- Ver PNG: `read` sí muestra imágenes; reducir con `max_image_dimension: 768`.

## Clics por coordenadas en el TPV (pendiente de resolver)

- `click x,y` background sobre la ventana SERIES → `background_unavailable`
  (`uia_status: busy`, recomienda foreground). Correcto según contrato.
- Reintento foreground → `foreground_unavailable` (HWND foreground real `0x0`
  tras el clic). El clic en `Ini` no tuvo efecto visible (registro vacío igual).
- Hipótesis abiertas: (a) verificación de foreground demasiado estricta tras el
  swap; (b) la tabla SERIES está genuinamente vacía y `Ini` no hace nada;
  (c) coordenadas ligeramente desviadas. Siguiente: probar `F11` (búsqueda) por
  `press_key` y/o confirmar por SQL si la tabla de series está vacía.

## Feedback para la beta de pc-control

1. Documentar el caso **target High-IL**: el error UIPI solo sale al hacer
   clic; `get_window_state` con daemon Medium devuelve 0 elementos sin avisar
   (parece "sin UIA" cuando es falta de integridad). Proponer: avisar IL en el
   snapshot o soportar daemon elevado oficial.
2. `click` con `capture_id` de un solo uso + `background_unavailable` que
   obliga a reintentar con capture nueva: el ciclo consume 2 snapshots por
   clic. Considerar no consumir la captura cuando el intento no se ejecuta.
3. `foreground_unavailable` con `actual foreground HWND 0x0`: informar qué
   significa (¿sin ventana en foco? ¿swap a medias?) para poder reaccionar.

## 2026-10-01 ~16:15 — Ventana AVISOS modal bloquea P0 (pendiente cierre)
- CONFIGURACIÓN DE AVISOS DE ACCIONES [1639888] es MODAL: MENÚ GENERAL [592486] queda nabled=False; su proveedor UIA no responde (PARTIAL TREE 0 nodos); invoke_menu → menu_path_unavailable.
- Cierre intentado SIN efecto (coords verificadas con cursor real, mapping screen=(510+1.61*cap_x, 281+1.62*cap_y)):
  X esquina (754,10)→(1724,298); botón ESC toolbar (600,39)→(1476,345); clic grid + X; Escape bg + fg x2; Alt+F4 fg; Ctrl+F4 fg.
- Proceso vivo (Responding=True, 41 hilos), ventana fg + enabled. O el form veta el cierre o el input no llega al loop.
- Driver CAMBIÓ a mitad de sesión: invoke_element/	ake_window_snapshot → Permission denied: no reviewed risk classification; hay invoke_menu (bloqueado por modal); click exige capture_id de un solo uso; press_key/hotkey aceptan delivery_mode:foreground (SendInput real, mueve cursor de verdad).
- Calibración útil: get_cursor_position antes/después de clic fg revela el transform exacto captura→pantalla.

## LECCIÓN 2026-10-01 — Cerrar ventanas BDP: cancelar la operación primero
- Una ventana de configuración BDP con una operación/edición en curso (fila nueva '*' en el grid, editor abierto) IGNORA X, ESC, Alt+F4 y Ctrl+F4 hasta cancelar la operación. No es fallo de puntería del clic: es estado de la ventana.
- Protocolo antes de cerrar cualquier ventana BDP: 1) mirar si hay fila '*' o celda en edición; 2) Escape para cancelar la edición (sin guardar nada); 3) verificar que no queda prompt; 4) recién entonces X / botón ESC de la toolbar.
- No insistir con más clics/teclas de cierre si la ventana no responde: revisar estado de edición primero.

## P0 — Parámetros de Exportación (CONFIGURACIÓN DE EXPORTACIÓN DE DATOS)
- Sin perfil configurado: Código vacío, Descripción vacía, Terminal Inicial/Final vacíos, Prefijo vacío.
- Ruta Exportación por defecto: C:\Bdp-Net\NetHos\Temp. Tipo Exportación sin marcar (Texto/XML/WebLink).
- Datos Venta / Ficheros / Stock: todo desmarcado. Opciones: solo 'Incluir Serie' (desmarcado).
- Hay pestaña inferior 'Configuración Exportación Movimientos Stock' (no abierta).
- Implicación T5: hay que CREAR un perfil de exportación (código+formato+ruta) para integración por ficheros; reversible (alta de parámetro, sin tocar ventas).

## 2026-10-01 tarde - Cierres con PC quieto + cambios del driver
- Con el PC quieto TODO funciono: Consulta/Formas Pago cerradas (Salir + Escape),
  Terminales y Empleados x Terminal recorridos enteros con F11/Return/Escape.
- Driver volvio a cambiar a mitad de sesion:
  - \get_window_state\ ya NO acepta \eturn_image\ (invalid_arguments); la captura
    viene por defecto en \screenshot_png_b64\ (+ \capture_id\, \window_bounds\).
  - \click\ exige ahora \window_id\ ademas de pid+capture (ambiguous_window_target si falta).
  - Las capturas caducan en ~1-2 min (capture_expired): capturar y clicar en el mismo minuto.
  - \	ool-schema\ tambien da Permission denied (sin clasificacion de riesgo).
- Recetas que funcionan: \invoke_menu\ en Start-Job + \list_windows\ a los 25 s
  (abre aunque la llamada no retorne); F11 abre Consulta, Return equivale a
  Seleccionar, Escape en Consulta cierra en cascada (cierra tambien el form padre).
- Ojo: clic en 'Exportar a Excel' abre 'Guardar como' (cancelar con Escape, sin guardar).
- Escape sobre form BDP abre su Consulta (no cierra); segundo Escape cierra ambas.
- Clics en pestanas del TabControl (Facturas/Parametros) no conmutan ni con Ctrl+Tab;
  pendiente otra via. Los botones Ini/Ant/Sig/Ult SI responden al clic.
- Sobre formulario vacio, Ant/Sig/Ult no hacen nada (no es fallo de punteria):
  ir por F11-Consulta para cargar el registro existente.

## P0 - Parametros del Terminal (TERMINALES [62]) y Empleados x Terminal [63]
- Terminales: solo \1 TERMINAL 1\. Generalidades: sin bloqueo, cierre 0,
  Local 1 (LOCAL 1), Almacen salida 1 (ALMACEN GENERAL), comandas ext 0,
  motivo regul. 0, impresoras 0, monitor off, avisos sin marcar.
- Empleados x Terminal: Terminal 1, modo 'Solo los siguientes': 1 Camarero 01,
  2 Camarero 02, 3 Camarero 03. Orden por codigo.
- Perfiles de exportacion [256]: ninguno (codigo vacio). P0 COMPLETO.

## LECCION 2026-10-01 - Botones F1-F12 de la toolbar (aviso del usuario)
- Los formularios BDP se manejan con los botones F de la barra superior
  (F2 Nuevo, F3 Guardar, F4..., F11 Consulta, ESC Salir): USARLOS siempre en
  vez de buscar atajos con clic. Se olvidaban y se intentaba todo con
  coordenadas. F11 abre Consulta, Return equivale a Seleccionar, Escape en
  Consulta cierra en cascada (tambien el form padre). Escape sobre un form
  abre su Consulta; segundo Escape cierra ambas.

## LECCION 2026-10-01 - Precision con objetivos pequenos: metodo de la lupa
- El fallo en el radio de tipo de serie no era entrega (el input SI llega:
  letras escritas y otros clics funcionaron) sino punteria: la captura se ve
  reducida y el error de estimacion (3-4 px) supera el tamano del objetivo
  (~10 px). El mapeo captura->pantalla del driver es bueno (~1 px).
- Metodo: recortar la zona (System.Drawing Clone del PNG), ampliarla 3-4x,
  elegir el punto en grande y traducir: pto_original = origen_recorte + punto.
- Regla: objetivos grandes a ojo; objetivos <20 px SIEMPRE con lupa.
- Alternativa sin clic: teclado (Tab hasta el grupo + flechas + Espacio),
  que ya demostro entrega perfecta (F11/Return/Escape/letras).

## Sintaxis CLI cua-driver 0.31.0 (fijada 2026-10-02)
- Invocacion: `cua-driver --socket <pipe> call <tool> '<json>'`
  (pipes: `\\.\pipe\opencode-pc` medio, `\\.\pipe\opencode-pc-admin` elevado).
  PS 5.1: sin `-NoNewline`; JSON multiclave por fichero (`Out-File ascii` +
  `Get-Content -Raw` + `.Trim()`). `describe <tool>` da el esquema real.
- `invoke_menu` usa `"path"` = array de ETIQUETAS (no indices):
  `{"pid":..,"window_id":..,"path":["Tpv-Táctil"]}`. Es fire-and-forget
  (puede colgar 45-90 s): lanzar con `Start-Process` + `WaitForExit(45000)`
  + kill y verificar con `list_windows`.
- `press_key` NO acepta `capture_id` (solo click/drag). `get_window_state`
  devuelve captura por defecto (`screenshot_png_b64` + `capture_id` un uso).
- Herramientas nuevas utiles: `type_text` (tecleo real, sustituye a `type`
  denegado), `zoom` (lupa nativa), `set_value` (ValuePattern UIA).
- `click` exige `window_id` + `capture_id` fresco; `background_unavailable`
  -> reintentar con `"delivery_mode":"foreground"` (mueve el cursor real:
  PC quieta). Error tipico de JSON roto: `capture_...56ture_id` (el
  `-replace` toco la clave `capture_id`: construir el JSON por
  concatenacion, no por reemplazo).

## P1 - Tpv-Tactil y ticket de prueba (2026-10-02)
- `invoke_menu ["Tpv-Táctil"]` abre `APERTURA Y SELECCIÓN DE TURNOS`
  (window_id 2231006, 718x526): Terminal 1, fecha hoy, Turno 1 precargados.
- El clic en Aceptar por coordenadas fallo (muy alto, zona gris vacia);
  `press_key return` era la via pero no se llego a probar: el usuario pulso
  Aceptar a mano y entro como Camarero 01. Flujo manual: CAFÉS -> articulo ->
  Total -> Efectivo 0.75 -> Cerrar Sin Imprimir. Sin dialogo de serie.
- Barra de estado tras cerrar: `00001TB00000001` = serie+numero del doc.
- SQL de la sesion: instancia `.\BDPSOFTWARE` (servicios `MSSQL$BDPSOFTWARE`,
  `MSSQL$SQLEXPRESS` activos); `SELECT * FROM SeriesTpv` confirma guardados.
