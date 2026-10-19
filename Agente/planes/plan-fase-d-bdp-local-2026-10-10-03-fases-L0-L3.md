# Fase D BDP local — 03 Fases L0–L3

> Parte del [plan Fase D sobre BDP local](plan-fase-d-bdp-local-2026-10-10.md) (índice y checklist). Las referencias «§N» son secciones del plan original; su mapa está en el índice.

## 5. Fases

### L0 — Preparación (solo lectura y registro; sin escrituras al TPV)

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L0.0 | Registrar RESTAURANTE en `workspace-manager/scripts/dev/registro.json` (cambio en otro repo; se integra en su repo al cierre). Anotar problemas previos: `GET http://localhost:8787/api/gate/analisis?clave=RESTAURANTE` | Principal | Entrada en registro; problemas previos anotados (AGENTS §6.1) |
| L0.1 | Host y puerto del WebLink local, sin tocar la app: proceso que escucha en el portátil (`netstat -ano`) y `/Auth/Login` de solo lectura contra ese candidato | Principal | URL base anotada sin IP real. Si hay varios candidatos, decisión 4. **Estado 2026-10-10 (10:18):** TPV reabierta por la usuaria como administrador (TpvNet PID 22312, SvrWCF PID 21252; token elevado comprobado). `Config_WebLink` 0 filas (sqlcmd); sin listener de WebLink (SvrWCF escucha en 9011 y 9021; no confirmado como WebLink). Computer-use no controla ventanas elevadas (UIPI); pc-control (cua-driver) tampoco: su SKILL.md declara UAC/High-IL fuera de alcance. La activación de Servicios Web la hace la usuaria en la TPV. Puerto candidato libre según netstat: 8070. Licencia (DEMO) por verificar. Bloqueado hasta que la usuaria active WebLink y diga el puerto |
| L0.2 | Login contra esa URL, sin escrituras | Principal | Token o error documentado |
| L0.3 | Estado actual en BD (SQL de solo lectura): `bdp_base_url`, `bdp_sync_enabled`, `bdp_sync_mode`, filas de `bdp_push_pendientes` y `bdp_write_arming` | Principal | Cola vacía, o pendientes resueltos con la usuaria antes de L1. **Resultado 2026-10-10:** `bdp_base_url` sigue en el BDP real (`IP_BDP:8068`); `bdp_sync_enabled=false`, `bdp_sync_mode=read_only`, `modo_operacion=standalone`, `push_modalidad=automatico`; `bdp_write_arming` y `bdp_poll_schedule` vacías. Cola: 0 `pendiente`, 9 `sincronizado`, 4 `error` (37b18564, 5d92004c, 67b8d9ae, dc84a08a: «ALMACÉN 1 NO EXISTE» ×3 y «CÓDIGO DE IVA 0 NO EXISTE» ×1). `error` es estado activo: el flush las reintenta. Decisión pendiente (ver L2.3) |
| L0.4 | Decidir `CancelOrder` | Usuaria (decisión 2) | **Cerrado 2026-10-17** con la recomendación de §9.2: se mantiene «pendiente de prueba local» y el roadmap (`roadmap.md:118`) ya está alineado. La prueba real exige la caja abierta (L3.c) |
| L0.5 | Corregir contradicciones de documentación: roadmap 01AA-1 ("P0 completo; siguiente P1", según `plan-alcance-local-bdp-2026-10-01.md:144`) y 199A-1 (quitar "12 preguntas a BDP") | Principal (decisión reversible del agente) | Roadmap coherente con el plan de alcance y con la decisión 2026-09-20 |
| L0.6 | Rama: no se crea todavía. Se crea desde `main` con el primer commit autorizado | Principal | Ninguna rama suelta en ningún momento (CLAUDE.md "Ramas") |

L0 cierra cuando L0.1–L0.4 tienen respuesta y L0.3 no deja pendientes sin resolver.

### L1 — Conmutar la app a BDP local

Precondición: L0.1–L0.3 OK y permiso expreso (decisión 1).

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L1.1 | Volcar la fila de `configuracion_restaurante` a `C:\tmp\fase-d-local\config-antes.sql` (incluye credenciales, por eso fuera del repo) | Principal | Archivo fuera del repo |
| L1.2 | Poner `bdp_sync_enabled = false`. Luego, en Configuración → BDP, la URL local (L0.1), login, password, código integrador y pos. La usuaria escribe los valores en la pantalla, no en el chat. Antes de guardar, comprobar que la URL no es `IP_BDP` | Usuaria escribe; principal verifica | `GET /api/configuracion` devuelve la URL local, `bdp_sync_enabled = false` y `bdp_sync_mode = read_only` |
| L1.3 | Solo si el host no es loopback: añadir la URL exacta a `BDP_WRITE_ALLOWED_ORIGINS` en `.env` y reiniciar con `Arranque up "<id>"` (requiere L0.0) | Principal, con permiso | Escritura permitida solo a esa URL |
| L1.4 | Buscar `IP_BDP` en el log del backend (`Arranque logs "<id>"`) tras L1.2 y tras L2 | Verificador (devuelve solo incidencias) | Cero apariciones |

**Rollback:** restaurar los campos de conexión desde `config-antes.sql`. `bdp_sync_enabled` y `bdp_sync_mode` quedan en `false` y `read_only`; no se reactiva ni la sincronización ni el armado de escrituras.

### L2 — Lectura contra BDP local (sin escribir)

Precondición: L1 OK.

| Paso | Qué | Quién | Criterio de aceptación |
|---|---|---|---|
| L2.1 | Script de lectura (solo `Get*` y `Export*`): login, `GetStock`, `ExportArticles`, `ExportDepartments`, pedidos abiertos | Implementador escribe en `C:\tmp`; principal lo ejecuta | Respuestas, tiempos y campos críticos registrados |
| L2.2 | Comparar conteos básicos con la pantalla del TPV | Usuaria + principal | Coincidencia o incidencia anotada |
| L2.3 | Activar `bdp_sync_enabled` solo para lectura, con `read_only`. Precondición: decidir las 4 filas `error` de la cola (L0.3). Al pasar a modo BDP, el flush automático (`push_modalidad=automatico`) las reintentaría contra el TPV local | Principal | Stock, Explorador e Historial leen del TPV local |

**Gate L2:** lectura OK o incidencias documentadas. Sin escrituras en este paso.

### L3 — Escritura controlada (`PRUEBA-*`, reversible)

Precondición: L2 OK y permiso por lote. Un lote cada vez:

- **L3.a HECHA (2026-10-17).** Crear comanda `PRUEBA-1` y verificar en TPV (ficha D03). Precondición: caja del POS 1 abierta en TpvNet; sin ella el envío falla con `301400` (sin comanda en BDP, venta local intacta). Resultado y evidencia en §8. Queda una consecuencia abierta: `bdp_default_article_name` no coincide con el catálogo real del código 1001.
- **L3.a2** Crear con envío a cocina (D03a, `OrderEndType=0`). La app usa hoy `OrderEndType=1`; el PDF espera `0` (crea, aparca y envía a cocina). **No replicable en local** (sin impresora de cocina; ver regla de alineación): se declara y no se persigue.
- **L3.b NO SOPORTADA (decidido 2026-10-19).** Modificar líneas de `PRUEBA-1` (D05–D07): la API WebLink no tiene operación para modificar líneas de una comanda enviada (solo `CreateOrder`, `GetOrder`, `CancelOrder`, `AddOrderTip`, `AddOrderPayment`, `InvoiceOrder`; manual `# WEBLINK RESTAPI.md`, categoría Comandas). Decisión: D05–D07 quedan «no soportado: API sin ruta, pendiente de BDP»; la web no ofrece modificar comandas enviadas y los cambios se hacen en el TPV (D07 prevé «función desactivada si no soportada»). Descartado: cancelar y recrear (cambia el identificador, manda anulación a cocina y no cumple D05). Sin código nuevo.
- **L3.c HECHA (2026-10-17).** Cancelar `PRUEBA-1` (`CancelOrder`, según decisión 2; D09 y D09a). Empezó bloqueada por dos defectos del guard (§9.15 y §9.16), corregidos en el mismo día; después el push salió y la comanda desapareció del BDP y de las tablas vivas del TPV. Evidencia en §8. D09a (efecto en cocina) sigue sin testigo de impresora.
- **L3.d** Pagos y facturas: pasan a L6 (bloqueados hasta confirmar suscripción).

L3 es ensayo exploratorio previo a la base T2: ninguna ficha D se da por superada con su resultado (falta la evidencia en `bdp_id_map` y demás tablas de T2). Se repite en L6–L9 con T2.

Por lote, el verificador compara la fila de `bdp_audit_log`, la respuesta de BDP y el estado en el TPV, y devuelve solo discrepancias. El modo efectivo se comprueba antes de cada lote y se vuelve a `read_only` al terminar.

**Gate L3:** lote sin discrepancias → siguiente lote. Discrepancia → parar y analizar. No reintentar a ciegas.
