/* [287A-5] Verificación explícita de la migración de perfiles de lectura.
 * Está ignorada porque modifica únicamente el esquema PostgreSQL local. */

use sqlx::PgPool;

#[tokio::test]
#[ignore = "aplica migraciones en DATABASE_URL; ejecutar solo contra PostgreSQL local"]
async fn applies_bdp_read_profiles_migration_on_loopback_database() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL es obligatorio");
    let parsed = reqwest::Url::parse(&database_url).expect("DATABASE_URL inválida");
    assert!(
        matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "::1")),
        "la prueba solo admite PostgreSQL local"
    );

    let pool = PgPool::connect(&database_url)
        .await
        .expect("no se pudo conectar a PostgreSQL local");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("las migraciones deben aplicarse sin errores");

    let columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.columns \
         WHERE table_name = 'configuracion_restaurante' \
         AND column_name IN ('bdp_catalog_price_type', 'bdp_purchase_notes_profile_id')",
    )
    .fetch_one(&pool)
    .await
    .expect("no se pudieron comprobar las columnas");
    assert_eq!(columns, 2);
}
