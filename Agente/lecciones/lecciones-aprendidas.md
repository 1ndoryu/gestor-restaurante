# Lecciones Aprendidas

Registro de errores recurrentes, patrones que funcionaron, y conocimiento adquirido durante el desarrollo.
Cada lección debe ser concisa y accionable.

---

## 2026-03-25 — Emojis Unicode en JSX

**Problema:** Se usaron emojis Unicode directamente en componentes React en vez de SVG/iconos.
**Causa raíz:** No había regla que lo prohibiera ni detección automática.
**Solución:** Regla 18 en protocolo + regla `emoji-en-codigo` en Glory Sentinel.
**Acción preventiva:** Sentinel lo detecta automáticamente ahora.

## 2026-05-08 — Node/Vite dependencias por rama

**Problema:** Cambiar de rama no actualiza `frontend/node_modules`; Git cambia `package.json`/`package-lock.json`, pero el arbol instalado queda como estado local compartido.
**Causa raiz:** Vite puede detectar el lockfile nuevo y reoptimizar, pero no instala paquetes ausentes.
**Solucion:** El launcher compartido `glory-rs/scripts/dev.mjs` sincroniza una vez por huella de lockfile antes de arrancar Vite.
**Prevencion:** Si un script es agnostico del flujo local (`dev`, limpieza de target, reparacion de dependencias), debe vivir en `glory-rs/scripts/`; la raiz solo conserva wrappers o scripts especificos inevitables. Recalcular la huella despues de `npm install`, porque npm puede ajustar `package-lock.json`.

## 2026-03-25 — node_modules/ raíz sin .gitignore

**Problema:** `npm install` en la raíz creó `node_modules/` que no estaba en `.gitignore`, causando 2596 archivos sin trackear.
**Causa raíz:** `.gitignore` solo excluía `/frontend/node_modules/`, no `node_modules/` global.
**Solución:** Añadido `node_modules/` sin prefijo al `.gitignore`.
**Acción preventiva:** Regla 12.1 en protocolo (git limpio).

## 2026-03-25 — Comentarios JS con glob paths

**Problema:** `/* **/node_modules/** */` dentro de comentarios JS rompe el parser de TypeScript porque interpreta `*/` como cierre de comentario.
**Solución:** No usar patrones glob con `**/` dentro de comentarios JS/TS.

## 2026-03-25 — Orval v8 customInstance signature

**Problema:** Login no funcionaba. `customInstance` retornaba `response.data` pero Orval v8 genera tipos que esperan `{ data, status, headers }`.
**Causa raíz:** Los componentes checaban `respuesta.status === 200` que nunca era true.
**Solución:** Retornar `{ data: response.data, status: response.status, headers: response.headers } as T`.

## 2026-03-25 — PowerShell 5 limitaciones en scripts

**Problema:** Scripts PS1 con em dash (`—`), ternarios inline, o comas entre hashtables en arrays fallan en PS5.
**Causa raíz:** PS5 tiene parser más limitado que PS7.
**Solución:** Solo ASCII, if/else estándar, sin comas entre elementos de array de hashtables.

## 2026-03-25 — Git submodule con ruta local

**Problema:** `git submodule add ../glory-rs-framework glory-rs` resuelve contra la URL remota de GitHub, no contra el filesystem local.
**Causa raíz:** Git interpreta paths relativos en relación al remote origin, no al directorio actual.
**Solución:** Usar path absoluto + `git config --global protocol.file.allow always` (Git moderno bloquea file:// por defecto).

## 2026-03-25 — TypeScript en submódulo necesita sus propias deps

**Problema:** Componentes TSX en un submódulo no resuelven `react` types. Errores como "Property 'children' does not exist on BotonProps" aunque extends `ButtonHTMLAttributes`.
**Causa raíz:** TypeScript busca `node_modules` ascendiendo desde la ubicación del archivo. El submódulo no tiene `node_modules` y el del proyecto consumidor está en `frontend/node_modules`, no en la raíz.
**Solución:** El submódulo necesita su propio `package.json` con devDependencies (react, @types/react, lucide-react) y `npm install`. También necesita `server.fs.allow: ['..']` en Vite.

## 2026-03-26 — validator range() incompatible con rust_decimal::Decimal

**Problema:** `#[validate(range(min=0.0, max=100.0))]` no compila con `rust_decimal::Decimal` — no implementa `ValidateRangeType`.
**Causa raíz:** El crate `validator` solo soporta tipos numéricos primitivos para `range()`.
**Solución:** Crear función de validación custom (`validar_iva(&rust_decimal::Decimal) -> Result<(), ValidationError>`). OJO: el derive de `Validate` con `Option<Decimal>` pasa `&&Decimal`, no `&Option<Decimal>`.

## 2026-03-26 — PowerShell Out-File genera UTF-8 BOM que Orval no parsea

**Problema:** `cargo run --bin dump_openapi | Out-File openapi.json` genera archivo con BOM (byte order mark). Orval falla al parsearlo.
**Causa raíz:** PowerShell 5 usa UTF-8 BOM por defecto con `Out-File` y `>`.
**Solución:** Usar `[System.IO.File]::WriteAllText($path, $output, [System.Text.UTF8Encoding]::new($false))` para escribir sin BOM.

## 2026-03-26 — dump_openapi usa stdout, no argumentos

**Problema:** El binario `dump_openapi` usa `print!` para escribir el JSON, no acepta path como argumento CLI.
**Solución:** Capturar stdout y redirigir manualmente: `$output = cargo run --bin dump_openapi 2>$null; [System.IO.File]::WriteAllText(...)`.

## 2026-03-26 — Orval encoding: ó se corrompe en nombre de archivo schemas

**Problema:** Al regenerar con Orval, `gestiónRestauranteAPI.schemas.ts` se convierte en `gestiNRestauranteAPI.schemas.ts` (ó → N por encoding).
**Causa raíz:** El título del OpenAPI spec tiene `ó` que se corrompe en el filesystem dependiendo del encoding del pipe.
**Solución:** Actualizar el barrel `generated.ts` para importar desde el nombre real del archivo generado. No intentar forzar el nombre — aceptar el que Orval genera.

## 2026-03-26 — TypeScript union types en respuestas Orval requieren narrowing

**Problema:** Hooks Orval retornan `ConfiguracionRestaurante | ErrorResponse`. Acceder a campos de datos sin narrowing causa error TS.
**Causa raíz:** Orval genera tipos union para cubrir respuestas 200 y errores. TypeScript no permite acceder a campos específicos sin discriminar.
**Solución:** Usar `datos.status === 200` como discriminante antes de acceder a campos de datos.

## 2026-03-26 — Coolify beta 460 no soporta dockerfile_inline en servicios

**Problema:** `POST /api/v1/services` acepta un compose con `build: dockerfile_inline:`, pero Coolify transforma el compose reemplazando `dockerfile_inline` con `dockerfile: Dockerfile.rust` y el build nunca se ejecuta.
**Causa raíz:** Limitación de Coolify beta 460 — los servicios no soportan builds inline, solo imágenes pre-construidas.
**Solución:** Construir la imagen Docker directamente en el servidor via SSH (`docker build -t nombre:latest .`) y usar `image: nombre:latest` en el compose de Coolify.
**Acción preventiva:** El template de coolify-manager-rs debería generar un workflow que construye en el servidor, no depender de Coolify para builds.

## 2026-03-26 — Submodulo git con URL local no funciona en Deploy

**Problema:** `.gitmodules` apuntaba a ruta Windows local (`c:\Users\...`). El `git clone --recurse-submodules` en Linux fallaba.
**Solución:** Crear repo público en GitHub para el submodulo y actualizar `.gitmodules` con URL HTTPS.
**Acción preventiva:** Nunca usar paths locales en `.gitmodules` — siempre URLs de repositorios remotos.

## 2026-03-26 — Dockerfile: verificar MSRV antes de elegir imagen Rust

**Problema:** `rust:1.83` y `rust:1.85` no soportaban crates que requieren Rust ≥1.88 (time-core, home).
**Causa raíz:** Las dependencias actualizan su MSRV con cada release, pero la imagen Docker se fijó en una versión antigua.
**Solución:** Usar `rust:1.88-bookworm` (mínimo requerido por las dependencias). Verificar MSRV con `cargo check` localmente.

## 2026-03-26 — Traefik necesita estar en la misma red Docker

**Problema:** Gateway Timeout al acceder via dominio. La app era accesible directamente por puerto pero Traefik no la alcanzaba.
**Causa raíz:** Traefik (coolify-proxy) no estaba conectado a la red del servicio (`b8s0cks...`). Solo podía resolver el contenedor desde redes donde ambos estuvieran presentes.
**Solución:** `docker network connect <red-servicio> coolify-proxy`. También funciona conectar la app a la red `coolify`.

## 2026-03-26 — Coolify API: docker_compose_raw requiere base64

**Problema:** PATCH al campo `docker_compose` retorna 422 "This field is not allowed". El campo correcto es `docker_compose_raw` pero debe ser base64-encoded.
**Solución:** Usar `docker_compose_raw` con el YAML codificado en base64. PATCH envs via `/envs` sin UUID en la ruta.

## 2026-03-26 — Axum 0.7 path params: :param vs {param}

**Problema:** TODAS las rutas con path params devolvían 405 (DELETE) o 404 (GET) porque usaban sintaxis `{id}` en `.route()`.
**Causa raíz:** Axum 0.7.x usa matchit 0.7.x, que solo soporta `:param`. La sintaxis `{param}` es de matchit 0.8+ / Axum 0.8+. Las rutas con `{id}` eran tratadas como paths literales y caían al fallback SPA.
**Solución:** Cambiar `{id}` a `:id` en todas las llamadas `.route()`. Las anotaciones `#[utoipa::path]` mantienen `{id}` (sintaxis OpenAPI).
**Prevención:** No copiar la sintaxis de utoipa a los `.route()`. Agregar test que haga GET/DELETE por ID en CI.

## 2026-04-02 — SQLx offline: cambiar .sqlx cache no dispara recompilación

**Problema:** Cambiar solo el JSON en `.sqlx/` (nullable flag) no regenera el binario — cargo no trackea `.sqlx/` como dependencia.
**Causa raíz:** `query_as!` macro usa `include_str!` implícito al `.sqlx/` en compile-time; pero cargo mira timestamps de .rs, no de .sqlx.
**Solución:** Usar type override `"column?"` en la query SQL del .rs para forzar nullable. Esto cambia el .rs → cargo recompila.
**Prevención:** Para LEFT JOIN nullable, siempre usar `AS "col?"` en lugar de confiar en el cache.

## 2026-04-02 — useMutation: siempre onError con toast en acciones de usuario

**Problema:** `mutateAsync` sin try/catch causa "Uncaught (in promise)" en consola. Reportado ya en 024A-6 y repetido en 024A-9.
**Solución:** Usar `mutate` (no `mutateAsync`) + `onError` con `toast.error()` + helper `extraerMensajeError()`.
**Prevención:** Patrón obligatorio para todo useMutation con acción de usuario visible.

## 2026-03-26 — LEFT JOIN + NULL en SQLx: unexpected null

**Problema:** Query con LEFT JOIN producía `cr.nombre = NULL` para filas sin relación, causando 500 "unexpected null; try decoding as an Option" en SQLx.
**Causa raíz:** `canal_nombre` en la query no tenía `?` override y el struct tenía `Option<String>`, pero la cache `.sqlx` marcaba la columna como `nullable: false` (heredado de la definición de tabla, no del JOIN). En runtime con datos reales, NULL causaba error de decodificación.
**Solución:** `COALESCE(cr.nombre, 'Sin canal')` garantiza non-null. También agregar todos los campos no-agregados al GROUP BY.
**Prevención:** Siempre usar COALESCE en LEFT JOINs o el override `as "col?"` en SQLx para columnas que pueden ser NULL.
**Recurrencia (2026-04-02, 024A-4):** Mismo problema con `c.email AS email_cliente` en scheduler de recordatorios. El struct ya tenía `Option<String>` pero el `.sqlx` cache decía `nullable: false`. Fix: cambiar nullable flag directamente en `.sqlx/query-*.json`. Alternativa: usar `AS "email_cliente?"` (override con `?`). Lección: SQLx offline NUNCA infiere nullable correctamente desde LEFT JOINs — siempre verificar manualmente.

## 2026-03-30 — SQLx offline cache (.sqlx/) debe regenerarse tras cambios en queries

**Problema:** Deploy falló con 4 errores `SQLX_OFFLINE=true but there is no cached data for this query`. El contenedor se construía pero cargo build abortaba.
**Causa raíz:** Se modificaron queries SQL en `reserva.rs` y `cliente.rs` pero no se ejecutó `cargo sqlx prepare` para regenerar el hash de las queries en `.sqlx/`.
**Solución:** Ejecutar `cargo sqlx prepare` (requiere DB local corriendo) y commitear los archivos `.sqlx/` resultantes.
**Prevención:** SIEMPRE ejecutar `cargo sqlx prepare` después de modificar cualquier query `sqlx::query!`/`sqlx::query_as!`. Incluir en checklist pre-commit para archivos `.rs` que toquen queries.

## 2026-03-31 — Viewports condicionales no deben medirse con effects de una sola vez

**Problema:** El minimapa recibía `viewportWidth=0` y `viewportHeight=0`, así que el cuadro azul parecía roto aunque la matemática del canvas era correcta.
**Causa raíz:** El viewport se renderizaba condicionalmente y algunos componentes seguían midiéndolo con `useLayoutEffect(..., [])`; si el nodo aún no existía en ese primer commit, la medición quedaba en 0 para todo el ciclo.
**Solución:** Extraer una estrategia compartida (`useViewportSize`) que mida tras el layout real con `getBoundingClientRect()`, reintentos por `requestAnimationFrame` y `ResizeObserver`.
**Prevención:** Cuando un nodo dependa de datos async o render condicional, no usar mediciones one-shot con deps vacías; compartir el hook de medición entre consumidores para que no diverjan.

## 2026-03-31 — Un inline style puede invalidar por completo un overlay bien calculado

**Problema:** El minimapa seguía viéndose fuera de la esquina del plano incluso después de corregir `viewportWidth` y `viewportHeight`.
**Causa raíz:** `CanvasMinimap` seguía renderizando el `<canvas>` con `style={{ position: 'relative' }}`, lo que anulaba la regla CSS `.planoMinimap { position: absolute; ... }` y sacaba el overlay del comportamiento esperado.
**Solución:** Quitar el wrapper de debug y dejar que el posicionamiento dependa únicamente de la clase CSS compartida.
**Prevención:** En overlays reutilizables, evitar mezclar posicionamiento crítico en CSS con inline styles contradictorios; si la posición la define una clase, no sobrescribirla desde JSX salvo que sea imprescindible.

## 2026-04-01 — Migraciones manuales rompen SQLx migrate run

**Problema:** Se aplicó una migración manualmente via `docker exec psql` para desbloquear `cargo sqlx prepare`. El siguiente deploy falló con `constraint already exists` porque SQLx intentó re-ejecutar la migración (no había registro en `_sqlx_migrations`).
**Causa raíz:** Aplicar SQL manualmente no registra la migración en `_sqlx_migrations`. SQLx la ve como pendiente.
**Solución:** Eliminar el constraint duplicado (`DROP CONSTRAINT IF EXISTS`) y dejar que el contenedor reinicie — SQLx ejecuta la migración limpiamente y la registra.
**Prevención:** Nunca aplicar migraciones manualmente en producción. Usar `cargo sqlx migrate run` o, si no es posible, insertar manualmente el registro en `_sqlx_migrations` CON EL CHECKSUM CORRECTO (computado desde dentro del contenedor, no localmente — los line endings pueden diferir).

## 2026-04-01 — SQLx macros: patrones de conversión importantes

**Problema:** Convertir queries runtime (`query_as::`, `query_scalar::`, `query(`) a macros compile-time requiere ajustes específicos.
**Patrones clave:**
- `COUNT(*)` retorna `Option<i64>` → necesita `.unwrap_or(0)` o `AS "count!"`
- `COALESCE(col, default) AS col` en LEFT JOIN sigue siendo `Option<T>` → usar `AS "col!"`
- `Option<String>` params necesitan `.as_deref()` para pasar como `Option<&str>`
- Queries dinámicas (SQL en variable, loops sobre arrays de SQL) NO se pueden convertir — dejar como `query(sql)`
- `.bind()` desaparece: los params van directamente como argumentos del macro `query!("SQL", param1, param2)`

## 2026-04-02 — React Query mutateAsync re-lanza errores aunque exista onError

**Problema:** `enviarAMeta` en ListaPlantillas usaba `mutateAsync` con `await`, causando "Uncaught (in promise) AxiosError" en consola aunque la mutation tenía `onError`.
**Causa raíz:** `mutateAsync` SIEMPRE re-lanza el error como excepción — `onError` se ejecuta pero NO suprime la excepción. Si el caller hace `await mutateAsync(...)` sin try-catch, el error es "unhandled".
**Solución:** Usar `mutate` (fire-and-forget) cuando el caller no necesita manejar el resultado. Los errores se manejan exclusivamente en `onError`.
**Prevención:** Usar `mutateAsync` SOLO cuando se necesita `await` del resultado (ej: encadenar acciones post-éxito). Para fire-and-forget con toast de error, siempre `mutate`.

## 2026-04-03 — Docker BuildKit mount cache sirve binarios stale con --no-cache

**Problema:** `docker compose build --no-cache` no invalida `--mount=type=cache,target=/app/target`. Cargo reutiliza el binario compilado previo sin recompilar, aunque el source cambió (git clone fresco).
**Causa raíz:** `--no-cache` solo invalida layer cache, pero `--mount=type=cache` es una cache mount de BuildKit que persiste entre builds. Cargo no siempre detecta cambios si el fingerprint del target coincide.
**Solución:** Se eliminó `--mount=type=cache,target=/app/target` del Dockerfile inline permanentemente. `docker builder prune -af` puede tardar >30s y si se interrumpe, las caches persisten. Sin el mount de target, la compilación toma ~8 min en vez de ~1 min, pero garantiza binario fresco.
**Prevención:** No usar `--mount=type=cache,target=/app/target` en Dockerfiles con `git clone` como fuente. Los caches de registry/git sí son seguros (solo almacenan crates descargados).

## 2026-04-06 — serde rename_all=camelCase NO produce "ID" (capital)

**Problema:** Haddock API exige `externalID` (capital ID) pero `#[serde(rename_all = "camelCase")]` convierte `external_id` → `externalId` (lowercase d). Todos los syncs habrían fallado con 400 "Bad Request".
**Causa raíz:** `camelCase` trata cada segmento entre guiones bajos como una palabra → "external" + "id" → "externalId". Para acrónimos como "ID", "URL", "API", camelCase no los detecta.
**Solución:** Añadir `#[serde(rename = "externalID")]` override explícito en los campos afectados. El `rename` tiene prioridad sobre `rename_all`.
**Prevención:** Al integrar APIs de terceros, SIEMPRE comparar el JSON serializado contra la spec OpenAPI/Swagger real antes del primer deploy. Nunca confiar en que camelCase coincida con los nombres exactos de la API. Especial cuidado con campos que contienen acrónimos (ID, URL, API, HTML).

## 2026-04-13 — Drag de mesas y paredes: clamp con pos_x/pos_y crea "límite imaginario" (BUG RECURRENTE)

**Problema:** Las mesas no podían moverse libremente (134A-23). Luego las paredes mostraban límite como si siempre fueran horizontales (134A-24+25). El bug reapareció con cada reescritura del drag.
**Causa raíz — mesas:** `onPointerMove` clampeaba a `[0, zonaAncho*zoom - mesa.ancho]`. `zonaAncho/zonaAlto` son valores canónicos del backend que no representan el canvas visible.
**Causa raíz — paredes (VARIANTE SUTIL):** `Math.max(0, pos_x)` aplicado sobre la esquina top-left de la pared SIN considerar su rotación. Para una pared vertical (90°), `pos_x = centro_x - largo/2` es negativo aunque la pared esté dentro del plano visualmente. El clamp sobre `pos_x` actúa como si la pared fuera horizontal → límite imaginario.
**Solución:** Para todo elemento draggable en el plano: NO clampear nada con zonaData. NO hacer Math.max(0, pos_x) sobre coordenadas sin rotar. Pasar coordenadas directamente al API (solo Math.round).
**Patrón correcto:** Sin clamp durante drag, sin clamp en onMoveEnd. Si se necesita un límite, calcularlo sobre el CENTRO del bounding box rotado (bbW/bbH), nunca sobre pos_x/pos_y directo.
**Prevención:** Antes de agregar cualquier clamp en un elemento del plano, preguntarse: "¿Esta coordenada es la esquina top-left de algo que puede estar rotado?" Si sí, el clamp directo ES el bug.

## 2026-04-14 — package.json no debe invocar cargo directo en Windows

**Problema:** `npm run dev` devolvía el error opaco `"cargo" no se reconoce como un comando interno o externo` cuando Rust no estaba instalado o la terminal no había refrescado el PATH de rustup.
**Causa raíz:** Los scripts npm llamaban `cargo` directamente desde `cmd.exe`/`concurrently`, sin resolver la ruta estándar `~/.cargo/bin` ni mostrar una instrucción accionable.
**Solución:** Centralizar todas las llamadas a Rust en `scripts/run-cargo.mjs`, hacer que busque `cargo` en PATH y en la ruta estándar de rustup, y reutilizarlo también en `self-check.ps1`.
**Prevención:** En este template, nunca invocar `cargo` directamente desde `package.json`; siempre pasar por el wrapper para que el fallo de prerrequisitos sea claro y consistente.

## 2026-05-06 — Orval tags no ASCII generan carpetas corruptas en Windows

**Problema:** El tag OpenAPI `Reseñas` produjo carpetas generadas `rese├▒as`, dejando duplicados junto a la carpeta correcta.
**Causa raíz:** Orval usa el tag para nombres de carpeta y el pipe/codepage de Windows puede corromper caracteres no ASCII.
**Solución:** Usar tags ASCII (`resenas`) para codegen, activar `clean: true` en Orval y actualizar imports/barrel generados.
**Prevención:** Mantener tags OpenAPI de Orval en ASCII y hacer grep de `rese├`, `Rese├` o carpetas con caracteres raros tras regenerar.

## 2026-05-06 — Deploy local sin wrappers opacos

**Problema:** El wrapper `scripts/deploy.ps1` fallo en el backup previo y oculto el flujo real de `coolify-manager-rs`.
**Causa raíz:** El proyecto ya depende de `coolify-manager-rs`, pero el roadmap apuntaba a un wrapper local con supuestos de backup no validos para este servicio.
**Solución:** Eliminar el wrapper y documentar el comando directo desde el workspace de `coolify-manager-rs` con `--skip-backup` cuando el usuario lo indique.
**Prevención:** No introducir wrappers de deploy por proyecto si duplican `coolify-manager-rs`; actualizar el roadmap con el comando exacto que debe ejecutarse.
