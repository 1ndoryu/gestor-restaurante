# Plan de verificación — Guía de Guillermo (8 bloques)

Fecha: 2026-10-05. Origen: `C:\Users\Owner\Downloads\Guia_operativa_paso_a_paso_Wan_BDP.docx`
(Guillermo, 5/10/2026) + nota de voz del mismo día.
Alcance propio: `Agente/usuario/alcance-fase-d-2026-10-02.md` (D3).
Nota: la guía salta de 2.1 a 2.3 (no existe 2.2 en el original).

## Checklist de ejecución

Estado: sin bloques verificados; cada bloque cierra con la tabla de veredictos de su subagente.

- [ ] B1 — Avisos e inbox
- [ ] B2 — Comandas abiertas y lista 36.2
- [ ] B3 — Conciliación y exportación automática
- [ ] B4 — Maestros automáticos y plano
- [ ] B5 — Pagos, propina, puntos y factura
- [ ] B6 — Stock sin doble movimiento
- [ ] B7 — Capacidades, formulación y operativa diaria
- [ ] B8 — Aceptación, piloto y condición final

## Lo que pide Guillermo
Revisar con calma su plan de acción, confirmar o corregir cada punto contradictorio
entre D3 y la documentación de BDP, y solo entonces ejecutar. Reconoce que él
también puede equivocarse. Espera un sí/no por punto, no un visto bueno genérico.

## Fuentes (códigos de la guía)
- D1: Manual WebLinkRestAPI entregado al proyecto (pp. citadas en cada bloque).
- D2: Avisos de Acciones V1, pp. 2–5. D3: Alcance Fase D 02/10 (pruebas reales).
- D4: Guía bidireccional. D5: portal BDP, mejoras 36.2 (lista de comandas).

## Método por bloque (vale para los 8)
1. **Verificar**: comprobar en D1/D2/D5 y en las pruebas D3 qué está documentado,
   comprobado, pendiente o no expuesto.
2. **Retar**: enfrentar lo que dice D3 contra lo que dice Guillermo; ninguna
   afirmación absoluta sin evidencia (regla anti-sesgo §3).
3. **Concluir**: qué corregir, qué ajustar, qué preguntar a BDP por escrito.
Entrega del subagente por bloque: tabla veredicto por afirmación
(confirmada / corregida / pendiente de BDP) + evidencia + ajuste propuesto a D3.

## Base transversal (T2 + §5, vale para los 8)
La Tarea 2 (tablas `bdp_event_inbox`, `bdp_outbox`, `bdp_id_map`,
`bdp_sync_cursor`, `bdp_dead_letter`, `bdp_stock_ledger`, IDs estables, estados
pendiente/confirmado/dudoso/error/agotado, "consultar antes de repetir" y
pantalla técnica) no es de un bloque: cada subagente la presupone y B8 la
verifica como prerrequisito. Las 4 acciones inmediatas de §3 quedan repartidas:
matriz → B7, preguntas 36.2 → B2, inbox/outbox/mapeos → transversal, 7 avisos → B1.
Regla de mantenimiento de la guía (§8): si documentación nueva de BDP la
contradice, prevalece la nueva; se actualiza la matriz y se repiten pruebas.

---

## B1 — Avisos e inbox (2.1, 2.4, T3)
- ¿Existen los 7 eventos? (estado cambiado, creada, modificada, borrada, línea
  traspasada, traspasada, finalizada) [D2, p. 3].
- ¿Documenta BDP cola persistente, reintento, acuse o reentrega? (Guillermo: no)
  [D2, pp. 2–5].
- ¿El buzón recupera un aviso enviado con el receptor caído? (D3 dice que no;
  prueba de caída del 02/10: solo llegó `Finalizar id11`, `Crear` perdido).
- ¿Qué trae cada aviso? (D3: solo señal + identificador, sin líneas/pagos).
- Retar: ¿basta inbox + conciliación, o hay reentrega real? ¿duplicado/fuera de
  orden corrompen?
- Cierre: diseño inbox exigido (guardar cuerpo → responder → procesar en fondo,
  estados recibido/procesando/completado/error/agotado).

## B2 — Comandas abiertas y lista 36.2 (2.3, T1, T5)
- ¿`Orders/Get` enumera abiertas o solo recupera una conocida? [D1, pp. 140–141].
- ¿Existe la "lista de comandas" de 36.2? Ruta, método, parámetros, JSON,
  estados y alcance por terminal (D5 la anuncia, D1 no la describe).
- ¿Cubre abiertas, aparcadas, todos los terminales, manuales, para llevar,
  traspasadas?
- Retar: D3 asume "lista propia web cuadrada contra cerrados"; Guillermo exige
  demostrar cómo se descubre una comanda con aviso perdido o declarar límite.
- Cierre: preguntas escritas a BDP (Tarea 1) o límite formal documentado.

## B3 — Conciliación y exportación automática (2.5, 2.6, T7)
- ¿La exportación manual probada (perfil 1 `Fra. Simpl. Barra`, `_CTB/_LTB/_PTB`
  con cabeceras/líneas/pagos) equivale a automática? (Guillermo: no)
  [D1, pp. 186–197].
- ¿`ExportProfiles/Documents` invocable desde código? ¿`OnlyLastMovements`
  true/false qué hace realmente?
- ¿Basta repaso diario? (Guillermo: no; propone 30–60 s abiertas, 1–5 min
  cerrados, 5–15 min maestros, barrido nocturno). ¿Degrada el TPV?
- ¿`Status 3` + `Order` vacío = ya facturada? ¿`InvoiceNumber` enlaza aviso
  finalizada ↔ exportación?
- Retar: recuperación real de un cierre con aviso bloqueado, sin pulsar botones.
- Cierre: frecuencias acordadas y medidas + cursor/deduplicación documental.

## B4 — Maestros automáticos y plano (2.7, T4)
- ¿Existen y qué devuelven? `Articles/Export` o `GetFullList`,
  `Customers/Export`, `Departments/Export`, `Employees/Get` o
  `POS/Employees/Get`, `Tenders/GetList` o `GetPOSList`, `Rooms/GetTables`
  [D1, pp. 29, 69, 131, 150, 177–181, 230–232].
- ¿La API entrega coordenadas gráficas del plano? (Guillermo: no demostrado →
  la web las conserva).
- ¿Alta, cambio, desactivación y borrado lógico por recurso?
- Retar: D3 dice "se traen igual que ahora"; Guillermo exige mecanismo
  programado + conservar posición al renombrar mesa.
- Cierre: autoridad por campo, `bdp_id_map`, `bdp_sync_cursor`, editor de plano
  propio de la web.

## B5 — Pagos, propina, puntos y factura (2.8, T6)
- ¿`Payment/Add` admite ID local único? ¿`PaymentId` evita duplicado tras
  timeout? (pregunta a BDP, Tarea 1).
- ¿`Orders/Tip/Add` suma o sustituye según `AddTip`? [D1, p. 143].
- ¿`Loyalty/AddPoints` reintentado a ciegas duplica? ¿`GetPoints` + saldo
  antes/después basta? [D1, pp. 169–171].
- ¿Facturar una vez y guardar `InvoiceNumber` enlaza OrderId + aviso + exportación?
- Retar: timeout de pago + doble clic sin cobro doble; propina y puntos con
  flujos/tablas separados.
- Cierre: estados dudosos + "consultar antes de repetir" en toda operación económica.

## B6 — Stock sin doble movimiento (2.9, T8)
- ¿Qué operaciones son necesarias en v1? (consulta, regularización, traspaso,
  actualización, inventario) [D1, pp. 198–217].
- ¿Hay clave general de idempotencia? (Guillermo: no).
- ¿Una venta procesada por BDP ya descontó stock? → no re-descontar al sincronizar.
- Retar: timeout/doble clic, dos movimientos concurrentes mismo artículo/almacén,
  almacenes y motivos reales (dato del distribuidor, pendiente).
- Cierre: BDP maestro de stock + `bdp_stock_ledger` (ID único, artículo, almacén,
  unidades, motivo, estado, verificación) + serializar por artículo/almacén.

## B7 — Capacidades, formulación y operativa diaria (2.10, 2.11, T9)
- Reformular cada "BDP no tiene esa operación" (borrar, rectificativas,
  devoluciones) a los 4 estados: documentado / comprobado / pendiente de BDP /
  no expuesto confirmado.
- ¿La web puede iniciar traspaso de mesa/línea? (recibir el aviso ≠ poder iniciarlo).
- Cubrir: apertura/cierre turno-caja, arqueo, efectivo, factura simplificada y
  completa, rectificación, devolución, PDF, series, cocina, reimpresión, cambios
  tras cobro, mesa/barra/llevar.
- Retar: cada excepción con responsable (Guillermo, administración, distribuidor,
  BDP) y procedimiento; nada al propietario sin acuerdo; validación del
  propietario antes del piloto.
- Cierre: matriz de capacidades + matriz de operaciones con evidencia o pregunta.

## B8 — Aceptación, piloto y condición final (T10, T11, A01–A20, §6–§7)
- ¿Qué casos A01–A20 son verificables en local con la serie `PRUEBA`?
  (reutilizar tickets 1–3 `00001TB`/`PRUEBA`, receptor `C:\tmp\receptor-bdp.ps1`,
  caídas ya probadas).
- Objetivos de arranque: aviso en segundos, detalle 30–60 s, cerrado sin aviso
  1–5 min, maestros 5–15 min, barrido nocturno. ¿Medibles sin degradar TPV?
- Piloto: modo observación → etapas (maestros, lectura, comandas, pagos/factura,
  stock último); frenar ante duplicado económico, descuadre fiscal, pérdida de
  comanda o TPV degradado; rollback probado; **no actualizar BDP solo por la
  integración**.
- Retar: "no basta que funcione una vez: cada caso crítico incluye fallo y
  recuperación".
- Cierre: hoja de resultados + lista de límites + aceptaciones por escrito
  (Guillermo y propietario).
- Asignación función→prueba (matriz D01–D32 del alcance; cada prueba define
  esperado / qué se compara / qué error la detiene / evidencia en T2; fichas
  completas en el anexo B8):
  catálogo y departamentos→D24; clientes→D25; plano→D24-p; formas de pago y
  empleados→D24-m; detalle→D27; documentos→D28+D28a; albaranes auto→D28a;
  exportación auto→D28-auto; crear→D03+D04+D03a;
  propina→D11; puntos→D12; anular→D09+D10+D09a; modificar→D05–D08; detectar
  traspaso→D21+D22+D21a+D22a; cobrar→D13–D17+D13a+D15t;
  facturar→D18–D20+D18a–D18d+D19t; stock→D26+D26a–D26g;
  llamar→D31; excepciones→D32; caídas y reinicios→D23a+D23b+D23-m+D29
  (transversal); piloto→D30. Iniciar
  traspaso y borrar artículos, sin prueba de ejecución (pendientes de BDP,
  cubiertos por D32); reservas, fuera
  de alcance.
- D24-p (plano): lectura Rooms/GetTables; la web conserva las coordenadas;
  compara mesas/nombres sin mover posición; detiene pérdida de posición;
  evidencia diff + cursor.
- D24-m (formas de pago y empleados): lectura Tenders/GetList + Employees/Get;
  compara listas; detiene alta fantasma; evidencia diff.
- D31 (llamar): notifica en caja sin dejar estado; compara aviso vs TPV;
  detiene silencio; evidencia log + hora.
