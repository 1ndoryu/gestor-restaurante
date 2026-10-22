# Fase D BDP local — 10 Cobertura de fichas D01–D32

> Parte del [plan Fase D sobre BDP local](../plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 10. Cobertura del PDF (anexo B8, fichas D01–D32)

Cada ficha tiene fase. Estado a 2026-10-10: **ninguna superada**; «parcial» = ensayada en lectura. Fuente: `Agente/documentacion/bdp/alcance-fase-d-pdf-2026-10-10.html` y `Agente/planes/anexo-b8-aceptacion-2026-10-08.md` (rev. 10/10; 65 fichas, todas con fase asignada — comprobado el 2026-10-17).

| Fichas | Qué comprueba | Fase | Estado / dependencia |
|---|---|---|---|
| D01, D02 | Salud, versión, login, licencia 84; logs sin secretos | L2.1, L10 | **D01 cerrado el 2026-10-20** (HTTP 200, versión 36.2, fila `salud` en `bdp_sync_cursor`). **D02 cerrado el 2026-10-21** (rotación con la contraseña actual, `auth_intentos` y tarjeta en Configuración). Licencia 84: solo en el TPV real, pendiente |
| D03, D03a | Crear comanda sin y con envío a cocina | L3.a, L3.a2 → L6–L9 | **D03 superada en L3.a (2026-10-17)**: comanda 15 en el BDP y en `Comandas_Cabecera`/`Comandas_Lineas` del TPV. D03a (envío a cocina, `OrderEndType=0`) pendiente de impresora |
| D04, D08 | Respuesta perdida al crear; toque simultáneo caja y web | L9 | F2 (base T2 lista) |
| D05–D07 | Modificar venta (añadir, quitar, cambiar líneas) | L3.b | **No soportado (2026-10-19):** la API WebLink no tiene modificación de líneas (punto 9) |
| D09, D09a | Anular desde la web, con efecto en cocina | L3.c | **D09 superada en L3.c (2026-10-17)**: la anulación llegó al BDP (`301000` en `Orders/Get`) y la comanda desapareció de `Comandas_Cabecera`. D09a (efecto en cocina) pendiente de testigo de impresora auxiliar |
| D10 | Anular en caja y reflejo en la web | L9 | **Cerrado el 2026-10-21, en vivo**: comanda 46 del BDP local → `cancelled`/`anulada` con fecha y motivo; un aviso repetido no lo pisa; el sondeo también anula |
| D11a–c | Propina: suma, sustitución, cero | L6.a | **Cerrados el 2026-10-21 con lectura entre pasos** (comanda abierta 58, venta `3aa9c8b9`): 0,00→2,00 con `AddTip=true` (`Total` 10→12); `false` fija 3,00 y luego 5,00 (no suma); 0,00 sin residuo. La conclusión previa de que BDP no suma la propina venía de leer una comanda anulada (`Order: null`) |
| D12a–c | Puntos: saldo ±N, sin reintento ciego | L6.b | **Cerrados el 2026-10-19**: D12a (+5 → 5,00) y D12b (−2 → 3,00) contra el BDP local; D12c (respuesta perdida tras aplicar +1) reconcilió por `GetPoints` sin reenviar. El límite anotado entonces —`encolar` reemplazaba el payload y dos sumas antes del flush perdían importe— quedó cerrado con `acumular_puntos`, que suma deltas sobre la fila activa y descarta el neto cero (commit `05a9983`, 2 tests en `src/services/bdp_push/cola.rs`) |
| D13, D13a, D14, D14s, D15, D15t, D16, D17 | Pagos: un cobro por ID, consulta antes de repetir, tope de tres, pago de caja | L6.c | Precondición de suscripción (punto 12) |
| D18, D18a–d, D19, D19t, D20 | Facturas: número único, simplificada, `InvoiceOrder`, `Status=3`, `BillingDetails` | L6.d | Idem |
| D21, D21a | Siete avisos; traspaso de comanda completa | L4, L9 | El receptor de los siete avisos ya existe (F1, 2026-10-17); L4.2 (eventos reales) sigue pendiente de la usuaria en el TPV |
| D22, D22a | Aviso duplicado y fuera de orden; traspaso de línea | L9 | D22 probado el 2026-10-18 con cargas sintéticas (duplicado 200, conflicto 409, desorden 202). **D22a superada a medias el 2026-10-21 (API)**: el aviso `traspasar-linea` **real** de la comanda 47 se reenvió al receptor —idéntico → 200 `duplicado`; con `TimeStamp` anterior → 202 y `procesado`—. Falta el testigo de la línea movida (`GetOrder` de origen y destino): la comanda 47 está facturada y el WebLink devuelve `Order:null` para cerradas. El disparador existe: `scripts/pc/tpv.mjs traspasar-linea <mesa> [salón]` |
| D23a, D23b | Trabajador caído; receptor inaccesible | L9 | **Cerrados**: D23a (`6c34b2c`, buzón procesado al arrancar) y D23b (2026-10-21, ventas «en vuelo» reconciliadas, 5/5 tests y corrida viva) |
| D23-m | Comanda manual con aviso perdido | L9 | **Cerrado el 2026-10-21 con límite declarado**: el WebLink no lista comandas (`Orders/GetList` → 404 en v36.2), así que una comanda de caja sin aviso no se recupera por API; procedimiento en D32 |
| D29 | Tres reinicios (sincronización, WebLinkRestAPI, BDP.NET) | L9 | **2 de 3 corridos el 2026-10-21** por inyección (proceso de sincronización y caída del WebLink, sin pérdida ni duplicado). El reinicio de `TpvNet.exe` y el de BDP.NET son presenciales: UAC y corte del TPV |
| D31 | Llamar al camarero | L9 | Tras F2 |
| D24a–d | Alta, modificación, desactivación y borrado en caja | L8 | **D24a–c cerrados el 2026-10-21 con corrida real** contra el BDP local (`sincronizados:1`). D24d no tiene servicio WebLink: va por D32 |
| D24-p, D24-m | Plano, formas de pago y empleados | L8 | **Cerrados el 2026-10-21**: `GET /api/bdp/tenders` + tarjeta «Formas de pago del BDP» (D24-m) y `GET /api/bdp/maestros` + «Comprobar contra BDP» (D24-p) |
| D25 | Clientes: cambio en BDP, en la web (`Overwrite=true`), contradictorios | L8 | Decisión (punto 14) |
| D26a, D26b | `GetStock` y `GetListStock` | L7 | **D26b cerrado el 2026-10-20**: `refrescar_stock_almacen` (`src/services/bdp_sync_stock.rs`), ruta `sync-stock-almacen` y botón «Stock por almacén» |
| D26, D26c–D26g, D27 | Escritura de stock, libro y ventas de caja | L7 | **D26d, D26f y D26g cerrados** (traspaso real `1>2` y `2>1`; inventario absoluto; relectura S/S+V con «Libro de stock»). **D27 declarado no verificable**: no existe escritor de stock por venta (solo la usuaria o un conteo disparan `ajustar_stock`), así que la ficha no tiene caso que correr; la guarda `venta_puede_mover_stock` queda como precondición del escritor futuro, con test, y se activa junto a él |
| D28, D28-auto, D28-auto-R, D28a | Exportar documentos, perfil programado, reinicio, albaranes | L8 | **D28 y D28-auto/R cerrados y vivos el 2026-10-21**: `GET/POST /api/bdp/documentos`, tarjeta con cadencia y perfil por cuenta (D32), `OnlyLastMovements` fijado a `false` porque con `true` el BDP local devuelve vacío |
| D32 | Mensaje de procedimiento ante cada operación que la API no expone | L10 | **Cerrado en código y pantalla** (`7cc…`, `OperacionesNoApiCard` en Configuración → A qué se conecta). Falta que cliente y Guillermo escriban responsable y procedimiento |
| D30 | Piloto y despliegue por etapas | Fuera (F4) | Permiso aparte; requisitos en L10 |

**Base T2:** L5.0 (roadmap 09AA-2), hecha el 2026-10-17 (5 tablas; *outbox* ya era `bdp_push_pendientes`). **Despliegue por etapas del PDF** (observación → maestros → lectura → escritura → pagos/factura → stock): se ejecuta en F4, no aquí; aquí solo se deja probado el rollback en local. **Paradas del PDF** (duplicado económico, descuadre fiscal, pérdida de comanda, degradación del TPV): criterio de aborto de cualquier lote de L6–L10.
