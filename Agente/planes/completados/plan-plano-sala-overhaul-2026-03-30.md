# Plan: Plano de Sala Overhaul + Tareas Data IV (2026-03-30)

## Contexto
El cliente reportó varios problemas (Data IV) y el usuario identificó mejoras adicionales en el plano de sala. También hay tareas pendientes de configuración y un MD de aclaraciones.

## Tareas (orden de ejecución — dependencias respetadas)

### 303A-11: Zoom no reduce altura + overflow:auto
**Problema:** Canvas `height = zonaData.alto * zoom` → zoom out reduce la altura del canvas.
**Solución:**
- Canvas tiene altura FIJA desde store (`canvasHeight`, default 600px), NO dependiente de zoom
- `overflow: auto` en vez de `hidden` (PlanoSala) — PlanoOcupacion ya lo tiene
- Wrap mesas en div interno `position: relative` con `minHeight: zonaData.alto * zoom`
- Cuando zoom-in, contenido crece → scrollbars aparecen; zoom-out → contenido reduce pero canvas fijo
- zoomStore.ts: agregar `canvasHeight` + `setCanvasHeight` + localStorage

**Archivos:** zoomStore.ts, PlanoSala.tsx, PlanoOcupacion.tsx, PlanoSala.css, PlanoOcupacion.css, usePlanoSala.ts

### 303A-12: Pan mouse, minimap, indicadores off-screen
**Problema:** Mesas fuera de vista sin indicación, no hay forma de navegar rápido.
**Solución:**
- **Pan:** Middle-click o Shift+click drag para scroll del canvas (ambos paneles)
- **Minimap:** Div overlay en esquina inferior-derecha, muestra todas las mesas como puntos y un rectángulo de viewport
- **Indicadores off-screen:** Flechas en los bordes del canvas cuando hay mesas fuera de la zona visible
- Componentes reutilizables: `CanvasMinimap.tsx`, `OffScreenIndicators.tsx`

**Archivos:** Nuevos: CanvasMinimap.tsx, OffScreenIndicators.tsx. Modificar: PlanoSala.tsx, PlanoOcupacion.tsx, PlanoSala.css, PlanoOcupacion.css

### 303A-13: Altura ajustable manualmente + sincronizada
**Problema:** El usuario quiere arrastrar el borde inferior del canvas para cambiar su altura, y que se sincronice entre PlanoSala y PlanoOcupacion.
**Solución:**
- Handle draggable en el borde inferior del canvas
- onMouseDown/Move/Up para resize
- Almacenado en zoomStore.canvasHeight (de 303A-11) → ya sincronizado entre ambos paneles

**Archivos:** PlanoSala.tsx, PlanoOcupacion.tsx, PlanoSala.css

### 303A-9: PanelConfigMesa no actualiza al cambiar mesa
**Problema:** useState inicializado desde mesa prop pero no se resetea al cambiar mesa.
**Solución:** Agregar `key={mesa.id}` al `<PanelConfigMesa>` en PlanoSala.tsx.
**Archivos:** PlanoSala.tsx (1 línea)

### 303A-10: Cuadrada ≠ rectangular CSS
**Problema:** Ambas formas se ven igual.
**Solución:** `.planoMesa.rectangular` con aspect-ratio 2:1 y border-radius mínimo. `.mesaOcupacion.rectangular` igual.
**Archivos:** PlanoSala.css, PlanoOcupacion.css

### 303A-15: Filtro rango fechas en reservas
**Problema:** Solo se puede filtrar por un día.
**Solución:**
- Backend: agregar `fecha_desde` y `fecha_hasta` a ReservasQuery (manteniendo `fecha` para compat)
- Repository: `AND (fecha >= $X AND fecha <= $Y)` cuando ambos vienen
- Frontend: dos `<Input type="date">` en ListaReservas
- Hook: agregar `fechaDesde` / `fechaHasta` a filtros

**Archivos:** src/models/reserva.rs, src/repositories/reserva.rs, frontend ListaReservas.tsx, useVistaReservas.ts

### 303A-16: Fecha y hora en campos obligatorios (configuración)
**Problema:** Faltan fecha y hora en los checkboxes de campos obligatorios.
**Solución:**
- Migration: agregar `reserva_fecha_obligatorio`, `reserva_hora_obligatorio` (default true)
- Backend struct + handler + repo
- Frontend: agregar checkbox en configuración

**Archivos:** nueva migration, configuracion.rs (model, repo, handler), frontend configuración

### 303A-17: MD aclaraciones para el cliente
**Contenido:**
1. Reserva ≠ Venta: son sistemas independientes, una reserva finalizada NO genera venta automática
2. Plantillas WhatsApp: cabecera = título/header, pie = footer pequeño, media = solo URL (no upload local)
3. IVA: configuración = default global, en ventas se puede poner excepciones por venta
4. API chatbot: "Groq" es el nombre del proveedor, pero funciona con cualquier API compatible OpenAI
5. Datos de prueba: eliminar = vaciar todo, recargar = restaurar demo
6. Reportar error: llega a CORREO_AUTOR via SMTP

**Archivo:** Agente/documentacion/aclaraciones-cliente-2026-03-30.md
