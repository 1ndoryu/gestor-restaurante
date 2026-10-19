# Fase D BDP local — 05 Fases L6–L10

> Parte del [plan Fase D sobre BDP local](plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 5. Fases (continuación)

### L6 — Pagos, propina, puntos y factura (D11a–D20)

Precondición: L3 OK, L5.0 hecho (ID de pago estable e idempotencia en T2), caja abierta, serie `PRUEBA`, suscripción de `Payment/Add` e `InvoiceOrder` confirmada en el TPV local (licencia DEMO por verificar) y permiso por lote. Pérdida de respuesta y timeout (D15, D15t, D19, D19t, D12c) se simulan con un proxy local en `127.0.0.1` (en `C:\tmp`, se para al terminar) que corta la respuesta tras reenviar: no se toca el TPV.

| Lote | Fichas | Qué se comprueba |
|---|---|---|
| L6.a Propina | D11a, D11b, D11c | `AddTip=true` suma; `AddTip=false` sustituye; a cero sin residuo |
| L6.b Puntos | D12a, D12b, D12c | Saldo ±N con ID único; sin reintento ciego de `AddPoints`. Pendiente de soporte en el terminal: si no lo soporta, se declara. **Resultado 2026-10-19:** D12a (+5 → 5,00) y D12b (−2 → 3,00) OK. D12c: respuesta perdida tras aplicar +1 (BDP 3→4); el flush reconcilió (`GetPoints` 4.0 = ledger 4.00), no reenvió y dejó la fila `sincronizado` (commit `72ec58b`). Límite: `encolar` reemplaza el payload de una fila activa del mismo cliente; dos sumas antes del flush pierden importe (tarea aparte) |
| L6.c Pagos | D13, D13a, D14, D14s, D15, D15t, D16, D17 | Un solo cobro por ID; consulta antes de repetir; tope de tres pagos; pago de caja visible tras el repaso |
| L6.d Facturas | D18, D18a, D18b, D18c, D18d, D19, D19t, D20 | Número único; simplificada al cobrar; `InvoiceOrder` solo sobre no facturada; guarda `Status=3`; `BillingDetails`; enlace aviso-documento |

- **Resultado L6.c (2026-10-19, venta 26 en BDP local, GetOrder solo lectura):**
  - D13 (parcial 4,00) y D13a (parcial 2,00, sin factura) OK. D13a dejó ver un defecto: `InvoiceNumber: ""` marcaba la venta como facturada; corregido en `bdp_sync_pago.rs` (`numero_factura`), commit `2657ab9`.
  - Desfase de saldo 3,00 frente a 4,00: BDP tiene `Total` 10,00 (sin propina); el ledger local suma 11,00 con la propina de 1,00. El validador remoto usa 10,00 (correcto para BDP); el local, 11,00. La propina no llega a BDP: es D11c, pendiente.
  - D14 (pago total 3,00): OK. BDP queda con 4 pagos y `Total` 10,00 pagado. La repetición de 1,00 la rechaza BDP (`excede el saldo pendiente 0.00`) y no crea fila `exito`. Nota: el validador local aún cuenta 1,00 pendiente por la propina.
  - D14s (misma clave en paralelo): una petición entra y la otra recibe `Ya hay un pago en proceso` (422). Sin cobro doble: BDP sigue con 4 pagos.
  - D17 (cuarto pago): **defecto encontrado y corregido.** El 3,00 de D14 fue el cuarto cobro porque no había tope. Se añadió `MAX_PAGOS_VENTA = 3` en `reconciliar_orden_pago` (`tope_pagos_alcanzado`, test). Verificado: el cuarto (1,00, clave nueva) devuelve `la venta ya tiene 3 pagos registrados en BDP` (422). La venta de prueba queda con 4 pagos y no se corrige.
  - Pendiente: D15 (respuesta perdida), D15t (timeout) y D16 (pago de caja). Necesitan proxy que corte la respuesta y una venta nueva con saldo en BDP; la venta 26 ya no tiene saldo remoto.

### L7 — Stock (D26–D27)

- **Lectura (ya posible):** D26a (`GetStock`, incluido almacén inexistente) y D26b (`GetListStock`: artículo inexistente solo falla su línea; almacén inexistente). `GetStock` está probado en L2.1 (stock 0).
- **Escritura (D26, D26c–D26g, D27):** precondición L5.0, ya cumplida (libro de movimientos y bloqueo por artículo y almacén), permiso por lote y los datos del distribuidor que el PDF deja pendientes (cuál de los 6 almacenes vistos en local es el real; motivos válidos). Las ventas de caja también mueven stock: D27 se prueba con una venta web convergida, y un valor ajeno no se trata como error sin comprobar si viene de una venta.
- D26b, D26f y D26g (`Units` null, almacén inexistente, fiabilidad de `GetStock` tras escribir) se ensayan en local pero no se dan por aprobadas hasta confirmarse en el BDP real.
- **Resultado del ensayo L7 (2026-10-19, almacén 1, BDP local 8070):**
  - Causa del fallo anterior: el ajuste se encolaba pero **nada lo enviaba**. El flush solo corre por `POST /api/bdp/push/flush` (Admin); no hay worker automático. Además, `encolar` reemplazaba el payload: dos ajustes ±1 dejaban solo el último. Corregido con `acumular_regularizacion` (`cola.rs`) y su uso en `ajustar_stock` (`stock.rs`).
  - El −1.0 de BDP ya existía antes de L7 y el stock local era 0. Decisión de la usuaria: corregir la cola y empujar +1. Se dejó la fila de 1001 en +1 y se disparó un único armado (`unidirectional`, `regularize_stock`, 1 operación, 10 min) con flush: `sincronizados: 1`, `GetStock` 1001/almacén 1 pasó de −1.0 a 0.0, igual que local. Desarmado con 200.
  - Limpieza de la cola de prueba: `90000003` (ARTÍCULO NO EXISTE) y `ZZPRUEBA99` (IVA 0) pasaron a `descartado`.
  - Pendiente: el `Store` del payload sale de `bdp_almacen_default`, no del `warehouse_id` pedido. Coincide con el almacén 1 solo porque el default es 1; corregirlo antes de usar otro almacén.

### L8 — Maestros y documentos (D24–D25, D28)

- **Plano, formas de pago y empleados (D24-p, D24-m):** lectura con `Rooms/GetTables`, `Tenders/GetList`, `Employees/Get`; ya posible con la app; criterio: sin altas fantasma.
- **Alta, modificación, desactivación y borrado en caja (D24a–D24d):** la usuaria opera en TpvNet; la app lee sin botón (necesita lectura periódica con cursor de T2). Si la API no expone el borrado, se verifica el mensaje de D32.
- **Clientes (D25):** (a) cambio solo en BDP, (b) solo en la web (`CreateCustomer` con `Overwrite=true`: requiere aprobación expresa de la usuaria; sin ella el caso se declara no soportado), (c) cambios contradictorios con conflicto visible.
- **Documentos (D28, D28-auto, D28-auto-R, D28a):** perfil de exportación programado en el TPV local; invocar `ExportProfiles/Documents` desde código, `OnlyLastMovements` true/false, ventanas solapadas, reinicio a mitad y albaranes por `ExportPurchaseNotes`. Precondición: L5.0, ya cumplida (cursor).

### L9 — Avisos duplicados, caídas y reinicios (D04, D08, D10, D21–D23, D29, D31)

Precondición: L4 hecho y F1–F2 implementados (L5.0 ya está) (receptor, inbox y worker).

- **D21, D21a:** se ensayan ya en L4 (matriz de 7 eventos y traspaso de comanda completa con `GetOrder`); aquí se repiten con el receptor definitivo de F2 y su log por tipo.
- **D10:** anular en caja una venta vinculada; el aviso `Cancelar` deja la venta anulada en la web (estado en ambos lados).

- **D22, D22a:** cada uno de los 7 avisos normal, duplicado y fuera de orden, con los payloads reales de L4 (repetición desde el log). Traspaso de línea (D22a) con origen y destino consultados con `GetOrder`.
- **D23a, D23b:** trabajador caído con receptor operativo; receptor inaccesible. D23b solo recupera lo localizable y declara el límite.
- **D23-m:** comanda manual con aviso perdido. No hay lista de abiertas confirmada: se resuelve por listado si existe o por límite formal declarado. La pregunta a BDP la decide la usuaria (§9 punto 11).
- **D04, D08:** respuesta perdida al crear (consulta antes de repetir, comanda única en `bdp_id_map`); toque simultáneo caja y web (conflicto visible, sin pisar).
- **D29:** tres reinicios en pruebas separadas: proceso de sincronización, WebLinkRestAPI o su servicio, BDP.NET. El de BDP.NET local lo hace la usuaria (TpvNet elevado). En cada uno: alerta, cola, cursor, reanudación y conciliación.
- **D31:** llamar al camarero desde una mesa vinculada; avisa en caja sin dejar estado.

### L10 — Excepciones, accesos y piloto (D01, D02, D30, D32)

- **D01, D02:** salud, versión y login ya verificados en L2.1; el registro formal (`bdp_sync_cursor`) tiene ya su tabla (L5.0), falta escribirlo. La licencia del TPV local es DEMO: la comprobación de la licencia 84 se hace en el TPV real. D02: revisión de logs sin secretos legibles en L1.4 y al cierre.
- **D32:** mensaje con procedimiento y responsable ante cada operación que la API no expone (traspaso desde la web, borrar artículos, rectificativas, devoluciones, apertura y cierre, arqueo, efectivo, PDF, reimpresión en cocina, controles de cocina, series, cambios tras el pago). Es código de la web (F2/F3) más la lista de responsables, que asignan Guillermo y el propietario por escrito.
- **D30 piloto y despliegue por etapas:** fuera de este plan (F4, permiso aparte). Requisitos antes de abrir F4: todas las fichas superadas con su evidencia, responsables de las excepciones, umbral de rendimiento y rollback por escrito.
