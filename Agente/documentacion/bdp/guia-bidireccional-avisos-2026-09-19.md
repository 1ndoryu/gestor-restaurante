# Guía bidireccional + Avisos de Acciones — nota de nueva información (2026-09-19)

> Origen: `Guia_integracion_bidireccional_BDP_para_Wan.pdf/.docx` (Downloads, 19/09/2026)
> + `Avisos de accion-20260904T234725Z-1-001.zip` (Downloads, 04/09/2026 19:49, sin abrir hasta hoy).
> El ZIP **no entra al repo** (92 MB, binarios Win). Referencia externa + copia del PDF en `C:\tmp\avisos-acciones.pdf`.

## 1. Tesis (una frase)

La web es el único punto de operación ordinario; BDP es la autoridad fiscal.
"Bidireccional" = la web manda órdenes y **absorbe los cambios excepcionales del BDP**,
nunca dos maestros escribiendo a la vez. Valida nuestro diseño (un solo escritor +
confirmación por estado) y define la vuelta pendiente como **fase D**.

## 2. Reglas de oro de la guía

1. Lo que vuelve del BDP actualiza **la misma venta**: primero por `bdp_order_id`,
   si no por `market_id + marketplace_order_id`. Sin facturario paralelo.
2. La web **no lleva facturario propio**: pide la factura al BDP y guarda su identificador.

## 3. Lo nuevo: Avisos de Acciones (fase D pendiente)

- BDP-Net puede avisar por **HTTP GET/POST a servidor externo** o guardar JSON en ficheros.
  Config: BDP-Net → Utilidades → Configuración de servicios Web → Configuración de Avisos.
- 7 eventos (verificados en el código demo `ComandasController.cs`): Crear, Modificar,
  CambiarEstado, Borrar, Finalizar, Traspasar, TraspasarLinea. Cada uno con endpoint GET+POST.
- **Clave de diseño (demo): la estructura del receptor es del integrador y BDP no la conoce.**
  Nombres de parámetros configurables por acción; campos fijos libres (p.ej. identificador
  de establecimiento); el match de estados es manual.
- Contratos mínimos: Crear/Modificar/Borrar = id comanda + establecimiento + timestamp UTC.
  CambiarEstado añade `Estado` (0 EsperandoValidacion, 1 Acceptada, 2 Cancelada, 3 Facturada).
  Finalizar añade `TipoDocumento` (1 TicketBarra, 2 FacturaMesa, 3 Albaran, 4 FacturaDeAlbaran,
  5 FacturaRectificativa, 6 TicketMesa) + Serie + Numero.
  Traspasos añaden comanda/línea origen + salón/mesa origen/destino (0 = barra).
- Detalle de líneas básico **solo en exportación a ficheros**, no por API.
- El integrador puede **exportar/importar la configuración del punto de exportación en XML**
  para que el distribuidor la importe (vía operativa para el restaurante).
- Hay botón "Importar Configuración Demo" que apunta al servidor de ejemplo del ZIP.
- **Discrepancia anotada (T):** el PDF solo documenta valores 0/1 para "Estado Cambiado",
  pero el enum del demo incluye 0–3. Verificar en TPV v36 qué estados llegan de verdad.

## 4. Límites del BDP que la guía confirma por escrito

Sin evento de pago · `GetOrder` vacío tras facturar (Status 3) · sin rectificación/anulación
fiscal documentada · sin ETag/versiones · máx 3 pagos · sin endpoint de comandas abiertas ·
manual disponible hasta v35.1 con TPV en v36. Compras, gastos, reservas, marketing y empleados
**sin cobertura bidireccional** en la documentación entregada.

## 5. Etiquetas de la guía (honestidad del documento)

Cada afirmación lleva D (documentado), T (probar en TPV) o C (BDP debe confirmar).
Avisos, unicidad de `MarketplaceOrderId`/`PaymentId` y semántica de update son **T/C, no hechos**.
Las **12 preguntas bloqueantes** (ap.8 de la guía) van a BDP **por escrito** antes de codificar F2.

## 6. Documentos SÍ tomados en cuenta antes (base existente)

- Manual principal WebLink REST API 244 p. (`# WEBLINK RESTAPI.md` en raíz) — origen del
  "BDP no avisa", OnlyCheck, `Payments` en `CreateOrder`, suscripción de pago.
- `Agente/documentacion/bdp/maestro-organizacion-bdp-2026-07-26.md` (D3 bidireccional bloqueado),
  `maestro-auditoria`, `riesgos-produccion-bdp-2026-07-24.md`, `runbook-operativo`,
  `feature-flags`, `plan-pruebas-lectura`, `contrato-modifyarticle-2026-09-07.md`.
- `Agente/planes/plan-pendientes-bdp-2026-07-23.md` (D3 "Rechazado firme"),
  `plan-deploy-produccion-intuitividad-2026-08-08.md` (§6 alcance), planes 149A-1/149A-2.
- `Agente/usuario/guia-cliente-segunda-entrega-bdp-2026-09-18.md` (**línea 54 a corregir**:
  dice "el BDP no avisa solo").

## 7. Documentos NO tomados en cuenta antes (causa del desfase)

| Documento | Por qué no se usó |
| --- | --- |
| Guía bidireccional PDF/DOCX (19/09/2026) | No existía; llegó por Guillermo el 19/09 13:xx |
| ZIP Avisos de Acciones (04/09/2026 19:49) | En Downloads, fuera del repo; cero menciones en código/roadmap/Agente. Lote del 5-ago pasó sin separar |
| PDF "Avisos de Acciones V1" 6 p. | Dentro del ZIP sin abrir (leído hoy: ver §3) |
| Demo C# + Bin + LEEME | Dentro del ZIP (inventariado hoy: §3) |

## 8. Correcciones pendientes a nuestros documentos

1. `roadmap.md` fila "Sincronización bidireccional automática … no soportada por BDP … ❌ Rechazado"
   → matizar: el mecanismo (Avisos) **existe**; pendiente verificar en v36 + decidir fase D.
2. `guia-cliente-segunda-entrega-bdp-2026-09-18.md:54` ("el BDP no avisa solo") → matizar igual.
3. `plan-pendientes-bdp-2026-07-23.md` D3 ("Rechazado firme … no implementar") → matizar:
   el rechazo vale para **escritura ciega simultánea**, no para **absorción de eventos**.
4. Auditoría 189A-3 / textos "BDP no avisa" → referenciar esta nota.

## 9. Decisión pendiente del cliente

Fase D = trabajo nuevo y cobrable (bidireccional fuera de alcance; roadmap la marcaba rechazada).
**No urgente:** el restaurante ya opera desde la web sin ella. Plan en
`Agente/planes/plan-fase-d-avisos-bdp-2026-09-19.md`. Estimación previa: ~2.200–3.400
líneas nuevas, 7–12 laborables (2–4 dependientes de TPV/distribuidor/BDP).
