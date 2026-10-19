// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [199A-1/F1] Límite de frecuencia del receptor de avisos.
 *
 * Va en su propio binario a propósito: el contador es global al proceso y esta
 * prueba lo agota a propósito. En el mismo binario que el resto de pruebas del
 * receptor, cualquiera que corriera después se llevaría un 429. */

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use glory_backend::config::AppConfig;
use glory_backend::handlers::recibir_aviso_con_secreto;
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::NotifHub;
use glory_backend::services::ServicioModoOperacion;
use glory_backend::AppState;
use serde_json::json;
use sqlx::PgPool;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

const SECRETO: &str = "secreto-de-prueba";

fn estado(pool: PgPool) -> AppState {
    let notif_hub = Arc::new(NotifHub::new());
    AppState {
        pool,
        jwt_secret: "test-secret".to_string(),
        config: AppConfig {
            database_url: "postgres://localhost".to_string(),
            jwt_secret: "test-secret".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3000,
            smtp: None,
            app_url: "http://localhost".to_string(),
            error_report_email: None,
        },
        notif_hub,
        modo_operacion: ServicioModoOperacion::default(),
        resumen_cache: Default::default(),
        listados_cache: Default::default(),
    }
}

async fn llamar(estado: &AppState) -> Result<StatusCode, StatusCode> {
    let cuerpo = json!({
        "IdentificadorComanda": "15",
        "TimeStamp": "2026-10-17T14:00:00Z",
        "Secreto": SECRETO
    });
    let mut cabeceras = HeaderMap::new();
    cabeceras.insert("x-bdp-secreto", SECRETO.parse().expect("cabecera"));

    recibir_aviso_con_secreto(
        State(estado.clone()),
        Path("crear".to_string()),
        Query(BTreeMap::new()),
        cabeceras,
        axum::body::Bytes::from(serde_json::to_vec(&cuerpo).expect("serializar")),
        Some(SECRETO.to_string()),
    )
    .await
    .map(|(codigo, _)| codigo)
    .map_err(|error| axum::response::IntoResponse::into_response(error).status())
}

/// Una ráfaga corta pasa; a partir del tope, el receptor corta con 429 sin
/// llegar siquiera a la base.
#[sqlx::test(migrations = "./migrations")]
async fn una_rafaga_larga_se_corta_con_429(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("limite-{user_id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(&pool)
        .await
        .expect("crear usuario");
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("config por defecto");

    let estado = estado(pool.clone());

    /* El TPV manda el mismo aviso una y otra vez: el primero entra, los demás
     * se reconocen como duplicados, y todos cuentan para el límite. */
    for numero in 1..=120 {
        let codigo = llamar(&estado).await.expect("dentro del límite");
        assert_ne!(codigo, StatusCode::TOO_MANY_REQUESTS, "petición {numero}");
    }

    let codigo = llamar(&estado)
        .await
        .expect_err("la petición 121 ya no pasa");
    assert_eq!(codigo, StatusCode::TOO_MANY_REQUESTS);

    /* Y el buzón no se ha llenado: sigue habiendo un solo aviso. */
    let filas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bdp_avisos_inbox WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("contar");
    assert_eq!(filas, 1);
}
