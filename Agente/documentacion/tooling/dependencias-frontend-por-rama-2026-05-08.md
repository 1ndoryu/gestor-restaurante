# Dependencias frontend por rama — 2026-05-08

## Contexto

Al cambiar entre ramas, `frontend/node_modules` permanece como estado local compartido, pero `frontend/package.json` y `frontend/package-lock.json` pueden cambiar con la rama. Vite detecta que el lockfile cambió y reoptimiza dependencias, pero no instala paquetes ausentes; por eso aparecen errores como `react-helmet-async`, `i18next`, `dompurify`, Stripe o TipTap no resueltos aunque estén declarados en el `package.json` de la rama actual.

## Solución

`glory-rs/scripts/dev.mjs` calcula una huella SHA-256 de `frontend/package.json` y `frontend/package-lock.json`. Si la huella instalada en `frontend/node_modules/.glory-dev-install.json` no coincide, o si `node_modules` no existe, ejecuta:

```powershell
npm install --no-audit --no-fund
```

Después de instalar, recalcula la huella y la guarda en el marker local. En el siguiente `npm run dev`, si la rama no cambió dependencias, el arranque no paga ese coste.

## Uso

- `npm run dev` sincroniza automáticamente antes de lanzar backend + Vite.
- `node glory-rs/scripts/dev.mjs --sync-frontend` solo sincroniza dependencias frontend y sale.
- `GLORY_DEV_SKIP_FRONTEND_INSTALL=1 npm run dev` permite saltar esta reparación si se necesita diagnosticar npm manualmente.

## Ubicación

La lógica real vive en `glory-rs/scripts/dev.mjs`, no en `scripts/dev.mjs`, porque `glory-rs/` es el núcleo agnóstico compartido entre ramas y proyectos. La raíz conserva `scripts/dev.mjs` solo como wrapper de compatibilidad para ramas antiguas que todavía invoquen esa ruta.

## Gotchas

- `node_modules` no cambia solo al hacer checkout; Git no lo gestiona.
- Vite no instala dependencias, solo optimiza las que ya existen.
- Se usa `npm install` en vez de `npm ci` para evitar fricción entre versiones locales de npm y lockfiles generados por distintas ramas.
- Los scripts operativos puntuales no deben versionarse en `scripts/`; si hace falta conservarlos, deben ir como documentación o plan del agente, sin secretos y con `DryRun` por defecto.