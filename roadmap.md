# Roadmap — RESTAURANTE

Sistema de restaurante con integración BDP (WebLink REST API). Backend Rust (Axum) sirve la API y la SPA React (`frontend/src/`); rama `main`. Se compila, prueba y analiza en la VPS 89 (taller) y se despliega en producción (`restaurante.wandori.us`, servicio Coolify `glory-rest`) siempre vía `coolify-manager-rs`. Stack, deploy y resumen ejecutivo BDP, íntegros en `Agente/completados/roadmap-historico-2026-10-19.md`.

## Siguiente bloque

- `28AA-1`: comprobar la licencia 84 en el BDP local y repetir las cinco operaciones que daban «sin licencia». Necesita TpvNet abierto (UAC).

## Pendientes

- [ ] `28AA-1` — **Verificar la licencia 84 en el BDP local.** `GetApplicationVersion` devuelve `Application: "84"` y se repiten obtener comanda completa, modificar comanda existente, propina, añadir pago y facturar; cada una responde sin error de licencia, o se manda a Guillermo función, petición, respuesta y código (sin credenciales).
- [ ] `28AA-2` — **Ejecutar en el BDP local las fichas que esperaban la licencia.** Propinas, pagos, facturas, modificar comanda, salones y mesas, camarero y D21; evidencia por ficha (stock o caja antes y después) y sin segundo cobro ni segunda factura tras doble clic, pérdida de respuesta o reinicio.
- [ ] `28AA-3` — **Dejar un entorno de pruebas para Guillermo.** Web conectada al BDP local, usable como propietaria en un día normal; el enlace se le envía con tu autorización.
- [ ] `28AA-4` — **Cerrar los huecos del plan de Guillermo.** Definir cómo llega la web al BDP del restaurante (etapa 0) y probar el conflicto web contra TPV sobre la misma comanda con el TPV local; el cuestionario del restaurante (apartado 3 del plan) lo recoge Guillermo.

## Bloqueos

- Licencia de WebLinkRestAPI (servicio 84): BDP confirmó el 28/10/2026 que la suscripción 751080-84 ya es la completa, sin reactivar nada. Falta comprobar que el TpvNet local la refleje (`28AA-1`); hasta entonces, propinas, pagos, facturación, comandas existentes, salones y mesas, y camarero siguen sin probar.
- D21 (comanda pendiente de validación) depende de la licencia: se reintenta en `28AA-2`.
- Piloto con hardware: D03a y D09a (impresora de cocina) y D27 (el cierre de turno no movió el stock).
- D31 necesita la dirección del servidor de mensajes en el terminal; D32 necesita responsables nominales aprobados por Guillermo y el restaurante.

## Planes activos

- `Agente/planes/plan-correcciones-piloto-bdp-2026-10-26.md` — correcciones antes del piloto BDP (pruebas locales sin licencia hechas; falta repetir con la licencia 84 y el piloto). Guía de Guillermo: `Agente/usuario/plan-practico-avance-bdp-guillermo-2026-10-28.docx` (piloto por etapas 0–5).
- `Agente/planes/anexo-b8-aceptacion-2026-10-08.md` — `anexo-b8-aceptacion-2026-10-08` — fichas de aceptación D, pendientes de ejecución.
- `Agente/planes/plan-50k-2026-09-16.md` — `169A-5` — escalar a 50.000 personas con caché, CDN y re-medición.
- `Agente/planes/plan-alcance-local-bdp-2026-10-01.md` — `01AA-1` — alcance real con el TPV local licenciado.
- `Agente/planes/plan-auditoria-completa-2026-03-30.md` — `plan-auditoria-completa-2026-03-30` — auditoría completa del proyecto (abierta desde marzo).
- `Agente/planes/plan-catalogo-bdp-front-honesto-2026-09-15.md` — `plan-catalogo-bdp-front-honesto-2026-09-15` — catálogo BDP: front honesto y estado del push.
- `Agente/planes/plan-compras-bdp-2026-07-25.md` — `plan-compras-bdp-2026-07-25` — compras BDP fase 1 (lectura de albaranes).
- `Agente/planes/plan-correccion-auditoria-sentinel-2026-08-08.md` — `plan-correccion-auditoria-sentinel-2026-08-08` — simplificación y confiabilidad de Sentinel.
- `Agente/planes/plan-deploy-desde-cero-2026-09-18.md` — `plan-deploy-desde-cero-2026-09-18` — deploy a producción desde cero (BD limpia, env, usuario seguro); sin ejecutar.
- `Agente/planes/plan-deploy-produccion-intuitividad-2026-08-08.md` — `plan-deploy-produccion-intuitividad-2026-08-08` — intuitividad y despliegue BDP; borrador sin ejecutar.
- `Agente/planes/plan-desbordamiento-plano-2026-03-31.md` — `plan-desbordamiento-plano-2026-03-31` — desbordamiento del plano de sala (abierto desde marzo).
- `Agente/planes/plan-fase-d-avisos-bdp-2026-09-19.md` — `199A-1` — receptor de avisos BDP hacia la web.
- `Agente/planes/plan-fase-d-bdp-local-2026-10-10.md` — `plan-fase-d-bdp-local-2026-10-10` — fase D sobre BDP local.
- `Agente/planes/fase-d-bdp-local-2026-10-10/plan-fase-d-bdp-local-2026-10-10-07-bitacora-parte-1.md` — bitácora de la fase D local (parte 1; la parte 2 en la misma carpeta, `-08-bitacora-parte-2.md`).
- `Agente/planes/plan-fase-d-cierre-2026-10-20.md` — `10AA-3` — cierre del resto de la fase D.
- `Agente/planes/plan-glory-rs-template-2026-03-28.md` — `plan-glory-rs-template-2026-03-28` — glory-rs como framework reutilizable (abierto desde marzo).
- `Agente/planes/plan-origen50k-2026-09-17.md` — `179A-1` — abaratar el origen y llegar a 50k.
- `Agente/planes/plan-pagos-parciales-bdp-2026-07-25.md` — `plan-pagos-parciales-bdp-2026-07-25` — pagos parciales BDP (AddOrderPayment); pruebas con simulador pendientes.
- `Agente/planes/plan-pendientes-bdp-2026-07-23.md` — `plan-pendientes-bdp-2026-07-23` — pendientes de la integración BDP.
- `Agente/planes/plan-perf-vps-2026-09-16.md` — `plan-perf-vps-2026-09-16` — despliegue seguro en VPS e informe de rendimiento sin tocar el BDP real.
- `Agente/planes/plan-pruebas-escritura-bdp-real-2026-08-04.md` — `plan-pruebas-escritura-bdp-real-2026-08-04` — pruebas reales de escritura BDP (cliente, comanda, pago, factura); la fase 2.3 espera.
- `Agente/planes/plan-revision-integral-bdp-2026-09-14.md` — `149A-1` — revisión integral BDP desde cero, con confirmación visual.
- `Agente/planes/plan-saas-servidor-seguridad-2026-09-09.md` — `099A-1` — paquete restaurante: hosting UE, MFA, SaaS y seguridad.
- `Agente/planes/plan-seguridad-escrituras-bdp-2026-09-14.md` — `149A-2` — escrituras BDP con auditoría, simulaciones y confirmación visual.
- `Agente/planes/plan-stock-bdp-gestionable-2026-07-25.md` — `plan-stock-bdp-gestionable-2026-07-25` — stock BDP: página individual de solo lectura.
- `Agente/planes/plan-sync-importar-exportar-bdp-2026-09-15.md` — `159A-2` — unificar los botones de sync en Importar/Exportar.
- `Agente/planes/plan-verificacion-guia-guillermo-2026-10-05.md` — `plan-verificacion-guia-guillermo-2026-10-05` — verificación de la guía de Guillermo (8 bloques); ninguno verificado.
- `Agente/planes/plan-visibilidad-bdp-frontend-2026-07-23.md` — `plan-visibilidad-bdp-frontend-2026-07-23` — completar la visibilidad BDP en el frontend.
