// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [26AA-3] Permisos del modo automático por categoría. Cada categoría agrupa
 * alcances de escritura; sin fila en `bdp_permiso_automatico` está apagada.
 * El interruptor general (`bdp_sync_mode = automatic`) sigue siendo el freno
 * superior: estas categorías solo afinan qué escribe el modo automático.
 *
 * [21AB-5] Etapas de puesta en marcha, como las pidió Guillermo: se enciende
 * lectura, luego comandas, modificaciones, descuentos, pagos y tickets, cada
 * una por separado. `lectura` y `descuentos` no tienen alcance de escritura
 * (no hay ninguna escritura BDP detrás), así que `categoria_de_scope` sigue
 * resolviendo solo las de escritura; su comprobación va por `etapa_activa`. */
use serde::Serialize;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

/// Etapas de la puesta en marcha, en el orden en que se encienden.
pub const ETAPAS: &[&str] = &[
    "lectura",
    "comandas",
    "modificaciones",
    "descuentos",
    "pagos",
    "facturas",
];

/// Categorías y alcances de escritura que cubre cada una.
pub const CATEGORIAS: &[(&str, &[&str])] = &[
    ("lectura", &[]),
    ("comandas", &["create_order", "cancel_order"]),
    /* [21AB-5] Modificar una comanda ya creada se enciende aparte de crearla:
     * es la operación que reescribe una comanda viva en el TPV. */
    ("modificaciones", &["modify_order"]),
    /* [21AB-5] Los descuentos no son una escritura propia: viajan dentro del
     * payload de la comanda (`Discount` por línea). La etapa se comprueba en
     * la frontera del envío; sin alcance, no aparece en `categoria_de_scope`. */
    ("descuentos", &[]),
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

/// Estado de una categoría para la pantalla: siempre aparecen todas.
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

/// ¿Está encendida esta etapa? Vale para las etapas sin alcance de escritura
/// (`lectura`, `descuentos`), que no pasan por `motivo_bloqueo`.
pub async fn etapa_activa<'e, E: PgExecutor<'e>>(
    exec: E,
    user_id: Uuid,
    etapa: &str,
) -> Result<bool, String> {
    if !categoria_automatica_valida(etapa) {
        return Err(format!("Etapa desconocida: {etapa}"));
    }
    sqlx::query_scalar(
        r"SELECT COALESCE(
                 (SELECT activo FROM bdp_permiso_automatico WHERE user_id = $1 AND categoria = $2),
                 FALSE)",
    )
    .bind(user_id)
    .bind(etapa)
    .fetch_one(exec)
    .await
    .map_err(|e| format!("No se pudo leer la etapa '{etapa}': {e}"))
}

/// [21AB-5] Apaga todas las etapas de golpe: la vuelta rápida a solo lectura
/// que pide Guillermo. Un solo UPDATE, todo o nada, y cada etapa apagada queda
/// en el historial para que la actividad muestre cuándo se cortó. Las filas
/// que ya estaban apagadas no generan evento: no hubo cambio.
pub async fn apagar_todas_las_etapas(pool: &PgPool, user_id: Uuid) -> Result<u32, String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("No se pudo iniciar la vuelta a solo lectura: {e}"))?;
    let apagadas: Vec<String> = sqlx::query_scalar(
        r"UPDATE bdp_permiso_automatico
              SET activo = FALSE, actualizado_en = NOW()
            WHERE user_id = $1 AND activo
            RETURNING categoria",
    )
    .bind(user_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("No se pudieron apagar las etapas: {e}"))?;
    for categoria in &apagadas {
        sqlx::query(
            "INSERT INTO bdp_permiso_automatico_historial (user_id, categoria, activo) VALUES ($1, $2, FALSE)",
        )
        .bind(user_id)
        .bind(categoria)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("No se pudo registrar el apagado de '{categoria}': {e}"))?;
    }
    tx.commit()
        .await
        .map_err(|e| format!("No se pudo confirmar la vuelta a solo lectura: {e}"))?;
    /* Una cuenta tiene diez etapas como mucho: el cast no puede truncar. */
    Ok(u32::try_from(apagadas.len()).unwrap_or(u32::MAX))
}

/// [21AB-5] Últimos cambios de interruptores, para la pantalla de actividad.
/// Acotado aquí y no en el handler: el historial crece con cada cambio y la
/// consulta no debe poder pedirlo entero.
pub async fn actividad_reciente(
    pool: &PgPool,
    user_id: Uuid,
    limite: i64,
) -> Result<Vec<CambioActividad>, String> {
    let limite = limite.clamp(1, 100);
    let filas: Vec<(String, bool, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        r"SELECT categoria, activo, created_at
            FROM bdp_permiso_automatico_historial
           WHERE user_id = $1
           ORDER BY created_at DESC
           LIMIT $2",
    )
    .bind(user_id)
    .bind(limite)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("No se pudo leer la actividad: {e}"))?;
    Ok(filas
        .into_iter()
        .map(|(categoria, activo, cambiado_en)| CambioActividad {
            categoria,
            activo,
            cambiado_en,
        })
        .collect())
}

/// Un cambio de interruptor, con su fecha.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CambioActividad {
    pub categoria: String,
    pub activo: bool,
    pub cambiado_en: chrono::DateTime<chrono::Utc>,
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

    /* El listado siempre trae las diez categorías, todas apagadas sin fila. */
    #[sqlx::test(migrations = "./migrations")]
    async fn listado_muestra_todas_apagadas_sin_fila(pool: PgPool) {
        let user = usuario(&pool).await;
        let listado = listar_permisos(&pool, user).await.unwrap();
        assert_eq!(listado.len(), 10);
        assert!(listado.iter().all(|p| !p.activo));
    }

    /* [21AB-5] Las seis etapas se encienden por separado y en el orden que
     * pide Guillermo: encender pagos no enciende tickets ni modificaciones. */
    #[sqlx::test(migrations = "./migrations")]
    async fn cada_etapa_se_enciende_por_separado(pool: PgPool) {
        let user = usuario(&pool).await;
        for etapa in ETAPAS {
            assert!(!etapa_activa(&pool, user, etapa).await.unwrap(), "{etapa}");
        }
        for etapa in ["pagos", "modificaciones", "descuentos"] {
            fijar_permiso(&pool, user, etapa, true).await.unwrap();
        }
        for etapa in ETAPAS {
            let esperado = ["pagos", "modificaciones", "descuentos"].contains(etapa);
            assert_eq!(
                etapa_activa(&pool, user, etapa).await.unwrap(),
                esperado,
                "etapa {etapa}"
            );
        }
        assert!(etapa_activa(&pool, user, "inventada").await.is_err());
    }

    /* [21AB-5] `modify_order` es escritura propia: tiene su etapa y bloquea
     * sin ella, la modificación no sale aunque la creación esté encendida. */
    #[sqlx::test(migrations = "./migrations")]
    async fn modificar_una_comanda_necesita_su_propia_etapa(pool: PgPool) {
        let user = usuario(&pool).await;
        fijar_permiso(&pool, user, "comandas", true).await.unwrap();
        assert!(!bloqueado(&pool, user, "create_order").await);
        assert!(bloqueado(&pool, user, "modify_order").await);
        fijar_permiso(&pool, user, "modificaciones", true)
            .await
            .unwrap();
        assert!(!bloqueado(&pool, user, "modify_order").await);
    }

    /* [21AB-5] La vuelta a solo lectura apaga lo que esté encendido, deja
     * constancia de cada apagado y devuelve cuántas etapas cortó. */
    #[sqlx::test(migrations = "./migrations")]
    async fn la_vuelta_a_solo_lectura_lo_apaga_todo(pool: PgPool) {
        let user = usuario(&pool).await;
        for etapa in ["lectura", "comandas", "pagos"] {
            fijar_permiso(&pool, user, etapa, true).await.unwrap();
        }
        assert_eq!(apagar_todas_las_etapas(&pool, user).await.unwrap(), 3);
        for etapa in ETAPAS {
            assert!(!etapa_activa(&pool, user, etapa).await.unwrap(), "{etapa}");
        }
        assert!(bloqueado(&pool, user, "add_payment").await);
        /* Una segunda vuelta no corta nada: ya estaba todo apagado. */
        assert_eq!(apagar_todas_las_etapas(&pool, user).await.unwrap(), 0);
        let historial: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM bdp_permiso_automatico_historial WHERE user_id = $1",
        )
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(historial, 6, "tres encendidos y tres apagados");
    }
}
