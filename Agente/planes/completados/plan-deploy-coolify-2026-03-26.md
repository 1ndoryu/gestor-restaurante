# Plan: Deploy glory-rs en Coolify

## Fecha: 2026-03-26
## Estado: Fase 1 completada (Dockerfile + static serving)

## Requisitos previos
1. VPS con Coolify instalado (ya existe: IP_VPS:8000)
2. Repositorio GitHub: 1ndoryu/glory-rs
3. PostgreSQL database

## Fase 1 — Preparación local (COMPLETADA)
- [x] Dockerfile multi-stage (backend Rust + frontend Vite)
- [x] Backend sirve estáticos con `tower-http::services::ServeDir`
- [x] `.dockerignore` para builds eficientes
- [x] Fallback SPA (index.html) para client-side routing

## Fase 2 — Configuración en Coolify
1. Crear nuevo proyecto "glory-rs" en Coolify UI
2. Agregar servicio PostgreSQL al stack
3. Agregar servicio "Application" conectado al repo GitHub
   - Branch: `glory-rs-rest`
   - Build pack: Dockerfile
   - Puerto: 3000
4. Variables de entorno:
   - `DATABASE_URL=postgresql://user:pass@postgres:5432/glory_rs`
   - `JWT_SECRET=<generar>` 
   - `HOST=0.0.0.0`
   - `PORT=3000`
   - `APP_URL=https://<dominio-temporal>`
   - SMTP opcionales: `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`
5. Dominio temporal asignado por Coolify

## Fase 3 — Deploy y verificación
1. Trigger deploy desde Coolify
2. Health check: `GET /api/health`
3. Verificar Swagger UI: `GET /swagger-ui`
4. Verificar frontend SPA carga correctamente
5. Probar login con demo@restaurante.com / demo1234

## Notas
- coolify-manager-rs necesita soporte para apps genéricas (no solo WordPress)
  antes de poder automatizar este deploy completamente (ver QM3 en misión CM)
- El Dockerfile usa `SQLX_OFFLINE=true` — requiere que .sqlx/ cache esté actualizado
- Las migraciones se ejecutan al iniciar la app (`sqlx::migrate!().run()`)
