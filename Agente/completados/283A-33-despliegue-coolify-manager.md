# 283A-33 — Despliegue con coolify-manager-rs

## Cambios realizados

1. **Dockerfile** (`Dockerfile`): Actualizado `rust:1.88-bookworm` → `rust:1-bookworm` (siempre latest stable)

2. **update-coolify-compose.ps1** (`scripts/`): Actualizado el compose inline:
   - Rust image: `1.83` → `1` (latest stable)
   - Git clone con `--recurse-submodules` para incluir glory-rs submodule
   - Agregado paso `npm install` en glory-rs/ para tipos React (requerido por componentes @glory)
   - URL actualizada al dominio correcto: `app-b8s0cks444o0sogo8kg8wcgw.IP_VPS.sslip.io`
   - Agregada variable `APP_URL`

3. **coolify-manager-rs config** (`config/settings.json`):
   - Health check path: `/` → `/swagger-ui/`
   - Timeout: 20s → 30s
   - Fatal patterns actualizados para servicio Rust (no WordPress)
   - Dominio corregido a `http://app-b8s0cks444o0sogo8kg8wcgw.IP_VPS.sslip.io`

4. **coolify-manager-rs site_capabilities.rs**: Actualizado hints para template Rust (`app` / `glory`) para que el health check SSH encuentre el contenedor correcto

5. **Despliegue ejecutado**: Compose actualizado via API (PATCH exitoso), servicio arrancado. Build en progreso en el servidor.

## Flujo de deploy
1. `scripts/update-coolify-compose.ps1` → actualiza compose en Coolify API
2. `coolify-manager redeploy --name glory-rest` → stop + start + health check
3. Coolify reconstruye la imagen Docker automáticamente al start
