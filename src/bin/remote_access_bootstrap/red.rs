/* Descargas y puertos de red. Parte de remote_access_bootstrap (ver main.rs). */

use super::*;

pub(super) fn allow_bdp_port(port: u16) -> Result<StepState, BootstrapError> {
    let name = format!("Glory BDP WebLink {port}");
    let script = format!(
        "$rule = Get-NetFirewallRule -DisplayName '{name}' -ErrorAction SilentlyContinue; if (-not $rule) {{ New-NetFirewallRule -DisplayName '{name}' -Direction Inbound -Action Allow -Protocol TCP -LocalPort {port} -RemoteAddress 100.64.0.0/10 | Out-Null }}"
    );
    run_powershell(&script)?;
    Ok(StepState::Applied)
}

pub(super) async fn download_to_file(url: &str, path: &Path) -> Result<(), BootstrapError> {
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

