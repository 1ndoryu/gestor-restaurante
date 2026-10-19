# Matriz F2: worker de avisos BDP por acción (2026-10-19)

Qué hace el worker (`src/services/bdp_avisos/worker.rs`, `decidir`) con cada aviso `recibido` o `procesando` caducado, según el resultado de `GetOrder`:

| Resultado de BDP | Estado resultante | Intentos | Venta |
|---|---|---|---|
| Pedido encontrado | `procesado` (guarda el pedido) | — | Se enlaza por `bdp_order_id` si el pedido trae `Status` |
| `301000` (comanda no existe), acción `borrar` | `procesado` (esperado tras un borrado) | — | Sin cambios |
| `301000`, cualquier otra acción | `requiere_revision` | — | Sin cambios |
| Fallo transitorio | `recibido` (reintento) | `intentos` < 5 | Sin cambios |
| Fallo transitorio con `intentos` ≥ 5 (`MAX_INTENTOS`) | `requiere_revision` | 5 | Sin cambios |
| Sin `IdentificadorComanda` numérico | `requiere_revision` (no llama a BDP) | — | Sin cambios |
| `procesando` con `reclamado_en` > 600 s (`CADUCIDAD_RECLAMO_SEG`) | Se reclama de nuevo como si fuera `recibido` | +1 | Igual que arriba |

La decisión no depende de la acción salvo en `borrar`. Por eso las siete acciones comparten el mismo camino; la prueba por acción confirma que ninguna se queda fuera.

## Pruebas (89, `tests/bdp_avisos_worker.rs`)

Con doble de BDP (`DobleBdp`) y base de datos de pruebas:

| Prueba | Qué cubre |
|---|---|
| `cada_una_de_las_siete_acciones_se_procesa_con_pedido` (l. 368) | `crear`, `modificar`, `cambiar-estado`, `borrar`, `finalizar`, `traspasar`, `traspasar-linea`: cada una pasa a `procesado` con su pedido |
| `comanda_inexistente_depende_de_la_accion` (l. 393) | `301000` → `procesado` solo en `borrar`; en las demás, `requiere_revision` |
| `aviso_abandonado_en_procesando_se_retoma` (l. 422) | `procesando` con `reclamado_en` de hace 1 h se reclama; `intentos` 1 → 2 |
| `reclamo_reciente_en_procesando_no_se_toca` (l. 445) | `procesando` reciente no se reclama |
| `fallo_transitorio_vuelve_a_recibido_y_cuenta_el_intento` (l. 173) | Reintento |
| `fallo_agotado_pasa_a_revision` (l. 192) | Revisión tras 5 intentos |
| `aviso_sin_comanda_numerica_no_llama_a_bdp` (l. 215) | Sin consulta a BDP |
| `el_lote_respeta_su_limite_y_deja_el_resto_en_cola` (l. 239) | Límite de lote |
| `no_toca_avisos_de_otra_cuenta_ni_ya_procesados` (l. 264) | Aislamiento por cuenta |

Resultado en la 89 (2026-10-19): `bdp_avisos_worker` 15/15, `bdp_avisos` 11/11, `bdp_push` 12/12. `cargo check --all-targets` pasa.

## Observación en el TPV real (2026-10-18)

Los avisos que llegaron del TPV local al receptor están en [avisos-local-matriz-2026-10-18.md](avisos-local-matriz-2026-10-18.md): `crear`, `modificar`, `borrar`, `finalizar`, `traspasar` y `traspasar-linea` llegaron con 202 e inbox. `cambiar-estado` no se pudo provocar: lo emite el Monitor de Cocina, que no está instalado en este equipo.

## Límites declarados

- **Las pruebas por acción usan cargas sintéticas con doble de BDP**, no el TPV real. Prueban la decisión y la escritura del worker; no prueban que BDP responda así.
- **`cambiar-estado`: no replicable en local** (Monitor de Cocina ausente). Solo tiene prueba sintética.
- **`traspasar` / `traspasar-linea`: sin verificar** si el TPV envía en `IdentificadorComanda` la comanda de origen o la de destino. El worker consulta `IdentificadorComanda`. Queda abierto hasta comprobarlo con un traspaso real contra BDP.
- **Duplicados y desorden** de los traspasos en el TPV real: no probados. Solo el receptor se probó con carga sintética (D22a en el doc anterior).

## Fuera de esta matriz (otras tareas de la fase D)

Los códigos A05, A07, A09, A13 y A17–A20 de la guía (`alcance-fase-d-pdf-2026-10-10.html`) se cubren con D23-m, D23a, D13–D17, D18–D20, D29, D26 y D30–D32. Ninguno de ellos está terminado; el worker de avisos solo cubre la parte de avisos → BDP → venta.
