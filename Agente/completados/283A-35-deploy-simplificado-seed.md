# 283A-35 — Deploy simplificado + Seed en servidor

## Tarea 26: Datos de prueba en servidor
- Dockerfile y compose inline actualizados para compilar e incluir binario `seed`
- Script `deploy.ps1 -Seed` ejecuta el seed remotamente después de deploy exitoso
- Seed crea usuario demo (CORREO_DEMO / CONTRASENA_DEMO) con datos completos

## Tarea 27: Despliegue simplificado
- Creado `scripts/deploy.ps1`: script único de deploy
  - Actualiza compose en Coolify via API
  - Reinicia servicio (o inicia si está detenido)
  - Espera health check con timeout configurable
  - Opción `-Seed` para ejecutar datos de prueba
  - Uso: `.\scripts\deploy.ps1` o `.\scripts\deploy.ps1 -Seed`

## Archivos modificados
- `Dockerfile`: + `--bin seed` + COPY seed binary
- `scripts/update-coolify-compose.ps1`: + `--bin seed` + COPY + recurse-submodules + npm install glory-rs
- `scripts/deploy.ps1` (nuevo): script de deploy end-to-end
