# Roadmap — RESTAURANTE

Sistema de restaurante con integración BDP (WebLink REST API). Backend Rust (Axum) sirve la API y la SPA React (`frontend/src/`); rama `main`. Se compila, prueba y analiza en la VPS 89 (taller) y se despliega en producción (`restaurante.wandori.us`, servicio Coolify `glory-rest`) siempre vía `coolify-manager-rs`. Stack, deploy y resumen ejecutivo BDP, íntegros en `Agente/completados/roadmap-historico-2026-10-19.md`.

## Siguiente bloque

- `18AB-2`…`18AB-8`: los siete puntos restantes de la respuesta de Guillermo del 18/11, en el orden en que los pide. El correo a BDP (`18AB-9`) espera a las pruebas de los puntos 7 y 8.

## Pendientes


- [ ] `18AB-2` — **Separar pago y facturación.** Quitar `Invoice: true` del pago dentro de `Orders/Create` (`bdp_sync_venta/payload.rs`): el pago se envía sin factura, se comprueba con `GetOrder` que se guardó y, con saldo cero, se llama a `Orders/Invoice` por separado. Hecho = flujo completo probado contra el BDP demo: pago parcial, cuadre y factura con número.
- [ ] `18AB-3` — **Bloquear la cancelación de una comanda con pagos.** `venta::anular` y el push `CancelOrder` consultan `bdp_pagos` y el estado remoto antes de cancelar; con uno o más pagos, se rechaza desde el servidor con un mensaje que dice cuánto hay cobrado. Hecho = test de la puerta y prueba contra BDP con una comanda cobrada.
- [ ] `18AB-4` — **Aplicar `NotAllowedAsInvitation` en la web.** La web debe impedir marcar como invitación un artículo con ese campo activo, aunque BDP lo acepte. Hecho = el modelo y el editor de líneas llevan la marca, y se rechaza antes de llegar al servidor.
- [ ] `18AB-5` — **Impedir comandas duplicadas por `MarketId` + `MarketplaceOrderId`.** Índice único, bloqueo de envíos simultáneos y consulta a BDP ante respuesta dudosa antes de reenviar. Hecho = prueba con dos solicitudes simultáneas que crea una sola comanda.
- [ ] `18AB-6` — **Pedidos a domicilio con `Type=2` en Hostelería 01.** Crear, modificar, cobrar, facturar y recuperar tras reiniciar BDP. Hecho = comanda a domicilio completa en el BDP demo, con evidencia.
- [ ] `18AB-7` — **Prueba del producto eliminado en el ticket.** Dejar un producto a cero y emitir ticket: comprobar si la línea aparece.
- [ ] `18AB-8` — **Prueba del cambio de comentario de línea.** Crear una línea con comentario, cambiarlo por otro y ver cuál queda en BDP y en el ticket; si quedan los dos, buscar la forma segura de sustituirlo.
- [ ] `18AB-9` — **Correo a BDP con copia a Guillermo.** Verificar en BDP los seis puntos que pide (dividir y unir, retirar pago parcial, anular cobro, cambio de mesa, unicidad de `MarketId` + `MarketplaceOrderId`, `Type=2` en Hostelería 01) y los tres hechos ya comprobados. Copia a `guillermo.autoia@gmail.com`. Hecho = correo enviado y respuestas registradas.
- [ ] `28AA-3` — **Dejar un entorno de pruebas para Guillermo.** `rest-dev.wandori.us` está abierto sin franja: `bdp_sync_mode = automatic` con las siete categorías activas y un snapshot completo del 16/11 como red de seguridad (manual en `Agente/documentacion/bdp/entorno-pruebas-2026-10-31.md`). El TPV arranca solo al iniciar sesión (`bdp-tpv-arranque.vbs` en la carpeta Inicio, espera 90 s); el túnel `-R 8071` lo supervisa el workspace-manager. Falta: comprobar que el TPV entra sin login manual; el aviso de que puede entrar cuando quiera ya salió el 16/11.
- [ ] `28AA-4` — **Cerrar los huecos del plan de Guillermo.** Etapa 0 resuelta el 03/11/2026 (Tailscale ya conecta producción con el BDP del restaurante); el cuestionario del restaurante (apartado 3 del plan) lo recoge Guillermo.

## Bloqueos

- El TPV demo del portátil avisa «sin conexión con el servidor de subscripción», con límite de uso el 09/11/2026 12:42; «Volver a validar» no lo resuelve. Guillermo debe validarlo con BDP; si caduca, rest-dev pierde el WebLink. Comprobado el 15/11: el 09/11 no lo tumbó nada; la caída del 04/11 era el servidor WebLink REST parado (`LogServidoresRestFullWeblinkRestAPI.log`: «Parando Servidor Weblink REST API: 8070» a las 15:15). Arrancar el TPV y entrar con usuario/5555 lo levanta. El túnel inverso `-R 8071:8070` ya lo supervisa el workspace-manager (repara cada 60 s); lo que sigue a mano es el login del TPV.
- `Arranque up` imprime «túnel ssh no levantó» aunque el servicio esté sano si el túnel permanente ya ocupa el 3105 (falso negativo; comprobar con `curl 127.0.0.1:3105/api/health`). Corregir en el wrapper `Arranque`, no en el proyecto.
- Licencia WebLinkRestAPI (servicio 84): resuelta el 28/10/2026 (`Agente/completados/tareas-2026-10-28.md`).
- `28AA-4` 4.5: hecho el 29/10 (reinicio manual decidido por la usuaria). Pendiente: WebLinkRestAPI no tiene proceso propio (sirve HTTP.sys) y BDP.NET es límite de hardware.
- Mesa 1 (`cd4224d1`): era mesa con comanda abierta (300033), no tipo de comanda; ya se comprueba antes de crear (02/11).
- 24 `.sqlx` «D» en la 89: causa determinada (réplica atrasada en `02ee3c5`, anterior a `e8d4e23`; sin pérdida). Pendiente solo sincronizar la réplica con `main` (la usuaria decide cuándo; no se toca por iniciativa propia).
- Piloto con hardware: D03a y D09a (impresora de cocina); D31 no bloquea el piloto (cocina y llamada al camarero quedan apagadas hasta que el restaurante las confirme, y el ajuste es del terminal); D32 necesita responsables nominales aprobados por Guillermo y el restaurante.
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
