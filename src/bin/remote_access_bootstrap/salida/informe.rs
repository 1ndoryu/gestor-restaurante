/* Informe final y del fallo. Parte de remote_access_bootstrap (ver main.rs). */

use super::super::*;

pub(crate) fn build_report(
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
    let _ = writeln!(
        report,
        "Tailscale auth: {}",
        if tailscale.authenticated {
            "ok"
        } else {
            "pendiente"
        }
    );
    let _ = writeln!(
        report,
        "Tailscale IP: {}",
        tailscale.ip.as_deref().unwrap_or("pendiente")
    );
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
        let _ = writeln!(
            report,
            "RustDesk ID: {}",
            rustdesk.id.as_deref().unwrap_or("pendiente")
        );
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

pub(crate) fn write_failure_report(report_path: &Path, error: &BootstrapError) -> Result<(), BootstrapError> {
    let mut report = String::new();
    let _ = writeln!(report, "Glory Remote Bootstrap - ERROR");
    let _ = writeln!(report, "Cuando: {}", chrono::Utc::now().to_rfc3339());
    let _ = writeln!(report, "Error: {error}");
    let _ = writeln!(
        report,
        "Sugerencia: reenviar este archivo completo a soporte."
    );
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, report)?;
    Ok(())
}

fn render_step(state: &StepState) -> &'static str {
    match state {
        StepState::Applied => "aplicado",
        StepState::Skipped => "omitido",
    }
}

pub(crate) fn default_report_path() -> PathBuf {
    env::var_os("USERPROFILE")
        .map_or_else(
            || env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            PathBuf::from,
        )
        .join("Desktop")
        .join("glory-remote-bootstrap.txt")
}

