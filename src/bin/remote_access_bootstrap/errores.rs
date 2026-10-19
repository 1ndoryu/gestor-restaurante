/* Salida con fallo. Parte de remote_access_bootstrap (ver main.rs). */

use super::*;

pub(super) fn exit_with_failure(report_path: PathBuf, error: &BootstrapError) -> ! {
    let final_path = if write_failure_report(&report_path, error).is_ok() {
        report_path
    } else {
        let fallback = env::temp_dir().join("glory-remote-bootstrap-error.txt");
        let _ = write_failure_report(&fallback, error);
        fallback
    };

    show_message_box(
        "Glory Remote Bootstrap",
        &format!(
            "Bootstrap remoto falló. Envía este reporte a soporte: {}",
            final_path.display()
        ),
        "Error",
    );
    eprintln!("Bootstrap remoto falló: {error}");
    std::process::exit(1);
}

