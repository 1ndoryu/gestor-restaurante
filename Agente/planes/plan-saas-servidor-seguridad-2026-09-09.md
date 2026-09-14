# Plan 099A-1 — Paquete restaurante 2026-09-09: hosting ES/UE + MFA + SaaS + seguridad

Fecha: 2026-09-09. Origen: chat Guillermo 04–09/09/2026
(`C:\Users\Owner\Downloads\Guillermo-conversacion-2026-09-09\chat.md:121-167`).
Roadmap: bloque 099A-1, pendientes 10–16.

## Objetivo

Ajustar la plataforma a lo pedido en la reunión del restaurante: hosting en España/UE fuera de
hiperescalares USA, MFA, endurecimiento (TLS/puertos/capas/escalado), entornos dev/prod,
base normativa, y dejar el SaaS multiempresa diseñado pero **en espera** hasta confirmar si
todos los clientes usarán BDP o habrá distintos softwares.

## Alcance

- F1: estructura de la página (nivel por confirmar) + plan + estimación. Entregable a Guillermo.
- F2: MFA (TOTP local u OAuth Google / Google Workspace delegado) en `login` y
  `login_trabajador` (`src/handlers/auth.rs:49`, `src/handlers/trabajadores.rs:269`,
  `src/services/auth.rs`).
- F3: migración del servicio `glory-rest` a CPD ES/UE vía coolify-manager-rs + verificación
  health/TLS/DNS. Requiere autorización explícita de deploy (no implícita).
- F4: endurecimiento — mínimos puertos expuestos, cifrado entre máquinas (texto plano solo
  intra-máquina, ya es así en la red Docker actual), separación de capas app/modelos/BD y
  plan de escalado documentado.
- F5: entorno dev/pruebas separado de producción + figuras responsables y documentación
  de seguridad/protección de datos.
- F6 (en espera): SaaS multiempresa — entidad empresa/tenant (hoy inexistente: sin
  `tenant`/`empresa_id` en `src/`), aislamiento por empresa, alta/suscripción, página inicio.

## No alcance

- SaaS (F6) hasta confirmación BDP-único vs multi-software. Pregunta ya trasladada a Guillermo.
- Registro del software en España (pendiente 16): solo aviso, sin acción.
- Nada de BDP funcional: pagos/factura/cancel siguen bloqueados por la suscripción (1/1c).
- Ningún deploy, migración ni escritura remota sin autorización explícita por operación+objetivo.

## Dependencias

- Dudas abiertas (ver §7): MFA, hosting, SaaS-BDP, estructura. Sin sus respuestas no hay F1 final.
- Proveedor/CPD ES/UE elegido + acceso configurado en coolify-manager-rs (F3).
- BDP online + suscripción solo si alguna verificación F2–F5 tocara escrituras (no previsto).

## Fases verificables

- [ ] **F0 — Dudas**: enviar las 4 preguntas y registrar respuestas. Evidencia: respuestas citadas.
- [ ] **F1 — Estructura + estimación**: documento de arquitectura (código y/o ejecutiva según
  respuesta) + estimación por fase. Evidencia: archivo en `Agente/documentacion/` + mensaje a Guillermo.
- [ ] **F2 — MFA**: implementar según opción elegida; tests login con/sin segundo factor;
  `cargo test` + `tsc` verdes. Evidencia: tests + demo en dev.
- [ ] **F3 — Hosting**: servicio recreado en CPD ES/UE, `health` PASS, TLS válido, DNS resuelve,
  smoke de la app. Evidencia: salidas de `deploy`/`health` + captura TLS. Solo con autorización.
- [ ] **F4 — Endurecimiento**: auditoría de puertos, TLS interno entre hosts, capas separadas,
  runbook de escalado. Evidencia: doc + diff de compose.
- [ ] **F5 — Entornos + normativo**: dev/prod separados, responsables designados, docs mínimas.
  Evidencia: entornos verificados + docs.
- [ ] **F6 — SaaS (tras confirmación)**: diseño tenant + migración aditiva + aislamiento verificado
  por empresa + alta/suscripción. Evidencia: plan hijo + tests de aislamiento. ~1 semana.

## Estado

F0 cerrada (dudas resueltas 2026-09-09). Siguiente: F1 (estructura ambos niveles + estimación).

## Próximo paso

Recoger respuestas a las 4 dudas → cerrar F1 (estructura + estimación) → avisar a Guillermo
con la lista de cambios hechos, como pidió (`chat.md:148`).

## Gate y Definition of Done

- Gate canónico del proyecto (`gate:check`) tras cada fase con código; F1/F5 documentales se
  cierran con revisión contra este plan.
- DoD por fase: evidencia reproducible citada arriba + roadmap actualizado + sin secretos en
  docs (prevención 2026-07-28: credenciales solo por `${VARIABLE}`, nunca literales).
- Cierre del plan: F0–F5 verificadas (F6 en plan hijo si se confirma), completada fechada en
  `Agente/completados/` y plan movido a `Agente/planes/completados/`.

## §7 Dudas — resueltas 2026-09-09

1. MFA: **OAuth Google delegado** (Google Workspace si aplica).
2. Hosting: **mantener Coolify**; recrear `glory-rest` en servidor ES/UE vía coolify-manager-rs.
3. SaaS: **en espera** hasta confirmación BDP-único vs multi-software.
4. Estructura: **ambos niveles** (ejecutiva + mapa de código).
