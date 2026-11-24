// sentinel-disable-file sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query_as! rompe el build.
/* [18AB-4] Guillermo (18/11): un artículo con «No permitir como invitación»
 * no se puede marcar como invitación en la web, aunque BDP lo acepte.
 * `Orders/Create` no comprueba `NotAllowedAsInvitation` (probado el 18/11 en el
 * demo: lo acepta con total 0), así que la única frontera posible es la nuestra
 * y va en la creación, antes de escribir la venta, para que el operador reciba
 * un 422 y no una comanda que el TPV cargará cobrada a cero. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::CrearVentaLineaRequest;

/* [21AB-5b] Guillermo (22/11): «hasta que se arregle, deja las invitaciones
 * desactivadas». El envío ya no sustituye el artículo en silencio, pero el
 * ticket del TPV todavía no se ha verificado con una invitación real
 * (`21AB-5b`): mientras este interruptor esté a `false`, marcar una se rechaza
 * antes de escribir nada. Al validarlo, se pone a `true` en una sola línea. */
pub const INVITACIONES_HABILITADAS: bool = false;

pub struct InvitacionService;

impl InvitacionService {
    /* Códigos cuyo mapeo BDP prohíbe la invitación (`NULL` = BDP no ha
     * informado el campo: la web no bloquea, igual que hace `setLineas`).
     * Los códigos locales puros no tienen mapeo y tampoco se bloquean: BDP no
     * conoce el artículo y el envío ya cae al artículo por defecto. */
    async fn prohibidos(
        pool: &PgPool,
        user_id: Uuid,
        codigos: &[&str],
    ) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT articulo_glory_codigo FROM bdp_article_map \
             WHERE user_id = $1 AND articulo_glory_codigo = ANY($2) \
               AND invitacion_prohibida IS TRUE",
        )
        .bind(user_id)
        .bind(codigos)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(|(codigo,)| codigo).collect())
    }

    /* [21AB-5b] Artículos invitados sin equivalencia BDP utilizable: sin
     * mapeo, desactivado o con código no numérico. Misma regla que
     * `resolve_line_articles`, que bloquea el envío si se cuela uno. */
    async fn sin_equivalencia_bdp(
        pool: &PgPool,
        user_id: Uuid,
        invitadas: &[&CrearVentaLineaRequest],
    ) -> Result<Vec<String>, AppError> {
        let mut faltan = Vec::new();
        for linea in invitadas {
            let codigo = linea.articulo_codigo.as_deref().unwrap_or_default();
            let valido = !codigo.is_empty()
                && matches!(
                    crate::repositories::BdpArticleMapRepository::buscar_por_codigo(
                        pool, user_id, codigo
                    )
                    .await,
                    Ok(Some(ref map)) if map.activo
                        && map.articulo_bdp_codigo.trim().parse::<i64>().is_ok_and(|id| id > 0)
                );
            if !valido {
                faltan.push(format!("«{}»", linea.descripcion));
            }
        }
        Ok(faltan)
    }

    /* `linea.invitacion` se guarda en `venta_lineas`; el `Invitation` del ítem
     * BDP lo escribe `items_de_lineas` desde ahí. */
    pub async fn validar_lineas(
        pool: &PgPool,
        user_id: Uuid,
        lineas: &[CrearVentaLineaRequest],
    ) -> Result<(), AppError> {
        let invitadas: Vec<&CrearVentaLineaRequest> = lineas
            .iter()
            .filter(|linea| linea.invitacion == Some(true))
            .collect();
        if invitadas.is_empty() {
            return Ok(());
        }
        if !INVITACIONES_HABILITADAS {
            let nombres: Vec<String> = invitadas
                .iter()
                .map(|linea| format!("«{}»", linea.descripcion))
                .collect();
            return Err(AppError::Validation(format!(
                "Las invitaciones están desactivadas: {} no se puede marcar como invitación \
                 hasta que el ticket del TPV muestre el artículo pedido. Cobra la línea normal \
                 o retira la invitación",
                nombres.join(", ")
            )));
        }

        /* [21AB-5b] Con el interruptor activo, la invitada exige equivalencia
         * BDP explícita: el artículo por defecto haría que el ticket mostrara
         * otro producto. Es la misma regla que aplica el envío, aquí antes de
         * escribir la venta para que el operador vea el aviso al marcar. */
        let sin_equivalencia = Self::sin_equivalencia_bdp(pool, user_id, &invitadas).await?;
        if !sin_equivalencia.is_empty() {
            return Err(AppError::Validation(format!(
                "{} no tiene equivalencia activa en BDP y el ticket mostraría otro artículo. \
                 Marca el artículo en el catálogo BDP o retira la invitación",
                sin_equivalencia.join(", ")
            )));
        }
        let codigos: Vec<&str> = invitadas
            .iter()
            .filter_map(|linea| linea.articulo_codigo.as_deref())
            .filter(|codigo| !codigo.is_empty())
            .collect();
        if codigos.is_empty() {
            return Ok(());
        }
        let prohibidos = Self::prohibidos(pool, user_id, &codigos).await?;
        if prohibidos.is_empty() {
            return Ok(());
        }
        let nombres: Vec<String> = invitadas
            .iter()
            .filter(|linea| {
                linea
                    .articulo_codigo
                    .as_deref()
                    .is_some_and(|codigo| prohibidos.iter().any(|p| p == codigo))
            })
            .map(|linea| format!("«{}»", linea.descripcion))
            .collect();
        Err(AppError::Validation(format!(
            "{} no se puede marcar como invitación: su ficha de BDP tiene «No permitir como invitación». Quita la invitación o elige otro artículo",
            nombres.join(", ")
        )))
    }
}

/* [21AB-5b] Con `INVITACIONES_HABILITADAS = false` el servicio corta antes de
 * mirar el catálogo. Las reglas por artículo se fijan aquí, contra las
 * funciones internas, para que sigan cubiertas mientras el interruptor esté
 * apagado y al encenderlo no haya que reescribir pruebas. */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CrearBdpArticleMapRequest;
    use crate::repositories::BdpArticleMapRepository;
    use rust_decimal::Decimal;

    async fn usuario(pool: &PgPool) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(format!("invitacion-{id}@example.com"))
            .bind("argon2_hash_placeholder")
            .execute(pool)
            .await
            .expect("usuario de prueba");
        id
    }

    async fn mapear(
        pool: &PgPool,
        user_id: Uuid,
        glory: &str,
        bdp: Option<&str>,
        activo: bool,
        prohibida: Option<bool>,
    ) {
        let _ = BdpArticleMapRepository::crear(
            pool,
            user_id,
            &CrearBdpArticleMapRequest {
                articulo_glory_codigo: glory.into(),
                articulo_bdp_codigo: bdp.map(Into::into),
                articulo_bdp_nombre: Some(glory.into()),
                descripcion: None,
                precio_tarifa1: None,
                iva_pct: None,
                departamento: None,
                familia: None,
                subfamilia: None,
                activo: Some(activo),
                barcode: None,
            },
        )
        .await
        .expect("crear mapeo");
        if let Some(prohibida) = prohibida {
            sqlx::query(
                "UPDATE bdp_article_map SET invitacion_prohibida = $1 \
                 WHERE user_id = $2 AND articulo_glory_codigo = $3",
            )
            .bind(prohibida)
            .bind(user_id)
            .bind(glory)
            .execute(pool)
            .await
            .expect("marcar prohibición");
        }
    }

    fn invitada(codigo: Option<&str>, descripcion: &str) -> CrearVentaLineaRequest {
        CrearVentaLineaRequest {
            articulo_codigo: codigo.map(Into::into),
            descripcion: descripcion.into(),
            cantidad: Some(Decimal::ONE),
            precio_unitario: Decimal::ONE,
            iva_pct: Some(Decimal::from(10)),
            descuento: Some(Decimal::ZERO),
            invitacion: Some(true),
            comentario: None,
        }
    }

    /* Solo el mapeo activo con código numérico positivo cuenta como
     * equivalencia; los demás casos se nombran para el aviso al operador. */
    #[sqlx::test(migrations = "./migrations")]
    async fn sin_equivalencia_nombra_solo_las_invitadas_que_fallan(pool: PgPool) {
        let user_id = usuario(&pool).await;
        mapear(&pool, user_id, "OK", Some("1001"), true, None).await;
        mapear(&pool, user_id, "APAGADO", Some("1003"), false, None).await;
        mapear(&pool, user_id, "TEXTO", Some("REF-9"), true, None).await;

        let lineas = [
            invitada(Some("OK"), "Con equivalencia"),
            invitada(Some("APAGADO"), "Desactivada"),
            invitada(Some("TEXTO"), "Código no numérico"),
            invitada(Some("SIN-MAPEO"), "Sin mapeo"),
            invitada(None, "Sin código"),
        ];
        let refs: Vec<&CrearVentaLineaRequest> = lineas.iter().collect();
        let faltan = InvitacionService::sin_equivalencia_bdp(&pool, user_id, &refs)
            .await
            .expect("consulta del catálogo");

        assert_eq!(
            faltan,
            vec![
                "«Desactivada»".to_string(),
                "«Código no numérico»".to_string(),
                "«Sin mapeo»".to_string(),
                "«Sin código»".to_string(),
            ],
            "solo la equivalencia utilizable queda fuera de la lista"
        );
    }

    /* La marca de la ficha se respeta cuando el interruptor esté encendido. */
    #[sqlx::test(migrations = "./migrations")]
    async fn prohibidos_devuelve_los_marcados(pool: PgPool) {
        let user_id = usuario(&pool).await;
        mapear(&pool, user_id, "1002", Some("1002"), true, Some(true)).await;
        mapear(&pool, user_id, "1004", Some("1004"), true, Some(false)).await;

        let prohibidos = InvitacionService::prohibidos(&pool, user_id, &["1002", "1004"])
            .await
            .expect("consulta de prohibiciones");
        assert_eq!(prohibidos, vec!["1002".to_string()]);
    }
}
