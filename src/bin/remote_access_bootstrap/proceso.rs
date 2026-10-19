/* Procesos, PowerShell y esperas. Parte de remote_access_bootstrap (ver main.rs). */

use super::*;

pub(super) fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

pub(super) fn run_powershell(script: &str) -> Result<String, BootstrapError> {
    run_capture(
        "powershell",
        [
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ],
    )
}

pub(super) fn command_exists(command: &str) -> bool {
    Command::new("where")
        .arg(command)
        .output()
        .is_ok_and(|output| output.status.success())
}

pub(super) fn wait_for_path(path: &Path, timeout_seconds: u64) -> Result<(), BootstrapError> {
    for _ in 0..timeout_seconds {
        if path.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(1));
    }
    Err(BootstrapError::Message(format!(
        "No apareció {}",
        path.display()
    )))
}

pub(super) fn wait_for_service(
    service_name: &str,
    timeout_seconds: u64,
) -> Result<(), BootstrapError> {
    for _ in 0..timeout_seconds {
        let status = try_run_capture(
            "powershell",
            [
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &format!(
                    "(Get-Service -Name '{service_name}' -ErrorAction SilentlyContinue).Status"
                ),
            ],
        )
        .unwrap_or_default();
        if status.trim().eq_ignore_ascii_case("Running") {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(1));
    }
    Err(BootstrapError::Message(format!(
        "El servicio {service_name} no arrancó"
    )))
}

pub(super) fn try_run_capture<S, I, A>(program: S, args: I) -> Result<String, BootstrapError>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = A>,
    A: AsRef<OsStr>,
{
    run_capture(program, args)
}

pub(super) fn run_capture<S, I, A>(program: S, args: I) -> Result<String, BootstrapError>
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
