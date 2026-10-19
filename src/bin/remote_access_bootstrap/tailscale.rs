/* Instalación y configuración de Tailscale. Parte de remote_access_bootstrap (ver main.rs). */

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::proceso::{command_exists, run_capture, try_run_capture, wait_for_path};
use super::red::download_to_file;
use super::{
    BootstrapError, Config, TailscaleState, TAILSCALE_EXE, TAILSCALE_PACKAGES_URL,
    TAILSCALE_WINGET_ID,
};

pub(super) async fn ensure_tailscale_installed() -> Result<PathBuf, BootstrapError> {
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
        if try_run_capture("winget", winget_args).is_ok()
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

pub(super) fn extract_tailscale_amd64_msi_url(page: &str) -> Result<String, BootstrapError> {
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

pub(super) fn configure_tailscale(
    exe: &Path,
    config: &Config,
) -> Result<TailscaleState, BootstrapError> {
    if let Some(auth_key) = &config.tailscale_auth_key {
        let mut args = vec![
            "up".to_string(),
            format!("--auth-key={auth_key}"),
            "--accept-dns=true".to_string(),
        ];
        if let Some(name) = &config.device_name {
            args.push(format!("--hostname={name}"));
        }
        if let Some(tags) = &config.advertise_tags {
            args.push(format!("--advertise-tags={tags}"));
        }
        run_capture(exe, args.iter().map(String::as_str))?;
    }

    let ip = try_run_capture(exe, ["ip", "-4"]).ok().and_then(|output| {
        output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToOwned::to_owned)
    });

    Ok(TailscaleState {
        authenticated: ip.is_some(),
        ip,
    })
}
