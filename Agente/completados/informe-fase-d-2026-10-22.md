# Fase D — Informe de cierre

**Integración web–TPV BDP.** Qué se ha comprobado, qué funciona solo, cómo se activan las
escrituras y qué queda pendiente.
Fecha del informe: 22 de octubre de 2026. Revisado y corregido el mismo día contra el código
y contra la instalación de producción.

> **Corrección del 26 de octubre de 2026.** La revisión dirigida a Wan
> (`Agente/usuario/correcciones-antes-del-piloto-2026-10-26.docx`) detectó que varias
> fichas se cerraron antes de disponer de la licencia, la configuración o la prueba que
> exige cada caso. Este informe ya **no afirma que la integración esté lista para el
> piloto**: los estados de la sección 1 y el cierre de la sección 10 sustituyen a los
> del 22 de octubre. Una prueba hecha con la edición DEMO o gratuita demuestra el
> código, no el flujo real de producción. Producción sigue en solo lectura. Plan:
> `Agente/planes/plan-correcciones-piloto-bdp-2026-10-26.md`.

---

## 1. Resumen

La fase D pedía una cosa por encima de todas: **que la integración funcione sin que
nadie pulse botones**. Lo que se toca en el TPV tenía que llegar solo a la web, y
lo que se vende en la web tenía que llegar solo al TPV, sin duplicados y sin
depender de que alguien se acuerde de sincronizar. El cliente lo concretó en
**65 pruebas de aceptación** (fichas D01–D32), una por funcionalidad, cada una con
su prueba. No valía «más o menos»: o se hace, o se declara por qué no se puede.

**Lo que se pedía, y está hecho:**

- **La automatización existe y corre sola.** Siete procesos en segundo plano: la
  cola envía lo pendiente cada minuto, los avisos del TPV se recogen cada 5
  segundos, y las lecturas (estados de comandas, maestros, clientes, plano,
  departamentos, documentos y albaranes) se refrescan con su propia cadencia. Las
  lecturas no escriben nada en el TPV.
- **La venta no se queda a medias.** Cada operación lleva su clave de
  idempotencia, no se admiten dos en vuelo sobre la misma comanda, y lo que queda
  «ambiguo» se resuelve releyendo el TPV en vez de reenviarlo a ciegas.
- **En automático, se envía y además se relee el resultado.** Antes la comanda se
  enviaba y se quedaba congelada en la web porque nadie volvía a preguntar cómo
  acabó; eso se corrigió en este cierre.
- **La escritura está protegida por diseño, no por confianza.** Tres modos (solo
  lectura, una operación autorizada a mano y automático), copia previa obligatoria,
  snapshot del día, confirmación expresa y una fila de auditoría por operación
  desde antes de salir hasta su resultado.
- **Está probado contra un TPV BDP real instalado en local** (licencia DEMO,
  versión 36.2): el mismo programa y las mismas tablas que el del cliente, con otra
  instalación y otra configuración. Fase D pedía exactamente eso: probar aquí
  antes de tocar su restaurante.

**Resultado de las 65 pruebas (estados corregidos el 26 de octubre).** Una ficha solo
figura como cerrada si el entorno cumplía sus requisitos, se ejecutó la operación real,
el resultado se vio en la web y en BDP y se conserva la evidencia. Si falta algo, lleva
uno de estos estados. Los recuentos exactos se recalculan al entregar las fichas de
evidencia (`26AA-6`).

| Estado | Fichas |
|---|---|
| **Cerrada** (prueba real y evidencia) | D02, D03, D04, D09, D10, D12a, D12b, D12c, D22, D23a, D24a, D24b, D24c, D25, D26b, D28, D28-auto, D28-auto-R, D28a. D08, D21a, D22a, D24-m, D26, D26a, D26c, D26e (pruebas locales del 26 y 27/10/2026, ver filas). D26d, D26f y D26g se mantienen cerradas |
| **Pendiente de licencia** (WebLinkRestAPI de pago, servicio 84) | D01 (el diagnóstico devuelve «sin licencia»); D05, D06, D07 (modificar comanda existente: la edición gratuita lo bloquea, no es un límite del TPV); D11a, D11b, D11c (propinas); D13, D13a, D14, D14s, D15, D15t, D16, D17 (pagos); D18, D18a, D18b, D18c, D18d, D19, D19t, D20 (facturación; D18d además debe emitir una factura nueva con datos fiscales); D24-p (salones, zonas y mesas); D31 (llamar al camarero) |
| **Pendiente de configuración** | D31 (dirección del servidor de mensajes en el terminal; cuenta también en licencia) |
| **Pendiente de prueba local** (se puede hacer en el BDP local) | D21 (la comanda pendiente 72 existe en el BDP local; falta aceptarla en la consola de autocomanda del TPV para ver el aviso de cambio de estado), D27 (venta de caja: tras un ticket cobrado el BDP local seguía con el mismo stock; falta el cierre de turno o la configuración de descuento), D29 (reinicio del BDP local) |
| **Pendiente de piloto** (hardware o terceros) | D03a y D09a (impresora de cocina), D30 (piloto), D32 (responsables y procedimientos nominales aprobados por Guillermo y el restaurante; la pantalla existe, la ficha no está cerrada) |
| **Límite confirmado** (el TPV no ofrece el servicio) | D23-m (sin listado de comandas por API), D23b (solo se recupera lo localizable), D24d (sin servicio de borrado; lo cubre D32) |

**Qué falta.** El 22 de octubre este apartado decía que no faltaba automatismo y que
todo lo pendiente dependía del entorno. No era exacto:

- **Licencia (WebLinkRestAPI de pago).** Afecta a más operaciones que pagos y facturas:
  propinas, lectura completa y actualización de comandas, salones y mesas y llamar al
  camarero. Hasta activarla, esas pruebas no demuestran el flujo de producción.
- **Automatización.** Las formas de pago y los empleados tienen lectura programada en
  código (`26AA-2`); falta comprobarla en vivo. El modo automático tiene permisos por
  categoría, todos apagados de fábrica (`26AA-3`, hecho en código y con tests).
- **Pruebas que se pueden hacer en local** y se enviaron al piloto sin necesidad
  (`26AA-4`, `26AA-5`).
- **Hardware y terceros.** Impresora de cocina, reinicio real de BDP.NET, dirección del
  servidor de mensajes del terminal y responsables nominales de D32.
- **Puesta en marcha con sus datos:** formas de pago, tipos de pedido y perfil de
  documentos de venta (§9). Son decisiones suyas, no cosas pendientes de programar.

---

## 2. Qué hace la integración hoy, sin intervención

Siete procesos automáticos, uno por necesidad, y una regla que vale para todos:
**las seis lecturas no escriben nada en el TPV**. De los siete, el único que escribe
es el envío de la cola, y solo cuando la cuenta tiene permiso de escritura
(sección 3). Fuera de esta lista hay dos acciones que también escriben, y solo a
mano: reintentar el envío de una venta concreta y dar de alta un cliente en el TPV.
Las dos pasan por el mismo permiso de escritura y la misma auditoría.

La columna «cada cuánto» es el valor real y de fábrica, no el recomendado. Las
cadencias marcadas «por cuenta» nacen **en cero (apagadas)** y se encienden desde
Configuración con el valor que decida el cliente.

| Proceso | Cada cuánto | Qué hace | ¿Escribe en el TPV? |
|---|---|---|---|
| **Envío de la cola** | 60 s | Manda al TPV lo que la web tiene pendiente (ventas, artículos, stock, pagos, facturas) | **Sí**, solo con permiso y con auditoría por operación |
| **Estados de las comandas** | 60 s por cuenta (10–600) | Relee cómo acabó cada comanda enviada: si el TPV la cerró, la web lo sabe | No |
| **Avisos del TPV** | 5 s | Procesa los avisos que el TPV manda a la web (crear, modificar, borrar, finalizar, traspasar…) | No |
| **Documentos de venta** | por cuenta, **apagado de fábrica** | Relee facturas y albaranes del TPV: cabecera, líneas y pagos | No |
| **Maestros (artículos y precios)** | por cuenta, **apagado de fábrica**; recomendado 15 min | Un cambio hecho en el TPV llega solo a la web | No |
| **Albaranes de compra** | 5 min | Importa los albaranes del TPV de los últimos 31 días sin pulsar nada | No |
| **Clientes, plano y departamentos** | por cuenta, **apagado de fábrica**; recomendado 15 min | Relee los clientes, los salones/zonas/mesas y las familias del catálogo, cada uno con su cadencia | No |

Los ciclos no se solapan: si una pasada tarda más que su intervalo, la siguiente
se retrasa en vez de encadenar llamadas al TPV.

La regla que se corrigió en este cierre es la más importante de todas: **elegir
«enviar en automático» enciende también la lectura del resultado**. Antes, la
comanda se enviaba y se quedaba congelada en la web, porque nadie volvía a
preguntar cómo acabó. Eso era exactamente lo contrario de lo que pide un
restaurante que elige automático.

---

## 3. Los tres modos de escritura, y cuál hay que activar

La web puede escribir en el TPV de tres maneras. **Ninguna se activa sola**: toda
cuenta nace en solo lectura y el modo lo cambia a mano un usuario con rol de
administrador.

| Modo | Qué permite | Quién y cómo lo activa | Cómo se sale |
|---|---|---|---|
| **Solo lectura** (por defecto) | Nada. Solo se lee e importa del TPV | Es el estado de partida | — |
| **Escritura autorizada a mano** | **Una** operación concreta: un alcance, una entidad, **una** operación, entre 1 y 15 minutos y con un motivo escrito | Un administrador, desde Configuración, eligiendo la operación y confirmando la URL del TPV | **Sola**: al gastar la operación, al vencer el plazo o al pulsar «volver a solo lectura» |
| **Automático** | Permiso permanente: la cola y los botones escriben sin confirmar cada operación, siempre con una fila de auditoría por escritura | Un administrador, con confirmación expresa, la URL exacta del TPV y un **snapshot completo del día** de esa conexión | A mano, volviendo a solo lectura, o **solo**, si cambia cualquier dato de la conexión |

Dos precisiones que importan en la práctica:

- Cada categoría del modo automático (comandas, artículos y clientes, propinas y puntos,
  pagos, facturas, stock y camarero) nace **apagada** (`26AA-3`). Una categoría apagada
  deja la operación en espera con su motivo visible, y activar una sola no permite
  escribir las demás. Aun así, la recomendación es activar cada categoría solo después
  de haber ensayado a mano esa operación.
- «Llamar al camarero» está permitido en los dos modos con escritura, pero **no
  gasta autorización** porque no cambia ningún dato del TPV: es un aviso al
  terminal. Lo único que exige es que la cuenta no esté en solo lectura.

---

## 4. Primera activación del modo automático: el riesgo y lo que lo frena

**El riesgo es real y es este:** en automático la web escribe en el TPV del
restaurante sin que nadie confirme cada operación. Un mapeo equivocado (un
artículo, un terminal, un tipo de pedido) o una credencial de pruebas apuntando
al TPV de producción se convierte en datos escritos en el TPV real, y algunos no
tienen vuelta atrás.

Lo que se ha puesto por delante, capa a capa:

1. **Confirmaciones que no se pueden saltar.** Activar el modo exige rol de
   administrador, una confirmación explícita y escribir la URL exacta del TPV. Sin
   las tres cosas el cambio se rechaza, y el resto de la configuración general ni
   siquiera admite cambiar el modo por otra vía.
2. **El destino queda atado.** La cuenta guarda una huella de la conexión (URL,
   login, contraseña, código de integrador, terminal, empleado y perfil de
   artículos). **Si cambia cualquier dato de esa lista, el modo vuelve a solo
   lectura automáticamente** y hay que activarlo otra vez de forma deliberada.
3. **Sin desvíos de destino.** La dirección del TPV debe ser un origen limpio
   (sin ruta, usuario, contraseña, `?` ni `#`) y las peticiones no siguen
   redirecciones: una escritura no puede acabar en un host distinto del configurado.
4. **Snapshot completo antes de empezar.** Para activar el automático (y para cada
   autorización manual) hace falta un snapshot **completo** del TPV de esa conexión
   exacta, de **menos de 24 horas**, con artículos, clientes, departamentos,
   salones y empleados leídos. Si falta o está incompleto, la activación se
   rechaza: sin foto reciente no hay forma de saber qué se va a sobrescribir.
5. **Copia previa a cada escritura.** El respaldo previo es obligatorio: si está
   desactivado, la escritura se bloquea. En pagos y facturas, además, se guarda el
   estado real de la comanda en el TPV justo antes de tocar nada; si esa lectura
   falla, no se escribe.
6. **Nada se duplica.** Cada operación lleva su clave de idempotencia y el sistema
   no admite dos operaciones en vuelo sobre la misma comanda: un doble clic o un
   reintento no crean dos pagos ni dos facturas.
7. **Permiso permanente, no auto-concedido.** El barrido de la cola **se niega a
   escribir** si la cuenta no está en automático; no puede darse el permiso a sí
   mismo. Ese era el defecto anterior y está corregido.
8. **Todo queda registrado.** Cada escritura deja una fila de auditoría desde antes
   de salir (`pendiente`) hasta su resultado (`éxito`, `error` o `ambiguo`), con lo
   que se envió y por qué estaba autorizada.
9. **Freno de mano en un clic.** Volver a «solo lectura» desactiva las escrituras al
   instante sin tocar nada más. Las lecturas y las importaciones siguen.
10. **La prueba de contrato del TPV no crea nada.** El diagnóstico que comprueba
    el contrato pide la comanda con la marca de «en seco» (`OnlyCheck`), que el TPV
    no debe persistir. Se ejecuta a propósito, desde el botón de diagnóstico: no
    forma parte de la operación normal y no crea comandas en el TPV.

---

## 5. Cómo poner esto en marcha, modo por modo

Recomendación, en este orden y sin saltarse pasos:

1. **Solo lectura, con las lecturas encendidas.** Encender las cadencias de
   artículos, clientes, plano y departamentos (15 minutos es la recomendada) y
   dejar que el catálogo se llene solo. Comprobar que los nombres, los precios y las
   mesas coinciden con el TPV. **Aquí no se escribe nada en el TPV.**
2. **Escritura autorizada a mano, una operación por vez.** Con el snapshot del día
   hecho, autorizar **una** operación, ver el resultado en el TPV y esperar a que el
   modo vuelva solo a lectura. Repetir con la siguiente: crear comanda, anular,
   propina, puntos, pago, factura, stock. Cada ensayo queda en la auditoría con su
   motivo.
3. **Automático, cuando lo anterior haya ido bien.** Activar con la confirmación y
   el snapshot del día. A partir de ahí la cola envía sola. Vigilar el primer día
   las filas de auditoría en estado `error` o `ambiguo`, que son las que necesitan
   una persona.

En cualquier momento, «volver a solo lectura» es el freno: no borra datos, solo
deja de escribir.

---

## 6. La tabla del alcance, cómo queda hoy

Es la misma tabla que se entregó en el documento de alcance («Después de la fase
D»), fila por fila, con la última columna puesta al día. **Se trae** = del TPV a la
web. **Se envía** = de la web al TPV. **Automático** = viaja solo, sin pulsar nada.
**Cómo queda** = lo que hay hoy.

| Funcionalidad | Se trae | Se envía | Automático | Cómo queda |
|---|:---:|:---:|:---:|---|
| Catálogo (artículos, precio, familia) | Sí | Sí | Sí | **Hecho.** Lectura sola cada 15 min (cadencia por cuenta) y envío por la cola automática |
| Clientes | Sí | Sí | Sí | **Hecho.** Importación sola con su propia cadencia (cada 15 min por cuenta); el botón con confirmación queda para forzar el cambio en el momento |
| Plano de sala | Sí | — | Lectura sí | **Hecho.** Salones, zonas y mesas se releen solos con su cadencia; la lectura no mueve ni renombra nada |
| Departamentos | Sí | Sí | Sí | **Hecho.** Se refrescan solos con su propia cadencia, aparte de la del catálogo |
| Formas de pago, empleados | Sí | — | Lectura sí | **Hecho en código, sin comprobar en vivo.** Lectura programada por cuenta (apagada de fábrica), sin escribir en el TPV. Falta ver un cambio del TPV llegar a la web sin abrir la pantalla (`26AA-2`) |
| Albaranes de compra | Sí | — | Por exportación | **Hecho.** Importación automática cada 5 min, últimos 31 días |
| Detalle de comanda / stock | Sí | — | Sí | **Hecho.** Se lee tras cada aviso y con los documentos de venta |
| Crear comanda | Sí | Sí | Sí | **Hecho en local.** Falta el envío a cocina, que necesita la impresora |
| Propina (en comanda) | — | Sí | Sí | **Hecho.** Sumar, sustituir y dejar a cero |
| Puntos fidelización (cliente) | — | Sí | Sí | **Hecho.** Abono, cargo y pérdida de respuesta, sin duplicar |
| Anular comanda | Sí | Sí | Sí | **Hecho.** El efecto en cocina queda para la sesión presencial |
| Modificar comanda enviada | Sí | Sí | Sí | **Pendiente de licencia.** El manual de BDP bloquea la actualización de una comanda existente en la edición gratuita y remite a la de pago; se prueba allí antes de declarar ningún límite |
| Detectar traspaso de TPV (avisos) | Sí | — | Sí | **Hecho en parte.** Traspaso de línea probado; el de comanda completa es presencial |
| Iniciar traspaso desde la web | — | Pendiente de TPV | No | **Fuera.** El TPV no lo expone |
| Cobrar comanda existente | Sí | Sí | Sí | **Hecho.** Parcial, total, doble clic y respuesta perdida |
| Facturar | Sí | Sí | Sí | **Hecho**, con la salvedad de la suscripción de pago del TPV |
| Consultar stock (TPV→web) | Sí | — | Lectura sí | **Hecho.** Por artículo y en lista por almacén |
| Regularizar stock (web→TPV) | — | Sí | Sí | **Hecho en parte.** Falta confirmar en el TPV del cliente el rechazo por motivo inválido |
| Traspasar stock entre almacenes | — | Sí | Sí | **Hecho.** Ida y vuelta real |
| Actualizar inventario (web→TPV) | — | Sí | Sí | **Hecho.** Nunca envía cantidad vacía |
| Llamar al camarero | — | Sí | Sí | **Hecho de nuestro lado.** El TPV lo rechaza hasta que se configure en el terminal |
| Borrar artículos | Pendiente de TPV | Pendiente de TPV | No | **Límite del TPV.** La pantalla dice quién lo hace y cómo |
| Reservas, gastos, marketing | — | — | — | **Fuera de alcance.** Son solo web y ya funcionan |

Resumen de la tabla (23 filas): **17 hechas**, 2 con matiz declarado (formas de pago
y regularización de stock, ambas por lo que falta confirmar en el TPV del cliente),
2 que el TPV no expone (modificar comanda y borrar artículos) y 2 fuera de alcance
(iniciar traspaso y reservas/gastos/marketing). De las 17 hechas, dos dependen de
configuración del terminal: el envío a cocina (impresora) y el aviso al camarero
(servidor de mensajes). **Ninguna fila queda pendiente por falta de automatismo.**

---

## 7. Resultado de cada prueba

Las 65 fichas del alcance, agrupadas por línea. El estado de cada una es el de la
**sección 1**: **Cerrada** = prueba real y evidencia; **Pendiente de licencia**, **de
configuración**, **de prueba local** o **de piloto** = falta ese requisito; **Límite
confirmado** = el TPV no ofrece el servicio. Una fila describe lo comprobado hasta hoy:
si su estado es pendiente, la comprobación con licencia o en la instalación real aún
no se ha hecho, aunque el código esté implementado.

| Ficha | Qué se comprueba y cómo queda |
|---|---|
| D01 | Salud, versión, licencia 84 y acceso registrados — **Pendiente de licencia.** El TPV respondió versión 36.2 y la ficha quedó registrada. La licencia 84 se consulta ya en el diagnóstico de la pantalla; en el TPV DEMO responde «sin licencia», que es lo esperado |
| D02 | La contraseña vieja no entra, la nueva sí, sin secretos legibles — **Cerrada.** Rotación que exige la contraseña actual y registro de intentos que nunca guarda la contraseña ni su huella cifrada |
| D03 | Una venta web crea una sola comanda en caja, vinculada. **Cerrada.** Comanda 15, presente en el TPV y en la web, con el vínculo |
| D03a | Crear con envío a cocina. **Pendiente de piloto.** Necesita la impresora de cocina |
| D04 | Respuesta perdida al crear: se recupera la misma, no otra. **Cerrada en vivo.** Se cortó la respuesta con un proxy: la app no reenvió a ciegas, la relectura recogió la misma comanda (nº 60) y un reintento devolvió el mismo identificador con una sola entrada de auditoría |
| D05–D07 | Añadir, cambiar o retirar línea de una comanda. **Pendiente de licencia.** El manual indica que la edición gratuita bloquea actualizar una comanda existente y que la de pago lo permite. Se prueba con WebLinkRestAPI de pago. |
| D08 | Toque simultáneo en caja y web. **Cerrada (26/10/2026, BDP local).** Peticiones paralelas reales sobre el mismo artículo: el stock final fue exactamente el esperado (+3), sin pérdida ni duplicado; además, tests de bloqueo y unicidad 5/5 |
| D09 | Anular desde la web anula en caja. **Cerrada.** La anulación llegó al TPV y la comanda desapareció de su tabla |
| D09a | Efecto en cocina de esa anulación. **Pendiente de piloto.** Necesita testigo de impresora auxiliar |
| D10 | Anular en caja se refleja en la web. **Cerrada en vivo.** La comanda 46 pasó a anulada en la web con fecha y motivo; un aviso repetido no la vuelve a tocar; el sondeo también la anula |
| D11a, D11b, D11c | Propina que suma, que sustituye y que se queda a cero. **Pendiente de licencia.** Con lectura entre pasos. Se corrigió una conclusión anterior falsa: se había leído una comanda ya anulada |
| D12a, D12b, D12c | Puntos que suman, que restan y respuesta perdida. **Cerradas.** La pérdida de respuesta se reconcilió leyendo el saldo, sin reenviar. Y dos sumas seguidas ya no se pisan: se acumulan sobre la misma fila |
| D13, D13a | Pago parcial, con y sin factura. **Pendiente de licencia.** Dejaron ver un defecto real: un número de factura vacío marcaba la venta como facturada. Corregido |
| D14 | Pago total liquida una sola vez. **Pendiente de licencia.** Un pago repetido se rechaza porque excede el saldo |
| D14s | Dos pagos a la vez. **Pendiente de licencia.** Una petición entra y la otra recibe «ya hay un pago en proceso». No hay cobro doble |
| D15, D15t | Respuesta perdida o sin respuesta al pagar. **Pendiente de licencia.** La app consulta antes de repetir y encuentra el pago que sí se aplicó |
| D16 | Pago hecho en la caja se ve en la misma venta web. **Pendiente de licencia.** Con un lote automatizado contra el TPV (4 pasos en verde, documento cerrado) |
| D17 | Tope de tres pagos. **Pendiente de licencia.** Defecto encontrado y corregido: no había tope |
| D18, D18a, D18b, D18c | Número único, no re-facturar. **Pendiente de licencia.** Una segunda factura devuelve la existente en vez de emitir otra |
| D18d | Factura completa con NIF. **Pendiente de licencia.** Hoy solo se comprueba la exportación de una factura existente, que no prueba la emisión: falta facturar una comanda de prueba con los datos fiscales y comparar el documento. |
| D19, D19t | Respuesta perdida al facturar. **Pendiente de licencia.** Número único, sin segunda factura |
| D20 | Aviso y documento enlazados en una sola venta. **Pendiente de licencia.** Se enlazan por el número de factura, con índice único que impide duplicar |
| D21 | Avisos del cambio de estado de una comanda pendiente de validación (OrderEndType 1 y 0). **Pendiente de prueba local (27/10/2026).** Una venta registrada desde la web con `bdp_order_end_type = 1` creó en el BDP local la comanda 72 en estado pendiente (la escritura funciona); aceptarla exige el botón «Consola Autocom.» del TPV, que el controlador no consiguió abrir. La documentación no vincula este aviso al Monitor de Cocina. Falta aceptar la comanda a mano y comprobar el aviso en el buzón. |
| D21a | Aviso de traspaso de comanda completa. **Cerrada (27/10/2026, BDP local).** Traspasos reales en el TPV local (mesa 5→6, comanda 66): el aviso `Traspasar` llegó al buzón de la web y se procesó a la primera (intentos 1). El 26/10 no llegaba por una causa distinta de la supuesta: el receptor resolvía la cuenta destino y, con más de una cuenta con sincronización activa y credenciales completas, respondía 503 (falla cerrado). Se desactivó la sincronización en las dos cuentas de prueba (`fase-d@prueba.test`, `d26g-evidencia@restaurante.test`) y quedó una sola. Un aviso se marcó `requiere_revision` mientras el túnel 8071 estaba caído (se reabre con `bdp-local.sh tunel abrir`). |
| D22 | Aviso repetido o desordenado. **Cerrada.** Repetido = ignorado; desordenado = aceptado y ordenado. Traspaso de línea real (comanda 47) reenviado: idéntico → «duplicado», con fecha anterior → aceptado. |
| D22a | Traspaso de línea movida. **Cerrada en el testigo (26/10/2026, BDP local).** Traspaso de línea de la mesa 6 a la 7: `GetOrder` da la línea una sola vez en destino (comanda 67) y la mesa 6 ya no tiene comanda. La repetición y el orden inverso del aviso están cubiertos por los tests del receptor (D22); el aviso real `traspasar-linea` del TPV llegó y se procesó (27/10/2026, intentos 1). |
| D23-m | Comanda manual con aviso perdido. **Límite confirmado.** El TPV no ofrece listado de comandas por API, así que una comanda de caja sin aviso no se recupera sola; queda el procedimiento escrito para que una persona la resuelva |
| D23a | Trabajador caído con receptor operativo. **Cerrada.** El aviso espera en el buzón y se procesa al arrancar |
| D23b | Receptor inaccesible. **Límite confirmado.** Solo se recupera lo que se puede localizar por su identificador de pedido |
| D24-m | Formas de pago y empleados leídos. **Cerrada (26/10/2026, BDP local).** Lectura programada (`26AA-2`) de formas de pago y empleados sin abrir la pantalla; empleados en la web = 3 = WebLink. |
| D24-p | Plano leído sin mover posiciones. **Pendiente de licencia.** Comprobación del terminal, empleado y perfil contra el TPV |
| D24a, D24b, D24c | Alta, modificación y desactivación de artículos. **Cerradas** contra el TPV, con el artículo leído después en su maestro |
| D24d | Borrado. **Límite confirmado.** El TPV no tiene servicio de borrado: la web muestra el procedimiento y el responsable |
| D25 | Clientes, con y sin sobrescritura. **Cerrada.** La sobrescritura es explícita y solo se aprobó en el TPV local |
| D26 | Ajuste de stock desde la web: un solo movimiento. **Cerrada (26/10/2026, BDP local).** Escritura completa con copia previa; un solo movimiento por ajuste. Se corrigió un fallo real: el reintento con la misma clave reencolaba el movimiento (`fed7ddd`, test `reintento_idempotente_no_reencola_el_ajuste`, 25/25). |
| D26a | Consulta unitaria y error con almacén inexistente. **Cerrada (27/10/2026, BDP local).** La lectura es correcta y el almacén 99 (inexistente) devuelve el error 200005 del propio TPV, sin reintento ciego. |
| D26b | Consulta en lista por almacén. **Cerrada** con botón propio y llamador en producción |
| D26c | Regularización con motivo válido e inválido. **Cerrada (26/10/2026, BDP local).** Motivo válido aplica y motivo inválido se rechaza; el stock se relee tras cada paso. |
| D26d | Traspaso entre almacenes. **Cerrada** con traspaso real de ida y vuelta |
| D26e | Ajuste por diferencia (inventario absoluto). **Cerrada (26/10/2026, BDP local).** El inventario absoluto calcula la diferencia respecto al stock existente y el BDP queda en el valor pedido. |
| D26f | Inventario absoluto. **Cerrada.** Nunca envía cantidad vacía; la misma cantidad da diferencia cero |
| D26g | Sin respuesta en stock. **Cerrada.** Si hay duda, no se reintenta a ciegas; si el TPV rechazó con un motivo claro, ya no se marca «ambiguo» |
| D27 | La venta de caja no descuenta dos veces. **Pendiente de prueba local.** Venta real en el TPV local (ticket 00001TB00000036, COCA-COLA, efectivo): el BDP seguía en 7 tras varias lecturas y un artículo inventariable. Falta saber si descuenta al cerrar turno o requiere configuración; hasta entonces no se afirma el comportamiento. |
| D28, D28-auto, D28-auto-R | Documentos, exportación programada y reinicio a mitad. **Cerradas y vivas.** Cadencia y perfil por cuenta. Se descubrió que un ajuste del TPV devolvía vacío y se fijó correctamente |
| D28a | Albaranes automáticos. **Cerrada.** Importación completa sin botón |
| D29 | Tres reinicios en mitad de una operación. **Pendiente de prueba local.** Dos de tres comprobados por inyección (proceso de sincronización y caída del WebLink), sin pérdida ni duplicado. Falta el reinicio real del BDP local. |
| D30 | Piloto y despliegue por etapas. **Pendiente de piloto.** Requiere permiso y un TPV en producción. |
| D31 | Llamar al camarero desde una mesa. **Pendiente de licencia y de configuración.** El aviso llega al TPV y el TPV lo rechaza por su configuración (falta la dirección del servidor de mensajes). Con licencia de pago, CallWaiter y ventana visible en BDP. |
| D32 | Procedimiento y responsable de lo que la API no hace. **Pendiente de piloto.** La pantalla existe con responsable por defecto y editable; falta que Guillermo y el restaurante aprueben los responsables nominales. |

**Fuera del alcance de las 65, y cerrada en esta entrega:** D33, configuración
legible y con un solo guardado. Ayudas por tarjeta, un solo guardado por pestaña y
aviso al cambiar de pestaña con cambios sin guardar.

---

## 8. Limitaciones encontradas

Todas las de esta lista están declaradas. Ninguna es un fallo de la web: son límites
de la edición gratuita del TPV, de su configuración o de lo que falta probar.

1. **Tres pruebas exigen hardware o una persona en el terminal físico.** D03a y D09a
   (impresora de cocina) y D29, en su reinicio real de BDP.NET. D08, D21a y D22a ya se
   ejecutaron en el BDP local (26 y 27/10/2026). D21 espera a que se acepte a mano
   la comanda pendiente 72 en el TPV.
2. **La edición gratuita bloquea modificar una comanda existente** (D05–D07). Según el
   manual, la edición de pago lo permite. Pendiente de licencia, no límite confirmado.
3. **El TPV no permite borrar artículos por API** (D24d). Se resuelve por D32: la
   pantalla muestra quién lo hace y cómo. Límite confirmado.
4. **El TPV no ofrece listado de comandas por API** (D23-m). Una comanda creada a mano
   en la caja sin que llegue su aviso no se puede recuperar sola. Límite confirmado,
   con procedimiento escrito.
5. **El aviso «Comanda Estado Cambiado»** (D21) se prueba con comandas pendientes de
   validación (OrderEndType 1 y 0). La documentación no lo vincula al Monitor de
   Cocina, así que no se atribuye a ese módulo.
6. **El aviso de «llamar al camarero»** (D31) depende de la configuración del terminal
   y de la licencia de pago. Sin la dirección del servidor de mensajes, el TPV lo
   rechaza con su propio código de error.
7. **Licencia 84:** el TPV de prueba es DEMO y responde «sin licencia». Las pruebas de
   propinas, pagos, facturación, salones y mesas y camarero se repiten con la licencia
   de pago antes de declararlas cerradas.
8. **Sobrescritura de clientes (D25):** se aprobó únicamente en el TPV local. En
   producción viene desactivada por defecto.
9. **Stock (D26, D26c, D26e, D27):** la regularización, el ajuste y el inventario
   absoluto están probados en el BDP local (26/10/2026) salvo D27, que sigue
   pendiente de prueba local.

---

## 9. Estado de la instalación de producción (22 de octubre de 2026)

Comprobado en vivo contra `restaurante.wandori.us`, sin escribir nada en el TPV y
dejando puesta la parte de la puesta en marcha que no lo necesita.

**Lo que funciona hoy.**

- El TPV del restaurante responde desde dentro del servidor (salud correcta) y la
  conexión pasa las comprobaciones de lectura: sesión y versión, terminal,
  empleado, empleado asociado al terminal, formas de pago, departamentos y
  artículos del perfil. Hay **560 artículos** mapeados entre la web y el TPV.
- Las dos cuentas están en **solo lectura**. La cola de envío está vacía, no hay
  autorizaciones pendientes ni avisos acumulados, y el registro de auditoría no
  tiene ninguna escritura nueva.
- La prueba de contrato del TPV se ejecuta a propósito, desde su botón, y pide la
  comanda con la marca de «en seco» (`OnlyCheck`): el TPV la valida y no la
  guarda. Nada fuera de esa prueba escribe en el TPV.
- **Ya hay un snapshot completo del TPV**, hecho hoy (artículos, clientes,
  departamentos, salones y empleados de la conexión exacta). Caduca a las 24
  horas, así que se rehará el día de la puesta en marcha: es el que exige el
  modo automático.
- **Las cadencias de lectura ya están encendidas** a 15 minutos: artículos,
  clientes, plano de sala y departamentos se refrescan solos, en solo lectura.

**Lo que falta configurar para poder escribir.** Ninguna de estas cosas es un
defecto: son datos de la puesta en marcha que hoy no están puestos.

| Qué falta | Quién lo hace |
|---|---|
| **El mapeo de formas de pago.** Hoy no hay ninguno: los pagos saldrían sin identificar la forma de pago. Como mínimo, efectivo y tarjeta. | **El cliente** elige qué formas usa; se introduce en pantalla (lo hacemos nosotros con su lista) |
| **El mapeo de tipos de pedido.** Hoy no hay ninguno: todas las ventas se enviarían como «Barra». | **El cliente** elige si usa salón, mesa o recogida; se introduce en pantalla (lo hacemos nosotros con su lista) |
| **La cadencia de documentos de venta**, hoy apagada: no tiene perfil asignado y adivinar cuál es el de este restaurante leería documentos ajenos. Se enciende en la puesta en marcha, con el perfil confirmado por el cliente. | **Nosotros**, en la puesta en marcha, con el perfil que confirme el cliente |

Aparte, para información: la cuenta de demostración tiene la sincronización
apagada, que es lo correcto en una demo.

---

## 10. Estado final y siguiente paso

**La integración no está lista para el piloto.** El 22 de octubre este apartado decía lo
contrario por el número de funciones programadas; el criterio correcto son las pruebas
realmente superadas. Mientras sigan abiertas las correcciones del plan
`plan-correcciones-piloto-bdp-2026-10-26`, producción permanece en **solo lectura** y no
se ejecutan pagos, facturas ni stock en el restaurante.

Orden de trabajo:

1. **Estados corregidos** (§1) — hecho (`26AA-1`).
2. **Licencia:** activar WebLinkRestAPI de pago en el BDP local y repetir allí las
   pruebas afectadas; decisión del cliente y trato con BDP.
3. **Automatización:** comprobar en vivo la lectura programada de formas de pago y
   empleados (`26AA-2`). Los permisos automáticos por categoría, todos apagados de
   fábrica, están hechos (`26AA-3`).
4. **Pruebas locales:** D21 reformulada (`26AA-4`), concurrencia, reinicio, traspaso,
   batería de stock, factura completa y actualización de comandas (`26AA-5`).
5. **Dependencias externas:** D31 (terminal) y D32 (responsables aprobados).
6. **Evidencia:** una ficha por prueba y este informe actualizado (`26AA-6`).

Solo entonces se plantea la puesta en marcha de la sección 9 y, en ella, el orden de la
sección 5: solo lectura con las lecturas encendidas, escritura autorizada a mano
operación por operación y, por categorías, el modo automático con su confirmación, su
snapshot del día y el freno de «volver a solo lectura» siempre a mano.
