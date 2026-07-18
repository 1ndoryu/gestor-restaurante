# Plan: BDP WebLink — Testing completo pre-producción

> **HISTÓRICO — PROHIBIDO EJECUTAR ESTOS PASOS.** Incluye contacto directo, `OnlyCheck`, creación/cancelación de comandas y supuestos de reversibilidad que no forman parte de la política vigente. Nuestro equipo no prueba el BDP real; el cliente usa la guía no técnica y excluye toda escritura.

> **Fecha:** 2026-06-07
> **Tarea:** 065A-4 (continuación)
> **Objetivo:** Verificar que la integración BDP funciona end-to-end antes de confirmar al cliente
> **Principio:** minimizar cambios reales en BDP-Net. Cancelar cualquier pedido de prueba inmediatamente.

---

## Contexto

La validación dry-run (`OrderOperationType=1`, OnlyCheck) ya pasa correctamente:
- POS 31, Type=0 (Barra), serie `00031TI` (IVA incluido)
- Artículo real `1001` ("CAFE BOMBON", 5.00€, IVA 10%)
- `ErrorMessage: ""` = validación exitosa

Ahora necesitamos probar el **ciclo completo**: crear → verificar → cancelar.

---

## Fase 1: Dry-run directo contra BDP (desde aquí, sin deploy)

**Qué:** Llamadas PowerShell directas a la API WebLink para replicar exactamente lo que hace nuestro backend.

### Paso 1.1 — Login + Health
```powershell
POST /Auth/Login
Body: {"Login":"admin","Password":"kamples2026","TiempoSession":59,"CodigoIntegrador":"VBW2MBM5"}
→ Guardar AuthSession.Token

GET /Service/Health
→ Confirmar IsAlive=true
```

### Paso 1.2 — Validar configuración POS 31
```powershell
POST /API/POS/Get         → {"Id": 31}  → Confirmar que existe
POST /API/Employee/Get    → {"Id": 31}  → Confirmar empleado
POST /API/Tenders/GetPOSList → {"POSId": 31} → Confirmar formas de pago
POST /API/Articles/GetPOSList → {ProfileCode:1, Art1:1, Art2:999...} → Confirmar artículos
```

### Paso 1.3 — Dry-run CreateOrder (OnlyCheck)
```powershell
POST /API/Orders/Create
Body: {
  "EmployeeId": 1, "ItemsProfileId": 1, "OrderEndType": 0,
  "OrderOperationType": 1,  ← OnlyCheck, NO crea pedido
  "Invoice": false,
  "Order": {
    "PosId": 31, "Type": 0, "RoomNumber": 0, "TableNumber": 0,
    "MarketplaceOrderId": "DRY-0607-1",  ← máx 15 chars
    "MarketId": 1, "MarketName": "Glory Test",
    "AlreadyInvoiced": false,
    "Customer": {"Id": 0, "Name": "Test", "TaxId": "00000000A", "TaxType": 1},
    "Items": [{"Lin":1, "Id":1001, "Name":"CAFE BOMBON", "Units":1.0,
               "Price":5.0, "Supplement":0.0, "Discount":0.0, "DiscountPct":false,
               "Total":5.0, "VatPct":10.0, "Comments":[], "Supplements":[],
               "OrderItemType":0, "TyC_D1":0, "TyC_D2":0, "TyC_D3":0, "OnSale":false}],
    "Discount": 0.0, "DiscountPct": false, "Total": 5.0,
    "ExecutionTime": "<now>", "Status": 0, "Comments": "", "Payments": []
  }
}
→ Esperado: {"OrderId": 0, "ErrorMessage": "", ...}
```

**Resultado esperado:** `ErrorMessage: ""` = pasa ✅

---

## Fase 2: Pedido real mínimo + cancelación inmediata

**Qué:** Crear un pedido real (`OrderOperationType=0`) con `OrderEndType=1` (pendiente de validación, NO finalizado), verificar que existe, y cancelarlo inmediatamente.

**Por qué `OrderEndType=1`:** El pedido llega a la "consola de autocomanda" del TPV como pendiente. No se imprime ticket ni se contabiliza como venta cerrada. Es el impacto mínimo posible.

### Paso 2.1 — Crear pedido real
```powershell
POST /API/Orders/Create
Body: (igual al 1.3 pero con "OrderOperationType": 0)
→ Guardar OrderId de la respuesta
→ Esperado: OrderId > 0, ErrorMessage = ""
```

### Paso 2.2 — Verificar que el pedido existe
```powershell
POST /API/Orders/Get
Body: {"OrderIdentifier": {"OrderId": <OrderId del paso 2.1>}}
→ Confirmar: Status != 3 (no facturado), Items contiene nuestro artículo
```

### Paso 2.3 — Cancelar el pedido inmediatamente
```powershell
POST /API/Orders/Cancel
Body: {"PosId": 31, "OrderIdentifier": {"OrderId": <OrderId>}}
→ Esperado: {"ErrorMessage": ""}
```

### Paso 2.4 — Verificar que ya no existe
```powershell
POST /API/Orders/Get
Body: {"OrderIdentifier": {"OrderId": <OrderId>}}
→ Esperado: Status = 2 (cancelada) u Order vacío
```

**Impacto real:** Un pedido pendiente en la consola de autocomanda durante ~5 segundos. No se factura, no se imprime ticket, no se contabiliza.

---

## Fase 3: Test vía nuestro backend (requiere deploy)

**Qué:** Probar el endpoint `/api/configuracion/bdp/sync-dry-run` de nuestro backend Rust en producción.

### Paso 3.1 — Deploy
```powershell
$cm = "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs\target\release\coolify-manager.exe"
& $cm deploy --name kamples --update --skip-backup
```

### Paso 3.2 — Health check
```powershell
& $cm health --name kamples
```

### Paso 3.3 — Probar dry-run desde el frontend
1. Abrir la app en el navegador
2. Ir a Configuración → BDP → "Probar sincronización segura"
3. Verificar que el resultado muestra `listo_para_sincronizar: true`

### Paso 3.4 — Probar diagnóstico
1. Click en "Diagnóstico BDP" 
2. Verificar: Health ✅, Login ✅, Versión ✅

---

## Fase 4 (opcional): Test real desde nuestro backend

**Qué:** Si el cliente quiere ver un pedido real creado desde la app (no desde PowerShell), probar con la integración completa.

**Pre-requisito:** Que el restaurante esté abierto (caja abierta, POS activo).

### Paso 4.1 — Crear pedido real desde la app
- Usar el flujo normal de la app (sincronización con `escritura_real=true`)
- Verificar que aparece en BDP-Net

### Paso 4.2 — Cancelar desde BDP-Net
- El camarero/admin cancela el pedido desde el TPV
- O cancelar via API con `CancelOrder`

---

## Checklist de verificación

| # | Test | Criterio de éxito | Estado |
|---|------|-------------------|--------|
| 1 | Health + Login | `IsAlive=true`, token obtenido | ⬜ |
| 2 | POS 31 existe | `Id=31, Name="CENTRAL 2026"` | ⬜ |
| 3 | Empleado 31 válido | Sin error en Employee/Get | ⬜ |
| 4 | Artículos disponibles | `ArticlesListData` con items | ⬜ |
| 5 | Dry-run (OnlyCheck) | `ErrorMessage: ""` | ⬜ |
| 6 | Pedido real creado | `OrderId > 0` | ⬜ |
| 7 | Pedido verificable | GetOrder devuelve el pedido | ⬜ |
| 8 | Pedido cancelado | CancelOrder `ErrorMessage: ""` | ⬜ |
| 9 | Post-cancelación verificada | Status = 2 (cancelada) | ⬜ |
| 10 | Deploy producción | Health OK post-deploy | ⬜ |
| 11 | Dry-run vía backend | `listo_para_sincronizar: true` | ⬜ |

---

## Riesgos y mitigaciones

| Riesgo | Probabilidad | Mitigación |
|--------|-------------|------------|
| Caja cerrada → error 301400 | Alta (fuera de horario) | Usar `OrderEndType=1` (autocomanda, no requiere caja abierta en todos los casos). Si falla, documentar como "requiere horario comercial" |
| El pedido aparece en el TPV | Media | `OrderEndType=1` lo manda como pendiente. Cancelar en <10 segundos |
| CancelOrder falla | Baja | Cancelar manualmente desde BDP-Net via RDP |
| Deploy falla | Baja | `redeploy` como fallback, o rollback manual |

---

## Nota para el cliente

> "La integración con BDP-Net WebLink está validada técnicamente. El dry-run pasa correctamente en POS 31 (CENTRAL 2026) con la serie de facturación `00031TI` (IVA incluido). El siguiente paso es probar en horario comercial con la caja abierta para confirmar el flujo completo de venta."
