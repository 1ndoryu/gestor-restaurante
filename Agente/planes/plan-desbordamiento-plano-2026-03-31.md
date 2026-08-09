# Plan: Desbordamiento del plano de sala

## Problema
El ancho del plano se desborda de la pantalla cuando hay mesas posicionadas lejos.

## Historial de intentos fallidos

### Intento 1: overflow:hidden en .planoCanvas (303A-18)
- **Qué:** Cambiar `overflow: auto` → `overflow: hidden` en CSS
- **Por qué falló:** Rompió el pan (shift+drag) y el minimap porque usaban `scrollLeft`/`scrollTop` programático que requiere `overflow: auto`

### Intento 2: overflow:auto + scrollbar-width:none (303A-18)
- **Qué:** `overflow: auto` con scrollbars ocultos via CSS
- **Por qué falló:** `overflow: auto` permite que el canvas se expanda si su padre no tiene constraint de ancho

### Intento 3: width:100% en .planoCanvas (303A-19)
- **Qué:** Agregar `width: 100%` al canvas
- **Por qué falló:** El CSS `width:100%` se calcula sobre el padre (`position:relative` wrapper) que tampoco tiene width fijo — se expande con su contenido

### Intento 4: overflow:hidden en wrapper position:relative (303A-20)
- **Qué:** `overflow: hidden` en el div wrapper padre del canvas
- **Por qué falló:** El wrapper sigue sin tener width constraint explícito. `overflow:hidden` solo clipea visualmente pero el wrapper ya se expandió

### Intento 5: Refactor a transform-based pan (303A-20)
- **Qué:** Eliminar scroll nativo, pan via `transform: translate(-x, -y)`, overflow:hidden
- **Por qué falló:** El div `.planoCanvasContent` tiene `width: contentBounds.w` en style normal-flow. Aunque overflow:hidden clipea, un hijo normal-flow CON width explícito sigue contribuyendo al sizing del parent en ciertas configuraciones flex.

## Análisis del DOM chain

```
<div className="flex gap-4">                    ← flex row
  <div className="flex-1 min-w-0">             ← flex child (¿min-w-0 funciona?)
    <div position:relative overflow:hidden>      ← wrapper
      <div .planoCanvas overflow:hidden>         ← viewport (height fijo)
        <div .planoCanvasContent                 ← ESTE tiene width: 1200px+
             width: contentBounds.w              ← NORMAL FLOW → empuja al padre
             transform: translate(-pan)>
```

## Root cause confirmado
El `.planoCanvasContent` es un elemento **en flujo normal** con `width` explícito grande. Aunque `overflow: hidden` en el padre clipea visualmente, el hijo en flujo normal **SÍ contribuye al min-content size** del padre en contextos flex/grid complejos.

## Solución definitiva
Hacer `.planoCanvasContent` **position: absolute**. Elementos absolute:
- NO contribuyen al sizing del padre
- NO pueden expandir el layout
- Se clipean correctamente con overflow:hidden del padre
- Transform funciona perfectamente

### Cambios necesarios:
1. CSS: `.planoCanvasContent` → `position: absolute; top: 0; left: 0;`
2. CSS: `.planoOcupacionContent` → `position: absolute; top: 0; left: 0;`
3. CSS: `.planoCanvas` → `position: relative; overflow: hidden;` (ya tiene overflow:hidden, necesita position:relative para anclar el absolute)
4. CSS: `.planoOcupacionCanvas` → `position: relative; overflow: hidden;`
5. JSX: Quitar `width` y `height` inline del content div (no necesitan constrainir nada, son absolute)
6. Verificar que mesas absolute-in-absolute siguen funcionando

## Checklist de cierre

- [ ] Aplicar la solución definitiva (cambios CSS/JSX 1-6) y validar mesas y ocupación en local.
- [x] Root cause documentado y causa de desbordamiento confirmada.
