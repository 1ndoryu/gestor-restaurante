# Arranque automático del TPV demo — causa y corrección (28/11/2026)

Tarea `28AA-3`. Hasta hoy constaba como «no se sabe qué arranca el TPV» y «el login es
manual». Las dos conclusiones eran consecuencia de un fallo propio, no de BDP.

## Causa 1: el `.vbs` de la carpeta Inicio estaba roto

`C:\Users\Owner\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup\bdp-tpv-arranque.vbs`
(529 bytes, del 16/11) llamaba a `GetObject("winmgmts:\.\root\cimv2")`: **una barra invertida
de menos**. El moniker correcto es `winmgmts:\\.\root\cimv2`, que en el literal VBScript se
escribe `winmgmts:\\.\root\cimv2`.

Medido: `cscript` sobre el fichero original devuelve `0x80041021` (`WBEM_E_INVALID_NAMESPACE`)
en la función que comprueba si el TPV ya está vivo — es decir, **muere en el primer paso, antes
de lanzar nada**. Como el `.vbs` corría sin ventana al iniciar sesión, el error no se veía en
ninguna parte: el TPV simplemente no arrancaba y no quedaba rastro.

Corregido el moniker y probado el fichero tal cual, dos veces y por las dos vías
(`cscript` y `wscript`, que es la que usa el arranque). Registro de cada intento en
`C:\tmp\pc\tpv-arranque.log`, que antes no existía.

## Causa 2: `TpvNet` corre elevado y la carpeta Inicio no puede elevarlo

Medido con el TPV vivo (PID 20888): mismo usuario (`WAN\Owner`) y misma sesión (1) que este
shell, pero `OpenProcess(PROCESS_QUERY_INFORMATION)` → **acceso denegado** y solo concedido el
acceso limitado, que es la firma de un proceso con integridad **alta**. `taskkill` y
`Stop-Process` sobre él fallan con «Acceso denegado», y `(Get-Process -Id 20888).Path` no
devuelve nada. Los accesos directos del escritorio (`TPV BDP 36.lnk`) no llevan la marca de
elevación, así que el TPV sube de nivel por otra vía.

Consecuencia: cualquier cosa que arranque desde la carpeta Inicio corre en nivel medio y **no
puede lanzar** un proceso elevado. Por eso el `.vbs`, aun con el moniker bien escrito, habría
fracasado igual en el portátil.

## Corrección

El arranque pasa a una **tarea programada elevada**, la misma vía que ya usa el driver de
pc-control (`cua-driver-serve`, `RunLevel=Highest`):

| Pieza | Ruta |
|---|---|
| Script de arranque (versionado) | `scripts/pc/bdp-tpv-arranque.vbs` |
| Instalador de la tarea (una vez, elevado) | `scripts/pc/instalar-arranque-tpv.ps1` |
| Prueba de punta a punta (elevado) | `scripts/pc/probar-arranque-tpv.ps1` |
| Registro de intentos | `C:\tmp\pc\tpv-arranque.log` |
| Tarea | `bdp-tpv-arranque`, al iniciar sesión, `RunLevel=Highest` |

El `.vbs` del Inicio queda apartado como `bdp-tpv-arranque.vbs.aparte`: no se borra, para que
revertir sea devolverle el nombre. Dos lanzadores del mismo proceso no se dejan conviviendo.

El script nuevo, además: espera 90 s (SQL Server y escritorio), **reintenta 6 veces cada 30 s**
si el TPV muere al arrancar (caso típico: SQL todavía no contesta), no lanza una segunda copia
si ya hay una viva, y anota cada intento con fecha.

## Lo que sigue sin automatizarse: el login es manual

Corrección de una conclusión anterior mía, que era falsa. Yo había dado por hecho que la
sesión del Bdp-Net persiste entre arranques del proceso, porque sobre el PID 20888 (vivo desde
días atrás) `tpv.mjs entrar` llegaba al `MENÚ GENERAL` sin pedir clave. **No es así**: ese
proceso tenía la sesión ya abierta de antes.

Medido con la prueba de punta a punta (PID 32416, recién arrancado por la tarea programada):
el TPV abre en `Control Usuario` con el rótulo «INTRODUZCA CONTRASEÑA USUARIO». Es decir,
**cada arranque del proceso pide la clave del usuario del restaurante**. La tarea programada
deja el TPV levantado, pero no logueado: alguien tiene que meter la clave.

Esa clave se puede meter por pc-control sin teclear a ciegas —el campo es
`Edit [id=txtConFoco]` con `actions=[set_value]`, o los dígitos del panel táctil y `Aceptar`—
pero automatizarlo obliga a guardar la clave del restaurante en un script, y eso no se hace
sin que lo decida la usuaria. Por ahora queda como requisito manual documentado.

Sin verificar: si la clave se pide también tras **reiniciar el portátil** (no solo tras
relanzar el proceso) o si en ese caso el Bdp-Net conserva algo. Es lo único que no se puede
probar sin reiniciar la máquina.
