# Bootstrap de acceso remoto permanente

## Objetivo

Dejar un PC Windows del restaurante accesible sin depender de soporte local constante. El binario nuevo instala y configura lo que normalmente se hace a mano para poder entrar después y terminar BDP/WebLink con calma.

El flujo pensado para cliente no técnico es: doble clic al exe, aceptar UAC y seguir un asistente mínimo solo si no le enviamos un archivo de configuración ya preparado.

## Binario

- Origen: src/bin/remote_access_bootstrap.rs
- El fuente vive dentro del repositorio, en `src/bin/remote_access_bootstrap.rs`.
- Build recomendado para dejar el `.exe` dentro del repo: `cargo build --release --target-dir target --bin remote_access_bootstrap`
- Salida esperada dentro del repo: `target/release/remote_access_bootstrap.exe`.
- Nota: si `CARGO_TARGET_DIR` está apuntando a otro directorio, Cargo escribirá fuera del repo. La ruta `C:/tmp/glory-target/release/remote_access_bootstrap.exe` fue solo un caso local de esa workstation, no la ubicación que debe usarse como referencia del proyecto.
- Config opcional compañera: `remote_access_bootstrap.config.json` en la misma carpeta que el exe.

## Qué automatiza

- Instalación de Tailscale por winget y fallback silencioso a MSI estable si el PC no trae winget.
- Alta automática en el tailnet si recibe un auth key.
- Desactivación de suspensión e hibernación.
- Habilitación de RDP cuando la edición de Windows lo soporta.
- Instalación silenciosa de RustDesk, servicio, ID y password permanente.
- Regla opcional de firewall para el puerto TCP de BDP limitada a IPs Tailscale.
- Reporte final en el escritorio con IP Tailscale, estado de RDP y credenciales de RustDesk.
- Si falla, genera reporte de error y muestra un popup diciendo qué archivo reenviar a soporte.
- Si no viene config prearmada, abre prompts guiados para auth key, nombre del equipo, password de RustDesk y puerto BDP.

## Modo recomendado para cliente no técnico

1. Nosotros generamos el archivo `remote_access_bootstrap.config.json`.
2. Enviamos al cliente una carpeta con el exe y ese JSON juntos.
3. El cliente solo hace doble clic en el exe.
4. Si Windows pregunta por permisos, acepta.
5. Al final nos envía el reporte del escritorio.

## Ejemplo de `remote_access_bootstrap.config.json`

```json
{
   "tailscale_auth_key": "tskey-auth-k1234567890",
   "rustdesk_password": "ClaveLargaSegura123",
   "device_name": "restaurante-bdp",
   "bdp_port": 9000
}
```

## Parámetros

- --tailscale-auth-key tskey-... : une el equipo al tailnet sin login interactivo.
- --rustdesk-password clave : fija password permanente de RustDesk. Si no se pasa, genera uno aleatorio.
- --rustdesk-config cadena : aplica configuración RustDesk self-hosted si existe.
- --device-name nombre : hostname visible en Tailscale.
- --advertise-tags tag:restaurante : tags Tailscale opcionales.
- --bdp-port 9000 : abre el puerto TCP solo para clientes de Tailscale.
- --skip-rdp : no toca RDP.
- --skip-rustdesk : no instala RustDesk.
- --skip-power : no cambia suspensión/hibernación.

## Variables de entorno equivalentes

- GLORY_TAILSCALE_AUTH_KEY
- GLORY_RUSTDESK_PASSWORD
- GLORY_RUSTDESK_CONFIG
- GLORY_DEVICE_NAME
- GLORY_TAILSCALE_TAGS
- GLORY_BOOTSTRAP_BDP_PORT
- GLORY_BOOTSTRAP_REPORT_PATH

## Uso recomendado

1. Generar un auth key de Tailscale de un solo uso y pre-approved para el tailnet.
2. Compilar el exe en release.
3. Enviar el exe al restaurante con un comando tipo:

   remote_access_bootstrap.exe --tailscale-auth-key tskey-... --device-name restaurante-bdp --rustdesk-password ClaveLargaSegura --bdp-port 9000

4. Pedir que lo ejecuten como administrador.
5. Leer el reporte del escritorio para obtener IP Tailscale y RustDesk ID.
6. Entrar por RustDesk o RDP y terminar la configuración de BDP-NET y WebLink.

## Auth key pre-approved de Tailscale

Sí: eso lo generas tú en el panel de administración de Tailscale, no el cliente.

Qué es: una clave temporal que permite que el PC entre al tailnet sin pedir login manual al usuario del restaurante.

Qué significa `pre-approved`: el dispositivo entra ya aprobado al tailnet y no queda esperando revisión manual en el admin panel.

Uso recomendado:

1. Crear una auth key reutilizable o de un solo uso según el lote de instalaciones.
2. Marcarla como pre-approved.
3. Si quieres, también asociarle tags del tipo `tag:restaurante`.
4. Meterla en el `remote_access_bootstrap.config.json` que vas a enviar con el exe.

## Límites reales

- Si winget no existe o falla, el binario descarga el MSI amd64 estable de Tailscale e intenta instalarlo con `msiexec /qn`; solo deja el instalador en `%TEMP%` si esa instalación silenciosa también falla.
- RDP host no existe en Windows Home/Core; en ese caso RustDesk cubre la parte gráfica.
- El binario no crea usuarios de Windows ni toca BIOS/arranque tras corte eléctrico.
- BDP sigue necesitando que WebLink esté licenciado/activo y que el puerto correcto exista.

## Validación actual

- `cargo test --bin remote_access_bootstrap`
- `cargo build --release --bin remote_access_bootstrap`