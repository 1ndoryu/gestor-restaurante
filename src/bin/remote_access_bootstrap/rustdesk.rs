/* Instalación y configuración de RustDesk. Parte de remote_access_bootstrap (ver main.rs). */

use super::*;

pub(super) async fn ensure_rustdesk_installed() -> Result<PathBuf, BootstrapError> {
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
                    asset["browser_download_url"]
                        .as_str()
                        .map(ToOwned::to_owned)
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| {
            BootstrapError::Message("No encontré el instalador de RustDesk x86_64".to_string())
        })?;

    let installer = env::temp_dir().join("rustdesk-latest-x86_64.exe");
    download_to_file(&download_url, &installer).await?;
    run_capture(&installer, ["--silent-install"])?;
    wait_for_path(Path::new(RUSTDESK_EXE), 90)?;
    Ok(PathBuf::from(RUSTDESK_EXE))
}

pub(super) fn configure_rustdesk(exe: &Path, config: &Config) -> Result<RustDeskState, BootstrapError> {
    let _ = try_run_capture(exe, ["--install-service"]);
    let _ = try_run_capture("sc", ["start", RUSTDESK_SERVICE]);
    wait_for_service(RUSTDESK_SERVICE, 30)?;

    if let Some(rustdesk_config) = &config.rustdesk_config {
        run_capture(exe, ["--config", rustdesk_config.as_str()])?;
    }
    run_capture(exe, ["--password", config.rustdesk_password.as_str()])?;
    let id = try_run_capture(exe, ["--get-id"]).ok().and_then(|output| {
        output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToOwned::to_owned)
    });

    Ok(RustDeskState {
        id,
        password: config.rustdesk_password.clone(),
    })
}

pub(super) fn random_password() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

