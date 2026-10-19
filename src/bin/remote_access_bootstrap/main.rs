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

mod companion;
mod config;
mod errores;
mod proceso;
mod red;
mod rustdesk;
mod salida;
mod tailscale;
#[cfg(test)]
mod tests;
mod windows;

use companion::*;
use errores::*;
use proceso::*;
use red::*;
use rustdesk::*;
use salida::*;
use tailscale::*;
use windows::*;

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
        exit_with_failure(
            default_report_path(),
            &BootstrapError::Message("Este bootstrap solo funciona en Windows".to_string()),
        );
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
#[allow(clippy::struct_field_names)]
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

async fn run(config: Config) -> Result<(), BootstrapError> {
    let edition = windows_edition()?;
    let tailscale = ensure_tailscale_installed().await?;
    let tailscale_state = configure_tailscale(&tailscale, &config)?;
    let power_state = if config.skip_power {
        StepState::Skipped
    } else {
        disable_sleep()?
    };
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
    println!(
        "Tailscale IP: {}",
        tailscale_state.ip.as_deref().unwrap_or("pendiente")
    );
    if let Some(rustdesk) = rustdesk_state {
        println!(
            "RustDesk ID: {}",
            rustdesk.id.as_deref().unwrap_or("pendiente")
        );
        /* La contraseña solo va al reporte (ya escrito en disco), nunca a stdout:
         * stdout acaba en logs y agregadores. */
        println!("RustDesk: contraseña disponible en el reporte");
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
