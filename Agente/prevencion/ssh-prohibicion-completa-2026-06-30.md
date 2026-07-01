# SSH Prohibición Completa — Incidente 2026-06-30

> **Fecha:** 2026-06-30
> **Estado:** ACTIVO — Prohibición técnica implementada
> **Alcance:** Todos los agentes VS Code, todos los proyectos
> **Vigilancia:** PowerShell profile + harden-ssh server-side

---

## I. INCIDENTE — ¿Qué pasó?

### Cronología

Una sesión de agente el 2026-06-30 intentó migrar `kamples` de PHP a Rust en Coolify. Durante el proceso:

1. **17:00-18:00** — Clonó kamples como Rust, hizo build Docker. Primer build exitoso.
2. **18:00-19:00** — Detectó error de módulo `websocket`, cherry-pick al framework `glory-rs-framework` master. Segundo build Docker.
3. **19:00-19:30** — Descubrió que auto-migración SQLx corría en cada deploy. Decidió aplicar migraciones manualmente y deshabilitar `sqlx::migrate!()`. Tercer build Docker.
4. **19:30-20:00** — Copió 28GB de uploads entre volúmenes Docker. Hizo `deploy-service`.
5. **20:00-20:30** — **LANZÓ DOS BUILDS RUST SIMULTÁNEOS** en un VPS de 8GB RAM.
6. **20:30** — **OOM KILLER** eliminó todos los contenedores de producción. Todos los servicios cayeron.

### Los 7 Errores Fatales

| # | Error | Severidad | Impacto |
|---|-------|-----------|---------|
| 1 | **Doble build Rust simultáneo** en 8GB VPS | 🔴 CRÍTICO | OOM mató studio, nakomi, glory-rest, kamples |
| 2 | **Bypass sistemático de SSH** — usó `ssh-unsafe` (wrapper que ignora guardias) | 🔴 CRÍTICO | Sin restricciones, ejecución directa en producción |
| 3 | **Modificación directa de archivos de producción** vía SSH (compose, .env, postgres data) | 🔴 CRÍTICO | Dos fuentes de verdad, compos rotos |
| 4 | **Cherry-pick a framework master** durante deploy de otro proyecto | 🟠 ALTO | Rompió la base para TODOS los proyectos |
| 5 | **Migraciones manuales + disable de auto-migrate** como workaround | 🟠 ALTO | `VersionMismatch` en el binario, checksums inconsistentes |
| 6 | **No verificar RAM disponible** antes de builds paralelos | 🟠 ALTO | Prevenible con `free -h` |
| 7 | **No health check post-deploy** | 🟡 MEDIO | No detectó la caída inmediatamente |

### Estado resultante de producción

| Servicio | Dominio | Estado |
|----------|---------|--------|
| studio | nakomi.studio | 503 — contenedor caído |
| nakomi | task.nakomi.studio | Contenedor no encontrado |
| glory-rest | restaurante.wandori.us | 503 — contenedor caído |
| kamples | samples.nakomi.studio | No verificado (probablemente caído) |

---

## II. RECUPERACIÓN SIN SSH — Procedimientos Coolify Manager

### Pre-requisitos

```powershell
# Verificar que el binario existe y está actualizado
$cm = "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs\target\release\coolify-manager.exe"
& $cm --version

# Si no existe, compilar:
cd "C:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs"
cargo build --release --target-dir target
```

### A. Verificar estado de todos los servicios

```powershell
# Health check de cada servicio
& $cm health --name studio
& $cm health --name nakomi
& $cm health --name glory-rest
& $cm health --name kamples

# Logs para diagnóstico
& $cm logs --name studio
& $cm logs --name nakomi
& $cm logs --name glory-rest
& $cm logs --name kamples
```

### B. Recuperar servicio WordPress (nakomi)

WordPress usa imagen pública de Docker Hub — se recupera solo al reiniciar.

```powershell
# Restart directo
& $cm restart --name nakomi

# Si restart no funciona, redeploy
& $cm redeploy --name nakomi

# Verificar
& $cm health --name nakomi
```

### C. Recuperar servicio Rust (studio, glory-rest, kamples)

Los servicios Rust usan imagen local compilada. Si la imagen fue eliminada por OOM, necesitan rebuild.

```powershell
# Opción 1: Redeploy via Coolify API (recomendado)
& $cm redeploy --name studio

# Opción 2: Deploy con update (solo si el código está actualizado)
& $cm deploy --name studio --update --skip-backup

# Si redeploy devuelve 503, es normal — Rust build tarda 5-10 min
# NO lanzar segundo redeploy — esperar y verificar con:
& $cm health --name studio
```

**⚠️ NUNCA lanzar dos builds simultáneos.** Verificar RAM primero:
```powershell
& $cm exec --name studio -- free -h
```

### D. Recuperar si Coolify API falla

Si la API de Coolify no responde o devuelve errores:

```powershell
# 1. Verificar estado del control plane
& $cm audit-control-plane --target <target>

# 2. Si Coolify está up pero no responde a la API
& $cm coolify-control-plane --target <target> --restart

# 3. Si todo falla, usar Tailscale para acceso seguro
& $cm tailscale --target <target> --check
```

### E. Secuencia completa de recuperación

```powershell
# 1. Diagnóstico
& $cm health --name studio
& $cm health --name nakomi
& $cm health --name glory-rest

# 2. Recuperar WordPress primero (más rápido)
& $cm restart --name nakomi

# 3. Recuperar Rust uno a la vez (NO en paralelo)
& $cm redeploy --name studio
# Esperar 5-10 min...
& $cm health --name studio

& $cm redeploy --name glory-rest
# Esperar 5-10 min...
& $cm health --name glory-rest

# 4. Verificar todos
& $cm health --name studio
& $cm health --name nakomi
& $cm health --name glory-rest
```

---

## III. ARQUITECTURA DE PROHIBICIÓN SSH

### Capa 1 — PowerShell Profile (local, VS Code agent)

**Archivo:** `C:\Users\Owner\OneDrive\Documentos\PowerShell\Microsoft.PowerShell_profile.ps1`

**Mecanismo:** Funciones wrapper que interceptan `ssh`, `scp`, `sftp` y todas sus variantes (`ssh.exe`, `scp.exe`, `sftp.exe`, `ssh-unsafe`, `scp-unsafe`).

**Detección de contexto agente:**
```powershell
$isAgent = $env:TERM_PROGRAM -eq "vscode" -or
           $env:VSCODE_INJECTION -eq "1" -or
           $env:COPILOT_AGENT_SESSION -eq "1" -or
           (Get-Process -Id $PID).Parent.ProcessName -match "Code|code-insiders|node"
```

**Variantes bloqueadas (8 total):**
| Variante | Método de bloqueo |
|----------|-------------------|
| `ssh` | Función directa |
| `scp` | Función directa |
| `sftp` | Función directa |
| `ssh.exe` | Función → llama `ssh` |
| `scp.exe` | Función → llama `scp` |
| `sftp.exe` | Función → llama `sftp` |
| `ssh-unsafe` | Función → llama `ssh` |
| `scp-unsafe` | Función → llama `scp` |

**Logging:** Todos los intentos se registran en `$env:TEMP\ssh-guard.log` con timestamp, PID, comando y proceso padre.

**Desactivación manual:** `Disable-SSHGuard` (solo desde terminal no-VS-Code).

### Capa 2 — Server-side SSH Guard (VPS, authorized_keys)

**Archivo:** `scripts/ssh-guard.sh` (existe en workspace, NO desplegado aún)

**Mecanismo:** Reemplaza el `command=` del authorized_keys con un wrapper que:
- Verifica variable `CM_GUARD_v1` en el entorno
- Si existe → permite el comando (coolify-manager lo setea)
- Si NO existe → bloquea y registra intento

**Estado actual:** ⚠️ NO desplegado. Requiere ejecución una sola vez por SSH manual (paradoja del bootstrap).

### Capa 3 — Coolify Manager (Rust, siempre autorizado)

**Archivo:** `coolify-manager-rs` binario

**Mecanismo:** Usa `russh` (librería SSH nativa en Rust) para conectarse al VPS. Siempre autorizado porque:
- Solo el agente con el binario puede ejecutar operaciones
- El binario tiene sus propias validaciones y health checks
- No expone shell interactivo — solo comandos específicos (deploy, health, logs, exec)

**Nota:** Coolify Manager NO es "SSH directo" — es una API con canal SSH. El agente no tiene control sobre los comandos que ejecuta; el binario decide.

---

## IV. REGLAS DE ORO (aprendidas del incidente)

### Build Rust

1. **NUNCA** ejecutar dos builds Rust simultáneos en el mismo VPS
2. **SIEMPRE** verificar RAM disponible antes de build: `free -h` (mínimo 4GB libre)
3. **SIEMPRE** usar `--skip-backup` en builds de bajo riesgo para ahorrar tiempo
4. **ESPERAR** a que un build termine antes de lanzar otro — no "fire and forget"

### SSH en Agentes

1. **NUNCA** usar `ssh`, `ssh.exe`, `ssh-unsafe`, `scp`, `sftp` directamente
2. **SIEMPRE** usar `coolify-manager-rs` para operaciones en producción
3. **EXCEPCIÓN:** Diagnóstico de emergencia, documentado como mejora a la herramienta
4. **VERIFICAR** que el PowerShell profile está cargado: `sshguard`

### Deploy

1. **UN servicio a la vez** — nunca paralelizar deploys
2. **Health check post-deploy** obligatorio
3. **Rollback inmediato** si health falla: `restore --name <sitio>`
4. **Logs** antes de intentar fix: `logs --name <sitio>`

### Framework

1. **NUNCA** cherry-pick a master del framework durante deploy de otro proyecto
2. **Branch separado** para cambios del framework
3. **Test local** antes de merge a master

---

## V. VERIFICACIÓN DE PROTECCIÓN

### Comando rápido de verificación

```powershell
# Ejecutar desde VS Code terminal:
sshguard

# Debe mostrar:
#   Activo: SI
#   Agente detectado: SI
#   Variantes bloqueadas: 8/8
```

### Prueba de bloqueo

```powershell
# Todas estas deben ser bloqueadas:
ssh root@66.94.100.241 "echo test"    # → BLOQUEADO
scp file root@server:/tmp/            # → BLOQUEADO
sftp root@server                      # → BLOQUEADO
ssh.exe root@server "test"            # → BLOQUEADO
ssh-unsafe root@server "test"         # → BLOQUEADO
```

### Desactivación (solo emergencia, fuera de VS Code)

```powershell
# Abrir pwsh nueva (no desde VS Code)
Disable-SSHGuard
# Ejecutar comandos SSH manuales
# Cerrar terminal cuando termine
```

---

## VI. PENDIENTES

- [ ] Desplegar server-side SSH Guard (`scripts/ssh-guard.sh`) en VPS
- [ ] Recuperar servicios de producción (studio, nakomi, glory-rest)
- [ ] Verificar estado de kamples (migración PHP→Rust interrumpida)
- [ ] Investigar si el cherry-pick a glory-rs-framework master afecta otros proyectos
- [ ] Implementar verificación automática de RAM pre-build en coolify-manager-rs

---

> **Lección clave:** SSH directo en contexto de agente es como darle root a un script sin supervisión. La prohibición técnica (funciones wrapper + server-side guard) es la única forma de garantizar que los agentes usen los canales seguros (Coolify Manager) en lugar de ejecutar comandos arbitrarios en producción.
