/* Configuración: entorno, archivo companion y argumentos. Parte de remote_access_bootstrap (ver main.rs). */

use super::*;

impl Config {
    pub(super) fn parse() -> Result<Self, BootstrapError> {
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
    pub(super) fn parse_from_iter<I>(args: I) -> Result<Self, BootstrapError>
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
                .map_or_else(default_report_path, PathBuf::from),
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
                "--tailscale-auth-key" => {
                    self.tailscale_auth_key = Some(next_value(&mut args, &arg)?);
                }
                "--rustdesk-password" => self.rustdesk_password = next_value(&mut args, &arg)?,
                "--rustdesk-config" => self.rustdesk_config = Some(next_value(&mut args, &arg)?),
                "--device-name" => self.device_name = Some(next_value(&mut args, &arg)?),
                "--advertise-tags" => self.advertise_tags = Some(next_value(&mut args, &arg)?),
                "--report-path" => self.report_path = PathBuf::from(next_value(&mut args, &arg)?),
                "--bdp-port" => {
                    self.bdp_port = Some(next_value(&mut args, &arg)?.parse().map_err(|_| {
                        BootstrapError::Message("--bdp-port debe ser un entero valido".to_string())
                    })?);
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
            &self
                .bdp_port
                .map_or_else(String::new, |value| value.to_string()),
        )? {
            self.bdp_port = Some(port.parse().map_err(|_| {
                BootstrapError::Message("El puerto BDP debe ser un entero válido".to_string())
            })?);
        }

        Ok(())
    }
}

fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn next_value(
    args: &mut impl Iterator<Item = String>,
    arg: &str,
) -> Result<String, BootstrapError> {
    args.next()
        .ok_or_else(|| BootstrapError::Message(format!("Falta valor para {arg}")))
}
