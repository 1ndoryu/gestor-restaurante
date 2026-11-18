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
