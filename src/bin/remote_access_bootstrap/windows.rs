/* Pasos de Windows: suspensión, RDP, admin y edición. Parte de remote_access_bootstrap (ver main.rs). */

use super::*;

pub(super) fn disable_sleep() -> Result<StepState, BootstrapError> {
    run_capture("powercfg", ["/change", "standby-timeout-ac", "0"])?;
    run_capture("powercfg", ["/change", "standby-timeout-dc", "0"])?;
    run_capture("powercfg", ["/hibernate", "off"])?;
    Ok(StepState::Applied)
}

pub(super) fn enable_rdp(edition: &str) -> Result<RdpState, BootstrapError> {
    if !edition_supports_rdp(edition) {
        return Ok(RdpState::Unsupported(edition.to_string()));
    }
    run_powershell(rdp_firewall_script())?;
    Ok(RdpState::Enabled)
}

pub(super) fn rdp_firewall_script() -> &'static str {
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

pub(super) fn ensure_admin_or_relaunch() -> Result<bool, BootstrapError> {
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
        let _ = write!(script, " -ArgumentList @({})", args.join(", "));
    }
    run_powershell(&script)?;
    Ok(())
}

fn is_admin() -> Result<bool, BootstrapError> {
    Ok(run_powershell("[bool](([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))")?
        .trim()
        .eq_ignore_ascii_case("true"))
}

pub(super) fn windows_edition() -> Result<String, BootstrapError> {
    run_powershell(
        r"(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').EditionID",
    )
}

pub(super) fn edition_supports_rdp(edition: &str) -> bool {
    let normalized = edition.to_ascii_lowercase();
    !normalized.contains("core") && !normalized.contains("home")
}
