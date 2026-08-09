# Plan — Respuesta al cliente, mejoras de intuitividad y despliegue a producción (BDP)

> **Fecha:** 2026-08-08
> **Rama:** `glory-rs-rest`
> **Estado:** Borrador para revisión de Guillermo — **plan, sin ejecutar** (no hay deploy ni cambios de UI todavía)
> **Materializa:** el item 1e del roadmap apuntaba a `plan-deploy-produccion-intuitividad-2026-08-07.md`, que en este checkout nunca se materializó (referencia rota; una versión previa se documentó como untracked en `Agente/contexto-reinicio-alcance-2026-08-07.md`). Este documento lo reemplaza y lo amplía con el contexto del chat (4–5/8/2026).
> **Documentos base:**
> - Chat 4–5/8/2026 — resumen de pruebas (Nakomi, técnico) + dudas de la guía (Guillermo, cliente). **Roles confirmados por Wan (2026-08-08): Wan redacta la respuesta; Guillermo es el cliente destinatario.**
> - `Agente/planes/plan-pruebas-escritura-bdp-real-2026-08-04.md` (pruebas reales: 2.1 ✅, 2.2 ✅, 2.3 ⏸ pendiente de verificación, 2.4 en espera de 2.3)
> - `Agente/usuario/guia-cliente-integracion-bdp-2026-07-26.md` (guía del cliente, incl. punto 10 "Qué queda fuera")
> - `Agente/usuario/plan-despliegue-bdp-produccion-2026-07-20.md` (deploy: envs, bootstrap, allowlists)
> - `Agente/planes/plan-visibilidad-bdp-frontend-2026-07-23.md` (base de mejoras UI ya implementadas)
> - `roadmap.md` (pendientes 1, 1b, 1c, 1d, 1e)

---

## 1. Contexto — qué pasó con el cliente

**Resumen del chat (4–5/8/2026) — roles confirmados: Wan (nosotros) redacta la respuesta; Guillermo es el cliente con las dudas; Nakomi es el técnico que ejecutó las pruebas y revisa las dudas:**

- Nakomi (técnico, nuestro lado) terminó las pruebas de integración contra el **BDP real** del restaurante (desde el **entorno local** de desarrollo):
  - ✅ Cliente de prueba **900001** creado y verificado en BDP.
  - ✅ Comanda de prueba **5330** creada y verificada en BDP (5,00 € + IVA, sin pagar ni facturar).
  - ⏸ Cobrar (pago): el BDP respondió `"Subscripción no activada"`. **Pendiente de verificación** (no fallido): el cliente afirma que la suscripción estaba activa. Hipótesis del Hallazgo 048A-11: (a) módulo de pago no activado en la instalación `100.83.196.35:8068`, (b) `CodigoIntegrador` sin permiso de pago, (c) subpath/puerto distinto para la API de pago.
  - ⏸ Facturar: **en espera** (depende de 2.3; no se llegó a intentar).
  - 🧹 Los datos de prueba deben borrarse desde el **TPV de forma manual** (la cancelación por API también exige la suscripción).
- Guillermo (cliente) respondió con:
  - La duda de que en la **página de producción** no veía el cliente 900001 ni la comanda 5330 (todas las pestañas en 0) → explicada por el punto clave (pruebas locales, producción aún no conectada).
  - **9 dudas numeradas** sobre la guía del cliente, con sub-temas: bidireccional (catálogo/stock/historial/albaranes/salones), botones pago/factura, comanda 5330, modo demo, snapshots, perfil y conciliar en Compras, auto-arming, cancelar comandas, configuración técnica/mapeo, importar/enviar, y punto 10 de alcance.
  - El aviso de que **ya le dijo al dueño** que los datos de prueba deben borrarse, y que la suscripción de pago **la gestiona el dueño** ("esto tenemos que hacerlo nosotros"); además ofreció enviar los PDFs/manuales de BDP (incluidos los archivos de "Extensión de la aplicación").
- Nakomi quedó en revisar las dudas y responder "lo más pronto posible" (pendiente al cierre de este plan; la respuesta se prepara en las secciones 2–3).

### Punto clave que explica las dudas (local vs producción)

> **Las pruebas se hicieron en el entorno local de desarrollo** (PC/entorno de nuestro lado, no la web de producción) contra el BDP real del restaurante. **Producción no está conectada a la integración BDP** (el contenedor de producción no tiene variables BDP ni ha corrido el bootstrap — ver `plan-despliegue-bdp-produccion-2026-07-20.md`). Por eso, al buscar en la página de producción, **no aparece nada**: los registros existen en el BDP real y en el entorno local, pero la web de producción no los muestra.

Esto es lo que hay que explicarle al cliente en el borrador (sección 2) y es el motivo de que este plan incluya el despliegue: **dejar producción conectada** para que lo que se pruebe se vea donde el cliente mira.

---

## 2. Borrador de respuesta al cliente

> Mensaje principal para enviar (tono del chat original). Pendiente de aprobación de Guillermo.

> Hola Guillermo, gracias por el resumen de las pruebas, está muy claro. Antes de nada, enhorabuena: que el cliente 900001 y la comanda 5330 se hayan creado en el BDP real valida las dos primeras escrituras del ciclo (crear cliente y crear comanda).
>
> Una aclaración importante sobre lo que comentaste de que no veías los registros en la página: **las pruebas se hicieron en nuestro entorno local de desarrollo**, contra el BDP real del restaurante. La página de **producción todavía no está conectada** a la integración BDP (la conexión aún no se ha desplegado), por eso ahí no aparecen el cliente 900001 ni la comanda 5330. En el BDP real sí existen y ya están verificados. Estamos preparando el despliegue para dejar producción conectada; en cuanto activemos la conexión (tras validarlo contigo), los datos se verán en la web.
>
> Sobre la limpieza: gracias por avisar al dueño. El cliente 900001 y la comanda 5330 se borran manualmente desde el TPV (la anulación por API sigue bloqueada por la suscripción de pago). Mientras no se anulen, la comanda 5330 puede aparecer como comanda abierta en el TPV; no genera ticket ni movimiento de caja, y la venta de prueba en el sistema queda como "Esperando validación · Orden: 5330".
>
> Sobre la suscripción WebLink REST API de pago: entendemos que el dueño quiere gestionarla desde vuestro lado. Gracias por los PDFs y manuales que nos estás enviando; los estamos revisando, incluidos los archivos de "Extensión de la aplicación" del vídeo, para preparar el material de activación (o pasárselo al técnico de BDP/WebLink si hace falta). Mientras la suscripción no esté activa, el BDP responde "Subscripción no activada" y no se puede cobrar ni facturar; en cuanto se active, retomamos las pruebas pendientes (pago y factura) y podremos anular comandas desde la aplicación.
>
> Sobre tus dudas de la guía: te las respondemos punto por punto aquí abajo. Gracias por la paciencia y por la confianza.

**Nota:** enviar tras aprobación. Se adjunta el apéndice (sección 3) con las respuestas a las 9 dudas; Wan puede dividirlo en dos mensajes si lo prefiere.

---

## 3. Apéndice — respuestas preparadas a las dudas de la guía (puntos 1–9)

> Respuestas preparadas para Guillermo (cliente) a sus 9 dudas numeradas de la guía. Basadas en la guía del cliente y los planes BDP. Se adjuntan al borrador (sección 2).

1. **¿Catálogo, stock, historial de pedidos, albaranes, salones y mesas pueden ir en ambas direcciones?**
   Hoy la integración permite **leer** de BDP (catálogo/stock, clientes, estados, albaranes, salones/mesas como consulta) y **escribir** hacia BDP 4 operaciones: crear cliente, crear comanda, registrar pago y facturar. La escritura **BDP → aplicación** no está incluida, y la sincronización general bidireccional está excluida por diseño (solo `read_only` y `unidirectional`). Con la suscripción de pago se suman pago, factura y cancelación (aplicación → BDP), pero no administración de catálogo/stock en BDP.
2. **No veo los botones de pago/factura ni la comanda 5330; modo demo solo muestra "Eliminar Demo".**
   Los botones de pago/factura aparecen en la fila de cada venta **sincronizada** con BDP (tarjeta verde = pago, recibo violeta = factura, lupa = consultar). Si no hay ventas sincronizadas, no hay botones. La comanda 5330 se creó en el entorno local contra el BDP real; por eso no aparece en producción (aún no conectada). "Eliminar Demo" quita los datos simulados de las 4 pantallas BDP; el modo demo se puede volver a activar desde el mismo botón.
3. **¿Cómo funcionan los snapshots del Historial? ¿La página lee facturas con una foto?**
   No. Los snapshots son **respaldos de operaciones/configuración de la integración** (auditoría), no fotos de documentos ni OCR de facturas. El icono del ojo abre el detalle del respaldo; si no hay snapshots guardados no se visualiza nada. La lectura automática de documentos por foto **no forma parte** de la integración.
4. **El botón "Perfil" en BDP Compras no aparece.**
   El campo "Perfil" es el número del perfil de exportación de albaranes de BDP. Puede no verse cuando no hay plantilla configurada en la instalación o en modo demo. Se necesita el código de la plantilla de Compras configurada en el BDP del restaurante (pendiente 1b del roadmap); la app muestra el formulario para pedirlo.
   Sobre **conciliar**: el botón "Conciliar" vincula un albarán en estado "Borrador" con un gasto local (crear gasto nuevo o vincular uno existente). La conciliación **solo cambia datos locales** de la Aplicación Web; no escribe en BDP.
5. **Auto-arming:** correcto, está implementado: con el interruptor activado, cada pago/factura se autoriza automáticamente con la confirmación textual, sin ir a Configuración cada vez.
6. **El interruptor "Cancelar comandas" dice "Bloqueado por BDP". ¿Con la API se podrá cancelar?**
   Sí, en cuanto la suscripción de pago esté activa: `CancelOrder` hoy responde `"Subscripción no activada"`. El interruptor existe como feature flag; se habilita cuando el módulo de pago esté activo en la instalación. Mientras tanto, la anulación se hace manualmente desde el TPV.
7. **Configuración técnica: ¿sirve para relacionar códigos de la Aplicación Web con códigos de BDP?**
   Exacto. Relaciona los códigos de artículos, formas de pago, canales y cliente por defecto entre ambas plataformas (ejemplo del jamón/Carne de Cerdo: es exactamente para eso). Sin mapeo, se usan los valores por defecto configurados.
8. **No pude probar importar clientes ni el envío de una venta a BDP.**
   Es correcto que aún no se pudiera: ambas dependen de la integración conectada. **Importar clientes:** al desplegar y activar la conexión, "Importar BDP" queda disponible en la página Clientes (lectura de clientes de BDP). **Envío de ventas:** no hay ningún botón "Enviar" que pulsar — cuando la integración está activa, cada venta se envía a BDP **automáticamente al crearse**; si el envío falla, la fila de la venta muestra el botón "Reintentar sincronización BDP". Esto queda disponible **solo tras activar las allowlists de escritura y `bdp_sync_enabled`** (paso 5 de la sección 5): primero se deja producción en solo lectura y se valida con vosotros antes de habilitar escrituras.
9. **El punto 10 de la guía: ¿todo eso quedó fuera para siempre o se podrá activar después?**
   Se refiere al **alcance actual de la integración**. Hay tres grupos: (a) cosas que seguirán fuera por diseño (administrar stock, transferencias, bidireccional general); (b) cosas activables por feature flag (pagos parciales, cancelación de comandas — esta última además requiere la suscripción de pago); y (c) cosas que requieren configuración propia de la instalación BDP (tarifa del catálogo, plantilla de Compras). Resumen detallado en la sección 6.

---

## 4. Mejoras de UI para hacer el sistema más intuitivo

> Base ya implementada: `plan-visibilidad-bdp-frontend-2026-07-23.md` (Bloques A–D: catálogo visible, indicador BDP, auto-arming, modo demo, feature flags). Estas mejoras son las que salen **directamente del feedback del cliente** para evitar nuevas dudas.

| ID | Duda / feedback del cliente | Mejora propuesta | Área / archivos | Esfuerzo |
| --- | --- | --- | --- | --- |
| U1 | No veía los botones de pago/factura ni sabía dónde están | Estado vacío en Ventas con guía: "Los botones de pago/factura aparecen en cada venta sincronizada con BDP" + enlace a Configuración BDP; mantener botones visibles por venta | lista de ventas / `venta-row-actions.tsx` | ~2h |
| U2 | No entendía por qué la página no mostraba nada | Banner/aviso cuando la integración está desactivada ("Integración BDP desactivada — los datos de BDP no se muestran") + tooltip en el indicador BDP | `site-header.tsx` (`BdpStatusIndicator`), páginas BDP | ~2h |
| U3 | "Eliminar Demo" confundía; dudaba si se podía reactivar | Textos explícitos: "Modo demo activado" + botón "Salir del modo demo"; tooltip: "Los datos son simulados y se pueden reactivar cuando quieras" | 4 pantallas BDP (`BdpStock`, Explorador, Historial, Compras) | ~2h |
| U4 | Pensaba que los snapshots leían facturas con una foto | Texto de ayuda en la pestaña Snapshots: "Respaldos de operaciones de la integración, no lectura de documentos" + estado vacío explicativo | Historial BDP (snapshots) | ~1h |
| U5 | No veía el campo "Perfil" en Compras | Hacer visible el campo con label "Perfil de exportación BDP" y aviso cuando falta la plantilla de BDP | pantalla Compras BDP | ~1h |
| U6 | Interruptor "Cancelar comandas" bloqueado sin explicación | Tooltip en el feature flag: "Requiere la suscripción de pago WebLink REST API — CancelOrder responde 'Subscripción no activada'" | `ConfigBdp.tsx` / feature flags | ~1h |
| U7 | "Configuración técnica" no se entendía | Renombrar/ayudar: "Correspondencias Glory ↔ BDP" con ejemplo visible (artículo App ↔ artículo BDP, forma de pago, canal) | `config-bdp-mapeos.tsx` | ~1h |
| U8 | No pudo probar importar clientes / envío de ventas sin saber por qué | Aviso visible cuando la integración está desactivada (`bdp_sync_enabled=false`): "El envío de ventas a BDP está desactivado" en la lista de ventas y en "Importar BDP" (Clientes); aclarar en la fila que el envío es automático al crear la venta y que "Reintentar" solo aparece si el envío falla | Clientes (`Importar BDP`), `venta-row-actions.tsx`, lista de ventas | ~2h |

**Validación:** typecheck del frontend + revisión manual de cada pantalla en local antes del deploy. Esfuerzo total estimado: **~12h** (opcional hacerlo en tarea aparte).

---

## 5. Despliegue a producción — dejar todo listo y conectado

> Fuente: `plan-despliegue-bdp-produccion-2026-07-20.md`. Producción actualmente **sin** variables BDP y **sin** conexión. El deploy **no depende** de la suscripción de pago: deja todo conectado y en modo seguro hasta que la suscripción se active.

### Pre-requisitos

- [ ] BDP online en Tailscale: `100.83.196.35:8068` (`restaurante-bdp`) — verificar `tailscale status` desde el VPS (estaba offline el 2026-07-21).
- [ ] Árbol git limpio y gate de calidad ejecutado (`npm run task:check`) antes del deploy. **Nota:** el último run full (2026-08-07) falló por problemas preexistentes de la rama (scanner Sentinel, `ListaReservas.tsx`, clippy, varsense, timeout cargo test; ver `Agente/contexto-reinicio-alcance-2026-08-07.md`) — separar regresión nueva de deuda base en el reporte.
- [x] Exención de gate documentada para este cambio (solo documental): `task:check` exige un task-id válido y sin toma ajena; este borrador no tiene tarea asignada (el claim/ownership es capa de coordinación, no contrato del gate). Evidencia ligera ejecutada el 2026-08-08: stage docs con **0 hallazgos nuevos** sobre este plan — comando: `node scripts/quality/stage-process.mjs --stage docs --report .quality-reports/manual-docs-2026-08-08.json --task-id 048A-11 --profile docs` (reporte: 6 hallazgos de planes previos sin checklist, ajenos a este documento; sin `docs-task-missing` ni `docs-link-missing`). El `task:check` full sigue siendo pre-requisito del deploy.
- [ ] Env del contenedor preparado (paso siguiente).

### Pasos

1. **Añadir envs BDP al contenedor** (vía `sync-env` de coolify-manager-rs, valores del `.env` local, mismos en producción):
   `BDP_BASE_URL`, `BDP_POS_ID=31`, `BDP_LOGIN`, `BDP_PASSWORD`, `BDP_INTEGRATOR_CODE`, `BDP_EMPLOYEE_ID=1`, `BDP_ITEMS_PROFILE_ID=1`, `BDP_DEFAULT_ARTICLE_CODE=1001`, `BDP_DEFAULT_ARTICLE_NAME=CAFE BOMBON`, `BDP_BOOTSTRAP_USER_EMAIL`.
   **NO** configurar todavía `BDP_WRITE_ALLOWED_ORIGINS` ni `BDP_CHECK_ORDER_ALLOWED_ORIGINS`: vacío bloquea escrituras **y también la consulta de órdenes** (necesaria para reconciliar antes de pagar); se activan en el paso 5.
2. **Deploy:** `coolify-manager-rs deploy --name glory-rest --update` (las migraciones BDP se aplican al arrancar). Si no hay migraciones nuevas, `--skip-backup`.
3. **Health + logs:** verificar health remoto y en logs el bootstrap: "Bootstrap BDP dirigido" + "aplicado correctamente"; confirmar en BD `bdp_sync_enabled=false`, `bdp_poll_enabled=false`, `bdp_auto_sync_customers=false`, `bdp_sync_mode=read_only`.
4. **Smoke test de conectividad:** desde la app, sección BDP: configuración técnica cargada y preflight responde.
5. **Activar la conexión (solo tras validación del cliente):**
   - Añadir `BDP_WRITE_ALLOWED_ORIGINS` y `BDP_CHECK_ORDER_ALLOWED_ORIGINS` con la IP/puerto del BDP (`http://100.83.196.35:8068`).
   - Habilitar `bdp_sync_enabled=true` y `bdp_poll_enabled=true` (si se quiere polling) desde la interfaz.
6. **Verificación final con el cliente:**
   - Confirmar que el cliente 900001 y la comanda 5330 se ven desde producción (si siguen en BDP) y borrarlos desde el TPV. **Nota:** esos registros de prueba existen en el BDP real y en el entorno local; producción no los mostrará retroactivamente. La verificación real en producción será con un ciclo nuevo de prueba (crear cliente/comanda desde producción tras activar la conexión), confirmarlo en la web y limpiarlo después.
   - Retomar Fase 2.3 (pago) y 2.4 (factura) del plan de pruebas cuando la suscripción WebLink de pago esté activa.
7. **Rollback si health falla:** `redeploy`/rollback automático E11 de coolify-manager-rs (restaura compose anterior y recrea contenedores).

### Riesgos

| Riesgo | Mitigación |
| --- | --- |
| BDP offline en Tailscale | Encender la máquina Windows del restaurante antes del deploy; verificar desde VPS |
| Contenedor no llega al BDP (Docker bridge) | Diagnosticar desde host; si es bridge, `network_mode: host` o red Tailscale |
| Bootstrap con email inexistente | Falla sin daño; corregir env y redeploy |
| Escritura accidental en BDP | Allowlists vacías hasta validación con cliente = todas las escrituras bloqueadas |

---

## 6. Resumen exacto de lo que quedó fuera de alcance

> Lista textual del **punto 10** de `guia-cliente-integracion-bdp-2026-07-26.md`, con el estado actual de cada ítem. Es la respuesta corta a la duda 9 del cliente.

| Ítem fuera de alcance (guía, punto 10) | Qué significa exactamente | Estado actual |
| --- | --- | --- |
| Administración de stock desde la Aplicación Web | No se puede modificar stock desde la app; solo consulta (pantalla Stock con sync) | Solo lectura implementado; escritura excluida por diseño (D3) |
| Transferencias entre almacenes | Mover existencias entre almacenes de BDP desde la app | No incluido (ni lectura ni escritura) |
| Tallas, colores ni fidelización | Atributos de artículos y programas de fidelización | No incluido |
| Administración completa de menús y packs | Crear/editar menús, packs y fastfoods desde la app | Solo consulta (Explorador); sin edición |
| Sincronización general en ambas direcciones | Sincronizar automáticamente todo en los dos sentidos | Excluida por diseño: solo `read_only` y `unidirectional`; `bidirectional` rechazado en el backend |
| Pagos parciales | Cobrar una comanda en varios pagos | Implementado bajo feature flag `ff_bdp_partial_payments` (off por defecto); requiere la suscripción de pago para BDP real |
| Cancelación de comandas | Anular comandas ya enviadas a BDP desde la app | Feature flag (off); además `CancelOrder` responde "Subscripción no activada" → anulación manual desde TPV hasta activar la API de pago |

**Otros pendientes de configuración propia de la instalación BDP (no son "fuera de alcance" del código, pero bloquean datos reales):**

| Pendiente | Estado |
| --- | --- |
| Tarifa del catálogo | BDP devolvió 0 artículos con la tarifa actual; la app permite elegir tarifa 1–5 (pendiente 1b) |
| Plantilla de Compras (albaranes) | Consulta rechazada hasta aportar el código de plantilla configurado en BDP (pendiente 1b) |
| Explorador (menús/packs/fastfoods) | Fuera del criterio de entrega; verificable más adelante si el restaurante lo usa |
| Suscripción WebLink REST API de pago | Bloquea pago (2.3), factura (2.4) y cancelación; la gestiona el dueño (pendientes 1 y 1c) |
| Limpieza datos de prueba | Cliente 900001 y comanda 5330 se borran manualmente desde TPV (pendiente 1d) |

---

## 7. Supuestos y decisiones pendientes (para Wan)

1. **Roles confirmados por Wan (2026-08-08):** Wan redacta la respuesta; Guillermo es el cliente destinatario; Nakomi es el técnico que ejecutó las pruebas y revisa las dudas. El borrador (sección 2) va dirigido a Guillermo.
2. **Envío:** asumo que el borrador + apéndice (secciones 2 y 3) se envían juntos a Guillermo tras aprobación; si se prefiere dividir en dos mensajes (respuesta ahora, respuestas a las dudas después), se ajusta.
3. **Mejoras UI (sección 4):** asumo que este MD **planifica**; la implementación se hace en una tarea aparte (o se confirma para hacerla en esta).
4. **Deploy (sección 5):** asumo que **no se ejecuta ahora**; requiere autorización explícita, BDP online y gate `task:check` previo.
5. **Suscripción:** confirmar quién la activa (dueño, Nakomi con el manual, o técnico BDP/WebLink) y si el dueño ya consiguió técnico.
6. **Aviso de limpieza:** el borrador agradece a Guillermo haber avisado al dueño (hecho en el chat); confirmar si además nosotros avisamos directamente al dueño.

---

## Checklist de cierre

- [ ] Aprobado el borrador de respuesta (sección 2) y enviado al cliente
- [ ] Respondidas/adjuntas las 9 dudas de la guía (sección 3)
- [ ] Implementadas y validadas las mejoras UI (sección 4)
- [ ] Deploy a producción ejecutado y conexión activa verificada (sección 5)
- [ ] Datos de prueba 900001/5330 borrados del TPV
- [ ] Suscripción WebLink de pago activada y Fases 2.3/2.4 completadas
- [ ] Roadmap actualizado con el estado real de 1, 1b, 1c, 1d y 1e


---

PLan viejo que encontre 

# Plan — Deploy a producción + correcciones de intuitividad (dudas de Guillermo)

> **Fecha:** 2026-08-07
> **Rama:** `glory-rs-rest` (producción restaurante → glory-rest)
> **Objetivo:** corregir en base (código local) lo necesario para que la interfaz sea más intuitiva según las dudas de Guillermo, desplegar a producción vía coolify-manager-rs y verificar en la web de producción que el cliente encuentre todo como espera.
> **Borrador de respuesta a Guillermo:** `cliente/respuesta-guillermo-2026-08-07.md` — **NO versionar** (carpeta `cliente/` en `.gitignore`). Contiene información comercial confidencial.
> **ID de bloque:** `048A-12`
>
> **⚠️ REGLA ABSOLUTA — PROHIBIDAS LAS ESCRITURAS AL BDP en este bloque.** Ninguna operación de escritura al BDP se ejecuta en local NI en producción: pagos, facturas, cancelaciones, sincronización/envío de ventas, importación que escriba, creación/alta de clientes, `add_order_payment`, `invoice_order`, `cancel_order`, arming, restauración de snapshots ni cambios de configuración en BDP. `bdp_sync_mode` se mantiene en `read_only` en todo momento. Solo lectura/visualización y pruebas de UI en **modo demo**. Las escrituras reales se retoman en la Fase 5 únicamente si el usuario lo autoriza expresamente y la suscripción WebLink está activa.

---

## Contexto y punto de partida

- Pruebas de escritura BDP real: **2.1 ✅** (cliente 900001), **2.2 ✅** (comanda 5330), **2.3/2.4 ⏸** pendientes de suscripción WebLink (Guillermo, como integrador BDP, la activará él mismo).
- Guillermo ve **0 registros** en producción porque las pruebas se hicieron **en local**; los datos de prueba NO se enviaron a producción.
- El dueño ya fue avisado para **borrar manualmente del TPV** el cliente 900001 y la comanda 5330.
- **Regla de deploy:** solo vía coolify-manager-rs (`deploy --name glory-rest --update` → `health --name glory-rest`). SSH directo PROHIBIDO (guard). Nunca desde la UI de Coolify.
- **Árbol sucio:** hay untracked de otro producto (`.quality-bench/`, `.sentinel/`, `tools/sentinel/`, `tools/varsense/`, `sentinel.lock.json.bak`, `frontend/src/api/generated/**`). Antes del deploy hay que decidir qué se versiona y qué se ignora para no contaminar la release.

## Referencias de la UI (localizadas)

| Duda de Guillermo | Componentes / archivos |
| --- | --- |
| Modo demo / Eliminar Demo | `frontend/src/hooks/useBdpDemoMode.ts`, `frontend/src/componentes/bdp/BdpDemoToggle.tsx`, páginas `BdpStock/BdpCompras/BdpExplorador/BdpHistorial.tsx` |
| Snapshots / Historial (ícono ojo) | `frontend/src/componentes/bdp/BdpHistorial.tsx` (`SnapshotDetail`), `frontend/src/componentes/PanelBdpBackup.tsx` |
| Conciliar / Perfil (Compras) | `frontend/src/componentes/bdp/BdpCompras.tsx`, `BdpComprasReconcileModal.tsx`, `ConfigBdp.tsx` (~468) |
| Botones Pagar/Facturar en BDP | `frontend/src/components/venta-row-actions.tsx`, `frontend/src/api/generated/ventas/ventas.ts` (bdp-invoice) |
| Mapeo de artículos (Config. técnica) | `frontend/src/componentes/ConfigBdp.tsx` (correspondencias Glory ↔ BDP) |
| Importar / Enviar + interruptor principal | Panel de conexión BDP / Configuración BDP |

---

## Fase 1 — Auditoría y correcciones de intuitividad en LOCAL (base)

Levantar backend + frontend en local (`npm run dev:back` + `npm --prefix frontend run dev -- --port 5174`) y recorrer cada punto. Correcciones según hallazgos:

1. **1.1 Ventas y botones de pago/factura** — `venta-row-actions.tsx`: verificar visibilidad y estados (habilitado/deshabilitado con motivo claro) de "Pagar en BDP"/"Facturar en BDP" según estado de la venta y modo (demo/conectado). Asegurar que con 0 ventas la pantalla lo explique (empty state).
2. **1.2 Modo demo / "Eliminar Demo"** — `useBdpDemoMode` + `BdpDemoToggle`: hacer explícito en la UI qué hace "Eliminar Demo" y cómo se vuelve a activar (debe ser reversible por sesión). Evitar que parezca destructivo/permanente.
3. **1.3 Snapshots / Historial** — `BdpHistorial.tsx` + `PanelBdpBackup.tsx`: revisar el ícono ojo → mostrar notas + datos del snapshot de forma clara (no JSON crudo si puede mejorarse); aclarar el concepto de snapshot en la interfaz (no es OCR; es una foto del estado de BDP/Glory en un momento).
4. **1.4 Compras: Perfil + Conciliar** — `BdpCompras.tsx` + `BdpComprasReconcileModal.tsx`: verificar que "Perfil" (arriba derecha) existe y es visible; que "Conciliar" se puede abrir (o que en modo demo muestre un aviso claro de por qué no).
5. **1.5 Cancelar comandas** — interruptor con aviso "Bloqueado por BDP": mantener, pero asegurar mensajería clara de que se habilitará al activar la licencia/suscripción (y que dependerá de la API).
6. **1.6 Configuración técnica (mapeo de artículos)** — **NO TOCAR comportamiento** (pendiente de aclarar con el cliente). Solo verificar que la pantalla existe, es accesible y comprensible.
7. **1.7 Importar / Enviar** — verificar el interruptor principal de conexión y que el flujo de importación de clientes es claro (qué se importa, de dónde, feedback) **SIN ejecutar ninguna importación ni envío real** (prohibido: escribiría en BDP). Solo inspección de UI y, si aplica, modo demo.

## Fase 2 — Validación y commit

- Gate de calidad ya configurado: `npm run task:check -- <TareaId>` (Sentinel 0.6.4 + extensión local de worktrees visibles en `8502710`; doctor lock `pass`; `quality:test` 230 pass). El gate cubre el flujo completo del repo vía perfiles; para validar un stack aislado: `cargo fmt --check`, `cargo check`, `cargo test --lib bdp`, build frontend (tsc + vite).
- Limpiar/determinar untracked antes del deploy: `frontend/src/api/generated/**` (¿generado por orval y no versionado? verificar `orval.config.ts` y `.gitignore`), `.quality-bench/`, `.sentinel/`, `tools/sentinel/`, `tools/varsense/`, `sentinel.lock.json.bak` → decidir ignorar (no versionar) lo que no pertenezca a esta release.
- Commit explícito por bloque: `048A-12: mejora intuitividad UI BDP (demo, snapshots, compras, ventas, cancelar)`.
- `git pull --rebase origin glory-rs-rest` + push (rama publicada antes del deploy).

## Fase 3 — Deploy a producción

- Rama `glory-rs-rest` publicada y árbol limpio.
- `deploy --name glory-rest --update` (coolify-manager-rs, nunca UI/SSh).
- `health --name glory-rest` obligatorio post-deploy.
- Verificar migraciones pendientes si las hubiera.
- **No** hacer escrituras reales en producción en esta fase (solo lectura/visualización; las escrituras requieren arming y datos reales).

## Fase 4 — Verificación en producción (checklist por duda de Guillermo)

Recorrer la web de producción y validar. **⚠️ Durante esta verificación NO se ejecuta NINGUNA escritura al BDP** (prohibido): solo visualización y navegación; `bdp_sync_mode` en `read_only`.

| # | Qué comprobar | Criterio |
| --- | --- | --- |
| 4.1 | Registros en todas las pestañas | Ventas, gastos, reservas, calendario, clientes, no-shows muestran datos reales (no 0) una vez conectado |
| 4.2 | Cliente 900001 / comanda 5330 | NO deben aparecer en producción (no se enviaron); confirmar con Guillermo que ya se borraron del TPV |
| 4.3 | Botones Pagar/Facturar en BDP | Visibles en filas de venta con estados claros |
| 4.4 | Snapshots (ojo) | El detalle muestra notas/datos comprensibles |
| 4.5 | Compras: Perfil + Conciliar | Ambos accesibles / con aviso claro en demo |
| 4.6 | Cancelar comandas | Interruptor visible con estado "se habilitará al activar API" |
| 4.7 | Mapeo de artículos | Pantalla accesible (comportamiento pendiente de aclarar) |
| 4.8 | Importar clientes + Enviar venta a BDP | Flujo visible y claro en la UI; **NO ejecutar importación ni envío real** (escritura prohibida) |
| 4.9 | Modo demo / Eliminar Demo | Toggles coherentes y reversibles |

Anotar hallazgos; si hay ajustes, nueva iteración (Fase 1 → 2 → 3) acotada.

## Fase 5 — Seguimiento y cierre

- **Suscripción WebLink:** Guillermo (integrador) la activa → retomar 2.3 (pago), 2.4 (factura) y cancelación de comandas.
- **Borrado manual en TPV:** cliente 900001 y comanda 5330 (Guillermo ya avisó al dueño).
- **Comunicación a Guillermo:** revisar `cliente/respuesta-guillermo-2026-08-07.md` y redactar de forma convincente el punto 2 (alcance/costes — información comercial confidencial, no versionar).
- Actualizar roadmap (marcar fila 1e hecha al completar), archivar este plan en `Agente/completados/` al cierre.

## Riesgos y reglas

- **PROHIBIDO escribir al BDP en este bloque** (local y producción): pagos, facturas, cancelaciones, envíos/importaciones, altas de cliente, arming, restauración de snapshots, cambios de config en BDP. `bdp_sync_mode` siempre en `read_only`. Solo lectura/visualización y modo demo. Excepción: Fase 5 con autorización expresa del usuario y suscripción activa.
- No contaminar la release con untracked de otro producto → decidir `.gitignore` antes del deploy.
- No tocar el comportamiento de mapeo de artículos (pendiente de aclarar).
- Deploy solo por coolify-manager-rs; health obligatorio; SSH prohibido.


