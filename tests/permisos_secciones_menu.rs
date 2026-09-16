// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [169A-3/M4] Tests del guard por sección de menú (`verificar_seccion`):
 * el dueño pasa siempre, el trabajador solo con la sección concedida,
 * sección desconocida → 403 (fail-closed). Sin BDP real: solo el guard. */

use glory_backend::errors::AppError;
use glory_backend::middleware::AuthUser;
use glory_backend::models::UserRole;
use glory_backend::repositories::TrabajadorRepository;
use glory_backend::services::verificar_seccion;
use sqlx::PgPool;
use uuid::Uuid;

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("sec-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    id
}

async fn create_test_worker(pool: &PgPool, user_id: Uuid) -> Uuid {
    TrabajadorRepository::create(
        pool,
        user_id,
        "Camarero Test",
        &format!("camarero-{user_id}@example.com"),
        "argon2_hash_placeholder",
        "Camarera",
    )
    .await
    .expect("crear trabajador de prueba")
    .id
}

fn auth_dueno(user_id: Uuid) -> AuthUser {
    AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    }
}

fn auth_trabajador(user_id: Uuid, trabajador_id: Uuid) -> AuthUser {
    AuthUser {
        user_id,
        role: UserRole::Trabajador,
        effective_role: UserRole::Trabajador,
        impersonator: None,
        trabajador_id: Some(trabajador_id),
    }
}

fn es_forbidden(err: &AppError) -> bool {
    matches!(err, AppError::Forbidden(_))
}

/* El dueño pasa cualquier sección válida, tenga o no filas de permisos. */
#[sqlx::test(migrations = "./migrations")]
async fn seccion_dueno_pasa_siempre(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let dueno = auth_dueno(user_id);
    for seccion in ["campanas", "configuracion", "ventas", "notificaciones"] {
        verificar_seccion(&pool, &dueno, seccion)
            .await
            .unwrap_or_else(|_| panic!("el dueño debe pasar '{seccion}'"));
    }
}

/* Sección desconocida → 403 incluso para el dueño (fail-closed). */
#[sqlx::test(migrations = "./migrations")]
async fn seccion_desconocida_403(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let dueno = auth_dueno(user_id);
    let err = verificar_seccion(&pool, &dueno, "marketing")
        .await
        .expect_err("la clave gruesa histórica ya no es válida");
    assert!(es_forbidden(&err), "debe ser 403, fue: {err:?}");
}

/* Trabajador sin filas de permisos → 403 en todo. */
#[sqlx::test(migrations = "./migrations")]
async fn seccion_trabajador_sin_permisos_403(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let tid = create_test_worker(&pool, user_id).await;
    let worker = auth_trabajador(user_id, tid);
    let err = verificar_seccion(&pool, &worker, "ventas")
        .await
        .expect_err("sin filas no hay acceso");
    assert!(es_forbidden(&err), "debe ser 403, fue: {err:?}");
}

/* Trabajador con "campanas" pasa campanas pero no plantillas_wa. */
#[sqlx::test(migrations = "./migrations")]
async fn seccion_trabajador_solo_concedida(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    let tid = create_test_worker(&pool, user_id).await;
    TrabajadorRepository::set_permisos(&pool, tid, &["campanas".to_string()])
        .await
        .expect("conceder campanas");
    let worker = auth_trabajador(user_id, tid);

    verificar_seccion(&pool, &worker, "campanas")
        .await
        .expect("la sección concedida pasa");

    let err = verificar_seccion(&pool, &worker, "plantillas_wa")
        .await
        .expect_err("la no concedida no pasa");
    assert!(es_forbidden(&err), "debe ser 403, fue: {err:?}");
}
