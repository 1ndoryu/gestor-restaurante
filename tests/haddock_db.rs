/* [064A-13] Tests de integración con base de datos real (PostgreSQL).
 * Usa #[sqlx::test] — crea BD temporal por test, aplica migraciones, destruye al final.
 * Requiere DATABASE_URL apuntando a un servidor PostgreSQL accesible.
 * Estos tests validan que las operaciones de repositorio funcionan contra SQL real,
 * no solo contra el cache offline de SQLx. */

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use glory_backend::models::Venta;
use glory_backend::repositories::venta::{NuevaVenta, VentaRepository};
use glory_backend::repositories::ConfiguracionRepository;

/* Helper: crea un usuario mínimo en la BD para satisfacer FK de ventas.
 * Usa query() runtime (sin macro) porque este SQL no está en el cache offline. */
async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("test-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("create_test_user: fallo al crear usuario de prueba");
    id
}

/* Helper: construye NuevaVenta con datos válidos para los CHECK constraints de la tabla. */
fn nueva_venta(user_id: Uuid) -> NuevaVenta<'static> {
    NuevaVenta {
        user_id,
        fecha: NaiveDate::from_ymd_opt(2026, 4, 6).expect("fecha válida"),
        comensales: Some(4),
        descripcion: "Menu del dia - test",
        iva_porcentaje: Decimal::from(10),
        turno: "mediodia",
        canal: "comedor",
        metodo_pago: "tarjeta",
        importe_base: Decimal::from_str("45.00").expect("decimal válido"),
        importe_iva: Decimal::from_str("4.50").expect("decimal válido"),
        reserva_id: None,
        cliente_id: None,
    }
}

/* Helper: lee una venta de BD y la retorna (panic si no existe). */
async fn find_venta(pool: &PgPool, venta: &Venta) -> Venta {
    VentaRepository::find_by_id(pool, venta.id, venta.user_id)
        .await
        .expect("find_venta: error de BD")
        .expect("find_venta: venta no encontrada")
}

/* ── Tests de VentaRepository ───────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_venta_create_and_find_roundtrip(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let data = nueva_venta(user_id);

    let created = VentaRepository::create(&pool, &data)
        .await
        .expect("create debe funcionar");

    assert_eq!(created.user_id, user_id);
    assert_eq!(created.fecha, NaiveDate::from_ymd_opt(2026, 4, 6).unwrap());
    assert_eq!(created.turno, "mediodia");
    assert_eq!(created.canal, "comedor");
    assert_eq!(created.metodo_pago, "tarjeta");
    assert_eq!(created.importe_base, Decimal::from_str("45.00").unwrap());
    assert_eq!(created.importe_iva, Decimal::from_str("4.50").unwrap());
    assert!(!created.haddock_synced, "Nueva venta no debe estar synced");
    assert!(created.haddock_synced_at.is_none());
    assert!(created.haddock_sync_error.is_none());

    let found = find_venta(&pool, &created).await;
    assert_eq!(found.id, created.id);
    assert_eq!(found.descripcion, "Menu del dia - test");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_update_haddock_status_marca_synced(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta = VentaRepository::create(&pool, &nueva_venta(user_id))
        .await
        .unwrap();

    VentaRepository::update_haddock_status(&pool, venta.id, true, None)
        .await
        .expect("update_haddock_status debe funcionar");

    let updated = find_venta(&pool, &venta).await;
    assert!(updated.haddock_synced, "Debe quedar synced=true");
    assert!(
        updated.haddock_synced_at.is_some(),
        "Debe tener timestamp de sync"
    );
    assert!(updated.haddock_sync_error.is_none(), "Sin error");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_update_haddock_status_registra_error(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta = VentaRepository::create(&pool, &nueva_venta(user_id))
        .await
        .unwrap();

    VentaRepository::update_haddock_status(
        &pool,
        venta.id,
        false,
        Some("Haddock respondió HTTP 500"),
    )
    .await
    .expect("update_haddock_status debe funcionar");

    let updated = find_venta(&pool, &venta).await;
    assert!(!updated.haddock_synced, "Debe quedar synced=false");
    assert!(
        updated.haddock_synced_at.is_none(),
        "Nunca sincronizada, sin timestamp"
    );
    assert_eq!(
        updated.haddock_sync_error.as_deref(),
        Some("Haddock respondió HTTP 500")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_update_haddock_status_error_luego_exito(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta = VentaRepository::create(&pool, &nueva_venta(user_id))
        .await
        .unwrap();

    /* Primer intento falla */
    VentaRepository::update_haddock_status(&pool, venta.id, false, Some("timeout"))
        .await
        .unwrap();

    let after_error = find_venta(&pool, &venta).await;
    assert!(!after_error.haddock_synced);
    assert_eq!(after_error.haddock_sync_error.as_deref(), Some("timeout"));

    /* Retry exitoso */
    VentaRepository::update_haddock_status(&pool, venta.id, true, None)
        .await
        .unwrap();

    let after_success = find_venta(&pool, &venta).await;
    assert!(after_success.haddock_synced, "Retry exitoso → synced=true");
    assert!(
        after_success.haddock_synced_at.is_some(),
        "Debe registrar timestamp de sync"
    );
    assert!(
        after_success.haddock_sync_error.is_none(),
        "Error limpiado tras éxito"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_delete_venta(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let venta = VentaRepository::create(&pool, &nueva_venta(user_id))
        .await
        .unwrap();

    let deleted = VentaRepository::delete(&pool, venta.id, user_id)
        .await
        .expect("delete debe funcionar");
    assert!(deleted, "Debe reportar que se eliminó");

    let not_found = VentaRepository::find_by_id(&pool, venta.id, user_id)
        .await
        .expect("find_by_id no debe fallar");
    assert!(not_found.is_none(), "Venta eliminada no debe existir");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_delete_venta_wrong_user_returns_false(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let other_user = create_test_user(&pool).await;
    let venta = VentaRepository::create(&pool, &nueva_venta(user_id))
        .await
        .unwrap();

    /* Otro usuario no puede borrar la venta */
    let deleted = VentaRepository::delete(&pool, venta.id, other_user)
        .await
        .expect("delete no debe fallar");
    assert!(!deleted, "No debe borrar venta de otro usuario");

    /* Venta original sigue existiendo */
    let still_there = VentaRepository::find_by_id(&pool, venta.id, user_id)
        .await
        .unwrap();
    assert!(still_there.is_some(), "Venta debe seguir existiendo");
}

/* ── Tests de ConfiguracionRepository ───────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_configuracion_defaults_haddock_desactivado(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let config = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("obtener_o_crear debe funcionar");

    assert_eq!(config.user_id, user_id);
    assert!(
        !config.haddock_sync_enabled,
        "Haddock sync desactivado por defecto"
    );
    assert!(
        config.haddock_api_token.is_empty(),
        "Token vacío por defecto"
    );
    assert!(config.url_haddock.is_empty(), "URL vacía por defecto");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_configuracion_obtener_idempotente(pool: PgPool) {
    let user_id = create_test_user(&pool).await;

    let primera = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .unwrap();
    let segunda = ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .unwrap();

    assert_eq!(
        primera.id, segunda.id,
        "Dos llamadas retornan la misma configuración"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn test_configuracion_actualizar_haddock_fields(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .unwrap();

    /* Activar Haddock con token y URL */
    let req = glory_backend::models::ActualizarConfiguracionRequest {
        haddock_sync_enabled: Some(true),
        haddock_api_token: Some("dG9rZW46c2VjcmV0".to_string()),
        url_haddock: Some("https://pos-api.haddock.app".to_string()),
        bdp_base_url: None,
        bdp_login: None,
        bdp_password: None,
        bdp_integrator_code: None,
        bdp_sync_enabled: None,
        bdp_pos_id: None,
        bdp_employee_id: None,
        bdp_items_profile_id: None,
        bdp_catalog_price_type: None,
        bdp_purchase_notes_profile_id: None,
        google_review_url: None,
        telefono_restaurante: None,
        url_reservas: None,
        reserva_email_obligatorio: None,
        reserva_telefono_obligatorio: None,
        reserva_nombre_obligatorio: None,
        reserva_apellidos_obligatorio: None,
        iva_por_defecto: None,
        nombre_restaurante: None,
        groq_api_key: None,
        auto_venta_reserva: None,
        hora_desayuno_inicio: None,
        hora_desayuno_fin: None,
        hora_comida_inicio: None,
        hora_comida_fin: None,
        hora_cena_inicio: None,
        hora_cena_fin: None,
        bdp_tender_map: None,
        bdp_order_type_map: None,
        bdp_default_customer_code: None,
        bdp_default_article_code: None,
        bdp_default_article_name: None,
        bdp_poll_interval_secs: None,
        bdp_poll_enabled: None,
        bdp_auto_sync_customers: None,
        bdp_sync_mode: None,
        bdp_backup_retention_days: None,
        bdp_auto_backup_before_write: None,
        /* [XT2-1] Feature flags BDP */
        ff_bdp_auto_arm: None,
        ff_bdp_partial_payments: None,
        ff_bdp_cancel_order: None,
        ff_bdp_purchase_notes_read: None,
        ff_bdp_purchase_notes_draft: None,
        ff_bdp_purchase_notes_receive: None,
        modo_operacion: None,
        anulacion_modalidad: None,
        permisos_catalogo_edicion: None,
        permisos_stock_ajuste: None,
        permisos_albaranes_gestion: None,
        permisos_anulacion_ventas: None,
    };

    let updated = ConfiguracionRepository::actualizar(&pool, user_id, &req)
        .await
        .expect("actualizar debe funcionar");

    assert!(updated.haddock_sync_enabled);
    assert_eq!(updated.haddock_api_token, "dG9rZW46c2VjcmV0");
    assert_eq!(updated.url_haddock, "https://pos-api.haddock.app");
}

/* ── Tests de FK constraints ────────────────────────────────── */

#[sqlx::test(migrations = "./migrations")]
async fn test_venta_sin_usuario_falla_por_fk(pool: PgPool) {
    let fake_user = Uuid::new_v4();
    let result = VentaRepository::create(&pool, &nueva_venta(fake_user)).await;
    assert!(result.is_err(), "FK violation debe retornar error");
}
