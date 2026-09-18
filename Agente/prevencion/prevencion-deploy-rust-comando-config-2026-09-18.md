# Prevención: deploy de Rust con comando y config equivocados deja prod en crash-loop (2026-09-18)

## Caso mínimo
1. `coolify-manager.exe deploy --name glory-rest --update` ejecutado con CWD en
   `RESTAURANTE/` (sin `--config`).
2. El binario resolvió `settings.json` por orden: CWD → manifest-dir de compilación
   (`.../glorytemplate/.agent/coolify-manager-rs`, STALE: `repoUrl=glory-rs-template.git`,
   `gloryBranch=glory-rs-rest`) en vez del checkout efectivo
   (`area-trabajo/coolify-manager-rs`, con `repoUrl=gestor-restaurante.git`, `gloryBranch=main`).
3. `deploy` (pensado para WordPress) regeneró el compose con el wiring viejo
   (`glory-rs.git#glory-rs-rest`, rama del 2026-08-30) y construyó binario sin la
   migración `20260813000000` ni el mes posterior.
4. La BD prod (migrada por el deploy bueno del 2026-09-18) quedó por delante del binario:
   crash-loop `Error: VersionMissing(20260813000000)`, health 503.
5. El rollback E11 no revivió el sitio porque el tag de imagen ya estaba sobrescrito
   por el build malo (mismo síntoma en `--no-build` y rebuild).

## Capa responsable
`coolify-manager-rs`: `deploy` no debería regenerar compose de servicios `template=rust`
con defaults del template ignorando `repoUrl`/`gloryBranch`/`appBin`/`frontendDir` del
`settings.json` efectivo; y el rollback debería verificar el tag de imagen, no solo el compose.

## Detección esperada
- `diagnose` antes de desplegar: comprobar `REPO_URL`/`BRANCH`/`dockerfile` contra
  `settings.json` del checkout efectivo.
- Tras el build: si el log del contenedor muestra `VersionMissing(<n>)`, el origen del
  build es anterior a la migración `<n>` → parar y revisar wiring, no reintentar a ciegas.

## Regla operativa (hasta automatizar)
- Servicios Rust: SOLO `deploy-service --name <sitio> --config <settings-efectivo>`.
  Nunca `deploy`/`restart --all` en `glory-rest`.
- Siempre `--config` explícito al checkout efectivo
  (`area-trabajo/coolify-manager-rs/config/settings.json`); el CWD decide el config
  si se omite (`resolve_config_path`: explícito > env > CWD > manifest-dir > exe-dir).
- No tocar el `settings.json` del checkout legacy del tema WP (runtime de backups por
  Task Scheduler); la unificación de checkouts queda como tarea aparte.
