# Plan — reorganizar la configuración (`10AA-3-D33`)

Pedido de la usuaria (2026-10-21): «todas las configuraciones están muy
desorganizadas y son complicadas de entender; hay que organizar mejor,
simplificar». Breve a propósito: primero el diagnóstico medido, luego tres
pasos con criterio de hecho propio.

## Estado actual (medido, no supuesto)

| Archivo | Líneas | Qué es |
|---|---|---|
| `Configuracion.tsx` | 136 | contenedor: 4 pestañas y la columna de General |
| `configuracion/TarjetasNegocio.tsx` | 134 | nombre, dirección, moneda, IVA… |
| `configuracion/TarjetasOperativas.tsx` | 68 | mesas, turnos, impresión |
| `configuracion/TarjetasIntegraciones.tsx` | 114 | marketing |
| `configuracion/TarjetaTurnos.tsx` | 85 | turnos |
| `configuracion/TarjetaSeguridadTerminal.tsx` | 188 | seguridad del terminal |
| `ConfigChatbot.tsx` | 184 | chatbot |
| `IntegracionesMarketing.tsx` | 207 | marketing (otra vez) |
| `ConfigBdp.tsx` | 182 | BDP |
| `PanelBdpBackup.tsx` | 101 | copias BDP |

Cinco problemas concretos:

1. **Un «Guardar configuración» invisible.** Vive a media página, dentro de la
   columna de General, después de cinco tarjetas. Se guarda con botón lo que
   está arriba y sin botón (cada tarjeta por su cuenta) lo que está abajo: la
   misma pantalla mezcla dos modos de guardado sin decirlo.
2. **Dos entradas para lo mismo.** «Integraciones» es una pestaña y también una
   tarjeta dentro de General, que a su vez renderiza `IntegracionesMarketing`.
3. **Turnos partido en dos tarjetas** (`TarjetasOperativas` y `TarjetaTurnos`).
4. **Datos de prueba en General.** El modo demo del terminal (y su restablecer)
   está en «BDP» como cabecera, como si fuera ajuste del WebLink.
5. **Nombres que no dicen qué cambia.** «Configuración BDP», «Panel BDP»,
   «Seguridad del terminal», «Integraciones de marketing»: ninguna dice qué
   ajuste concreto toca ni en qué afecta a la operación.

## Destino

Cinco grupos por **la pregunta que se hace la usuaria**, no por módulo técnico:

1. **Mi restaurante** — nombre, dirección, moneda, impuestos, horario.
2. **Cómo cobro y opero** — formas de pago, mesas, turnos, impresión, tickets.
3. **A qué se conecta** — BDP (conexión, almacén por defecto, mapeos), chatbot,
   marketing; cada bloque con su estado («conectado / sin configurar / error»)
   visible sin entrar.
4. **Quién entra** — usuarios, roles, permisos, contraseña, seguridad del
   terminal.
5. **Datos de prueba** — modo demo y restablecer, fuera de «A qué se conecta» y
   con aviso de que toca datos.

Reglas transversales:

- **Un solo modo de guardado por grupo.** Lo que necesita botón va en un
  formulario con su «Guardar» al principio y al final del grupo, y estado sucio
  visible («cambios sin guardar»). Lo que se guarda solo lleva una marca
  explícita («se guarda al instante») y no comparte formulario con lo demás.
- **Cada tarjeta empieza por el resultado**, no por el nombre del módulo: «El
  nombre que sale en los tickets» en lugar de «Datos del negocio».
- **Nada de ajustes de conexión mezclados con datos de prueba.**

## Pasos

1. **Inventario y mapa.** Recorrer las 10 piezas, anotar cada campo con su
   grupo destino y si guarda solo o con botón. Salida: tabla de mapeo en este
   plan. *Hecho* cuando no queda ningún campo sin grupo asignado.
2. **Reestructurar el contenedor.** `Configuracion.tsx` pasa a los cinco
   grupos; se elimina la pestaña «Integraciones» duplicada y se fusionan las dos
   tarjetas de turnos. Sin tocar la lógica de guardado todavía. *Hecho* cuando
   todo el contenido actual sigue accesible desde un grupo y el diff no toca
   ninguna llamada a la API.
3. **Unificar el guardado.** Formulario con un único estado sucio y un botón
   por grupo, separando lo que se guarda al instante. *Hecho* con pruebas
   reales en la app de la 89: guardar, recargar y comprobar que persiste;
   intentar salir con cambios sin guardar y que avise.

## Revisión real (no opcional)

En la app de la 89, cada grupo: responsive (móvil 375 y escritorio), estado
vacío (usuario recién creado, sin BDP ni chatbot), estado de error (BDP
inaccesible: el grupo no debe romperse), teclado (tabulación completa, foco
visible) y contraste. Gate `--cierre` a 0 nuevos.

## Lo que este plan NO hace

No cambia el contrato de la API ni el modelo de datos: es reorganización de la
pantalla. Un ajuste que hoy no existe (p. ej. un valor que la usuaria pidió
configurable y no está) sería otra tarea.
