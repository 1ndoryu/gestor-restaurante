# Fase D BDP local — 11 Automatización del TPV

> Parte del [plan Fase D sobre BDP local](../plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 11. Automatización obligatoria de acciones manuales (rev. 2026-10-18)

**Regla.** Toda acción que este plan exige en el TPV (TpvNet, BDP Net) o en la BDP local se ejecuta con `scripts/pc/tpv.mjs` (sobre `pc.mjs`), o con un lote de `scripts/pc/lote.mjs` cuando exista. Ningún paso queda como «lo hace la usuaria». Un agente menos capaz debe poder ejecutarla sin adivinar: comprueba la pantalla antes, actúa una vez, verifica el resultado y para al primer desvío con evidencia. Si la interfaz no permite una acción, se declara «no automatizable» con su motivo en esta tabla; no queda como paso manual sin nombre.

**Automatizar no es autorizar.** Cada lote que escribe, cierra, cobra, factura, borra o reinicia necesita permiso expreso de la usuaria en el chat (§4), con fecha y alcance. El runner lo exige en el archivo de lote. El BDP real (`IP_BDP:8068`) queda fuera: la automatización solo actúa sobre el TPV local (ventana «CONEXION LOCAL», Terminal 1).

### 11.1 Inventario (estado 2026-10-18)

| Acción | Pasos y fichas | Hoy | Falta |
|---|---|---|---|
| Cerrar Día | prerrequisito de turno (L3) | a mano con `pc.mjs` (17/10) | acción `cerrar-dia` (`CIERRE.cerrarDia` [499,315] + Sí); permiso por lote |
| Abrir turno | L3, L4.2 | `abrir-turno`; no confirma la ventana final a tiempo | confirmar turno abierto antes de dar éxito |
| Apertura de caja | L3 (`301400` si no está abierta) | a mano en TpvNet | acción nueva; permiso por lote |
| Crear comanda y añadir artículos | D03, `PRUEBA-n` | `familia`, `articulo` (probado) | `familia REFRESCO`: falso negativo; cantidad |
| Cantidad de línea | D03, D05–D07, L3.b | `tecla 2` no fija cantidad | secuencia «2», «X», artículo; test de 2 unidades |
| Modificar línea o comanda | D05–D07, L3.b | no existe | acciones nuevas en el panel; L3.b pasa a automatizable en cuanto exista la ruta BDP (§9.9) |
| Anular línea, borrar comanda | D09, D09a | `anular-linea`, `borrar-comanda` (probados) | nada; D09a sin testigo de cocina |
| Traspaso de mesa y de línea | D22a, aviso `traspasar`/`traspasar-linea` | no existe | acción nueva; permiso por lote |
| Enviar a cocina (`OrderEndType=0`) | D03a, L3.a2 | no existe | localizar botón; sin impresora no hay testigo |
| Cobro: efectivo, tarjeta, propina, puntos | D11–D17, L6 | no existe | acciones nuevas; L6 solo con suscripción confirmada (§9.12) |
| Factura, rectificativa, devolución | D18–D20, D32 | no existe | automatizar o declarar no automatizable; fiscal: permiso siempre |
| Alta, modificación, baja en maestros | D24a–d | no existe | acciones nuevas en TpvNet |
| Perfil de exportación de documentos | D28, L8 | a mano | acción nueva |
| URL y eventos de avisos | L4.2 | por fichero `CONFIG_WEBHOOKS.XML` (lote de 17/10) | nada: no se usa la pantalla |
| Cerrar turno | L3, cierre | `cerrar-turno` (probado de punta a punta, 18/10) | nada |
| Reinicio de BDP.NET o servicio | D29 | a mano (TpvNet elevado) | ejecutar con el daemon elevado; permiso por lote |
| Arqueo, efectivo, PDF, reimpresión, controles de cocina, series | D32 | a mano | automatizar o declarar no automatizable con motivo |

### 11.2 Orden de trabajo

1. **Control listo para un agente** (plan `17A-1`, «Quinta vuelta»): defectos de `tpv.mjs`, runner de lotes, protocolo de atasco, pruebas negativas. Sin esto no se ejecuta nada de §11.3.
2. **Acciones que faltan** del inventario, una a una, cada una con prueba positiva y negativa en el TPV local.
3. **Fase D**, por lotes automatizados y con permiso: L4.2 (los siete avisos; la comanda `PRUEBA-2` y los seis eventos los dispara el lote, no la usuaria), luego L4.3–L4.4, F2, y L6–L10 según su precondición.

### 11.3 Criterio de «automatizada»

Una acción cuenta como automatizada cuando: (a) pasa en `--dry-run` (solo mira, no pulsa); (b) pasa en el TPV local con permiso por lote y deja captura, log y verificación; (c) la prueba negativa (pantalla equivocada) para sin escribir nada; (d) está en la skill `pc-control`.

### 11.3b Manual y ayuda antes de cada acción (regla obligatoria, rev. 2026-10-18)

Antes de ejecutar cualquier acción del TPV, el agente consulta el manual o la ayuda de esa función. No se prueba a clics ni se adivina el comportamiento. Pasos:

1. Fuente: `C:\Bdp-Net\NetHos\Ayuda\AyudaHOS.chm`, descompilado con `hh.exe -decompile` a `C:/tmp/fase-d-local/ayuda/hos/` (temporal; se regenera si falta). Páginas por función en `Ayuda_BDP/` (p. ej. `Cocina.htm`, `Aparcar_Ticket.htm`, `Avisos_Cocina.htm`, `Anular_Linea.htm`, `Borrar_Comanda.htm`, `Barra_Mesa.htm`, `Comprobante.htm`).
2. Anotar en el plan (o en la ficha de la acción) qué dice la ayuda: qué hace, con qué precondición, qué se dispara solo.
3. Si la ayuda contradice la pantalla o el plan, parar y registrar la discrepancia antes de actuar.
4. Solo entonces ejecutar con `tpv.mjs` o el lote.

**Hallazgos ya verificados en el manual (2026-10-18):**
- **Cocina** (`Cocina.htm`): conmuta la impresión en cocina solo con la comanda vacía. No envía nada a cocina.
- **Aparcar Ticket** (`Aparcar_Ticket.htm`): aparca la operación de barra para liberar la comanda; la página no dice que envíe a cocina. **Corrección 2026-10-18:** la versión anterior de esta nota decía lo contrario sin verificarlo; el envío a cocina sigue sin confirmar y es lo que L4.2 debe medir.
- **Anular Línea** (`Anular_Linea.htm`): elimina las líneas seleccionadas (naranja); sin selección, borra la última. Si la línea ya fue a cocina, cocina recibe aviso de anulación.
- **Avisos a cocina** (`Avisos_Cocina.htm`): son mensajes de texto libres (tecla «Comentario») a impresoras auxiliares; no modifican comandas. No intervienen en `modificar` ni en `cambiar-estado`.

### 11.4 Lo que sigue igual

- BDP real: fuera de toda automatización.
- L1.2 (credenciales de la app): se fija por API con credenciales del entorno, nunca escritas en chat ni en el plan.
- Gate: `scripts/` no es repo; las pruebas de C5 del plan `17A-1` sustituyen al gate y se declaran en el cierre.
