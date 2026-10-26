// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [26AA-3] Permisos del modo automático por categoría. Cada categoría agrupa
 * alcances de escritura; sin fila en `bdp_permiso_automatico` está apagada.
 * El interruptor general (`bdp_sync_mode = automatic`) sigue siendo el freno
 * superior: estas categorías solo afinan qué escribe el modo automático. */
use serde::Serialize;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

/// Categorías y alcances de escritura que cubre cada una.
pub const CATEGORIAS: &[(&str, &[&str])] = &[
    ("comandas", &["create_order", "cancel_order"]),
    (
        "articulos_clientes",
        &[
            "create_article",
            "modify_article",
            "modify_prices",
            "create_department",
            "create_family",
            "create_customer",
        ],
    ),
    ("propinas_puntos", &["add_tip", "add_points"]),
    ("pagos", &["add_payment"]),
    ("facturas", &["invoice"]),
    (
        "stock",
        &["regularize_stock", "transfer_stock", "inventory"],
    ),
    ("camarero", &["call_waiter"]),
];

/// Categoría a la que pertenece un alcance; `None` si el alcance no tiene una.
#[must_use]
pub fn categoria_de_scope(scope: &str) -> Option<&'static str> {
    CATEGORIAS
        .iter()
        .find(|(_, scopes)| scopes.contains(&scope))
        .map(|(categoria, _)| *categoria)
}

/// Estado de una categoría para la pantalla: siempre aparecen las siete.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PermisoCategoria {
    pub categoria: String,
    pub activo: bool,
}

/// Indica si el nombre corresponde a una categoría del modo automático.
#[must_use]
pub fn categoria_automatica_valida(categoria: &str) -> bool {
    CATEGORIAS.iter().any(|(c, _)| *c == categoria)
}

/// Lista todas las categorías con su interruptor (apagado si no hay fila).
pub async fn listar_permisos(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<PermisoCategoria>, String> {
    let activas: Vec<String> = sqlx::query_scalar(
        "SELECT categoria FROM bdp_permiso_automatico WHERE user_id = $1 AND activo",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("No se pudieron leer los permisos automáticos: {e}"))?;
    Ok(CATEGORIAS
        .iter()
        .map(|(categoria, _)| PermisoCategoria {
            categoria: (*categoria).to_string(),
            activo: activas.iter().any(|a| a == categoria),
        })
        .collect())
}

/// Enciende o apaga una categoría y deja constancia en el historial, en la
/// misma transacción: no puede cambiar sin quedar registrado.
pub async fn fijar_permiso(
    pool: &PgPool,
    user_id: Uuid,
    categoria: &str,
    activo: bool,
) -> Result<(), String> {
    let categoria = CATEGORIAS
        .iter()
        .find(|(c, _)| *c == categoria)
        .map(|(c, _)| *c)
        .ok_or_else(|| format!("Categoría desconocida: {categoria}"))?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("No se pudo iniciar el cambio de permiso: {e}"))?;
    sqlx::query(
        r"INSERT INTO bdp_permiso_automatico (user_id, categoria, activo, actualizado_en)
               VALUES ($1, $2, $3, NOW())
               ON CONFLICT (user_id, categoria)
               DO UPDATE SET activo = EXCLUDED.activo, actualizado_en = NOW()",
    )
    .bind(user_id)
    .bind(categoria)
    .bind(activo)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("No se pudo guardar el permiso: {e}"))?;
    sqlx::query(
        "INSERT INTO bdp_permiso_automatico_historial (user_id, categoria, activo) VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(categoria)
    .bind(activo)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("No se pudo registrar el cambio de permiso: {e}"))?;
    tx.commit()
        .await
        .map_err(|e| format!("No se pudo confirmar el permiso: {e}"))
}

/// Motivo por el que el modo automático no puede escribir este alcance, o
/// `None` si puede. Fail-closed: un alcance sin categoría se bloquea.
pub async fn motivo_bloqueo<'e, E: PgExecutor<'e>>(
    exec: E,
    user_id: Uuid,
    scope: &str,
) -> Result<Option<String>, String> {
    let Some(categoria) = categoria_de_scope(scope) else {
        return Ok(Some(format!(
            "el alcance '{scope}' no tiene categoría de permiso automático"
        )));
    };
    let activo: bool = sqlx::query_scalar(
        r"SELECT COALESCE(
                 (SELECT activo FROM bdp_permiso_automatico WHERE user_id = $1 AND categoria = $2),
                 FALSE)",
    )
    .bind(user_id)
    .bind(categoria)
    .fetch_one(exec)
    .await
    .map_err(|e| format!("No se pudo leer el permiso automático: {e}"))?;
    Ok((!activo).then(|| format!("la categoría '{categoria}' no tiene permiso automático")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::bdp_write_guard::armado::VALID_BDP_WRITE_SCOPES;

    #[test]
    fn todo_alcance_de_escritura_tiene_categoria() {
        for scope in VALID_BDP_WRITE_SCOPES {
            assert!(
                categoria_de_scope(scope).is_some(),
                "sin categoría: {scope}"
            );
        }
    }

    #[test]
    fn alcance_desconocido_no_tiene_categoria() {
        assert_eq!(categoria_de_scope("borrar_todo"), None);
        assert_eq!(categoria_de_scope("add_payment"), Some("pagos"));
        assert_eq!(categoria_de_scope("call_waiter"), Some("camarero"));
    }

    async fn usuario(pool: &PgPool) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(format!("{id}@test.local"))
            .bind("x")
            .execute(pool)
            .await
            .unwrap();
        id
    }

    async fn bloqueado(pool: &PgPool, user: Uuid, scope: &str) -> bool {
        motivo_bloqueo(pool, user, scope).await.unwrap().is_some()
    }

    /* Sin fila la categoría está apagada; encender una no abre las demás. */
    #[sqlx::test(migrations = "./migrations")]
    async fn categoria_apagada_bloquea_y_encender_una_no_abre_otra(pool: PgPool) {
        let user = usuario(&pool).await;
        assert!(bloqueado(&pool, user, "add_payment").await);
        fijar_permiso(&pool, user, "pagos", true).await.unwrap();
        assert!(!bloqueado(&pool, user, "add_payment").await);
        assert!(bloqueado(&pool, user, "create_order").await);
        assert!(bloqueado(&pool, user, "add_tip").await);
    }

    /* Apagar vuelve a bloquear y cada cambio queda en el historial. */
    #[sqlx::test(migrations = "./migrations")]
    async fn cada_cambio_queda_en_el_historial(pool: PgPool) {
        let user = usuario(&pool).await;
        fijar_permiso(&pool, user, "stock", true).await.unwrap();
        fijar_permiso(&pool, user, "stock", false).await.unwrap();
        assert!(bloqueado(&pool, user, "transfer_stock").await);
        let filas: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM bdp_permiso_automatico_historial WHERE user_id = $1",
        )
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(filas, 2);
    }

    /* El listado siempre trae las siete categorías, todas apagadas sin fila. */
    #[sqlx::test(migrations = "./migrations")]
    async fn listado_muestra_las_siete_apagadas_sin_fila(pool: PgPool) {
        let user = usuario(&pool).await;
        let listado = listar_permisos(&pool, user).await.unwrap();
        assert_eq!(listado.len(), 7);
        assert!(listado.iter().all(|p| !p.activo));
    }
}
