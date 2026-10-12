# Respuesta a la guía de Guillermo (21/09/2026) — versión fácil para revisar

Este documento es PARA TI, para que lo leas y lo revises. Todavía no es lo que se envía al cliente. Explica, en palabras sencillas, qué nos corrige Guillermo, qué aceptamos y qué le vamos a responder.

## Lo que pasó (contexto en 4 líneas)

- Le mandamos a Guillermo una guía que decía qué se puede y qué no se puede hacer entre la web y el BDP.
- Él la leyó, revisó los manuales de BDP y nos devolvió otra guía diciendo: "en estos puntos te equivocas, sí se puede".
- Lo revisamos a fondo y resulta que **tiene razón en casi todo**.
- Este documento deja por escrito qué corregimos y qué le respondemos. Cuando lo apruebes, se convierte en el documento para el cliente.

## Dónde nos equivocamos (7 correcciones)

**1. Cambiar una comanda ya enviada.**
Antes dijimos: "no se puede, hay que anularla y crear otra".
Realidad: el manual dice que la versión gratuita lo bloquea (error 300041) y manda a la de pago. O sea, con la versión de pago SÍ se puede: se vuelve a enviar la comanda con los mismos datos y las líneas corregidas.
Lo que aún no sabemos: si al reenviar sustituye las líneas o las suma, ni si deja tocar comandas ya pagadas. Eso hay que probarlo.
Mientras no se pruebe, seguimos anulando y creando de nuevo.

**2. El buzón no lo guarda todo.**
Antes dijimos: "si la web está apagada, al volver se procesa todo y nada se pierde".
Realidad: el buzón solo guarda lo que llega. Si un aviso se pierde mientras la web está caída, BDP no lo reenvía (no está escrito en ningún documento que lo haga).
Lo correcto: el receptor tiene que estar siempre encendido, y además la web repasará cada cierto tiempo por si algo no llegó. Aun así, una comanda creada en caja cuyo aviso se pierda puede quedar invisible hasta que se finalice.

**3. Sin botón no significa con botón.**
Antes dijimos: "catálogo, clientes y stock seguirán obligatoriamente con botón".
Realidad: que BDP no avise de esos cambios no obliga a pulsar un botón. Se puede programar que la web pregunte sola cada cierto tiempo (traer lo modificado, comparar y actualizar).
Aceptado: así se hará.

**4. Las pruebas de céntimos NO se pueden deshacer.**
Antes dijimos: "pruebas con importe mínimo, todo reversible".
Realidad: BDP permite añadir pagos, pero no existe forma de borrarlos o devolverlos. Un pago de prueba, aunque sea de céntimos, se queda ahí.
Lo correcto: probar en comandas de mentira (desechables), con procedimiento aprobado, nunca en ventas reales.

**5. Formas de pago: solo se pueden leer.**
Nuestra tabla decía que se podían importar y exportar.
Realidad: solo se pueden consultar. Crearlas o cambiarlas se hace en el BDP; la web solo elige cuál usar en cada cobro.

**6. Sí existe consulta directa de artículo.**
En documentos internos habíamos escrito que no había forma de pedir un artículo concreto; releyendo el manual, sí existe (`GetArticle`). La usaremos para comprobaciones puntuales.

**7. Cobrar con suscripción: matiz.**
Guillermo dice: si el servicio de pago ya está activo, no es "pendiente de suscripción" sino "pendiente de prueba".
Realidad: a nosotros no nos consta que esté activo en el terminal, así que el primer paso es comprobarlo. Si está activo, pasamos directo a probar.

## En qué nos da la razón (no cambia nada)

Facturas rectificativas fuera, reservas/gastos/compras sin escritura, plano y empleados solo lectura, menús solo consulta, traspasos desde la web no documentados, sin lista de comandas abiertas, sin descarga directa del PDF de factura. Todo eso ya lo decíamos y él lo confirma.

## Lo que le vamos a responder (veredicto por punto)

1. Modificar comandas: **sí, con prueba previa** (sus pruebas A03–A05).
2. Repasos automáticos sin botón: **sí, con prueba de carga de noche** (que el terminal no vaya lento).
3. Stock desde la web: **sí, con prueba previa** (hay que crear el almacén 1 primero).
4. Pagos y facturas: **sí, con prueba previa y conjunta** (duplicados + enlace con factura real).
5. Avisos + reconciliación: **sí, así lo haremos** (receptor siempre encendido + repaso periódico).

## Lo que necesitamos pedir (para probar lo anterior)

- **Suscripción de pago activada y comprobada.** Sin ella no se puede probar nada.
- **Distribuidor:** crear almacén 1 y motivos de stock, dar el código del perfil de exportación, series de factura, conectar los 7 avisos y hacer copias antes de cada prueba.
- **BDP (vía Guillermo):** 10 preguntas concretas — cómo se actualiza una comanda (ejemplo real), si sustituye o suma líneas, qué identificadores enviar, unicidad de códigos, si el aviso salta al pagar en caja, reintentos de avisos, listado de comandas abiertas, campos de perfiles, devoluciones/rectificativas/PDF, y qué significa `OnlyLastMovements`. Ojo: el manual llega hasta la 35.1 y el entorno es 36.2; lo que diga la versión instalada manda.
- **Seguridad primero:** cambiar las credenciales que salieron en informes anteriores, guardar secretos fuera del código y no exponer el BDP a internet.
- **Restaurante y asesor:** aprobar series, fiscal, contingencia y turno piloto.

## Cómo queda la fase D (redefinida)

Ya no es "solo avisos": es dejar la web como herramienta diaria, con salida fiable al BDP, repasos automáticos y plan B para excepciones. Orden: seguridad → comprobar pago y versión → preguntas a BDP → entorno de pruebas → identidades y buzones (ante un fallo, primero se consulta y luego se decide; nunca reenviar a ciegas) → comandas → modificación → pagos → factura → receptor → reconciliación → repasos → stock → piloto.

**Regla de oro:** cero duplicados, caja cuadrada, todo visible en un panel. Si hay riesgo fiscal o de caja, vuelta a solo lectura y se trabaja en el TPV. Alcance final y presupuesto, tras las pruebas y antes de empezar: lo que no supere prueba, no entra.

## Decisiones pendientes (tuyas + del cliente)

- [ ] Fase D sí o no (tras las pruebas se fija alcance y precio).
- [ ] Pedir suscripción de pago + preguntas a BDP (sin esto no hay nada).
- [ ] Distribuidor: almacén, perfiles, series, avisos, copias.
- [ ] Fiscal y piloto aprobados.
