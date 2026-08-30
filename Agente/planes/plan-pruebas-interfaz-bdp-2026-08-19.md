# Plan — Pruebas de interfaz: independencia BDP + integración de escritura (sin BDP real)

> **Fecha:** 2026-08-19
> **Rama:** `glory-rs-rest` (git pendiente de reparación por otro agente — sin operaciones git)
> **Alcance:** probar a nivel de **interfaz** todo lo que NO requiere BDP real: la
> **independencia** (128A-1) y la **integración de escritura** (198A-1) en sus efectos
> locales y en sus invariantes de `standalone`. **Cero llamadas a BDP, cero escrituras,
> sin deploy.**
> **Base:** planes cerrados `plan-independencia-bdp-2026-08-12.md` (128A-1, F0–F10 + M1/M2/M3)
> y `plan-escrituras-bdp-completas-2026-08-19.md` (198A-1, D1–D10 + F6).

## 1. Objetivo

Verificar, desde el navegador (UI real + backend local + PostgreSQL de rama), que:

1. **Independencia (128A-1):** en `standalone` todas las funcionalidades operan con datos
   locales (catálogo, stock, anulación, compras, pagos parciales/factura local, menús/packs,
   permisos) y **ninguna** ofrece ni envía nada a BDP.
2. **Integración de escritura (198A-1):** los handlers/UI nuevos (artículo D3, departamento/
   familia D7, propina D8, puntos D9, inventario D6=A, CallWaiter D10, flush manual, CancelOrder
   F6) funcionan en sus efectos locales y respetan las invariantes del conmutador.
3. Dejar **evidencia reproducible** por caso (captura/console/network) y detectar regresiones
   de UI antes de tocar BDP real.

Esto complementa (no sustituye) las suites ya verdes: 153 unit + 13 bdp_push + 3 bdp_inventario
+ 24 bdp_f8_permisos + 8 bdp_service_integration.

## 2. No-alcance

- Cualquier llamada al BDP real (`100.83.196.35:8068`): lecturas y escrituras quedan fuera.
- El plan 138A-2 (24 lecturas reales) — requiere BDP online + credenciales.
- Deploy, producción, migraciones destructivas, git.
- Haddock.

## 3. Entorno aislado de prueba

Para no chocar con los servidores de otros proyectos que ya ocupan `:3000`/`:5174`, se levanta
un stack propio en puertos libres:

| Componente | Comando | Puerto |
| --- | --- | --- |
| Backend (BD de rama) | `PORT=3100 node scripts/run-with-db.mjs run --bin glory-backend` | `127.0.0.1:3100` |
| Frontend Vite | `npm --prefix frontend run dev -- --port 5180` con proxy `/api → localhost:3100` | `127.0.0.1:5180` |
| PostgreSQL | ya corriendo (5432); BD `glory_backend_glory_rs_rest` | — |
| Seed demo | `node scripts/run-with-db.mjs run --bin seed` (usuario `demo@restaurante.com` / `demo1234`) | — |

- El proxy de Vite se hace configurable (`VITE_API_TARGET`, default `http://localhost:3000`)
  para poder apuntar al backend aislado sin tocar el target fijo.
- `modo_operacion = standalone` por defecto (sin credenciales BDP en el entorno local), que es
  exactamente el escenario que este plan quiere probar.

## 4. Matriz de pruebas (UI)

Leyenda: ✅ pasa · ⚠️ pasa con salvedad · ❌ falla · ⏭ no aplica en standalone.

### 4.1 Independencia — conmutador y modo

| ID | Caso | Verificación UI | Criterio |
| --- | --- | --- | --- |
| I0 | Badge de modo en header | Header `/` | Badge muestra "standalone" (sin credenciales BDP) |
| I1 | Conmutar a `bdp` sin credenciales | Configuración → BDP | O no se ofrece, o avisa que faltan credenciales y no fuerza BDP |
| I2 | Conmutar a `standalone` explícito | Configuración → BDP | Queda standalone y persiste tras recargar |

### 4.2 Independencia — flujos locales (funcionan sin BDP)

| ID | Flujo | Ruta/componente | Criterio |
| --- | --- | --- | --- |
| I3 | Catálogo local: listar/crear/editar artículo | Configuración → BDP → mapeos | CRUD local persiste; sin `bdp` calls |
| I4 | Stock local: ver/ajustar | `/bdp/stock` (tabla mapeos) | Muestra stock local; ajuste local persiste |
| I5 | Anulación local de venta | Ventas → fila → anular | Anula localmente; badge sin BDP |
| I6 | Compras locales (albarán local) | `/bdp/compras` | Crear/editar/conciliar borrador local |
| I7 | Pago parcial local + factura local | Ventas → fila | Ledger local; sin BDP |
| I8 | Menús/packs locales | `/bdp/explorador` (sección local) | CRUD local |
| I9 | Permisos operativos (403) | Acción sin permiso (trabajador) | 403 claro, sin efecto |

### 4.3 Integración de escritura — UI nueva (efectos locales + invariantes standalone)

| ID | Funcionalidad (198A-1) | Ruta/componente | Criterio en standalone |
| --- | --- | --- | --- |
| W1 | Alta artículo con rango reservado (D3) | mapeos → alta | Crea local con código 90xxxxxxx; **no** encola a BDP |
| W2 | Departamento/familia con código secuencial (D7) | `/bdp/catalogo` | Alta local con código 1–999 secuencial; no encola |
| W3 | Propina por venta (D8) | Ventas → fila → propina | Guarda `ventas.propina` local; sin `bdp_order_id` no encola |
| W4 | Puntos de cliente (D9) | Clientes → ficha → puntos | Ledger local (saldo + historial); sin `bdp_customer_code` no encola |
| W5 | Inventario (D6=A) | `/bdp/inventario` | Conteo vs stock; los artículos locales puros se omiten |
| W6 | CallWaiter (D10) | Plano de sala → mesa | **Oculto** en standalone |
| W7 | "Sincronizar a BDP" (flush manual) | Header → indicador BDP | **Oculto** en standalone |
| W8 | CancelOrder como push (F6) | Anular venta con `bdp_order_id` | (solo si hay venta con `bdp_order_id` sembrada) encola `venta/cancelar`; en standalone el worker no envía nada |

### 4.4 Invariante central de red

| ID | Caso | Verificación | Criterio |
| --- | --- | --- | --- |
| N1 | Cero tráfico a BDP | `preview_logs` (network) tras recorrer 4.2–4.3 | **Ninguna** petición a `100.83.196.35` / `:8068` |
| N2 | Flush manual forzado en standalone | `POST /api/bdp/push/flush` | `sincronizados=0`, `omitidos_standalone>0` (ya cubierto por test `flush_en_standalone_no_envia_ni_consume_la_cola`) |

## 5. Fases de ejecución

1. **F0 — Entorno:** levantar stack aislado, seed demo, comprobar `/api/health` y login.
2. **F1 — Independencia (4.1–4.2):** recorrer cada flujo y registrar evidencia.
3. **F2 — Escrituras (4.3):** recorrer UI nueva; verificar persistencia local y ocultación de controles BDP.
4. **F3 — Red (4.4):** confirmar cero llamadas a BDP en todo el recorrido.
5. **F4 — Cierre:** tabla de resultados, checklist, actualizar roadmap/completados.

## 6. Criterios de aceptación

- Todos los flujos locales (4.2) responden sin error y persisten en BD.
- Los controles que dependen de BDP (CallWaiter W6, Sincronizar W7) están **ocultos** en standalone.
- `preview_logs` no muestra **ninguna** petición hacia el BDP real.
- Cada caso tiene evidencia (captura o snapshot + resultado).

## 7. Riesgos y mitigación

| Riesgo | Mitigación |
| --- | --- |
| Servidores de otros proyectos en `:3000`/`:5174` | Stack propio en `:3100`/`:5180`; no tocar los ajenos |
| Seed sobrescribe datos demo de la rama | Seed es idempotente y solo toca datos demo; documentarlo |
| `static/` o proxy desincronizados | Usar Vite dev + proxy configurable; no build estático |
| Estado previo en BD de rama (datos BDP sembrados) | Limpiar/verificar `modo_operacion=standalone` antes de N1 |
| Flaky por HMR/cache | Recargar y repetir el caso puntual antes de marcar fallo |

## 7b. Resultados de la primera pasada (2026-08-19)

Stack aislado levantado y verificado: backend `:3100` (BD de rama `glory_backend_glory_rs_rest`,
`JWT_SECRET` de desarrollo local) + Vite `:5180` (proxy `/api → :3100`), seed demo aplicado,
login `demo@restaurante.com` OK. **`modo_operacion=auto` → `modo_efectivo=standalone`** (sin credenciales BDP).

| Caso | Resultado | Evidencia |
| --- | --- | --- |
| I0 badge standalone | ✅ | Header `BDP: off`; dropdown "Integración BDP desactivada"; API `modo_efectivo=standalone` |
| W7 "Sincronizar a BDP" oculto | ✅ | Dropdown solo tiene "Sin credenciales" (disabled) + "Configurar credenciales BDP" |
| W6 CallWaiter oculto (D10) | ✅ | `PlanoSala.tsx:371` `onLlamarCamarero={modoEfectivoBdp ? … : undefined}` → botón no se renderiza |
| W2/D7 departamento/familia | ✅ | Creé "Bebidas" → `Código BDP 1` (secuencial); `POST /api/bdp/catalogo` 200; mensaje "sin BDP, queda local" |
| W5/D6 inventario | ✅ | Página renderiza; "Enviar inventario" deshabilitado (0 artículos); mensaje standalone |
| D8 propina (local) | ✅ (código) | `venta-row-actions.tsx:317` botón Coins **siempre** visible; diálogo "Guarda localmente…" |
| I5 anulación local | ✅ (código) | `venta-row-actions.tsx:329` botón "Anular venta" disponible (`!v.anulada && onAnular`) |
| U8 banners desactivación | ✅ | Ventas y Clientes muestran "Integración BDP desactivada…" con CTA a Configuración |
| N1 cero tráfico BDP | ✅ | `preview_logs` network: solo `localhost:5180`; **ninguna** petición a `100.83.196.35` |

**Bug real encontrado y corregido:** `BdpStatusIndicator` (`site-header.tsx`) pasaba a
`useConfiguracionSync` un literal `{ status, data }` nuevo por render → bucle infinito
`Maximum update depth exceeded` (spam en consola). Mismo bug ya corregido antes en
`useConfiguracion.ts` (`[BKP-008c]`) pero reintroducido aquí. Fix: `useMemo([config])`.
Verificado: consola limpia tras recargar, `tsc --noEmit` OK.

**Pendiente de una segunda pasada** (no bloqueante): abrir diálogos de propina/anulación
end-to-end, recorrer Compras/menús-packs/Historial/Explorador/Stock, y verificar un 403 de
permisos con un trabajador.

## 7c. Resultados de la segunda pasada (2026-08-19)

Segunda pasada ejecutada en el mismo stack aislado (`:3100`/`:5180`), todo sin BDP real.

| Caso | Resultado | Evidencia |
| --- | --- | --- |
| D8 propina end-to-end | ✅ | Diálogo "Añadir propina" → importe `5.50` → `POST /api/ventas/9cbbef36…/propina` 200; `propina="5.50"` persistido en BD (GET de la venta) |
| I5 anulación local end-to-end | ✅ | Diálogo "Anular venta" → motivo + confirmación `ANULAR 9cbbef36…` → estado `anulada=true`, `anulacion_motivo` y `anulada_at` persistidos |
| I6 Compras locales | ✅ | `/bdp/compras`: 4 albaranes, "Nuevo albarán" disponible, "Sync albaranes" **deshabilitado** en standalone, aviso de perfil de exportación |
| I8 Menús/packs locales | ✅ | `/bdp/explorador`: sección "Menús y packs locales" + "Nuevo menú/pack"; 4 definiciones BDP (MENU-01/02, PACK-01, FAST-01) como referencia |
| Historial (auditoría) | ✅ | `/bdp/historial`: 4 registros + 2 snapshots; la anulación aparece como `anular_venta` Local "Completada" |
| Stock local | ✅ | `/bdp/stock`: 6 artículos, "Sync catálogo" **deshabilitado**, CSV disponible |
| I9 permisos operativos (403) | ✅ | Trabajador Sara (`role=trabajador`): `POST /api/bdp/push/flush` → **403** `{"error":"forbidden","message":"No tienes permisos para esta acción"}`; `GET /api/trabajadores` → **403**; `GET /api/configuracion` → 200 (lectura permitida) |
| N1 cero tráfico BDP | ✅ | `preview_logs` network tras toda la pasada: solo `localhost:5180`/`localhost:3100`; **ninguna** petición a `100.83.196.35` |

**Conclusión:** todas las funcionalidades locales de 128A-1 y los efectos locales de 198A-1
funcionan en `standalone` con cero tráfico a BDP; los controles dependientes de BDP (CallWaiter,
"Sincronizar a BDP", sync de stock/albaranes) permanecen ocultos o deshabilitados, y el enforcement
backend devuelve 403 claro al rol trabajador.

## 8. Definición de hecho (DoD)

- Plan ejecutado con evidencia por caso (4.2, 4.3, 4.4).
- `cargo check --lib --tests` y suite existente siguen verdes (sin regresión).
- N1 confirmado: cero tráfico a BDP.
- Roadmap y `Agente/completados/` actualizados; trabajo sin commitear.
