# Error ID MAC: MAC ADDRESS PUENTEADA (TPV 36.2) - 2026-10-01

## 0. Resumen ejecutivo

El TPV BDP-NET 36.2 muestra ERROR ID MAC: MAC ADDRESS PUENTEADA desde el primer arranque y cae a DEMO. Tras eliminar TODA la virtualizacion (Hyper-V desactivado, cero vEthernet, sin VirtualBox/TAP/Tailscale), sanear WMI y auditar servicios, el error sigue identico. No hay puente ni nada que lo parezca: el fallo esta en el chequeo de la 36.2 o en la vinculacion del servidor (suscripcion 122417, que responde que esta en uso en otra maquina habiendose usado solo aqui).

## 1. Entorno

- Equipo WAN: Surface Laptop 4, i7-1185G7, Windows 11 Pro 10.0.26200, serie 016975711357, UUID E97CD5EA-E8FA-A619-A5AB-7902C5E2FA3B.
- BDP-NET Hosteleria 36.2 (TpvNet.exe 10/06/2025). SQL Server 2019 instancia BDPSOFTWARE (15.0.2000.5), sa verificado.
- La base Bdp-Net_NetHos_36 la creo el propio TPV al arrancar (ChkRes.log 30/09/2026 17:59:08).
- Suscripcion de prueba 122417 (Hosteleria). Extendida 751080-84 sin tocar: el campo N. solo admite 6 digitos, va en Suscripciones extendidas.

## 2. Sintoma (secuencia exacta)

1. Arranque: ERROR DE IDENTIFICACION DE MAQUINA / ERROR ID MAC: MAC ADDRESS PUENTEADA. Cae a VERSION DEMO (Licencia de Uso: DEMO).
2. Utilidades - Control de licencias - Usar suscripcion, N. 122417, ID guillermogarciasaez: Subscripción adquirida correctamente.
3. Tras Salir y reentrar: sigue en DEMO. Reintento: LA SUBSCRIPCION YA ESTA EN USO EN OTRA MAQUINA. DESEA LIBERARLA. No se pulso liberar.

## 3. Conectividad con el servidor de licencias (verificada)

- TpvNet (pid 22940) mantiene TLS contra 52.169.188.236:443 = portal.bdpcenter.com. Certificado CN=portal.bdpcenter.com verificado (sin SNI responde CN=*.azurewebsites.net).
- DNS: bdpcenter.com 150.171.109.74, www 150.171.110.193, portal 52.169.188.236, licencias 150.171.109.118. Cache DNS operativa.

## 4. Comandos de soporte (salidas reales)

### 4.1 Todos los adaptadores (estado final)

    Wi-Fi (Intel AX201, 40-1C-83-79-55-0E): Up. Unico fisico, MAC estable.
    WAN Miniport IP/IPv6/Network Monitor: Up, sin MAC (estandar Windows). Resto de Miniport: Disconnected.
    Teredo/6to4/IP-HTTPS/Kernel Debug: Not Present. vEthernet*: ninguno (Hyper-V desactivado).

### 4.2 Fisicos: solo Wi-Fi (40-1C-83-79-55-0E, ifIndex 7).

### 4.3 Get-VMSwitch (admin, antes de desactivar Hyper-V): Default Switch y WSL (Hyper-V firewall), ambos Internal y sin adaptador ligado. Un puente seria External ligado al Wi-Fi. No habia puente.

### 4.4 Binding vms_pp: Disabled en TODOS los adaptadores, incluido el Wi-Fi.

## 5. Verificaciones de puente (todas negativas)

- Sin driver BridgeMP (clave inexistente). Sin binding ms_bridge en el Wi-Fi. Ruta por defecto solo por Wi-Fi (192.168.0.1).

## 6. Analisis del codigo (binarios, sin fuente) — revisado 2026-10-01

Nota metodologica: `findstr` no ve cadenas UTF-16LE; los negativos anteriores sacados solo con `findstr` eran falsos negativos. Lo fiable fue decodificar los binarios como Unicode (recursos) y ANSI-1252 (metadatos y textos UI).

- La licencia va atada a la MAC de la tarjeta de red elegida, leida por WMI: `CapaDatosProcesos.dll` contiene `Win32_NetworkAdapterConfiguration` + `IPEnabled` + `MacAddress` juntos en el blob de recursos Unicode, y `ManagementObjectSearcher` (`System.Management`) en metadatos, tambien referenciado en `TpvNet.exe`. `NetworkInterface` tambien aparece en `CapaDatosProcesos.dll` (via alternativa .NET). No se recupero el texto literal del SELECT ni la comparacion: analisis de cadenas, sin descompilador.
- El programa obliga a elegir tarjeta interna (`Traducs.dll`, ANSI-1252): `SELECCION DE LA TARJETA DE RED`, `ERROR: NO HA SELECCIONADO NINGUNA TARJETA DE RED`, `DEBE SELECCIONAR LA TARJETA DE RED INTERNA DE SU EQUIPO. EVITE AQUELLAS QUE SE PUEDEN EXTRAER (USB) O AQUELLAS QUE PUEDEN DESACTIVARSE`. En el mismo blob hay marcador `00:00:00:00:00:00` como valor por defecto.
- Correccion: SI hay consultas WMI en claro (antes se afirmo que no): ademas de la de red, `SELECT * from Win32_SystemEnclosure`, `SELECT * FROM Win32_OperatingSystem`, `SELECT * FROM WmiMonitorID`, `root\CIMV2`. Proposito exacto no atribuible solo con strings (pueden ser inventario o diagnostico, no licencia).
- Correccion: `PUENTEADA` no existe como literal en ningun binario. Los 5 `PUENTE*` de `Traducs.dll` son `FICHERO PUENTE` (traspaso de albaranes de compra/venta), sin relacion con la licencia. El texto `MAC ADDRESS PUENTEADA` se compone en tiempo de ejecucion o vive en otro modulo.
- `HUELLA` (12 casos) es solo huella dactilar de empleados/usuarios; nada que ver con huella de maquina.
- Catalogo de mensajes de subscripcion (`Traducs.dll`): `SUBSCRIPCION NO ENCONTRADA / ANULADA / SUSPENDIDA`, `MOTIVO SUSPENSION: MAS DE UNA SEMANA SIN CONEXION AL SERVIDOR`, `LA SUBSCRIPCION YA ESTA EN USO EN OTRA MAQUINA. DESEA LIBERARLA...?`, `SE HAN DETECTADO CAMBIOS EN LA SUBSCRIPCION (MODULOS NUEVOS...)`, `FECHA LIMITE DE USO`, `LICENCIA TEMPORAL / ANULADA`, liberar/actualizar subscripcion (normal y extendida).
- Sin marcador de ofuscador conocido en los binarios propios (`ConfusedBy`, Dotfuscator, SmartAssembly, Reactor, Crypto, Eazfuscator, Babel, NetGuard, `ObfuscatedByAttribute`: ausentes). `GetAllNetworkInterfaces`, `OperationalStatus` y lista negra de OUI no re-verificados en esta sesion (el barrido anterior con `findstr` no es concluyente en Unicode).

## 7. Barrido local (cronologico, error identico en cada paso)

1. Deshabilitar VirtualBox Host-Only, TAP-Windows, OpenVPN Offload, Wi-Fi Direct (2), Bluetooth PAN, Kernel Debug.
2. Detener Tailscale y Docker, wsl --shutdown.
3. Deshabilitar extensiones Hyper-V y vEthernet (Default Switch y WSL).
4. Desinstalar los dispositivos virtuales (VirtualBox, TAP, Offload, Wi-Fi Direct, extensiones Hyper-V, Tailscale).
5. Intentar eliminar el Default Switch: bloqueado por ICS incluso con SharedAccess deshabilitado y tras reiniciar. Su MAC cambio sola (36-1D-41 a 38-01-00): es dinamica.
6. Desactivar Hyper-V por completo (Disable-WindowsOptionalFeature) y reiniciar: cero vEthernet. El error SIGUE IGUAL.

## 8. Linea WMI (a peticion de BDP)

- winmgmt Running/Automatic. verifyrepository (admin): el repositorio es coherente. Hardware correcto (serie 016975711357, UUID E97CD5EA..., i7-1185G7, BIOS 33.108.143).
- PhysicalAdapter=True devolvia tambien Tailscale Tunnel habilitado SIN MAC. Tailscale desinstalado por completo: ahora solo sale el Intel Wi-Fi.
- Persistian dos fantasmas Wi-Fi Direct (40:1C:83:79:55:0F y 42:1C:83:79:55:0E) con MAC gemelas del fisico.
- winmgmt /resetrepository + reinicio NO los elimina: el driver Intel 24.30.1.1 (03/2026) los recrea al arrancar. Red hospedada no soportada, hotspot (icssvc) detenido.
- Disable-PnpDevice deshabilito el primero (el segundo dio error generico 0x80041001); los dos fantasmas quedaron SIN MAC en WMI. El TPV sigue igual.
- Pendiente opcional: pnputil /remove-device sobre VWIFIMP_WFD ...11 y ...12 (mas fuerte que desinstalar; el driver los recreara al reiniciar).

## 9. Auditoria de servicios (modificaciones previas de Windows: inocentes)

- En marcha: nsi, NlaSvc, Dhcp, Dnscache, WlanSvc, RasMan, Winmgmt, COM/DCOM/RPC, CryptSvc.
- Deshabilitados: telemetria, App-V, registro remoto, NetTcpPortSharing, RemoteAccess, RetailDemo, agentes SQL, ssh-agent y similares. Nada de red ni hardware.

## 10. Hallazgos

1. No hay puente en Windows por ninguna via medible (driver, bindings, conmutadores, ruta).
2. El error persiste sin ningun adaptador virtual presente: no puede ser causa local.
3. La MAC del Default Switch es dinamica por arranque: si la huella la incluye, cada reinicio es otra maquina.
4. La huella cambio con cada desinstalacion/reinicio, asi que la vinculacion del servidor (otra maquina) requiere reset en servidor en todo caso.
5. Parchear el binario o falsear identidad queda descartado: invalidaria la evidencia de fase D y compromete al distribuidor.
6. Nuevo 2026-10-01: la vinculacion es a la MAC de la tarjeta elegida (WMI `Win32_NetworkAdapterConfiguration` + `IPEnabled`). Cada cambio en las NIC —incluidas nuestras propias desinstalaciones y la MAC dinamica del Default Switch— es otra maquina para el servidor. Esto explica el `YA ESTA EN USO EN OTRA MAQUINA` sin haber usado la suscripcion en otro PC, y hace el reset en servidor imprescindible antes de reintentar.
7. Nuevo 2026-10-01 (descompilacion solo-lectura de copias en `C:\tmp\bdp-decompile`, clase licencia `0x020008B7` ofuscada): `0x06005740` lee por WMI la MAC del adaptador con `IPEnabled=true` (itera `GetInstances` y SOBRESCRIBE por cada coincidencia: con varios seria el ultimo; hoy hay uno solo), sin excluir virtuales; `0x06005711` la normaliza (`""`→`"00:00:00:00:00:00"`); `0x06005712` la combina con campos cacheados; `0x0600E24F/50/51` la envian al servidor con `ConexBdp::SetInfoLic`. El estado se guarda en campos estaticos (`0x04002960-62`) cargados por `0x060056F5` desde blobs cifrados, y `0x06005691` lo traduce a mensajes via `Traducs` segun codigos `1/2/3/5/6/7/8/9/12/19/Lite`. El texto literal `MAC ADDRESS PUENTEADA` NO existe en ningun binario ni en `Traducs.dll` (verificado `#US` + ANSI/Unicode): lo devuelve el servidor de licencias en el handshake WCF. `PUENTE*` local es solo contabilidad (`Cta. Puente`, `DsInformesPuente`).
8. Nuevo 2026-10-01 (arranque): `0x06005756` construye la conexion (`0x0600E24F/51` + `ConexBdp::CopiarParametros` + `InicializarConexion`), `0x06005757` compone el veredicto (`ToUpper` + `CompareString` contra `"TRUE"/"5"` + `Traducs.Traduc`, con `Exception.Message` y `Beep`) y lo muestra con `CapaUtil.zInterCU::MessageBoxOk`. No hay log en disco con la linea `ERROR ID MAC` (buscado en todo `C:\Bdp-Net` y `Logs/Temp`: solo existe en pantalla). WMI hoy devuelve UN solo adaptador con IP: Intel Wi-Fi 6 AX201 `40:1C:83:79:55:0E` (`192.168.0.127`): esa es la huella que envia el TPV ahora. El dialogo `Seleccionar Tarjeta de Red Preferente` no esta en `CapaDatosProcesos/TpvNet/FormsUtilidades/CapaUtil/FormsFrontOffice/FormsBasicos/FormsBackOffice/Perifericos/CapaDatos_Comunicaciones` (verificado `#US`).

## 11. Estado actual y restauracion pendiente

- Hyper-V desactivado, SharedAccess tocado, Tailscale desinstalado, dispositivos virtuales desinstalados, un fantasma WFD deshabilitado por hardware.
- Restaurar tras la activacion (no antes: cualquier NIC nueva cambia la huella): Enable-WindowsOptionalFeature Hyper-V + reinicio, SharedAccess a Manual, reinstalar Tailscale, arrancar Docker, reparar VirtualBox/TAP con su app.

## 12. Peticion a BDP (por escrito, SIN sesion remota)

1. Liberar la vinculacion de la suscripcion 122417 en el servidor y registrar la MAC actual `40:1C:83:79:55:0E` (Intel Wi-Fi 6 AX201, unica con IP hoy, huella estable verificada por WMI).
2. Confirmar por escrito que huella esperan (¿MAC del `IPEnabled` via WMI?) y como fijar la tarjeta interna en un PC con Hyper-V/WSL para activar la extendida 751080-84.
3. NO ejecutar `AnulaLic.exe` (verificado 2026-10-01: es `ANULAR LICENCIA`, cancela y deja sin programa; no es una liberacion reutilizable). `ModoSeguro.exe` es un stub sin bypass de licencia. No hay auto-activacion local en `TpvNet.exe`.

## 13. Nivel de confianza por afirmacion (evidencia → confianza)

- No hay puente en Windows: driver BridgeMP inexistente + `ms_bridge` Disabled en Wi-Fi + `Get-VMSwitch` sin External + ruta solo por Wi-Fi + cero vEthernet. → MUY ALTA (5 vias independientes).
- Huella = MAC del `IPEnabled` via WMI, sin excluir virtuales: IL completo de `0x06005740` (`ManagementClass("Win32_NetworkAdapterConfiguration")` → `GetInstances` → `IPEnabled` → `MacAddress`, bucle que sobrescribe, `On Error` → `""`). → ALTA.
- Normalizacion `""`→`00:00:00:00:00:00`: IL de `0x06005711` (`CompareString` + `ldstr`). → ALTA.
- La MAC se envia al servidor: cola `0x06005711/0D/12/09` → `ConexBdp::SetInfoLic` en `0x0600E24F`, TLS vivo contra `portal.bdpcenter.com:443`. → ALTA.
- `MAC ADDRESS PUENTEADA` lo devuelve el servidor: ausente en `#US` de 11 modulos (`CapaDatosProcesos`, `TpvNet`, `FormsUtilidades`, `CapaDatos`, `CapaDatos_Gestion`, `CapaUtil`, `FormsFrontOffice/Basicos/BackOffice`, `Perifericos`, `CapaDatos_Comunicaciones`, `AnulaLic`, `ModoSeguro`, `BdpCheck`) + `Traducs.dll` en Unicode y ANSI-1252 + sin log local en todo `C:\Bdp-Net`. → ALTA.
- Estado por codigos → mensajes `Traducs`: `0x06005691` compara campo `0x04002962` con `1/2/3/5/6/7/8/9/12/19/Lite` y llama a `Traducs.Traduc`. → ALTA.
- Cadena de arranque: `0x06005756` (conexion) → `0x06005757` (veredicto `ToUpper`/`CompareString`/`Traduc`/`Exception.Message`/`Beep`) → `MessageBoxOk`; unico llamador de `0x0600E24F` es `0x06005756`. → ALTA.
- Huella actual `40:1C:83:79:55:0E`: WMI directo dos veces (01/10/2026), un unico `IPEnabled`. → MUY ALTA.
- Dialogo de tarjeta preferente no localizado: `#US` negativo en 11 modulos; quedan ~100 DLL menores sin revisar. → MEDIA-ALTA (alcance limitado).
- Bloqueo en servidor, nada util en local: todo lo anterior + persistencia tras limpieza total + sin fichero local de bloqueo hallado. → ALTA (~90%). Reserva: no se intercepto la respuesta del servidor en vivo.

## 14. Otras posibilidades (consideradas)

1. Fichero/caché local de veredicto: buscado (`PUENTEADA` en `C:\Bdp-Net` <2MB, `Logs`, `Temp`, perfiles) sin exito. Sin evidencia; improbable, no excluido al 100%.
2. Catalogo `Traducs` satelite (`es-CL/`, `en/`, etc.) con el texto: solo se reviso el `Traducs.dll` neutro. Menor: el mensaje del usuario lleva datos dinamicos (version, serie), asi que hay composicion con datos del servidor en todo caso.
3. `GetAllNetworkInterfaces`/lista negra OUI como via paralela: `NetworkInterface` aparece en metadatos pero la ruta de licencia usa WMI (`0x06005740` verificado en IL). No excluido exhaustivamente.
4. Default Switch con MAC dinamica como desencadenante original: plausible y coherente (§7.5, §10.3): cada reinicio = otra maquina para el servidor.
5. Fantasmas Wi-Fi Direct como "otra maquina": DESCARTADO por el propio IL (filtro `IPEnabled`; sin IP ni MAC no entran).
6. Tailscale sin MAC: irrelevante hoy (desinstalado); en su dia `get_Item("MacAddress")`→null habria caido al `On Error` (`""`→`00:00...`).
7. Probar la 122417 en otro PC para comparar: PROHIBIDO, confirmaria `EN OTRA MAQUINA` y empeoraria el estado.
8. Interceptar el TLS para leer la respuesta: DESCARTADO (innecesario, rompe confianza, pinning probable).
9. `AnulaLic.exe` / `ModoSeguro.exe` como auto-solucion: verificados (`ANULAR LICENCIA` destructivo; stub). NO usar.
10. `pnputil /remove-device` de fantasmas WFD: opcional, irrelevante para el desbloqueo (no entran en huella).

## 15. Que faltaria para el 100%

- Respuesta literal del servidor (solo BDP la tiene en sus logs: pedirles el motivo exacto registrado para la 122417).
- Confirmacion escrita de que huella esperan y donde se fija la tarjeta preferente.
- Tras el reset: arrancar el TPV con la huella actual intacta y comprobar que sale de DEMO.

## 16. Ampliacion 2026-10-01 (sesion 2, SOLO LECTURA — sin cambios en el sistema)

### 16.1 Herramienta de analisis (reproducible)

- Desensamblador IL propio en `C:\tmp\bdp-decompile`:
  - `probe.cs` (ya existente): `DecodeToken`, `FindCallers`, `FindStringUsers`, `FindFieldUsers`.
  - `calls2.cs` + `list.ps1` (**nuevo**): imprime ademas el **token** de cada operando
    (`call tok=0x...`, `stsfld tok=0x...`), necesario porque los nombres propios del
    ensamblado estan ofuscados a `?`.
  - Compilar: `Add-Type -Path 'C:\tmp\bdp-decompile\calls2.cs' -ReferencedAssemblies @([System.Reflection.Metadata.MetadataReader].Assembly.Location, [System.Collections.Immutable.ImmutableArray].Assembly.Location)`
  - Uso: `& 'C:\tmp\bdp-decompile\list.ps1' -Path 'C:\tmp\bdp-decompile\CapaDatosProcesos.dll' -Tokens @('0x060056F5')`
- Gotchas verificados:
  - En PowerShell 7 el `PEReader` de `System.Reflection.Metadata.dll` **no expone
    `GetMethodBody`** (ni `GetMetadataReader` como metodo instanciado): hay que pasar por
    C# con `Add-Type`. `probe.cs` si compila.
  - El decoder de `probe.cs` imprime los destinos de salto de **1 byte** (`br.s`,
    `brfalse.s`, `leave.s`) **1 por debajo** del real; los de 4 bytes (`br`, `switch`)
    son correctos. `calls2.cs` ya los imprime bien.
  - Los nombres de tipos/miembros propios del modulo son `?` (ofuscados); solo resuelven
    los externos (`Traducs.Traducs::Traduc`, `System.Management.*`, `Microsoft.VisualBasic.*`).

### 16.2 Correcciones a secciones anteriores (evidencia nueva)

1. **§10.6/§10.7 y §13 ("lo devuelve el servidor" → ALTA) NO esta probado: bajar a MEDIA.**
   Si existe **encriptado de cadenas** en el binario. En la clase `0x020008B7`, el metodo
   `0x060056F5` hace: `newarr byte` → `ldtoken` → `RuntimeHelpers::InitializeArray` →
   `call 0x0600E3B5` (= `Encoding.GetString`, usa `ldsfld 0x0400580C`) → `call 0x0600573D`
   (decoder con control-flow aplanado) → `stsfld`. Todos los blobs son `ldtoken` =
   **constantes embebidas**, no ficheros. Por tanto "ausente en `#US`" **no demuestra** que
   el texto venga del servidor: un literal cifrado nunca aparece en `#US` ni en un barrido
   de bytes.
2. **§10.7 "itera GetInstances y SOBRESCRIBE (seria el ultimo)" es FALSO.** IL completo de
   `0x06005740`: ante `IPEnabled=true` hace `stloc.0` + `leave` → **coge la PRIMERA tarjeta
   con IP y sale del bucle**. Con varias, gana la primera del enumerado WMI.
3. **§10.7 "0x060056F5 carga estado desde blobs cifrados" es impreciso:** no hay E/S de
   ficheros ni de registro en ese metodo. Son constantes de compilacion descifradas en runtime.
4. **`0x06005712` decide entre MAC fresca y campo cacheado:** `call ?` (lector) → rama A
   `call ?; stloc.0`, rama B `ldsfld ?; stloc.0`. **Pendiente:** quien escribe ese `ldsfld`
   (candidata a "tarjeta de red preferente" que manda un MAC antiguo al servidor).
5. **`0x06005757` (veredicto) — mensajes posibles:**
   - via `Traducs.Traducs::Traduc` con codigos **13840, 13842, 13843, 13929, 13936, 13937,
     13938, 13939** (dos de ellos en rutas distintas del mismo metodo);
   - via **`System.Exception::get_Message`** (dos rutas: una dentro del `On Error`, otra en
     el manejo de fallo de la llamada) → ahi cabria el texto del servidor;
   - compara `a[2].ToUpper() == "TRUE"` y `a[8] == "5"` donde `a` es un array (respuesta
     del servidor partida en campos), y llama a `Interaction::Beep`.

### 16.3 Barrido de disco ampliado (2026-10-01 sesion 2) — cubre el hueco anterior

- Palabras buscadas: `PUENTEADA`, `ERROR ID MAC`, `MAC ADDRESS` (ASCII-1252 **y** UTF-16LE,
  ignore-case).
- Alcance: `C:\Bdp-Net` **completo**: 2.254 ficheros <5MB **+ 21 ficheros >=5MB** (hasta
  60MB). El barrido anterior (<2MB) **no cubria** `CapaDatosProcesos.dll` (20MB),
  `FormsBasicos.dll` (22MB), `CapaDatos.dll` (16MB), `FormsUtilidades.dll` (6MB).
- Resultado: **0 hits** en todo el arbol, incluidos los satelites
  `ca\Traducs.resources.dll` y `es-CL\Traducs.resources.dll`.
- Ayuda decompilada `C:\tmp\ayuda` (500 ficheros): **0 hits**.
- Consecuencia: el texto **no esta en claro en ninguna parte de la instalacion**. Solo quedan
  dos vias posibles: **(a) literal cifrado en un binario**, **(b) respuesta del servidor**.

### 16.4 WMI actual (2026-10-01 sesion 2, orden real)

- `Win32_NetworkAdapterConfiguration WHERE IPEnabled=True` → **exactamente uno**: Index 0,
  `Intel(R) Wi-Fi 6 AX201 160MHz`, `40:1C:83:79:55:0E`.
- Con MAC pero `IPEnabled=False`: WAN Miniport (IP) `2C:05:20:52:41:53`, (IPv6)
  `2E:C8:20:52:41:53`, (Network Monitor) `30:80:20:52:41:53`.
- → la huella enviada hoy es `40:1C:83:79:55:0E` y **no depende del orden** del enumerado.

### 16.5 Llamantes (mapa para el paso siguiente)

- `0x0600573D` (decoder de cadenas): **7 llamantes** — `0x06005690`, `0x060056F5`,
  `0x060056F6`, `0x06005702`, `0x0600572B`, `0x0600572C`, `0x0600573F` (todos en la misma
  zona de licencia, clase `0x020008B7` y vecinas).
- `0x06005740` (lector WMI de MAC): **un solo llamante**, `0x06005711`.
- `0x0600E3B5` (`Encoding.GetString`): 18 llamantes, incluidos dos `?::EjecutarProceso`.

### 16.6 Estado y siguiente paso (pendiente de decision del usuario)

- Investigacion **pausada aqui**; no se ha tocado nada del TPV, ni registro, ni ficheros de
  la instalacion, ni se ha lanzado el programa.
- Opciones abiertas (orden de recomendacion):
  1. **Reimplementar `0x0600573D` y descifrar los literales de la zona de licencia** →
     decide de una vez si `MAC ADDRESS PUENTEADA` es local o remoto. Si es local, hay
     solucion **sin BDP**.
  2. **Prueba en vivo barata**: arrancar el TPV con la red caida. Mismo mensaje → local;
     mensaje distinto → servidor (ahi si haria falta BDP).
  3. Candidato no explorado a solucion sin BDP: contestar **"Si"** al dialogo
     `LA SUBSCRIPCION YA ESTA EN USO EN OTRA MAQUINA. DESEA LIBERARLA...?` (auto-desvinculacion
     en servidor). El informe no lo llego a pulsar; valorar riesgo antes de usarlo.
- Ver tambien `§12` (peticion a BDP): sigue siendo la via oficial si 1 o 2 confirman lado
  servidor.

## 17. RESUELTO (2026-10-01 sesion 3, SOLO LECTURA — sin ningun cambio aplicado)

### 17.1 Resultado

**El mensaje `ERROR ID MAC: MAC ADDRESS PUENTEADA` es LOCAL. NO viene del servidor.**
Se genera entero en `FormsBasicos.dll` y su causa en esta maquina esta identificada
(pasos 17.3/17.4). La §10.6/§10.7 y la entrada ALTA de §13 quedan **invalidadas**.

### 17.2 Como se descifro (herramienta nueva, reproducible)

Los literales del TPV no estan en claro: cada string va como array de bytes (`ldtoken`) que
se descifra en runtime con `0x0600573D` (en `CapaDatosProcesos.dll`) / `0x06006FD2` (en
`FormsBasicos.dll`).

Algoritmo reconstruido (validado: produce texto legible coherente):

```text
D(bytes):
  n     = (len-2)/2               # 0 si len<2
  size  = n+1
  for i in 0..size-1: l2[i]=bytes[2i]; l1[i]=bytes[2i+1]
  Array.Reverse(l1)
  for i in 0..size-1: l1[i] ^= l2[i]
  return size==1 && l1[0]==0 ? "" : UTF8.GetString(l1)
```

(El paso previo `0x0600E3B5` es identidad para estos blobs: basta `D` sobre los bytes crudos.)

Herramientas nuevas en `C:\tmp\bdp-decompile`:
- `blobs.cs` → `[Blobs]::GetFieldData(path, token)` (bytes de un FieldRVA) y
  `ListFieldRvas(path)` (**el length hay que tomarlo como `siguiente RVA - RVA propia`**: las
  firmas de campo de este ensamblado no son `SZARRAY`, estan ofuscadas a `VALUETYPE`).
- `dec.cs` → `[Dec]::A(bytes)` = el `D()` de arriba.
- `decode-all.ps1` → vuelca **todos** los FieldRVA de `C:\Bdp-Net\NetHos\Prog` a
  `C:\tmp\bdp-decompile\decrypted.txt` (`dll|token|len|texto`) y busca palabras clave.
- Correccion respecto al sesion anterior: si el array es de 1 byte la condicion de salida del
  `D` (a diferencia de lo anotado en 16.2) devuelve cadena vacia.

### 17.3 La cadena exacta del error (`FormsBasicos.dll`, metodo `0x06006FC9`, rva 0x4D9340)

```text
1) 0x06006FCA()  -> adaptador Win32_NetworkAdapter (SELECT * FROM Win32_NetworkAdapter
                    WHERE NOT PNPDeviceID LIKE 'ROOT\%')
   null          -> "ERROR ID MAC: NO HAY DISPOSITIVO DE RED"  (blob 0x040034E1)
2) loc2 = obj.GetPropertyValue("MACAddress").ToString().Replace(V5,"")   # V5 = separador
   loc3 = obj.GetPropertyValue("Index").ToString().Replace(V5,"")
   loc2=="" o loc3=="" -> "ID MAC: ERROR DE DETECCION DE RED"            (blob 0x04003536)
3) 0x06006FCE(loc3) -> HKLM\SYSTEM\CurrentControlSet\Control\Class\
                       {4D36E972-E325-11CE-BFC1-08002BE10318}\<loc3 con PadLeft(4,'0')>
   null -> "ID MAC: ERROR DE REGISTRO"                                   (blob 0x04003581)
4) 0x06006FCF(clave, "NetworkAddress") -> valor de la subclave
   SI NO ESTA VACIO -> "ERROR ID MAC: MAC ADDRESS PUENTEADA"             (blob 0x04003576)
   vacio/ausente    -> OK: *arg0 = loc2 (la MAC) y devuelve ""
5) excepcion        -> "EXCEPCION ID MAC: " + Exception.Message          (blob 0x0400355D)
```

- `0x06006FCF(clave, nombre)`: `Array.IndexOf(clave.GetValueNames(), nombre) < 0` → `""`;
  si existe → `clave.GetValue(nombre).ToString()`.
- `0x06006FCE(index)`: abre `LocalMachine` + rama anterior + `"\" + index.ToLower().PadLeft(4,'0')`.
- Decodificados (ver `decrypted.txt`) que aparecen en la ruta: `MACAddress`, `Index`,
  `NetworkAddress`, `IndiceTarjetaRed`, `\NetworkCard.txt`, `PhysicalAdapter`, `PNPDeviceID`,
  `Software\WmiUser`, `SYSTEM\CurrentControlSet\Control\Class\{4D36E972-...}`,
  `SELECT * FROM Win32_NetworkAdapter WHERE NOT PNPDeviceID LIKE 'ROOT\%'`.

### 17.4 Confirmacion en la maquina (2026-10-01, SOLO LECTURA)

- `HKLM\SYSTEM\CurrentControlSet\Control\Class\{4D36E972-E325-11CE-BFC1-08002BE10318}\0000`
  - `DriverDesc`            = `Intel(R) Wi-Fi 6 AX201 160MHz`
  - **`NetworkAddress`       = `8C4962AB7923`  ← CAUSA EXACTA del error**
  - `OriginalNetworkAddress` = `40-1C-83-79-55-0E` (MAC de fabrica)
  - `DriverVersion`         = `24.30.1.1`, `DriverDate` = `3-3-2026`
- Es **el unico** adaptador del arbol de clase con valor `NetworkAddress` definido
  (el resto ni siquiera tiene la clave).
- MAC efectiva en WMI/`Get-NetAdapter`: `40-1C-83-79-55-0E` → el driver **no esta aplicando**
  ese `NetworkAddress`; el valor es **stale/huerfano** (probablemente de otra herramienta o
  de un clonado de MAC previo). Pero al TPV le basta con que el valor este relleno para
  devolver PUENTEADA: **no mira si esta aplicado, solo si esta escrito**.
- `NetworkCard.txt` no existe bajo `C:\Bdp-Net`; `HKLM\SOFTWARE\WmiUser` y
  `HKCU\SOFTWARE\WmiUser` estan vacios/no existen.
- No se ha tocado nada: ni registro, ni red, ni la aplicacion; no se ha lanzado el TPV.

### 17.5 Solucion candidata SIN BDP (pendiente de autorizacion — NO aplicada)

1. Vaciar (o borrar) el valor `NetworkAddress` de la subclave `...\{4D36E972-...}\0000`.
   Riesgo: es una clave de sistema; conviene copia previa del valor y reinicio de la
   interfaz Wi-Fi (o del equipo) para que el driver relee.
2. Volver a lanzar el TPV y comprobar que desaparece el primer error.
3. Queda por determinar si el segundo (`LA SUBSCRIPCION YA ESTA EN USO EN OTRA MAQUINA`)
   depende del servidor o tambien es local: revisar `0x06005757` (compone veredicto con
   `Traduc(13840,13842,13843,13929,13936-13939)` y `Exception.Message`) y
   `ConexBdp::SetInfoLic` (`0x0600E24F/50/51`) para ver la MAC efectivamente enviada.

### 17.6 Leccion / prevencion

- En este producto "ausente de `#US` y de un barrido de bytes" **no prueba** que un texto
  venga del servidor: todo el texto de licencia va cifrado por bloques de bytes embebidos.
  Antes de concluir "es del servidor" hay que volcar los FieldRVA y aplicar el descifrador.
- El barrido de bytes anterior (<2 MB) se saltaba `CapaDatosProcesos.dll` (20 MB),
  `FormsBasicos.dll` (22 MB) y `CapaDatos.dll` (16 MB), justo donde estaban los textos.

## 18. Segundo error: origen SI es servidor, pero con salida del propio cliente (2026-10-01 sesion 4, SOLO LECTURA)

### 18.1 Tabla de textos descifrada: `Traducs.dll`

- `Traducs.Traducs::Traduc(int)` (rva `0x90324`, token `0x060038DF`):
  `ResourceManager.GetObject("T" + id, ci)`; si no existe → `""`. **Los textos NO estan
  en `ldstr`: estan en el recurso embebido `Traducs.ResIdioma.resources`.**
- Extraido con `C:\tmp\bdp-decompile\res.ps1` (solo lectura de bytes; no ejecuta codigo del
  proveedor). Gotchas del extractor: (a) `ManifestResource.Offset` hay que sumarlo a
  `PEHeaders.CorHeader.ResourcesDirectory.RelativeVirtualAddress` y despues convertir
  RVA→offset de fichero; (b) hay que saltar un prefijo de 4 bytes de longitud antes del
  magico `CE CA EF BE`; (c) la propiedad se llama `ResourcesDirectory`, no `Resources`.

Textos relevantes obtenidos:

| Clave | Texto |
|---|---|
| `T13840` | LA LICENCIA REMOTA ESTA EN MODO DEMOSTRACION |
| `T13842` | NO SE HA ENCONTRADO LA LICENCIA REMOTA |
| `T13843` | LA LICENCIA REMOTA HA SIDO MANIPULADA |
| `T13929` | LA LICENCIA REMOTA NO ES CENTRAL DE RED |
| `T20194` | LA SUBSCRIPCION YA ESTA EN USO EN OTRA MAQUINA. ¿DESEA LIBERARLA DE LA OTRA MAQUINA PARA USARLA EN ESTA MÁQUINA? |

### 18.2 Quien emite `T20194`

Barrido de **todos** los `.dll`/`.exe` de `C:\Bdp-Net\NetHos\Prog` buscando `ldc.i4 20194`
(herramienta `findid.ps1`): **solo 2, ambos en `FormsBasicos.dll`**, y los dos hacen lo
mismo: `Traduc(20194)` + `CapaUtil.zInterCU::MessageBoxQuestion(...)`.

- `0x0600011E` (rva `0xB74C`; registrado como manejador por el `.ctor` `0x06000109`)
- `0x0600019D` (rva `0x10F50`)

Condicion de muestra (`0x0600011E`, IL_01E1-IL_0215):

```text
valor = this.campo_0x04000052.campo_0x040033DA       # string
if (CompareString(valor,"S",0) == 0)  # valor == "S"
      -> MessageBoxQuestion(Traduc(20194), ...)   # SI: se muestra la pregunta
else  -> se salta
```

El campo `0x040033DA` es **`ExisteSubscripcionActiva`** de la respuesta HTTP: lo escribe
`0x06006F51` con `zWebRequest::RespuestaString("ExisteSubscripcionActiva")`.

### 18.3 La llamada al servidor (`0x06006F51`, rva `0x4CD870`)

- Construye `CapaUtil.zWebRequest` con la ruta decifrada `0x04003589 = "Subscripcion/Usar"`
  sobre la base `https://bdpcloudpublicapi.azurewebsites.net/api`
  (alternativas presentes en el binario: `https://licencias.bdpcenter.com/licenciasweb/index.php`,
  `https://api-integradores.bdpcenter.com/api`).
- **Peticion** (22 pares clave/valor): `SubscripcionID`, `SubscripcionNombre`,
  `CodigoAplicacion`, `DistribuidorID`, `DistribuidorPassword`, **`IdMaquina`**,
  `SubControl`, `Version`, `Subversion`, `Fecha`, **`LiberarSubscripcionActiva`**
  (el manejador la envia `"N"` en la primera pasada).
- **Respuesta** leida con `zWebRequest::RespuestaString`: `MensajeError`
  (→ campo `0x040033D9`), **`ExisteSubscripcionActiva`** (→ `0x040033DA`),
  `Lice01_ID`, `Lice02_Apli`, `Lice03_Red`, `Lice04_Modulos`, `FechaFinal`, `Clave`.
- Si `ExisteSubscripcionActiva = "S"` → se pregunta `T20194` y, si el usuario acepta, se
  reenvia con `LiberarSubscripcionActiva = "S"` (el propio programa llama a
  `Subscripcion/Liberar` / `Actualizar`, sin intervencion de BDP).

### 18.4 Cadena de `IdMaquina` (donde entra el error local)

`CapaDatosProcesos.dll`:
- `0x06005711`: `x = 0x06005740()` (primer adaptador `IPEnabled`); `x != "" ? x : "00:00:00:00:00:00"`.
- `0x06005712`: `0x0600572F() ? 0x06005711() : ldsfld 0x04002961` (valor cacheado/escrito
  por `0x0600568B/568E/56F5`).
- `0x06005712` es llamado desde `0x0600E24F`, `0x0600E250`, `0x0600E251` (los builders de
  la peticion de licencia, `ConexBdp`), que ademas llaman a `0x06005711`, `0x0600570D`
  (`ldsfld 0x04002962; PadLeft(2,'0')`) y `0x06005709`.

`FormsBasicos.dll` (getter propio, paralelo):
- `0x06006FC7` (rva `0x4D8E00`) llama a `0x06006FC9` con un `ByRef` para la MAC y recibe
  en `V_5` el **valor de retorno**: `""` si todo va bien, o **el texto de error**
  (`ERROR ID MAC: MAC ADDRESS PUENTEADA`) si falla.
- Ese `V_5` se guarda a traves de `0x06006FD1(...)` + `callvirt 0x06006F67` (asignacion a
  una clave de configuracion) **sin comprobar si es error**; solo cuando `V_5 == ""` se
  construye el valor "bueno" concatenando `loc1 + loc0` (IL_0170-IL_018E).

> **Hipotesis de trabajo (NO confirmada aun):** con `PUENTEADA` activo, el identificador de
> maquina que se persiste/envia no es la MAC sino el texto de error, por lo que el servidor
> no reconoce la maquina y contesta `ExisteSubscripcionActiva = "S"` → los dos errores
> serian consecuencia de la **misma** causa local de la §17.4.
> Confirmacion pendiente: ver `0x06006FD1`/`0x06006F67` (que clave escriben) y comparar el
> valor con el que la otra parte (`0x0600E24F`) envia como `IdMaquina`.

### 18.5 Estado local de ficheros de configuracion

- No existe **ningun** `.ini` bajo `C:\Bdp-Net` (`ServerAccess.ini`, `ServerAccess2.ini`,
  `EstadoProg.ini`, `CodReg.ini`, `LiceAnul{0}.ini`, `NetworkCard.txt`): se crean en
  runtime; por tanto no hay "huella" local de la MAC registrada.
- Barrido de `C:\Bdp-Net` (< 8 MB, ASCII-1252 y UTF-16) por `8C4962AB7923`,
  `8C-49-62`, `401C8379550E`, `40-1C-83`: **0 hits** en ficheros.
- `C:\Bdp-Net\NetHos\Logs\ChkRes.log` solo contiene la restauracion SQL del 30/09
  (`Base de Datos Bdp-Net_NetHos_36 restaurada.`).

### 18.6 Herramientas nuevas / arregladas (para reutilizar)

`C:\tmp\bdp-decompile\`: `p.ps1` (wrapper de `probe.cs`: `FindCallers`, `FindFieldUsers`,
`FindMethods`, `DecodeMethod`...), `dump.ps1` (IL por RVA con nombres resueltos),
`findid.ps1` (busca `ldc.i4 <id>` en todos los metodos de un ensamblado), `res.ps1`
(extrae recursos `.resources`), `list.ps1` (IL por token).

Tres bugs que hacian que **todas** las busquedas IL dieran falsos negativos:
1. `$map[$b]` con clave `byte` no coincide con clave `int` → todo salia `INVALID`; hay que
   escribir `$map[[int]$b]`.
2. `MetadataTokens` vive en `System.Reflection.Metadata.Ecma335.MetadataTokens`.
3. `GetMethodBody`/`GetMetadataReader` son metodos de extension de `PEReaderExtensions`
   (`[System.Reflection.Metadata.PEReaderExtensions]::GetX($pe, ...)`), no metodos de `PEReader`.
4. `[sbyte]$byte` falla en PowerShell con valores >127: convertir a `int` y restar 256.

## 19. RESUELTO (2026-10-01 tarde, verificado en el TPV)

- Aplicado §17.5 con autorizacion del usuario: copia previa
  `C:\tmp\backup-NetworkAddress-0000.reg` (valor original `8C4962AB7923`) y
  borrado de `NetworkAddress` en `...\{4D36E972-...}\0000` (requirio UAC; el
  primer intento se cancelo por error y se repitio).
- Verificacion: `Get-ItemProperty ... -Name NetworkAddress` → "la propiedad
  no existe". El TPV arranca **sin** `ERROR ID MAC` y la activacion se queda:
  `122417` (Usar suscripcion) + `751080`/`84` (Suscripciones extendidas)
  activadas OK. Detalle de pantallas en
  `problema-instalacion-sql-2026-09-30.md` §F15.
- Confirma la hipotesis §18.4: los dos errores tenian la **misma** causa local;
  no hizo falta reset en servidor. La hipotesis "texto del servidor" de §10-§15
  queda invalidada (ver §17-§18).
- Pendiente domestico: conservar `C:\tmp\backup-NetworkAddress-0000.reg` unos
  dias y borrarlo; si el error vuelve, este registro es lo primero a mirar.
