use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use serde_json::Value;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use thiserror::Error;

const TAILSCALE_EXE: &str = r"C:\Program Files\Tailscale\tailscale.exe";
const RUSTDESK_EXE: &str = r"C:\Program Files\RustDesk\rustdesk.exe";
const TAILSCALE_WINGET_ID: &str = "Tailscale.Tailscale";
const TAILSCALE_PACKAGES_URL: &str = "https://pkgs.tailscale.com/stable/#windows";
const RUSTDESK_RELEASE_API: &str = "https://api.github.com/repos/rustdesk/rustdesk/releases/latest";
const RUSTDESK_SERVICE: &str = "Rustdesk";
const COMPANION_CONFIG_FILE: &str = "remote_access_bootstrap.config.json";

#[tokio::main]
async fn main() {
    if env::consts::OS != "windows" {
        exit_with_failure(default_report_path(), &BootstrapError::Message("Este bootstrap solo funciona en Windows".to_string()));
    }

    match ensure_admin_or_relaunch() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => exit_with_failure(default_report_path(), &error),
    }

    let config = match Config::parse() {
        Ok(config) => config,
        Err(error) => exit_with_failure(default_report_path(), &error),
    };

    if let Err(error) = run(config).await {
        exit_with_failure(default_report_path(), &error);
    }
}

#[derive(Debug)]
struct Config {
    tailscale_auth_key: Option<String>,
    rustdesk_password: String,
    rustdesk_config: Option<String>,
    device_name: Option<String>,
    advertise_tags: Option<String>,
    report_path: PathBuf,
    bdp_port: Option<u16>,
    skip_power: bool,
    skip_rdp: bool,
    skip_rustdesk: bool,
}

#[derive(Debug, Default, Deserialize)]
struct CompanionConfig {
    tailscale_auth_key: Option<String>,
    rustdesk_password: Option<String>,
    rustdesk_config: Option<String>,
    device_name: Option<String>,
    advertise_tags: Option<String>,
    report_path: Option<PathBuf>,
    bdp_port: Option<u16>,
    skip_power: Option<bool>,
    skip_rdp: Option<bool>,
    skip_rustdesk: Option<bool>,
}

#[derive(Debug)]
struct TailscaleState {
    authenticated: bool,
    ip: Option<String>,
}

#[derive(Debug)]
struct RustDeskState {
    id: Option<String>,
    password: String,
}

#[derive(Debug)]
enum RdpState {
    Enabled,
    Unsupported(String),
    Skipped,
}

#[derive(Debug)]
enum StepState {
    Applied,
    Skipped,
}

#[derive(Debug, Error)]
enum BootstrapError {
    #[error("{0}")]
    Message(String),
    #[error("Falló el comando {program}: {stderr}")]
    Command { program: String, stderr: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Config {
    fn parse() -> Result<Self, BootstrapError> {
        let cli_args: Vec<String> = env::args().skip(1).collect();
        let (companion, loaded_from_file) = load_companion_config()?;

        let mut config = Self::env_defaults();
        config.apply_companion_config(companion);
        config.apply_args(cli_args.iter().cloned())?;

        if cli_args.is_empty() && !loaded_from_file {
            config.complete_with_guided_defaults()?;
        }

        Ok(config)
    }

    #[cfg(test)]
    fn parse_from_iter<I>(args: I) -> Result<Self, BootstrapError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut config = Self::env_defaults();
        config.apply_args(args)?;
        Ok(config)
    }

    fn env_defaults() -> Self {
        Self {
            tailscale_auth_key: env_var("GLORY_TAILSCALE_AUTH_KEY"),
            rustdesk_password: env_var("GLORY_RUSTDESK_PASSWORD").unwrap_or_else(random_password),
            rustdesk_config: env_var("GLORY_RUSTDESK_CONFIG"),
            device_name: env_var("GLORY_DEVICE_NAME").or_else(|| env_var("COMPUTERNAME")),
            advertise_tags: env_var("GLORY_TAILSCALE_TAGS"),
            report_path: env_var("GLORY_BOOTSTRAP_REPORT_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(default_report_path),
            bdp_port: env_var("GLORY_BOOTSTRAP_BDP_PORT").and_then(|value| value.parse().ok()),
            skip_power: false,
            skip_rdp: false,
            skip_rustdesk: false,
        }
    }

    fn apply_companion_config(&mut self, companion: CompanionConfig) {
        if let Some(value) = companion.tailscale_auth_key {
            self.tailscale_auth_key = Some(value);
        }
        if let Some(value) = companion.rustdesk_password {
            self.rustdesk_password = value;
        }
        if let Some(value) = companion.rustdesk_config {
            self.rustdesk_config = Some(value);
        }
        if let Some(value) = companion.device_name {
            self.device_name = Some(value);
        }
        if let Some(value) = companion.advertise_tags {
            self.advertise_tags = Some(value);
        }
        if let Some(value) = companion.report_path {
            self.report_path = value;
        }
        if let Some(value) = companion.bdp_port {
            self.bdp_port = Some(value);
        }
        if let Some(value) = companion.skip_power {
            self.skip_power = value;
        }
        if let Some(value) = companion.skip_rdp {
            self.skip_rdp = value;
        }
        if let Some(value) = companion.skip_rustdesk {
            self.skip_rustdesk = value;
        }
    }

    fn apply_args<I>(&mut self, args: I) -> Result<(), BootstrapError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--tailscale-auth-key" => self.tailscale_auth_key = Some(next_value(&mut args, &arg)?),
                "--rustdesk-password" => self.rustdesk_password = next_value(&mut args, &arg)?,
                "--rustdesk-config" => self.rustdesk_config = Some(next_value(&mut args, &arg)?),
                "--device-name" => self.device_name = Some(next_value(&mut args, &arg)?),
                "--advertise-tags" => self.advertise_tags = Some(next_value(&mut args, &arg)?),
                "--report-path" => self.report_path = PathBuf::from(next_value(&mut args, &arg)?),
                "--bdp-port" => {
                    self.bdp_port = Some(next_value(&mut args, &arg)?.parse().map_err(|_| {
                        BootstrapError::Message("--bdp-port debe ser un entero valido".to_string())
                    })?)
                }
                "--skip-power" => self.skip_power = true,
                "--skip-rdp" => self.skip_rdp = true,
                "--skip-rustdesk" => self.skip_rustdesk = true,
                unknown => {
                    return Err(BootstrapError::Message(format!(
                        "Parametro no soportado: {unknown}"
                    )))
                }
            }
        }

        Ok(())
    }

    fn complete_with_guided_defaults(&mut self) -> Result<(), BootstrapError> {
        show_message_box(
            "Glory Remote Bootstrap",
            "Este asistente pedirá solo los datos mínimos para dejar acceso remoto permanente. Si dejas el auth key vacío, Tailscale se instalará pero requerirá login manual después.",
            "Information",
        );

        if let Some(auth_key) = show_input_box(
            "Glory Remote Bootstrap",
            "Auth key de Tailscale. Si no tienes uno ahora, deja el campo vacío y continúa.",
            self.tailscale_auth_key.as_deref().unwrap_or(""),
        )? {
            self.tailscale_auth_key = Some(auth_key);
        }

        if let Some(device_name) = show_input_box(
            "Glory Remote Bootstrap",
            "Nombre del equipo en Tailscale.",
            self.device_name.as_deref().unwrap_or("restaurante-bdp"),
        )? {
            self.device_name = Some(device_name);
        }

        if let Some(password) = show_input_box(
            "Glory Remote Bootstrap",
            "Password permanente de RustDesk. Si no cambias nada, se usará el mostrado.",
            &self.rustdesk_password,
        )? {
            self.rustdesk_password = password;
        }

        if let Some(port) = show_input_box(
            "Glory Remote Bootstrap",
            "Puerto TCP de BDP/WebLink. Déjalo vacío si aún no lo sabes.",
            &self.bdp_port.map_or_else(String::new, |value| value.to_string()),
        )? {
            self.bdp_port = Some(port.parse().map_err(|_| {
                BootstrapError::Message("El puerto BDP debe ser un entero válido".to_string())
            })?);
        }

        Ok(())
    }
}

async fn run(config: Config) -> Result<(), BootstrapError> {
    let edition = windows_edition()?;
    let tailscale = ensure_tailscale_installed().await?;
    let tailscale_state = configure_tailscale(&tailscale, &config)?;
    let power_state = if config.skip_power { StepState::Skipped } else { disable_sleep()? };
    let rdp_state = if config.skip_rdp {
        RdpState::Skipped
    } else {
        enable_rdp(&edition)?
    };
    let rustdesk_state = if config.skip_rustdesk {
        None
    } else {
        let rustdesk = ensure_rustdesk_installed().await?;
        Some(configure_rustdesk(&rustdesk, &config)?)
    };
    let firewall_state = if let Some(port) = config.bdp_port {
        allow_bdp_port(port)?
    } else {
        StepState::Skipped
    };

    let report = build_report(
        &config,
        &edition,
        &tailscale_state,
        rustdesk_state.as_ref(),
        &rdp_state,
        &power_state,
        &firewall_state,
    );
    if let Some(parent) = config.report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config.report_path, &report)?;

    println!("Bootstrap remoto completado.");
    println!("Reporte: {}", config.report_path.display());
    println!("Tailscale IP: {}", tailscale_state.ip.as_deref().unwrap_or("pendiente"));
    if let Some(rustdesk) = rustdesk_state {
        println!("RustDesk ID: {}", rustdesk.id.as_deref().unwrap_or("pendiente"));
        println!("RustDesk password: {}", rustdesk.password);
    }
    show_message_box(
        "Glory Remote Bootstrap",
        &format!(
            "Bootstrap completado. Comparte este reporte con soporte: {}",
            config.report_path.display()
        ),
        "Information",
    );
    Ok(())
}

async fn ensure_tailscale_installed() -> Result<PathBuf, BootstrapError> {
    if Path::new(TAILSCALE_EXE).exists() {
        return Ok(PathBuf::from(TAILSCALE_EXE));
    }
    if command_exists("winget") {
        let winget_args = [
            "install",
            "--id",
            TAILSCALE_WINGET_ID,
            "--exact",
            "--silent",
            "--accept-source-agreements",
            "--accept-package-agreements",
        ];
        if try_run_capture(
            "winget",
            winget_args,
        )
        .is_ok()
            && wait_for_path(Path::new(TAILSCALE_EXE), 90).is_ok()
        {
            return Ok(PathBuf::from(TAILSCALE_EXE));
        }
    }

    install_tailscale_from_packages().await
}

async fn install_tailscale_from_packages() -> Result<PathBuf, BootstrapError> {
    let installer = env::temp_dir().join("tailscale-latest-amd64.msi");
    let download_url = latest_tailscale_msi_url().await?;
    download_to_file(&download_url, &installer).await?;

    let msi_args = vec![
        OsString::from("/i"),
        installer.as_os_str().to_os_string(),
        OsString::from("/qn"),
        OsString::from("/norestart"),
        OsString::from("TS_NOLAUNCH=1"),
        OsString::from("TS_UNATTENDEDMODE=always"),
    ];

    match run_capture("msiexec", msi_args) {
        Ok(_) => {
            wait_for_path(Path::new(TAILSCALE_EXE), 90)?;
            Ok(PathBuf::from(TAILSCALE_EXE))
        }
        Err(error) => Err(BootstrapError::Message(format!(
            "No pude instalar Tailscale automáticamente. Dejé el MSI en {}. Error: {error}",
            installer.display()
        ))),
    }
}

async fn latest_tailscale_msi_url() -> Result<String, BootstrapError> {
    let page = reqwest::Client::new()
        .get(TAILSCALE_PACKAGES_URL)
        .header("User-Agent", "glory-remote-bootstrap")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    extract_tailscale_amd64_msi_url(&page)
}

fn extract_tailscale_amd64_msi_url(page: &str) -> Result<String, BootstrapError> {
    page.split('"')
        .find(|chunk| {
            chunk.starts_with("https://pkgs.tailscale.com/stable/tailscale-setup-")
                && chunk.ends_with("-amd64.msi")
        })
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            BootstrapError::Message(
                "No encontré el MSI amd64 de Tailscale en el feed estable".to_string(),
            )
        })
}

fn configure_tailscale(exe: &Path, config: &Config) -> Result<TailscaleState, BootstrapError> {
    if let Some(auth_key) = &config.tailscale_auth_key {
        let mut args = vec!["up".to_string(), format!("--auth-key={auth_key}"), "--accept-dns=true".to_string()];
        if let Some(name) = &config.device_name {
            args.push(format!("--hostname={name}"));
        }
        if let Some(tags) = &config.advertise_tags {
            args.push(format!("--advertise-tags={tags}"));
        }
        run_capture(exe, args.iter().map(String::as_str))?;
    }

    let ip = try_run_capture(exe, ["ip", "-4"])
        .ok()
        .and_then(|output| output.lines().map(str::trim).find(|line| !line.is_empty()).map(ToOwned::to_owned));

    Ok(TailscaleState {
        authenticated: ip.is_some(),
        ip,
    })
}

fn disable_sleep() -> Result<StepState, BootstrapError> {
    run_capture("powercfg", ["/change", "standby-timeout-ac", "0"])?;
    run_capture("powercfg", ["/change", "standby-timeout-dc", "0"])?;
    run_capture("powercfg", ["/hibernate", "off"])?;
    Ok(StepState::Applied)
}

fn enable_rdp(edition: &str) -> Result<RdpState, BootstrapError> {
    if !edition_supports_rdp(edition) {
        return Ok(RdpState::Unsupported(edition.to_string()));
    }
    run_powershell(rdp_firewall_script())?;
    Ok(RdpState::Enabled)
}

fn rdp_firewall_script() -> &'static str {
    r"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Terminal Server' -Name 'fDenyTSConnections' -Value 0
# Permitir RDP con cuentas sin contrasena (LimitBlankPasswordUse = 1 bloquea con error 0xc07)
Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Lsa' -Name 'LimitBlankPasswordUse' -Value 0
# Agregar BUILTIN\Users al grupo Remote Desktop Users usando SID S-1-5-32-555 (language-neutral)
$rdpGroup = Get-LocalGroup | Where-Object { $_.SID.Value -eq 'S-1-5-32-555' } | Select-Object -First 1
if ($rdpGroup) { Add-LocalGroupMember -Group $rdpGroup.Name -Member 'BUILTIN\Users' -ErrorAction SilentlyContinue }
$rules = Get-NetFirewallRule -Name 'RemoteDesktop-*' -ErrorAction SilentlyContinue
if ($rules) {
    $rules | Enable-NetFirewallRule | Out-Null
} else {
    $rule = Get-NetFirewallRule -DisplayName 'Glory Remote Desktop Tailscale' -ErrorAction SilentlyContinue
    if ($rule) {
        $rule | Enable-NetFirewallRule | Out-Null
    } else {
        New-NetFirewallRule -DisplayName 'Glory Remote Desktop Tailscale' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 3389 -RemoteAddress 100.64.0.0/10 | Out-Null
    }
}"
}

async fn ensure_rustdesk_installed() -> Result<PathBuf, BootstrapError> {
    if Path::new(RUSTDESK_EXE).exists() {
        return Ok(PathBuf::from(RUSTDESK_EXE));
    }

    let release: Value = reqwest::Client::new()
        .get(RUSTDESK_RELEASE_API)
        .header("User-Agent", "glory-remote-bootstrap")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let download_url = release["assets"]
        .as_array()
        .and_then(|assets| {
            assets.iter().find_map(|asset| {
                let name = asset["name"].as_str()?;
                if name.ends_with("x86_64.exe") {
                    asset["browser_download_url"].as_str().map(ToOwned::to_owned)
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| BootstrapError::Message("No encontré el instalador de RustDesk x86_64".to_string()))?;

    let installer = env::temp_dir().join("rustdesk-latest-x86_64.exe");
    download_to_file(&download_url, &installer).await?;
    run_capture(&installer, ["--silent-install"])?;
    wait_for_path(Path::new(RUSTDESK_EXE), 90)?;
    Ok(PathBuf::from(RUSTDESK_EXE))
}

fn configure_rustdesk(exe: &Path, config: &Config) -> Result<RustDeskState, BootstrapError> {
    let _ = try_run_capture(exe, ["--install-service"]);
    let _ = try_run_capture("sc", ["start", RUSTDESK_SERVICE]);
    wait_for_service(RUSTDESK_SERVICE, 30)?;

    if let Some(rustdesk_config) = &config.rustdesk_config {
        run_capture(exe, ["--config", rustdesk_config.as_str()])?;
    }
    run_capture(exe, ["--password", config.rustdesk_password.as_str()])?;
    let id = try_run_capture(exe, ["--get-id"])
        .ok()
        .and_then(|output| output.lines().map(str::trim).find(|line| !line.is_empty()).map(ToOwned::to_owned));

    Ok(RustDeskState {
        id,
        password: config.rustdesk_password.clone(),
    })
}

fn allow_bdp_port(port: u16) -> Result<StepState, BootstrapError> {
    let name = format!("Glory BDP WebLink {port}");
    let script = format!(
        "$rule = Get-NetFirewallRule -DisplayName '{name}' -ErrorAction SilentlyContinue; if (-not $rule) {{ New-NetFirewallRule -DisplayName '{name}' -Direction Inbound -Action Allow -Protocol TCP -LocalPort {port} -RemoteAddress 100.64.0.0/10 | Out-Null }}"
    );
    run_powershell(&script)?;
    Ok(StepState::Applied)
}

fn build_report(
    config: &Config,
    edition: &str,
    tailscale: &TailscaleState,
    rustdesk: Option<&RustDeskState>,
    rdp: &RdpState,
    power: &StepState,
    firewall: &StepState,
) -> String {
    let mut report = String::new();
    let _ = writeln!(report, "Glory Remote Bootstrap");
    let _ = writeln!(report, "Windows edition: {edition}");
    let _ = writeln!(report, "Tailscale auth: {}", if tailscale.authenticated { "ok" } else { "pendiente" });
    let _ = writeln!(report, "Tailscale IP: {}", tailscale.ip.as_deref().unwrap_or("pendiente"));
    match rdp {
        RdpState::Enabled => {
            let _ = writeln!(report, "RDP: habilitado");
        }
        RdpState::Unsupported(found) => {
            let _ = writeln!(report, "RDP: no soportado por la edicion {found}");
        }
        RdpState::Skipped => {
            let _ = writeln!(report, "RDP: omitido");
        }
    }
    if let Some(rustdesk) = rustdesk {
        let _ = writeln!(report, "RustDesk ID: {}", rustdesk.id.as_deref().unwrap_or("pendiente"));
        let _ = writeln!(report, "RustDesk password: {}", rustdesk.password);
    }
    let _ = writeln!(report, "Suspension: {}", render_step(power));
    let _ = writeln!(report, "Firewall BDP: {}", render_step(firewall));
    if let Some(port) = config.bdp_port {
        let _ = writeln!(report, "BDP port objetivo: {port}");
    }
    let _ = writeln!(report, "Siguiente paso: conectarse por Tailscale y luego configurar BDP-NET y WebLink desde el PC del restaurante.");
    report
}

fn exit_with_failure(report_path: PathBuf, error: &BootstrapError) -> ! {
    let final_path = if write_failure_report(&report_path, error).is_ok() {
        report_path
    } else {
        let fallback = env::temp_dir().join("glory-remote-bootstrap-error.txt");
        let _ = write_failure_report(&fallback, error);
        fallback
    };

    show_message_box(
        "Glory Remote Bootstrap",
        &format!(
            "Bootstrap remoto falló. Envía este reporte a soporte: {}",
            final_path.display()
        ),
        "Error",
    );
    eprintln!("Bootstrap remoto falló: {error}");
    std::process::exit(1);
}

fn write_failure_report(report_path: &Path, error: &BootstrapError) -> Result<(), BootstrapError> {
    let mut report = String::new();
    let _ = writeln!(report, "Glory Remote Bootstrap - ERROR");
    let _ = writeln!(report, "Cuando: {}", chrono::Utc::now().to_rfc3339());
    let _ = writeln!(report, "Error: {error}");
    let _ = writeln!(report, "Sugerencia: reenviar este archivo completo a soporte.");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, report)?;
    Ok(())
}

fn ensure_admin_or_relaunch() -> Result<bool, BootstrapError> {
    if is_admin()? {
        return Ok(false);
    }

    relaunch_elevated()?;
    Ok(true)
}

fn relaunch_elevated() -> Result<(), BootstrapError> {
    let exe = env::current_exe()?;
    let mut script = format!(
        "Start-Process -FilePath '{}' -Verb RunAs",
        ps_single_quote(&exe.to_string_lossy())
    );
    let args: Vec<String> = env::args_os()
        .skip(1)
        .map(|arg| format!("'{}'", ps_single_quote(&arg.to_string_lossy())))
        .collect();
    if !args.is_empty() {
        script.push_str(&format!(" -ArgumentList @({})", args.join(", ")));
    }
    run_powershell(&script)?;
    Ok(())
}

fn load_companion_config() -> Result<(CompanionConfig, bool), BootstrapError> {
    let path = companion_config_path()?;
    if !path.exists() {
        return Ok((CompanionConfig::default(), false));
    }

    let content = fs::read_to_string(&path)?;
    let companion = serde_json::from_str(&content)?;
    Ok((companion, true))
}

fn companion_config_path() -> Result<PathBuf, BootstrapError> {
    Ok(env::current_exe()?
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(COMPANION_CONFIG_FILE))
}

fn show_input_box(title: &str, prompt: &str, default_value: &str) -> Result<Option<String>, BootstrapError> {
    let script = format!(
        "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.Interaction]::InputBox('{}','{}','{}')",
        ps_single_quote(prompt),
        ps_single_quote(title),
        ps_single_quote(default_value),
    );
    let value = run_powershell(&script)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

fn show_message_box(title: &str, message: &str, icon: &str) {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}','{}',[System.Windows.Forms.MessageBoxButtons]::OK,[System.Windows.Forms.MessageBoxIcon]::{}) | Out-Null",
        ps_single_quote(message),
        ps_single_quote(title),
        icon,
    );
    let _ = run_powershell(&script);
}

fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn run_powershell(script: &str) -> Result<String, BootstrapError> {
    run_capture(
        "powershell",
        ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script],
    )
}

fn is_admin() -> Result<bool, BootstrapError> {
    Ok(run_powershell("[bool](([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))")?
        .trim()
        .eq_ignore_ascii_case("true"))
}

fn windows_edition() -> Result<String, BootstrapError> {
    run_powershell(r"(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').EditionID")
}

fn edition_supports_rdp(edition: &str) -> bool {
    let normalized = edition.to_ascii_lowercase();
    !normalized.contains("core") && !normalized.contains("home")
}

fn command_exists(command: &str) -> bool {
    Command::new("where")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

async fn download_to_file(url: &str, path: &Path) -> Result<(), BootstrapError> {
    let bytes = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "glory-remote-bootstrap")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    fs::write(path, bytes)?;
    Ok(())
}

fn wait_for_path(path: &Path, timeout_seconds: u64) -> Result<(), BootstrapError> {
    for _ in 0..timeout_seconds {
        if path.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(1));
    }
    Err(BootstrapError::Message(format!("No apareció {}", path.display())))
}

fn wait_for_service(service_name: &str, timeout_seconds: u64) -> Result<(), BootstrapError> {
    for _ in 0..timeout_seconds {
        let status = try_run_capture(
            "powershell",
            [
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &format!("(Get-Service -Name '{service_name}' -ErrorAction SilentlyContinue).Status"),
            ],
        )
        .unwrap_or_default();
        if status.trim().eq_ignore_ascii_case("Running") {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(1));
    }
    Err(BootstrapError::Message(format!("El servicio {service_name} no arrancó")))
}

fn render_step(state: &StepState) -> &'static str {
    match state {
        StepState::Applied => "aplicado",
        StepState::Skipped => "omitido",
    }
}

fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn default_report_path() -> PathBuf {
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Desktop")
        .join("glory-remote-bootstrap.txt")
}

fn random_password() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

fn next_value(args: &mut impl Iterator<Item = String>, arg: &str) -> Result<String, BootstrapError> {
    args.next().ok_or_else(|| BootstrapError::Message(format!("Falta valor para {arg}")))
}

fn try_run_capture<S, I, A>(program: S, args: I) -> Result<String, BootstrapError>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = A>,
    A: AsRef<OsStr>,
{
    run_capture(program, args)
}

fn run_capture<S, I, A>(program: S, args: I) -> Result<String, BootstrapError>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = A>,
    A: AsRef<OsStr>,
{
    let output = Command::new(&program).args(args).output()?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let fallback = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(BootstrapError::Command {
        program: program.as_ref().to_string_lossy().to_string(),
        stderr: if stderr.is_empty() { fallback } else { stderr },
    })
}

#[cfg(test)]
mod tests {
    use super::{extract_tailscale_amd64_msi_url, edition_supports_rdp, ps_single_quote, rdp_firewall_script, Config};

    #[test]
    fn windows_home_no_soporta_rdp_host() {
        assert!(!edition_supports_rdp("Core"));
        assert!(!edition_supports_rdp("HomeSingleLanguage"));
    }

    #[test]
    fn windows_professional_y_server_si_soportan_rdp() {
        assert!(edition_supports_rdp("Professional"));
        assert!(edition_supports_rdp("ServerStandard"));
    }

    #[test]
    fn rdp_firewall_no_depende_del_nombre_visible_localizado() {
        let script = rdp_firewall_script();
        assert!(script.contains("RemoteDesktop-*"));
        assert!(!script.contains("DisplayGroup 'Remote Desktop'"));
    }

    #[test]
    fn parseo_cli_aplica_overrides_y_flags() {
        let config = Config::parse_from_iter(
            vec![
                "--tailscale-auth-key".to_string(),
                "tskey-demo".to_string(),
                "--rustdesk-password".to_string(),
                "ClaveSegura123".to_string(),
                "--device-name".to_string(),
                "restaurante-bdp".to_string(),
                "--advertise-tags".to_string(),
                "tag:restaurante".to_string(),
                "--report-path".to_string(),
                r"C:\temp\bootstrap.txt".to_string(),
                "--bdp-port".to_string(),
                "9000".to_string(),
                "--skip-rdp".to_string(),
                "--skip-rustdesk".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(config.tailscale_auth_key.as_deref(), Some("tskey-demo"));
        assert_eq!(config.rustdesk_password, "ClaveSegura123");
        assert_eq!(config.device_name.as_deref(), Some("restaurante-bdp"));
        assert_eq!(config.advertise_tags.as_deref(), Some("tag:restaurante"));
        assert_eq!(config.bdp_port, Some(9000));
        assert!(config.skip_rdp);
        assert!(config.skip_rustdesk);
        assert_eq!(config.report_path, std::path::PathBuf::from(r"C:\temp\bootstrap.txt"));
    }

    #[test]
    fn parseo_cli_rechaza_puerto_bdp_invalido() {
        let error = Config::parse_from_iter(
            vec!["--bdp-port".to_string(), "no-es-numero".to_string()].into_iter(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("--bdp-port"));
    }

    #[test]
    fn selector_tailscale_elige_msi_amd64() {
        let page = r#"
            <a href="https://pkgs.tailscale.com/stable/tailscale-setup-1.96.3.exe">exe</a>
            <a href="https://pkgs.tailscale.com/stable/tailscale-setup-1.96.3-x86.msi">x86</a>
            <a href="https://pkgs.tailscale.com/stable/tailscale-setup-1.96.3-amd64.msi">amd64</a>
        "#;

        let url = extract_tailscale_amd64_msi_url(page).unwrap();
        assert_eq!(
            url,
            "https://pkgs.tailscale.com/stable/tailscale-setup-1.96.3-amd64.msi"
        );
    }

    #[test]
    fn escape_powershell_single_quote_duplica_comillas() {
        assert_eq!(ps_single_quote("O'Brien"), "O''Brien");
    }
}