// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [159A-2/F1] Tests del import de departamentos BDP → clasificaciones locales:
 *   - aplanar cubre árbol con subdepartamentos (unit, sin BDP ni BD);
 *   - importar crea nuevos, vincula existentes por código sin pisar el nombre
 *     local, reporta conflictos de nombre y omite códigos fuera de 1..999.
 * Ningún test toca red BDP: el HTTP vive en el handler, aquí solo parse+BD. */

use glory_backend::models::TIPO_DEPARTAMENTO;
use glory_backend::repositories::BdpCatalogoClasificacionRepository;
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::{aplanar_departamentos, BdpImportDepartamentosService};
use sqlx::PgPool;
use uuid::Uuid;

async fn crear_usuario(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let email = format!("test-import-{id}@example.com");
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&email)
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario de prueba");
    ConfiguracionRepository::obtener_o_crear(pool, id)
        .await
        .expect("config por defecto");
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn importar_departamentos_crea_vincula_y_reporta(pool: PgPool) {
    let user_id = crear_usuario(&pool).await;

    /* Fila local previa: mismo código 8 con nombre propio (no se pisa). */
    BdpCatalogoClasificacionRepository::crear_con_code(
        &pool,
        user_id,
        TIPO_DEPARTAMENTO,
        8,
        "GINEBRAS MÍAS",
    )
    .await
    .expect("fila local previa");
    /* Fila local con mismo nombre pero otro código (conflicto). */
    BdpCatalogoClasificacionRepository::crear_con_code(
        &pool,
        user_id,
        TIPO_DEPARTAMENTO,
        90,
        "RON",
    )
    .await
    .expect("fila local conflicto");

    let valor = serde_json::json!({
        "Departamentos": [
            {"Codigo": 1.0, "Descripcion": "CAFES", "SubDepartamentos": []},
            {"Codigo": 8.0, "Descripcion": "GINEBRAS", "SubDepartamentos": []},
            {"Codigo": 10.0, "Descripcion": "RON", "SubDepartamentos": []},
            {"Codigo": 0.0, "Descripcion": "CERO", "SubDepartamentos": []},
            {"Codigo": 1000.0, "Descripcion": "MIL", "SubDepartamentos": []}
        ]
    });
    let items = aplanar_departamentos(&valor);
    assert_eq!(items.len(), 5);

    let r = BdpImportDepartamentosService::importar(&pool, user_id, &items, TIPO_DEPARTAMENTO)
        .await
        .expect("importar");
    assert_eq!(r.nuevos, 1, "solo CAFES es nuevo: {r:?}");
    assert_eq!(r.vinculados, 1, "código 8 ya existía: {r:?}");
    assert_eq!(r.conflictos_nombre, 1, "RON choca por nombre: {r:?}");
    assert_eq!(r.omitidos_fuera_rango, 2, "0 y 1000 fuera de 1..999: {r:?}");
    assert_eq!(r.errores, 0, "{r:?}");

    /* El nombre local del vinculado manda. */
    let vinculada =
        BdpCatalogoClasificacionRepository::buscar_por_code(&pool, user_id, TIPO_DEPARTAMENTO, 8)
            .await
            .expect("buscar vinculada")
            .expect("existe");
    assert_eq!(vinculada.nombre, "GINEBRAS MÍAS");

    /* Segunda pasada: todo vinculado/sin cambios nuevos. */
    let r2 = BdpImportDepartamentosService::importar(&pool, user_id, &items, TIPO_DEPARTAMENTO)
        .await
        .expect("reimportar");
    assert_eq!(r2.nuevos, 0, "{r2:?}");
    assert_eq!(r2.vinculados, 2, "{r2:?}");
}
