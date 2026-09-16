// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [149A-3/F4-B] Tests del guard de edición de catálogo usado por
 * import-catalog / sync-catalog / sync-prices (y del patrón require_role
 * Admin usado por sync-tables). Sin BDP real: solo el guard de permiso. */

use glory_backend::errors::AppError;
use glory_backend::middleware::AuthUser;
use glory_backend::models::UserRole;
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{verificar_permiso, AccionPermiso};
use sqlx::PgPool;
use uuid::Uuid;

async fn create_test_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("perm-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    id
}

fn auth_con_rol(user_id: Uuid, role: UserRole) -> AuthUser {
    AuthUser {
        user_id,
        role,
        effective_role: role,
        impersonator: None,
        trabajador_id: None,
    }
}

fn es_forbidden(err: &AppError) -> bool {
    matches!(err, AppError::Forbidden(_))
}

/* Default fail-closed 'admin': el dueño pasa, el trabajador no. */
#[sqlx::test(migrations = "./migrations")]
async fn catalogo_edicion_por_defecto_solo_admin(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");

    let admin = auth_con_rol(user_id, UserRole::Admin);
    verificar_permiso(&pool, AccionPermiso::CatalogoEdicion, &admin)
        .await
        .expect("el dueño pasa con el default 'admin'");

    let worker = auth_con_rol(user_id, UserRole::Trabajador);
    let err = verificar_permiso(&pool, AccionPermiso::CatalogoEdicion, &worker)
        .await
        .expect_err("el trabajador no pasa con el default 'admin'");
    assert!(es_forbidden(&err), "debe ser 403, fue: {err:?}");
}

/* El dueño puede delegar: 'admin_trabajador' deja pasar al trabajador. */
#[sqlx::test(migrations = "./migrations")]
async fn catalogo_edicion_delegable_a_trabajador(pool: PgPool) {
    let user_id = create_test_user(&pool).await;
    ConfiguracionRepository::obtener_o_crear(&pool, user_id)
        .await
        .expect("crear configuración por defecto");
    sqlx::query(
        "UPDATE configuracion_restaurante \
         SET permisos_catalogo_edicion = 'admin_trabajador' WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("delegar edición de catálogo");

    let worker = auth_con_rol(user_id, UserRole::Trabajador);
    verificar_permiso(&pool, AccionPermiso::CatalogoEdicion, &worker)
        .await
        .expect("el trabajador pasa cuando el dueño delega");
}

/* Patrón de sync-tables: require_role Admin puro. */
#[sqlx::test(migrations = "./migrations")]
async fn sync_tables_solo_admin(_pool: PgPool) {
    let user_id = Uuid::new_v4();
    let admin = auth_con_rol(user_id, UserRole::Admin);
    admin
        .require_role(&[UserRole::Admin])
        .expect("el dueño pasa require_role Admin");
    let worker = auth_con_rol(user_id, UserRole::Trabajador);
    let err = worker
        .require_role(&[UserRole::Admin])
        .expect_err("el trabajador no pasa require_role Admin");
    assert!(es_forbidden(&err), "debe ser 403, fue: {err:?}");
}
