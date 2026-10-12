# Plan 01AA-1 — Alcance real con el TPV local licenciado (activo 2026-10-01)

> Origen: TPV 36.2 local licenciado (`122417` + `751080-84`) tras el fix de
> registro (`Agente/documentacion/bdp/error-mac-puenteada-2026-10-01.md` §19).
> Propósito: cumplir lo pedido por el cliente en
> `Agente/usuario/borrador-revision-guia-bidireccional-2026-09-19.md` (fase D):
> resolver sus 8 dudas con pruebas locales y fijar el alcance real, sin esperar
> a la noche ni al entorno de desarrollo. El usuario no maneja el BDP: este plan
> incluye configurarlo y guiarlo paso a paso en sus pantallas.

> ## Modo de ejecución: herramienta de control de escritorio (2026-10-01)
>
> El usuario conectará una herramienta que permite al agente ver la pantalla y
> manejar aplicaciones Windows (captura + clic/teclado como mínimo). Cuando esté
> disponible, el agente opera directamente las pantallas del TPV; el usuario
> solo supervisa y autoriza. Hasta entonces vale el modo guiado (el usuario
> opera, el agente dirige). Cada acción sobre el TPV queda registrada con su
> captura de evidencia.

## Objetivo

Determinar empíricamente qué puede y qué no puede la integración contra el TPV
local (matriz + respuestas a las 8 dudas del borrador), para cerrar el alcance
de la fase D. Todo reversible; nada toca ventas reales ni caja del día.

## No alcance

Pagos/facturas reales salvo el mínimo P3 (serie de pruebas) · prueba de carga
(duda 6 → noche) · entorno de desarrollo Tailscale (después de fijar alcance) ·
rectificativas/devoluciones (sin operación API: fuera, ya decidido) · código de
fase D (empieza tras este plan).

## Dependencias

- Distribuidor: serie de pruebas para facturar, almacén/motivos reales, series,
  perfiles, avisos (ruta base, ruta por aviso, método). Sin esto, P0 parcial.
- Herramienta de control de escritorio (pendiente de conexión): sin ella, modo
  guiado con el usuario operando.

## Fases

### P0 — Configurar el BDP local (agente en pantalla; guiado si no hay herramienta)

- [ ] Inventario: serie de facturación actual (¿prueba o fiscal?), almacén,
      motivos, perfiles, avisos configurados. Registrar valores reales.
- [ ] Pedir al distribuidor lo que falte (serie pruebas, almacén, perfiles,
      avisos). Sin serie de pruebas no hay P3.
- [ ] DoD: checklist de configuración archivado en este plan.

### P1 — Pruebas solo API, reversibles (agente)

- [ ] T1 Modificar comanda: añadir línea / cambiar unidades / retirar línea
      → cierra duda 2 (¿sustituye o suma?) y A05–A07.
- [ ] T2 Detalle de comanda → cierra fila "Detalle de comanda".
- [ ] T3 Lista de comandas abiertas → cierra duda 5 (o documenta el límite).
- [ ] T4 Llamar al camarero → cierra esa fila.
- [ ] Todo con comandas `PRUEBA-*`, anuladas al terminar (`CancelOrder`
      verificado 2026-09-18). DoD: tabla resultado por prueba.

### P2 — Pruebas en pantalla + receptor local (agente; guiadas si no hay herramienta)

- [ ] T5 Avisos a receptor de pruebas en esta máquina; crear/modificar/anular/
      traspasar comanda de prueba → qué trae cada aviso (duda 1, A21).
- [ ] T6 Receptor devuelve error; repetir cambio → ¿reintenta o se pierde?
      (duda 3). Nunca cortar la red del TPV.
- [ ] T7 Perfil de exportación + almacén: si hay almacén válido, un movimiento
      (duda 4, stock); si no, dependencia del distribuidor.
- [ ] DoD: payloads capturados + veredicto por duda.

### P3 — Mínimo pagos/facturas (solo con serie de pruebas)

- [ ] Un pago parcial + un pago total + una factura, importes mínimos en
      efectivo. Cierra pendiente roadmap #1 (2.3/2.4) y A13–A14/A18.
- [ ] Prohibido sin serie de pruebas confirmada. DoD: nº factura + auditoría.

### P4 — Cierre y manejo

- [ ] Actualizar la tabla del borrador con lo demostrado (Sí/Pendiente/No).
- [ ] Alimentar F0 de 199A-1 (matriz empírica) con estos resultados.
- [ ] Mini-guía para el usuario: pantallas tocadas en este plan, qué no tocar.
- [ ] DoD: borrador actualizado + 199A-1 F0 alimentado + guía entregada.

## Estado

2026-10-01: creado. Siguiente paso: P0 (inventario de configuración en pantalla).

## Bitácora de ejecución (herramienta de escritorio)

> **Entorno: TPV local del portátil (pruebas).** Todos los valores de esta
> bitácora (series, almacenes, contadores) son de la base local
> `Bdp-Net_NetHos_36` del portátil, NO de la caja real del restaurante. Los
> datos visibles (p. ej. almacenes BAR/CAFETIN/COCINA) podrían venir de una
> copia de configuración real instalada por el distribuidor: tratarlos como
> datos de prueba igualmente.

- 2026-10-01: daemon elevado en `\\.\pipe\opencode-pc-admin` (UAC aceptada por
  el usuario); el daemon normal no toca el TPV (UIPI High-IL). Mapa completo
  del menú en `Agente/documentacion/bdp/mapa-menu-tpv-2026-10-01.md`; notas de
  uso en `Agente/documentacion/bdp/control-escritorio-2026-10-01.md`.
- P0: ventana SERIES abierta (Ficheros → Series [3] por token). Muestra
  registro vacío (contadores a 0, sin nº de serie). Clic en `Ini` sin efecto
  visible: pendiente confirmar si la tabla está vacía (vía F11-búsqueda o SQL)
  o si el clic por coordenadas falla (ver notas, §Clics pendientes).
  Evidencias: `C:\tmp\pc\series*.png`, `menu.md`.
- P0 series: `Series` (compras) = solo `S1 "Serie por defecto"`, contadores
  0/0/0, sin descripción. `Series TPV` (ventas) = solo `00001TB "1T Factura
  Simplificada"`, contador `Número de Documento = 0`, `IVA Incluido` marcado,
  ningún tipo visible marcado. Conclusión: **no hay serie de pruebas**; P3
  bloqueado hasta que el distribuidor confirme serie de pruebas o autorice
  `00001TB`   (fiscal — no emitir sin su OK).
- P0 almacenes: 6 definidos (`1 ALMACEN GENERAL`, `6 BAJAS MATERIAL`, `4 BAR`,
  `3 CAFETIN`, `2 COCINA`, `5 LAVANDERIA`). Hay configuración real de
  restaurante: T7 viable sin depender del distribuidor para el almacén.
- P0 motivos de traspaso: 4 (`3 BAJAS PERDIDA O ROTURAS`, `2 REGULARIZACIÓN
  STOCK`, `1 TRASPASO OTROS ALMACENES`, `4 TRASPASO VENTAS`). Siguiente: avisos
  [358] y perfiles/exportación [256].
- P0 avisos [358]: ventana vacía (sin puntos de exportación ni acciones).
  Se quedó modal por una fila en edición (vetaba el cierre); cerrada a mano
  por el usuario. Lección en control-escritorio: cancelar la operación antes
  de cerrar cualquier ventana BDP.
- P0 exportación [256] (Parámetros de Exportación): sin perfil (código vacío,
  ruta `C:\Bdp-Net\NetHos\Temp`, todo desmarcado). T5 exigirá crear un perfil
  (dato reversible).
- P0 formas de pago [6]: 1 Efectivo, 2 Tarjeta, 3 Cheque, 4 A 30-60-90
  (listados en la Consulta de F11; el formulario abre en código 0 aunque haya
  datos). P3 viable con Efectivo/Tarjeta.
- Pendiente P0: Parámetros del Terminal [62], Empleados por Terminal [63],
  perfiles. Freno actual: PC en uso por el usuario (el input sintético compite
  con su ratón); `invoke_menu` abre la ventana pero la llamada no retorna
  (usar como fire-and-forget y verificar con `list_windows`). Quedan abiertas
  FORMAS DE PAGO y su Consulta.
- P0 terminal [62] (Parámetros del Terminal): existe `1 TERMINAL 1`
  (Consulta F11; Ant/Sig sobre formulario vacío no hacen nada). Generalidades:
  sin bloqueo, cierre caja 0, Local `1 LOCAL 1`, Almacén salida `1 ALMACEN
  GENERAL`, comandas externas 0, motivo regularización 0, impresoras 0,
  monitor entregas off, avisos barra/mesas sin marcar. Pestañas Facturas/
  Parámetros no conmutan con clic (ni Ctrl+Tab); pendiente si hace falta.
  Evidencias: `C:\tmp\pc\termin*.png`.
- P0 empleados x terminal [63]: Terminal 1 = TERMINAL 1, modo "Sólo los
  siguientes", 3 empleados (1 Camarero 01, 2 Camarero 02, 3 Camarero 03),
  orden por código. Cerrada con Escape limpio. Evidencia:
  `C:\tmp\pc\empterm_t1.png`.
- P0 perfiles = perfiles de exportación [256]: ninguno creado (ver arriba).
- **P0 COMPLETO.** Solo queda MENÚ GENERAL abierto. Siguiente: P1 (T1–T4
  API reversible con comandas `PRUEBA-*` + receptor local; T5/T6 sin ejecutar).
- P1 serie PRUEBA creada (con ayuda del usuario): `SeriesTpv` =
  `PRUEBA "serie de pruebas"`, `Tipo_Documento=0` (simplificada), contador 0,
  IVA incluido, `Tipo_Iva=2`, diseños `TIBDRV080.dis`. Guardar exigió:
  marcar tipo + `Código IVA` válido (0→2) + `Diseño Facturas` y `Diseño
  Comprobante` = `TIBDRV080.dis` (error previo: `Tipo_Documento/Diseño/Tipo_Iva
  incorrectos`). Radio inaccesible por coordenadas (objetivo ~10px); lo marcó
  el usuario a mano. SQL: instancia `.\BDPSOFTWARE`, BD `Bdp-Net_NetHos_36`;
  `TerminalesSeries`: terminal 1, TipoMov 0/1 → `00001TB`, 2/3/4 vacíos.
- P1 ticket de prueba cerrado por el usuario en Tpv-Táctil (Camarero 01,
  turno 1, barra): `CAFÉ SÓLO 0,75 €` en Efectivo, Cerrar Sin Imprimir.
  Resultado en BD: `Documentos_Cabecera` Serie=`00001TB` Numero=1 Tipo=1,
  Total 0,75, IVA1=10 %, `NumImpresiones_Comprobante=0`, `Facturado=False`
  (+1 línea +1 pago). **BDP no emitió nada**: `C:\Bdp-Net\NetHos\Temp`
  vacío, sin QR fiscal, `ExportacionInstantaneaTickets` vacía,
  `Comandas_Cabecera`/`Caja_Lineas` a 0. Serie PRUEBA sin usar (el terminal
  auto-asigna 00001TB sin preguntar). Pendiente: cambiar terminal 1 a PRUEBA
  (SQL reversible o menú) y definir receptor = lectura de
  `Documentos_Cabecera` o perfilar exportación instantánea.
- `TerminalesSeries` cambiado a `PRUEBA` (mov 0/1, reversible a `00001TB`):
  el próximo ticket debe salir como `PRUEBA nº1`.
- Confirmado: el TPV cachea el mapeo al abrir turno (tickets 2-3 siguieron en
  00001TB); tras Salir + reentrar + Aceptar turno, ticket 4 = `PRUEBA nº1`
  (0,75 €, IVA 10 %, sin imprimir, sin facturar). Circuito de prueba OK.
- Receptor mínimo verificado: `C:\tmp\receptor-bdp.ps1` (poll 3 s a
  `Documentos_Cabecera`, log en `C:\tmp\receptor-bdp.log`) anunció solo
  `PRUEBA nº3` al cerrar el ticket. Proceso detenido tras la prueba
  (cleanup OK). Lección: los `Start-Job` no sobreviven entre llamadas bash
  (cada una es un proceso nuevo): para fondo usar `Start-Process ... hidden`
  + fichero `.pid` + `Stop-Process`.
- Prueba "facturar" cerrada sin clics: la serie tipo 0 = Facturas
  Simplificadas, el ticket YA es la factura (no hay botón Factura en el
  táctil; `Comprobante` solo imprime). `Facturado=False` = no incluido en
  factura recapitulativa, no un paso pendiente. `FacturasVenta_*` a 0 por
  diseño. `invoke_menu` con táctil delante falla (`menu_path_unavailable`
  seg.1); sesiones con nombre caducan: llamar sin `session`.
- `TerminalesSeries` devuelto a `00001TB` (mov 0/1) tras las pruebas
  (aplica al reabrir turno, por la caché). Serie PRUEBA conservada.
