use super::{
    edition_supports_rdp, extract_tailscale_amd64_msi_url, ps_single_quote, rdp_firewall_script,
    Config,
};

#[test]
fn windows_home_no_soporta_rdp_host() {
    assert!(!edition_supports_rdp("Core"));
    assert!(!edition_supports_rdp("HomeSingleLanguage"));
}

#[test]
fn windows_professional_y_server_si_soportan_rdp() {
    assert!(edition_supports_rdp("Professional"));
    assert!(edition_supports_rdp("ServerStandard"));
}

#[test]
fn rdp_firewall_no_depende_del_nombre_visible_localizado() {
    let script = rdp_firewall_script();
    assert!(script.contains("RemoteDesktop-*"));
    assert!(!script.contains("DisplayGroup 'Remote Desktop'"));
}

#[test]
fn parseo_cli_aplica_overrides_y_flags() {
    let config = Config::parse_from_iter(vec![
        "--tailscale-auth-key".to_string(),
        "tskey-demo".to_string(),
        "--rustdesk-password".to_string(),
        "ClaveSegura123".to_string(),
        "--device-name".to_string(),
        "restaurante-bdp".to_string(),
        "--advertise-tags".to_string(),
        "tag:restaurante".to_string(),
        "--report-path".to_string(),
        r"C:\temp\bootstrap.txt".to_string(),
        "--bdp-port".to_string(),
        "9000".to_string(),
        "--skip-rdp".to_string(),
        "--skip-rustdesk".to_string(),
    ])
    .unwrap();

    assert_eq!(config.tailscale_auth_key.as_deref(), Some("tskey-demo"));
    assert_eq!(config.rustdesk_password, "ClaveSegura123");
    assert_eq!(config.device_name.as_deref(), Some("restaurante-bdp"));
    assert_eq!(config.advertise_tags.as_deref(), Some("tag:restaurante"));
    assert_eq!(config.bdp_port, Some(9000));
    assert!(config.skip_rdp);
    assert!(config.skip_rustdesk);
    assert_eq!(
        config.report_path,
        std::path::PathBuf::from(r"C:\temp\bootstrap.txt")
    );
}

#[test]
fn parseo_cli_rechaza_puerto_bdp_invalido() {
    let error = Config::parse_from_iter(vec!["--bdp-port".to_string(), "no-es-numero".to_string()])
        .unwrap_err();

    assert!(error.to_string().contains("--bdp-port"));
}

#[test]
fn selector_tailscale_elige_msi_amd64() {
    let page = r#"
        <a href="https://pkgs.tailscale.com/stable/tailscale-setup-1.96.3.exe">exe</a>
        <a href="https://pkgs.tailscale.com/stable/tailscale-setup-1.96.3-x86.msi">x86</a>
        <a href="https://pkgs.tailscale.com/stable/tailscale-setup-1.96.3-amd64.msi">amd64</a>
    "#;

    let url = extract_tailscale_amd64_msi_url(page).unwrap();
    assert_eq!(
        url,
        "https://pkgs.tailscale.com/stable/tailscale-setup-1.96.3-amd64.msi"
    );
}

#[test]
fn escape_powershell_single_quote_duplica_comillas() {
    assert_eq!(ps_single_quote("O'Brien"), "O''Brien");
}
