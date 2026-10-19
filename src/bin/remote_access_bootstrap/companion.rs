/* Archivo de configuración companion. Parte de remote_access_bootstrap (ver main.rs). */

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::{BootstrapError, CompanionConfig, COMPANION_CONFIG_FILE};

pub(super) fn load_companion_config() -> Result<(CompanionConfig, bool), BootstrapError> {
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
