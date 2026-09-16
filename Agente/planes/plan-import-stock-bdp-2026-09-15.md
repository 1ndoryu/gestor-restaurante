# Subplan 159A-4 — Importar snapshot de stock desde BDP (PENDIENTE)

Origen: hallazgo 2026-09-15 — la columna Stock de `/bdp/stock` muestra "—" en las
557 filas porque `stock_actual` es 0 en todas (verificado vía API) y no hay stock
local registrado. `sync_catalog` nunca trae niveles de stock.

## Evidencia del diagnóstico

- `GET /api/bdp/article-maps?limit=600` → 557 filas, `stock_actual:"0"` en todas;
  `departamento:0`/`familia:0` en 556/557, `barcode:""` en todas → firma del
  fallback GetPOSList del perfil (`bdp_sync_catalogo.rs:96-117`), que por diseño
  no trae stock. Vía `ExportArticles` tampoco llegaría sin módulo de almacén
  activo (`CurrentStock` → `None` → 0, `bdp_sync_catalogo.rs:189-192`).
- La UI pinta el 0 como "—" (P3.1, `BdpStock.tsx:402-416`).
- El cliente BDP ya tiene `get_stock` / `get_list_stock` solo-lectura
  (`bdp_weblink.rs:446-453`) que hoy nadie llama.

## Objetivo

Rellenar `stock_actual` desde `GetStock`/`GetListStock` tras/durante `sync_catalog`,
para que la columna Stock muestre el snapshot BDP sin carga manual.

## No alcance

- No escribir stock en BDP (`regularize_stock`, `update_stock`, etc. quedan fuera;
  regla oro 149A-2: cualquier escritura requiere simulación + autorización).
- No cambiar la semántica "lo local manda" (P3.4).

## Fases

1. **Spike lectura**: llamar `get_list_stock` contra el BDP real (solo lectura,
   como "Importar del BDP") y documentar forma de respuesta (¿trae todos los
   artículos? ¿paginado? ¿requiere almacén activo?).
2. **Mapeo**: `art_code → stock` → `upsert_stock`/`stock_actual` sin pisar
   ediciones locales (`local_dirty`, mismo criterio que precios).
3. **Cableado**: invocar desde `sync_catalog` (o botón propio "Importar stock"),
   contador `stock_actualizados` en el resultado + aviso honesto si el BDP no
   devuelve stock.
4. **Verificación**: type-check + tests + navegador (columna con valores bdp,
   badge `bdp`; fallback "—" si el módulo no está contratado).

## Gate / DoD

- `sync_catalog` informa `stock_disponible=true/false` real (no asumido).
- Si el BDP no trae stock, la UI lo dice ("BDP sin datos de stock") en vez de
  "—" ambiguo.
- Cero escrituras en BDP en todo el flujo (auditoría limpia de `update_stock`).

## Estado

CERRADO 2026-09-15 — Fase 1 ejecutada contra el BDP real (solo lectura, vía
curl directo con credenciales del `.env`, sin exponerlas): **NO VIABLE**.

Evidencia:
- `POST /Auth/Login` OK (token 417 chars).
- `GetListStock {Store:1}` → `[200200]-EL ALMACÉN 1 NO EXISTE`; con
  `{Store:2}` + 5 códigos reales (10001–10006) → `EL ALMACÉN 2 NO EXISTE`.
- `GetStock` con artículos 1001/10001/90000003 × stores 1–31 → siempre
  `[200007]-EL ARTÍCULO ... NO ES DEL TIPO WEB` (ni siquiera 90000003,
  creado por nuestra API, es Web).
- Convergencia: `ExportArticles` ya devuelve vacío en este BDP (documentado
  039A-1/H-Q1-03, incidente 2026-09-05) porque solo trae artículos Web; el
  manual no ofrece endpoint para listar almacenes.

Conclusión: este BDP no tiene artículos de tipo Web (sin tienda web) y su
módulo de almacén no publica stock por API. El stock BDP no existe como dato
para este cliente; la fuente de stock es 100 % local (compras, conteos,
ajustes). Fases 2–4 CANCELADAS (sin objeto).

Cierre aplicado:
- `BdpStock.tsx`: subtítulo honesto en modo BDP ("El terminal no publica
  niveles de stock ... el stock se lleva en local").
- `BdpInventario.tsx`: paginación fija 50/pág verificada en navegador
  (12 págs, contadas sobreviven al cambio de página).
- Cero escrituras en BDP en todo el spike (solo `/Auth/Login` + lecturas
  `GetStock`/`GetListStock`).
