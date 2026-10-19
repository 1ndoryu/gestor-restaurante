/* Diálogos de Windows: entrada y mensajes. Parte de remote_access_bootstrap (ver main.rs). */

use super::super::proceso::{ps_single_quote, run_powershell};
use super::super::BootstrapError;

pub(crate) fn show_input_box(
    title: &str,
    prompt: &str,
    default_value: &str,
) -> Result<Option<String>, BootstrapError> {
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

pub(crate) fn show_message_box(title: &str, message: &str, icon: &str) {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}','{}',[System.Windows.Forms.MessageBoxButtons]::OK,[System.Windows.Forms.MessageBoxIcon]::{}) | Out-Null",
        ps_single_quote(message),
        ps_single_quote(title),
        icon,
    );
    let _ = run_powershell(&script);
}
