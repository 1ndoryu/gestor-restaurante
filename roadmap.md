# Roadmap — RESTAURANTE

Sistema de restaurante con integración BDP (WebLink REST API). Backend Rust (Axum) sirve la API y la SPA React (`frontend/src/`); rama `main`. Se compila, prueba y analiza en la VPS 89 (taller) y se despliega en producción (`restaurante.wandori.us`, servicio Coolify `glory-rest`) siempre vía `coolify-manager-rs`. Stack, deploy y resumen ejecutivo BDP, íntegros en `Agente/completados/roadmap-historico-2026-10-19.md`.

## Siguiente bloque

- `02AB-8`: responder a Guillermo (lo demás de su mensaje del 02/11 está hecho o bloqueado).
- `28AA-4`: etapa 0 resuelta (Tailscale ya conecta producción con el BDP del restaurante); recoger el cuestionario de Guillermo.
- `28AA-3`: entorno de pruebas para Guillermo (ver Pendientes).

## Pendientes


- [ ] `28AA-3` — **Dejar un entorno de pruebas para Guillermo.** `rest-dev.wandori.us` ya está listo (franja de pruebas, BDP del portátil; manual en `Agente/documentacion/bdp/entorno-pruebas-2026-10-31.md`); el correo de Guillermo ya está en Access y la cuenta `demo@restaurante.com` es su usuario; falta solo que la usuaria le envíe el enlace y las credenciales (`C:/tmp/restaurante-cliente/credenciales.txt`).
- [ ] `02AB-8` — **Responder a Guillermo y recabar sus datos.** Contestar si podrá ver el BDP además de la web, proponer fecha y hora de pruebas, y pedirle: datos de hosting/panel de Entre Tenedores (no los encuentra; buscar antes en Fiverr), formas de pago, tipos de pedido, perfil de documentos y responsables para D32. Stock, cocina, llamada al camarero e inventarios quedan opcionales y desactivados hasta que confirme su uso.
- [ ] `28AA-4` — **Cerrar los huecos del plan de Guillermo.** Etapa 0 resuelta el 03/11/2026 (Tailscale ya conecta producción con el BDP del restaurante); el cuestionario del restaurante (apartado 3 del plan) lo recoge Guillermo.

## Bloqueos

- El TPV demo del portátil avisa «sin conexión con el servidor de subscripción», con límite de uso el 09/11/2026 12:42; «Volver a validar» no lo resuelve. Guillermo debe validarlo con BDP; si caduca, rest-dev pierde el WebLink.
- `Arranque up` imprime «túnel ssh no levantó» aunque el servicio esté sano si el túnel permanente ya ocupa el 3105 (falso negativo; comprobar con `curl 127.0.0.1:3105/api/health`). Corregir en el wrapper `Arranque`, no en el proyecto.
- Licencia WebLinkRestAPI (servicio 84): resuelta el 28/10/2026 (`Agente/completados/tareas-2026-10-28.md`).
- `28AA-4` 4.5: hecho el 29/10 (reinicio manual decidido por la usuaria). Pendiente: WebLinkRestAPI no tiene proceso propio (sirve HTTP.sys) y BDP.NET es límite de hardware.
- Mesa 1 (`cd4224d1`): era mesa con comanda abierta (300033), no tipo de comanda; ya se comprueba antes de crear (02/11).
- 24 `.sqlx` «D» en la 89: causa determinada (réplica atrasada en `02ee3c5`, anterior a `e8d4e23`; sin pérdida). Pendiente solo sincronizar la réplica con `main` (la usuaria decide cuándo; no se toca por iniciativa propia).
- Piloto con hardware: D03a y D09a (impresora de cocina); D31 necesita la dirección del servidor de mensajes en el terminal; D32 necesita responsables nominales aprobados por Guillermo y el restaurante.
- Límites declarados por la usuaria el 28/10/2026: D21 séptimo aviso «cambiar estado» (Monitor de Cocina); D27 (cierre de turno sin movimiento de stock en el TPV del cliente); D23-m (comandas abiertas, sin decisión de producto); el TPV no emite aviso al cambiar líneas (4.3): ver `02AB-2`.

## Planes activos

- `Agente/planes/plan-modo-prueba-bdp-2026-10-31.md` — `31AA-1` — entorno de pruebas en la 89 conectado al BDP del portátil, con aviso fijo.
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
