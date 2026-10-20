# Fase D BDP local — 10 Cobertura de fichas D01–D32

> Parte del [plan Fase D sobre BDP local](../plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 10. Cobertura del PDF (anexo B8, fichas D01–D32)

Cada ficha tiene fase. Estado a 2026-10-10: **ninguna superada**; «parcial» = ensayada en lectura. Fuente: `Agente/documentacion/bdp/alcance-fase-d-pdf-2026-10-10.html` y `Agente/planes/anexo-b8-aceptacion-2026-10-08.md` (rev. 10/10; 65 fichas, todas con fase asignada — comprobado el 2026-10-17).

| Fichas | Qué comprueba | Fase | Estado / dependencia |
|---|---|---|---|
| D01, D02 | Salud, versión, login, licencia 84; logs sin secretos | L2.1, L10 | Parcial (L2.1 OK; licencia 84 solo en el TPV real; `bdp_sync_cursor` ya existe, falta escribirlo desde el código) |
| D03, D03a | Crear comanda sin y con envío a cocina | L3.a, L3.a2 → L6–L9 | **D03 superada en L3.a (2026-10-17)**: comanda 15 en el BDP y en `Comandas_Cabecera`/`Comandas_Lineas` del TPV. D03a (envío a cocina, `OrderEndType=0`) pendiente de impresora |
| D04, D08 | Respuesta perdida al crear; toque simultáneo caja y web | L9 | F2 (base T2 lista) |
| D05–D07 | Modificar venta (añadir, quitar, cambiar líneas) | L3.b | **No soportado (2026-10-19):** la API WebLink no tiene modificación de líneas (punto 9) |
| D09, D09a | Anular desde la web, con efecto en cocina | L3.c | **D09 superada en L3.c (2026-10-17)**: la anulación llegó al BDP (`301000` en `Orders/Get`) y la comanda desapareció de `Comandas_Cabecera`. D09a (efecto en cocina) pendiente de testigo de impresora auxiliar |
| D10 | Anular en caja y reflejo en la web | L9 | Tras F2 |
| D11a–c | Propina: suma, sustitución, cero | L6.a | Tras L3, T2 y permiso |
| D12a–c | Puntos: saldo ±N, sin reintento ciego | L6.b | Pendiente de soporte del terminal |
| D13, D13a, D14, D14s, D15, D15t, D16, D17 | Pagos: un cobro por ID, consulta antes de repetir, tope de tres, pago de caja | L6.c | Precondición de suscripción (punto 12) |
| D18, D18a–d, D19, D19t, D20 | Facturas: número único, simplificada, `InvoiceOrder`, `Status=3`, `BillingDetails` | L6.d | Idem |
| D21, D21a | Siete avisos; traspaso de comanda completa | L4, L9 | El receptor de los siete avisos ya existe (F1, 2026-10-17); L4.2 (eventos reales) sigue pendiente de la usuaria en el TPV |
| D22, D22a | Aviso duplicado y fuera de orden; traspaso de línea | L9 | Receptor probado 2026-10-18 con cargas sintéticas (duplicado 200, conflicto 409, desorden 202; `avisos-local-matriz-2026-10-18.md`). Falta el disparo real en el TPV: sin acción de traspaso en `tpv.mjs` |
| D23a, D23b | Trabajador caído; receptor inaccesible | L9 | Tras F2 |
| D23-m | Comanda manual con aviso perdido | L9 | Decisión (punto 11) |
| D29 | Tres reinicios (sincronización, WebLinkRestAPI, BDP.NET) | L9 | El de BDP.NET lo hace la usuaria |
| D31 | Llamar al camarero | L9 | Tras F2 |
| D24a–d | Alta, modificación, desactivación y borrado en caja | L8 | Operación en TpvNet por la usuaria; lectura con cursor (tabla lista) |
| D24-p, D24-m | Plano, formas de pago y empleados | L8 | Posible ya (lectura) |
| D25 | Clientes: cambio en BDP, en la web (`Overwrite=true`), contradictorios | L8 | Decisión (punto 14) |
| D26a, D26b | `GetStock` y `GetListStock` | L7 | Parcial (`GetStock` OK en L2.1, stock 0) |
| D26, D26c–D26g, D27 | Escritura de stock, libro y ventas de caja | L7 | Permiso y datos del distribuidor (punto 13); tabla del libro lista |
| D28, D28-auto, D28-auto-R, D28a | Exportar documentos, perfil programado, reinicio, albaranes | L8 | Cursor ya disponible (tabla lista) |
| D32 | Mensaje de procedimiento ante cada operación que la API no expone | L10 | Código F2/F3 y lista de responsables por escrito |
| D30 | Piloto y despliegue por etapas | Fuera (F4) | Permiso aparte; requisitos en L10 |

**Base T2:** L5.0 (roadmap 09AA-2), hecha el 2026-10-17 (5 tablas; *outbox* ya era `bdp_push_pendientes`). **Despliegue por etapas del PDF** (observación → maestros → lectura → escritura → pagos/factura → stock): se ejecuta en F4, no aquí; aquí solo se deja probado el rollback en local. **Paradas del PDF** (duplicado económico, descuadre fiscal, pérdida de comanda, degradación del TPV): criterio de aborto de cualquier lote de L6–L10.
