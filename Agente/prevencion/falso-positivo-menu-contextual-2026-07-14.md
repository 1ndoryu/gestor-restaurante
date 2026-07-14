# Falso Positivo: `componente-artesanal` en `article-autocomplete.tsx`

**Fecha:** 2026-07-14
**Regla:** `componente-artesanal`
**Archivo:** `frontend/src/components/article-autocomplete.tsx`
**Línea:** 27

## Problema

El Sentinel reporta "Patrón de menu/dropdown artesanal detectado (outside-click handler manual)" y sugiere usar `<MenuContextual>`.

## Por qué es falso positivo

`<MenuContextual>` **no existe** en el sistema de componentes del proyecto. No hay ningún archivo `MenuContextual.tsx` ni `menu-contextual.tsx` en `frontend/src/components/ui/`.

El dropdown con `mousedown` handler es el patrón estándar para autocomplete con búsqueda. No hay alternativa en el catálogo actual de componentes.

## Corrección necesaria en Sentinel

Cuando la regla `componente-artesanal` sugiera `<MenuContextual>`, debería verificar primero que el componente existe en `components/ui/`. Si no existe, downgrade a Hint en vez de Warning.

## Estado

- [ ] Implementar verificación de existencia del componente sugerido en code-sentinel
