# Problema de instalación SQL del entorno de prueba BDP (2026-09-30)

## Resumen
El `Setup.exe` de BDP 36.2 no deja avanzar porque su COMPROBACIÓN PREVIA no
reconoce un SQL Server instalado a mano. Su SQL incluido (2012) no se puede
instalar en Windows 11, la descarga del SQL 2019 está retirada por Microsoft y
la red de este PC no llega al servidor de descargas con la DNS actual. Se logró
instalar SQL 2025 Express con instancia `BDPSOFTWARE`, pero el checker de BDP
la lee como versión 0 y la rechaza. Investigación abierta (ver F11).

## Cronología con evidencia
- **F1 — Prerrequisito incluido falla.** `Setup.exe` del paquete 37.5c intenta
  instalar su SQL Express y devuelve `ERROR CODE: -2067529714`. El primer
  intento silencioso ni siquiera arrancó (sin `Setup Bootstrap\Log`,
  `Elevado=False`, posible UAC oculto).
- **F2 — Media ESN incompatible con el idioma.** Instalador directo
  `SQLEXPR_x64_ESN.exe` (273 MB) → error: *"This SQL Server setup media does
  not support the language of the OS…"*. El sistema es `es-VE` con cultura
  `es-MX` y la media ESN no lo acepta (captura 30/09). El display language del
  SO es English (US) según el log del setup 2012.
- **F3 — Con SQL instalado, BDP sigue pidiéndolo.** Con SQL 2019 ya corriendo,
  la COMPROBACIÓN PREVIA marca `SQL Server Express 2019` como pendiente (radio
  sin check) y bloquea con *"ANTES DE CONTINUAR, SE DEBEN INSTALAR Y
  CONFIGURAR LOS ELEMENTOS QUE FALTAN"* (captura 30/09). El checker de BDP
  (`BdpSystemChecker.exe` / `BdpCheck.dll`: lee `Instance Names\SQL` + `TcpPort`)
  no reconoce un SQL instalado fuera de su flujo.
- **F4 — Crear `BDPSOFTWARECF` por comandos falla.**
  `SETUP.EXE /QS … /INSTANCENAME=BDPSOFTWARECF` → `Summary … Final result:
  Failed`, `Exit code: -2147467261`, `error code: 16387`,
  `ValidateFeatureSettings System.ArgumentNullException`
  (`C:\Program Files\Microsoft SQL Server\150\Setup Bootstrap\Log\20260923_144515\`,
  `Detail.txt`; SETUP colgado pid 10112).
- **F5 — Solución manual (la que funciona para 37.5).** Instancia
  `BDPSOFTWARECF` (SQL 2019, media ENU), `sa` / `BdpTest2026!`, `LoginMode=2`,
  `TcpPort=60012`, `SQLBrowser` en marcha, `RESTORE` de `Estructura.bak` →
  `TpvNet` y después de `Dtp-1.bk` (datos de prueba: terminal 1, 421 artículos,
  3 camareros), login `BdpNetAppUser` sysadmin (password extraída de
  `BdpTask.dll`, clase `ConexBdp`). Verificado 30/09: servicios `Running`,
  registro 64-bit con ambas instancias, `app-login OK articulos=421`.
- **F6 — La media x86 del paquete 36.2 es SQL 2012 SP1, no 2019.**
  `SQLEXPR_x86_ENU.exe` = versión `11.0.3000.0`. El botón `ACTIVAR
  PRERREQUISITOS` lanza ese instalador (log `110\Setup
  Bootstrap\Log\20260930_095938\SystemConfigurationCheck_Report.htm`: todo
  `Correcto` salvo `RebootRequiredCheck` en `Error`). El reinicio pendiente era
  real (`WU_RebootRequired=True`, `CBS_RebootPending=True`,
  `PendingFileRename=False`); un reinicio no bastó (servicing a medias), el
  segundo tras Windows Update sí. Corrección a lo dicho antes: no era 2019 x86.
- **F7 — SQL 2012 imposible en Windows 11 (callejón sin salida).** El setup
  exige activar la característica `MicrosoftWindowsPowerShellV2` y falla:
  `Error Code: -2146498548` / `0x85940001`, `Final result: Failed`, `Exit code:
  -2053898239`, `Database Engine Services` + `Replication` en `Failed` (log
  `110\…\20260930_103312\`, `SQLCOLLATION Modern_Spanish_CI_AS`,
  `INSTANCENAME BDPSOFTWARE`). PowerShell 2.0 fue eliminado de Windows 11: ni
  el usuario ni el distribuidor el lunes podrán instalarlo. Ventana cerrada con
  `Close`; no deja restos útiles (`Reg_BDPSOFTWARE` vacío).
- **F8 — Sin red al servidor de descargas + 2019 retirado.** `HEAD` y BITS a
  `download.microsoft.com:443` dan timeout. DNS del ISP y `8.8.8.8` resuelven a
  `23.2.70.49` (TCP 443 y 80 muertos); sin ruta IPv6; otros hosts MS bien
  (`aka.ms` 301 OK, `www.microsoft.com` 200). Editar `hosts` da `Acceso
  denegado` (antivirus). `curl --resolve` sí conecta, pero la URL recordada del
  2019 ENU x64 devuelve **404** y la página oficial
  `details.aspx?id=101064` dice *"this download is no longer available"*:
  Microsoft retiró el Express 2019 (ahora sirve 2025 vía `aka.ms/sqlexpress`).
- **F9 — Salida: DNS `1.1.1.1` + Express 2025.** Solo `1.1.1.1` resuelve
  `download.microsoft.com` a un nodo vivo (`23.44.84.137`, luego
  `23.56.178.46`). Comandos (admin): fijar con `Get-NetAdapter -Physical |
  Where-Object Status -eq 'Up' | ForEach-Object { Set-DnsClientServerAddress
  -InterfaceIndex $_.ifIndex -ServerAddresses 1.1.1.1 }`, revertir con el mismo
  pero `-ResetServerAddresses`. Descargado `SQLExpr_ENU.exe` (4 606 792 bytes,
  arrancador web) versión `17.0.1000.9` = **SQL Server 2025 Express**, firma
  `Valid`, `CN=Microsoft Corporation`. Guardado en
  `C:\Users\Public\SoporteBDP\SQLExpr_ENU.exe`.
- **F10 — Instancia `BDPSOFTWARE` creada en SQL 2025.** Instalador `Custom`
  (no `Basic`), `Nueva instalación`, instancia denominada `BDPSOFTWARE`,
  `Modo mixto` (`sa` = `BdpTest2026!`), usuario actual como admin SQL,
  casilla `Azure Extension for SQL Server` desmarcada, `Server Configuration`
  por defecto. Servicio `MSSQL$BDPSOFTWARE` = `Running`, `Automatic`.
- **F11 — El checker 36.2 rechaza SQL 2025: la lee como versión 0 (ABIERTO).**
  Al pulsar `SYSTEM CHECK`: `¡ATENCIÓN! LA VERSIÓN DE LA INSTANCIA SQL
  PREVIAMENTE INSTALADA ES LA 0. LA VERSIÓN MÍNIMA COMPATIBLE ES LA 2012` y la
  COMPROBACIÓN PREVIA marca en rojo `Compatibilidad de instancias SQL previas`
  (capturas 30/09). El checker no entiende el motor v17. Evidencia hasta ahora:
  servicio `Running`; lectura del registro `…\MSSQL17.BDPSOFTWARE\Setup`
  (Version/Edition) vacía — pendiente confirmar ruta; prueba de conexión
   `SqlClient` a `WAN\BDPSOFTWARE` con `sa`: **la conexión abre bien** y el
   re-test con comillas correctas lee `ProductVersion=17.0.1000.7`,
   `ProductLevel=RTM`, `EngineEdition=4` — la conectividad NO es el problema.
   Inspección unicode de `BdpCheck.dll`: el checker resuelve la instancia vía
   `Instance Names\SQL`, lee `TcpPort` en `…\<instancia>\MSSQLServer\
   SuperSocketNetLib\Tcp\IPAll`, y valida la versión contra
   `…\MSSQLServer\CurrentVersion` / WMI `root\Microsoft\SqlServer\
   ComputerManagement{10,11,15}` con una tabla de versiones conocidas
   **`9..15`** (hasta SQL 2019). El motor v17 (2025) cae fuera de la tabla →
   versión 0 → error `MÍNIMA COMPATIBLE ES LA 2012`. Registro 2025
   verificado: `Instance Names\SQL\BDPSOFTWARE = MSSQL17.BDPSOFTWARE`,
   `Setup Version=17.0.1000.7 Express Edition, Language=1033`. Conclusión:
   BDP 36.2 exige motor ≤2019 (v15); 2022 (v16) tampoco pasaría.

## F12 30/09 tarde: media SQL 2019 ENU recuperada vía fwlink oficial
- La página de descarga 2019 está retirada (`details.aspx?id=101064`:
  `no longer available`), el 2017 también (winget
  `Microsoft.SQLServer.2017.Express` → URL `download.microsoft.com/…
  /SQLServer2017-SSEI-Expr.exe` → HTTP 400), y un GUID inventado de 2019 da
  404. Pero el fwlink **`https://go.microsoft.com/fwlink/?linkid=866658`**
  sigue vivo → `SQL2019-SSEI-Expr.exe` (5841264 bytes, firma `Valid`,
  `CN=Microsoft Corporation`).
- `SQL2019-SSEI-Expr.exe /ACTION=Download /MEDIAPATH=… /ENU /QUIET`
  (requiere UAC) descargó **273 MB, exit 0**… pero en español:
  `SQLEXPR_x64_ESN.exe` (firma válida) porque el SSEI elige idioma por locale
  del SO (`es-VE`/`es-MX`) e ignora `/ENU` para la media. ESN es la que falla
  con `does not support the language of the OS` (display en-US). Hay que
  repetir con `/LANGUAGE=en-US`.
- Segundo intento silencioso con `/LANGUAGE=en-US` cancelado por el usuario
  en el UAC (correcto: a partir de aquí cada paso con elevación pide
  aprobación explícita). Tercer intento silencioso también fallido (arrancó
  con UAC aprobado, murió a los 44 MB sin mensaje: el modo `/QUIET` no traga
  `/LANGUAGE`).
- Solución: modo visual. El usuario ejecutó el SSEI sin parámetros,
  `Download Media` → idioma **English**, paquete **Express Core**, destino
  `C:\Users\Public\SoporteBDP\SQL2019MediaENU` → **`SQLEXPR_x64_ENU.exe`,
  268090448 bytes, firma `Valid`**, verificado. La media ESN queda en
  `SQL2019Media\` (inservible, pendiente borrar).
- Desinstalación 2025 `BDPSOFTWARE` ejecutada por el usuario
  (`170\Setup Bootstrap\SQL2025\setup.exe /QS /ACTION=Uninstall
  /INSTANCENAME=BDPSOFTWARE /FEATURES=SQLENGINE`): verificada —
  sin servicio `MSSQL$BDPSOFTWARE`, sin valor en `Instance Names\SQL`;
  `BDPSOFTWARECF` y `SQLEXPRESS` intactas en `Running`.
- Instalación 2019 ENU como `BDPSOFTWARE` **OK** (`/QS` se cierra solo al
  terminar): servicio `MSSQL$BDPSOFTWARE` `Running Automatic`, registro
  `Instance Names\SQL\BDPSOFTWARE = MSSQL15.BDPSOFTWARE`, `sa`/`BdpTest2026!`
  conecta y da `ProductVersion=15.0.2000.5` (v15 ∈ tabla 9..15 del checker).
  Pendiente: re-ejecutar COMPROBACIÓN PREVIA 36.2.
- BDP 36.2 instalado en `C:\Bdp-Net` (`TpvNet.exe` 4,8 MB 10/06/2025,
  `Estructura.ini 36|711`; sin iconos: accesos `TPV BDP 36.lnk` creados en
  Owner + SoporteBDP + Público). `BDPSOFTWARE` aún sin base BDP (solo
  system DBs). Primer arranque da `ERROR: NO SE PUEDE CONECTAR CON EL
  SERVIDOR SQL` (igual que la primera vez con 37.5).
- Diagnóstico: `xp_readerrorlog` muestra que el TPV SÍ llega al servidor e
  intenta entrar como **`sa` con password incorrecto** (un intento,
  `<local machine>`). El puerto es dinámico (`TcpDynamicPorts=55609`,
  `TcpPort` vacío) pero la resolución local funciona. Defaults de diseño en
  las DLL (`Data Source=DissenySQL2005\DISENYBDP`, `programacio/qwe`) no son
  los de runtime. `TpvNet.exe.config` sin connection string; `idp-1.ini`
  vacío (0 bytes); sin claves BDP en registro; `BdpTask.dll` del paquete
  solo fija `NombreInstanciaBDPSOFTWARE`. Falta saber qué password `sa`
  espera el TPV (probablemente la que pidió el instalador) y crear la base
  `TpvNet` desde `DatProg\Tpv-Net\Datosgen\Estructura.bak`.

## Hallazgo 30/09 tarde: cada versión espera su propia instancia
El checker de la **36.2** busca literalmente la instancia **`BDPSOFTWARE`**
(`"Instancia SQL BDPSOFTWARE: Existe=… Activo=…"`, servicio `Mssql$…`). La
**37.5** esperaba `BDPSOFTWARECF`. La instancia existe desde F10, pero su
versión no pasa el filtro (F11).

## F13 30/09 noche: TPV 36.2 conecta; `Usar Subscripción` rechaza credencial válida
- Clave `sa` del instalador 36.2 encontrada en `BdpTask.dll` (19456 bytes):
  línea de setup con `/SECURITYMODE=SQL /SAPWD=x35mn1k2` + `IPALL BDPSOFTWARE
  60011`. `sa`/`x35mn1k2` verificado OK contra `WAN\BDPSOFTWARE` →
  `ProductVersion=15.0.2000.5`. Puerto de diseño: `60011` estático (el instalado
  quedó en dinámico `55609`, pero la resolución local funciona).
- `RESTORE` ejecutados: `TpvNet` desde `Estructura.bak` (`db=TpvNet`,
  `version=706`, fecha 04/24/2025; `Terminales=0`, `Articulos=0`) y
  `[Kdi-Net_Tpv-Net_36]` desde `Dtp-1.bk` (demo: `Terminales=1`,
  `Articulos=421`, `Camareros=3`). `RESTORE OK` ambos.
- El TPV arrancó y a las 17:59 creó su base de trabajo
  **`Bdp-Net_NetHos_36`** (`ChkRes.log`: "Base de Datos Bdp-Net_NetHos_36
  restaurada"). La parte SQL queda cerrada.
- `Usar Subscripción` (`122417`, Cliente `GUILLERMO GARCIA SAEZ`, ID
  `guillermogarciasaez`) → `USUARIO O PASSWORD INCORRECTOS (4)` persistente.
  La ayuda del diálogo (`AyudaHOS.chm` decompilado con `hh.exe -decompile` a
  `C:\tmp\ayuda`) solo dice: "En caso de haber adquirido una licencia de
  suscripción, mediante este apartado se permitirá usar dicha licencia".
- Análisis estático agotado: binarios ofuscados (strings de lógica cifrados;
  solo nombres de controles WinForms + `Traducs.dll` con los literales de UI).
  Sin tabla local de licencias en ninguna de las 3 bases → validación remota.
  Form: `FormsUtilidades.dll` → `FormPublicarCustomerFood_Identificacion`
  (`IDENTIFICACIÓN DE DISTRIBUIDOR`, `txtLogin` + `@bdpuser.com`,
  `cmdAceptar`, `lblProceso PROCESANDO ...`).
- Captura de red (`C:\tmp\cap2.ps1`, 5 min, conexiones TCP + diff DNS):
  `TpvNet.exe` (pid 22940, iniciado 18:05) mantiene TLS permanente contra
  **`52.169.188.236:443`** (Azure). Certificado con SNI =
  **`CN=portal.bdpcenter.com`**; `portal.bdpcenter.com` resuelve a esa IP.
  Conclusión: al pulsar USAR SUBSCRIPCIÓN el TPV valida contra el portal por
  su canal ya abierto (sin conexión nueva visible).
- **Causa: incompatibilidad lado BDP, no error de escritura.** La misma
  credencial de distribuidor entra a `portal.bdpcenter.com` en el navegador
  (login Microsoft Entra) pero el TPV 36.2 envía usuario+password clásico y
  recibe `(4)`: el portal migró a Entra y el esquema legacy del 36.2 ya no
  autentica esa cuenta. Descartados: sufijo `-84` (el campo N.º solo admite 6
  dígitos), la Ñ (probado sin acentos), y credencial caducada (válida en web).
- Acción requerida (distribuidor lunes o `bdp@bdpcenter.com`): liberar/activar
  `122417` y `751080-84` en el portal (hoy `Activa: No`, sin equipo ni
  cliente) y confirmar procedimiento de activación para TPV 36.2 con portal
  en Entra.

## F14 01/10 mañana: clave de licenciamiento separada; activación OK pero no se queda (MAC puenteada)
- BDP (vía Guillermo, chat 01/10 7:11-7:12): el TPV **no pide la clave del
  portal sino una clave específica de licenciamiento**, que el distribuidor
  acabó de crear (igualada en valor a la del portal). Matiza F13: no es que
  Entra rompiera el login legacy, es que son dos credenciales distintas.
- Orden oficial (importante): 1.º `Utilidades → Control de licencias → Usar
  suscripción` con `122417` + Cliente `DESARROLLO ASLEY - LA ESPAÑOLA` + ID
  `guillermogarciasaez` (sin `@bdpuser.com`, lo añade el programa solo);
  2.º cuando active, `Suscripciones extendidas` con `751080` + código `84` +
  mismo ID y clave. Corrección: el `751080` nunca iba en la primera
  pantalla (el campo N.º solo admite 6 dígitos; el `-84` es el código de
  servicio de la segunda pantalla).
- `122417` activó a la primera: **`Subscripción adquirida correctamente`**
  (captura). Pero tras `Salir` el TPV sigue en `VERSIÓN DEMO` / `Licencia de
  Uso: (DEMO)`, y al reintentar sale `LA SUBSCRIPCIÓN YA ESTÁ EN USO EN OTRA
  MÁQUINA. ¿DESEA LIBERARLA…?` — la "otra máquina" es este mismo PC: la
  licencia se ata a la huella del equipo y el TPV no se reconoce a sí mismo.
- Causa visible al arrancar (captura, texto exacto): `ERROR DE
  IDENTIFICACIÓN DE MÁQUINA / ERROR ID MAC: MAC ADDRESS PUENTEADA` → el TPV
  cae a DEMO y ninguna activación se queda.
- Adaptadores (`Get-NetAdapter`): Wi-Fi Intel AX201 `40-1C-83-79-55-0E` física
  `Up`; `vEthernet (WSL Hyper-V)` + `vEthernet (Default Switch)` (Hyper-V,
  `00-15-5D-*`, `Up`); `Tailscale Tunnel` `Up`; `VirtualBox Host-Only`
  `0A-00-27-00-00-04` `Up`; `OpenVPN TAP-Windows6` + `Data Channel Offload`
  desconectados. Ruta por defecto por Wi-Fi (`192.168.0.1`).
- Deshabilitados por el usuario (`ncpa.cpl`) VirtualBox Host-Only + TAP:
  el error de MAC **persiste**. Las `vEthernet` no ofrecen Deshabilitar (las
  gestiona Hyper-V) y Tailscale se reactiva solo; quitarlas rompería
  Docker/WSL (`wsl --status`: distribución `docker-desktop` v2) y Tailnet.
- Sin puente real: sin adaptador `Multiplexor`/`Bridge`, Wi-Fi con MAC de
  fábrica estable (sin aleatoria ni override). Conclusión: el chequeo del
  36.2 marca "puenteada" por los adaptadores virtuales de Hyper-V; no se
  puede resolver a clics sin romper Docker/Tailscale.
- Vía abierta: llamada a BDP en directo (cierre 15:30/17:30) con el texto
  exacto del error + estos datos; pedir identificación correcta del equipo
  en 36.2 o reset de vinculación en su lado. No pulsar más
  usar-suscripción/liberar hasta entonces (cada ciclo cuenta).

## Situación actual (01/10 mañana, tras F14)
- `BDPSOFTWARE` 2019 (`MSSQL15.BDPSOFTWARE`, v15) con `sa`/`x35mn1k2`;
  `TpvNet` + `Kdi-Net_Tpv-Net_36` restauradas; TPV 36.2 corriendo con base
  `Bdp-Net_NetHos_36`. SQL: CERRADO.
- BDP 36.2 instalado en `C:\Bdp-Net`. DNS aún en `1.1.1.1` (revertir al
  cerrar). Media ESN `SQL2019Media\` pendiente borrar. VirtualBox Host-Only
  + TAP-Windows6 deshabilitados (reversibles en `ncpa.cpl`).
- `122417` adquirida OK pero **no se queda**: `ERROR ID MAC: MAC ADDRESS
  PUENTEADA` → DEMO. Bloqueado a la espera de BDP (llamada en directo hoy;
  cierran 15:30/17:30): identificación del equipo en 36.2 o reset de
  vinculación. Sin tocar `Suscripciones extendidas` (`751080`+`84`) hasta
  que la base aguante un reinicio.

## Rutas de evidencia
- `C:\Users\Owner\OneDrive\Desktop\bdpnet362` (paquete 36.2; `_PrerrequisitosV2\sqlexpress\SQLEXPR_x86_ENU.exe` = 2012 SP1 x86 11.0.3000)
- `C:\Program Files (x86)\Microsoft SQL Server\110\Setup Bootstrap\Log\20260930_095938\` (check OK salvo reboot) y `\20260930_103312\` (fallo PS 2.0)
- `C:\Program Files\Microsoft SQL Server\150\Setup Bootstrap\Log\20260923_144515\`
- `C:\Users\Public\SoporteBDP\` (AnyDesk, `crear-soporte.ps1`, `limpiar-bdp.ps1`, `SQLExpr_ENU.exe`, copias `Dtp-1.bk` + `Estructura.bak`)
- `C:\Users\Public\SoporteBDP\SQL2019-SSEI-Expr.exe` (bootstrapper 2019 firmado, fwlink `linkid=866658`) + `SQL2019Media\SQLEXPR_x64_ESN.exe` (273 MB, inservible) + `SQL2019MediaENU\SQLEXPR_x64_ENU.exe` (268 MB, firma válida)
- `C:\tmp\bdp-bak\Estructura.bak`, `C:\tmp\bdp-bak\Dtp-1.bk`
- `C:\tmp\fix-bdp-sql.ps1`, `C:\tmp\fix-bdp-port.ps1`, `C:\tmp\fix-bdp-login.ps1`
- `C:\tmp\cap2.ps1` + `C:\tmp\cap2.log` (TpvNet → 52.169.188.236:443 =
  portal.bdpcenter.com), `C:\tmp\cert1.ps1`, `C:\tmp\cert2.ps1`
  (CN=portal.bdpcenter.com con SNI), `C:\tmp\fu-str.txt` (strings extraíbles
  de `FormsUtilidades.dll`: solo formato/impresora, lógica ofuscada),
  `C:\tmp\ayuda\Ayuda_BDP\` (`Usar_Subscripción.htm`,
  `Liberar_Subscripción.htm` — ambos vacíos de detalle técnico)

## F15 01/10 tarde: fix registro + activacion completa (licencias OK)

- Causa raiz del `MAC ADDRESS PUENTEADA`: valor huerfano `NetworkAddress` en el
  registro (detalle y sessiones en `error-mac-puenteada-2026-10-01.md` §17-§19).
  Borrado con copia previa `C:\tmp\backup-NetworkAddress-0000.reg`; el TPV
  arranca limpio. (Nota: la conclusion F14 "chequeo 36.2 marca puenteada por
  los virtuales de Hyper-V" queda invalidada.)
- `122417` activada en Usar suscripcion (`Subscripcion adquirida correctamente`)
  y `751080` + codigo `84` en Suscripciones extendidas. Ambas se quedan tras
  `Salir`: fin del modo DEMO forzoso.
- Gotcha de la pantalla Suscripciones extendidas: el campo **ID. Distribuidor
  acepta el ID de usuario** (`guillermogarciasaez`), no el `751080` que viene
  prerrellenado; con `751080` + la clave de licenciamiento responde
  `USUARIO O PASSWORD INCORRECTOS (1)`. No insistir muchos intentos seguidos
  (el contador cuenta fallos; riesgo de bloqueo).
- La clave de licenciamiento **no** se registra en este documento (secreto; solo
  contexto de sesion). Actualizar `Situacion actual`: TPV 36.2 licenciado
  (`122417` + `751080-84`), base `Bdp-Net_NetHos_36`; desbloqueadas las pruebas
  que exigian suscripcion de pago.
