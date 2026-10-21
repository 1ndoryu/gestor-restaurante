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

## Destino (replanificado el 2026-10-21, pedido de la usuaria)

La usuaria pidió tres cosas más al ver el primer esbozo: **más pestañas dentro de
Configuración**, **configuraciones más simples** y **un botón de ayuda por
sección que abra un modal con la información completa**. Eso cambia el destino:

**Seis pestañas**, cada una con una sola pregunta y un solo punto de guardado:

| Pestaña | Qué ajusta | Qué incluye |
|---|---|---|
| Mi restaurante | Cómo se llama y qué pide al reservar | Nombre, IVA, campos obligatorios, horario de los turnos |
| Cómo cobro y opero | Cómo se cierra una reserva y se anula una venta | Venta automática al completar, modalidad de anulación, Haddock, reseñas y WhatsApp |
| A qué se conecta | A qué sistema externo se habla | BDP (conexión, datos intercambiados, parámetros de escritura, funciones avanzadas, modo de operación), Groq, marketing |
| Chatbot | Cómo entra el chatbot | API keys del chatbot |
| Quién entra | Quién puede hacer qué | Permisos por acción, contraseña del terminal |
| Avanzado | Diagnóstico y datos de prueba | Respaldos y permiso de operación, libro de stock, avisos del TPV, datos de ejemplo. Sin formulario: todo guarda solo |

**Simplicidad** (lo que deja de verse en la primera pantalla):

- Los mapas en JSON (`bdp_tender_map`, `bdp_order_type_map`, `bdp_tav_map`) dejan
  de pedir que se escriba JSON: tabla de parejas «origen → destino» con añadir y
  borrar filas. El texto JSON solo aparece en «Avanzado», como último recurso.
- Cada pestaña empieza por una línea que dice qué hay dentro y —cuando cambia
  algo— un aviso «sin guardar» en la propia pestaña, sin tener que entrar
  tarjeta por tarjeta.

**Botón de ayuda** (`?` junto al título de cada pestaña): abre un modal con la
información completa de esa sección — qué hace cada ajuste, qué pasa si se deja
vacío, dónde se obtiene el dato (p. ej. el token de Haddock o el código de IVA
del TPV) y qué se rompe si está mal. El texto vive en un archivo por sección
(`configuracion/ayuda.ts`, un objeto por pestaña), no repartido entre tarjetas:
una sola fuente y se puede revisar sin tocar el formulario. Decidido: un archivo
y no seis, porque seis archivos de treinta líneas obligan a abrirlos todos para
ver si falta una pestaña; con `Record<IdSeccion, …>` el compilador avisa.

Reglas transversales:

- **Un solo modo de guardado por pestaña.** Lo que necesita botón va en un
  formulario con «Guardar» arriba y abajo y estado sucio visible; lo que guarda
  al instante lleva una marca explícita y no comparte formulario.
- **Cada tarjeta empieza por el resultado**, no por el nombre del módulo.
- **Nada de ajustes de conexión mezclados con datos de prueba.**

## Pasos (checklist)

- [x] **1. Inventario y mapa.** Cada campo, con su pestaña destino y si guarda
  solo o con botón. *Hecho*: `SECCIONES` en `configuracion/secciones.ts` declara
  los campos de cada pestaña; `tsc` avisa si un campo deja de existir.
- [x] **2. Contenedor de seis pestañas.** *Hecho*: `restaurante`, `operacion`,
  `conexiones`, `chatbot`, `acceso`, `avanzado`; el enlace profundo `bdpArming` /
  `bdpSection: 'bdp'` abre «A qué se conecta» y baja al panel; ninguna llamada a
  la API cambió.
- [x] **3. Guardado por pestaña.** Estado sucio por sección y un botón por
  pestaña, separando lo que se guarda al instante. *Hecho y comprobado* en la app
  de la 89: cambiar el IVA a 11 % marca `sin guardar` solo en «Mi restaurante»
  (ninguna otra pestaña), guardar deja «Guardado» en esa pestaña y
  `GET /api/configuracion` devuelve `"11.00"`; deshacer el cambio quita el badge
  (sin falsos positivos) y tocar un interruptor de «Cómo cobro y opero» marca esa
  pestaña y no la primera. Salir con cambios avisa vía `platform/avisos-navegador`.
- [x] **4. Simplificar los mapas.** Tabla de parejas con añadir/borrar fila para
  los tres mapas; el JSON crudo queda detrás de un desplegable. *Hecho*: el mapa
  de IVA se rellena sin escribir llaves y el JSON guardado conserva el formato
  anterior (`{"4":3,"10":1,"21":2}`), comprobado tras escribir una fila.
- [x] **5. Ayuda por pestaña.** Botón `?` y modal con el texto completo. *Hecho y
  comprobado*: las seis pestañas abren su propio modal con su título y sus
  bloques (4/4/7/2/2/4, los mismos de `ayuda.ts`) y Escape lo cierra.

## Revisión real (no opcional)

En la app de la 89, cada pestaña: responsive (móvil 375 y escritorio), estado
vacío (usuario recién creado, sin BDP ni chatbot), estado de error (BDP
inaccesible: la pestaña no debe romperse), teclado (tabulación completa, foco
visible, el modal se abre y se cierra con Escape) y contraste. Gate `--cierre`
a 0 nuevos.

Corregido en esta revisión (no se veía compilando): con `flex-wrap` la lista de
pestañas no crecía a su contenido y a 375 px la última fila se montaba encima de
la sección; ahora es una sola fila con scroll horizontal, la última pestaña se
alcanza y el panel queda siempre debajo. Un fallo de caché del navegador enseñó
el bundle viejo durante una comprobación: el bundle servido debe coincidir con
el recién construido (`index-*.js`) antes de dar nada por verificado.

## Lo que este plan NO hace

No cambia el contrato de la API ni el modelo de datos: es reorganización de la
pantalla. Un ajuste que hoy no existe (p. ej. un valor que la usuaria pidió
configurable y no está) sería otra tarea.
