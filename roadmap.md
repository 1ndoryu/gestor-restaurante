# Roadmap — RESTAURANTE

Sistema de restaurante con integración BDP (WebLink REST API). Backend Rust (Axum) sirve la API y la SPA React (`frontend/src/`); rama `main`. Se compila, prueba y analiza en la VPS 89 (taller) y se despliega en producción (`restaurante.wandori.us`, servicio Coolify `glory-rest`) siempre vía `coolify-manager-rs`. Stack, deploy y resumen ejecutivo BDP, íntegros en `Agente/completados/roadmap-historico-2026-10-19.md`.

## Siguiente bloque

- `28AA-6`: conflicto web–TPV sobre la misma comanda (docx §4.3). Hoy el aviso de comanda solo copia el estado; un cambio de líneas hecho en el TPV no llega a la web y no muestra conflicto. Abierto en la fase D junto con §4.5 (reinicios) y D15/D15t/D19/D19t.
- `28AA-3`: entorno de pruebas para Guillermo (ver Pendientes).

## Pendientes

- [ ] `28AA-3` — **Dejar un entorno de pruebas para Guillermo.** Web conectada al BDP local, usable como propietaria en un día normal; el enlace se le envía con tu autorización.
- [ ] `28AA-6` — **Resolver el conflicto web–TPV sobre la misma comanda.** Criterio: un cambio de líneas hecho en el TPV aparece en la web con aviso de conflicto o con una regla visible; nunca desaparece sin aviso (docx §4.3). Evidencia de partida: `src/services/bdp_avisos/worker.rs` (`enlazar_venta` copia solo estado y anulación).
- [ ] `28AA-4` — **Cerrar los huecos del plan de Guillermo.** Definir cómo llega la web al BDP del restaurante (etapa 0) y probar el conflicto web contra TPV sobre la misma comanda con el TPV local; el cuestionario del restaurante (apartado 3 del plan) lo recoge Guillermo.

## Bloqueos

- Licencia de WebLinkRestAPI (servicio 84): BDP confirmó el 28/10/2026 que la suscripción 751080-84 es la completa. Verificado en el BDP local el mismo día: `GetApplicationVersion` con `Application: 84` devuelve «WeblinkRestAPI» sin error y `Orders/Get` devuelve la comanda completa (la sonda con cuerpo vacío daba 01, que es el valor por defecto, no un fallo). Propina, pago y factura probados el 28/10 (`Agente/completados/tareas-2026-10-28.md`). La factura de la comanda 72 y 73 da 301300 («tipo de comanda no válido») por EndType 1 en esas comandas, no por licencia. Salones (lectura) y camarero se probaron el 28/10; modificar comanda no tiene servicio en el TPV (`28AA-2`).
- Límites declarados por la usuaria el 28/10/2026 (con motivo, fuera de esta fase de código): D21 séptimo aviso «cambiar estado» (necesita el Monitor de Cocina); modificar comanda (el TPV no tiene servicio para cambiar líneas); D27 (cierre de turno sin movimiento de stock en el TPV del cliente); D23-m (comandas abiertas, sin decisión de producto).
- Piloto con hardware: D03a y D09a (impresora de cocina).
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
