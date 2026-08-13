/* [128A-1/F7] Repositorio de menús/packs locales (D2, §4.10).
 * CRUD local sobre `bdp_menus_locales` + `bdp_menu_local_lineas`.
 * Consultas dinámicas (sin macro) para no depender del cache offline `.sqlx/`. */

use rust_decimal::Decimal;
use sqlx::Arguments;
use sqlx::PgPool;
use std::collections::HashMap;
use std::fmt::Write as _;
use uuid::Uuid;

use crate::models::{
    ActualizarBdpMenuLocalRequest, BdpMenuLocal, BdpMenuLocalConLineas, BdpMenuLocalLinea,
    BdpMenuLocalLineaRequest, BdpMenuLocalListParams, BdpMenuLocalTipo, CrearBdpMenuLocalRequest,
};

pub struct BdpMenuLocalRepository;

const COLUMNAS_MENU: &str =
    "id, user_id, tipo, nombre, descripcion, precio, activo, created_at, updated_at";
const COLUMNAS_LINEA: &str =
    "id, menu_id, articulo_codigo, descripcion, cantidad, precio_unitario, orden, created_at";

impl BdpMenuLocalRepository {
    /// Lista los menús/packs locales de un usuario con sus líneas.
    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
        params: &BdpMenuLocalListParams,
    ) -> Result<Vec<BdpMenuLocalConLineas>, sqlx::Error> {
        let menus = Self::listar_menus(pool, user_id, params).await?;
        Self::cargar_lineas(pool, &menus).await
    }

    /// Obtiene un menú/pack por ID con sus líneas, validando propiedad.
    pub async fn find_by_id(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<BdpMenuLocalConLineas>, sqlx::Error> {
        let query =
            format!("SELECT {COLUMNAS_MENU} FROM bdp_menus_locales WHERE id = $1 AND user_id = $2");
        let menu: Option<BdpMenuLocal> = sqlx::query_as::<_, BdpMenuLocal>(&query)
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
        let Some(menu) = menu else {
            return Ok(None);
        };
        let lineas = Self::lineas_de(pool, id).await?;
        Ok(Some(Self::a_con_lineas(menu, lineas)))
    }

    /// Crea un menú/pack local con sus líneas en una transacción.
    pub async fn crear(
        pool: &PgPool,
        user_id: Uuid,
        req: &CrearBdpMenuLocalRequest,
    ) -> Result<BdpMenuLocalConLineas, sqlx::Error> {
        let id = Uuid::new_v4();
        let tipo: BdpMenuLocalTipo = req.tipo.as_str().into();
        let precio = req.precio.unwrap_or_else(|| sumar_lineas(&req.lineas));
        let activo = req.activo.unwrap_or(true);

        let mut tx = pool.begin().await?;
        sqlx::query(
            "INSERT INTO bdp_menus_locales \
                (id, user_id, tipo, nombre, descripcion, precio, activo, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
        )
        .bind(id)
        .bind(user_id)
        .bind(&tipo)
        .bind(&req.nombre)
        .bind(req.descripcion.as_deref())
        .bind(precio)
        .bind(activo)
        .execute(&mut *tx)
        .await?;

        Self::insertar_lineas(&mut tx, id, &req.lineas).await?;
        tx.commit().await?;

        Self::find_by_id(pool, id, user_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Actualiza un menú/pack local (COALESCE por campo) y, si llegan líneas,
    /// las reemplaza. Todo en una transacción.
    pub async fn actualizar(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: &ActualizarBdpMenuLocalRequest,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let tipo: Option<BdpMenuLocalTipo> = req.tipo.as_deref().map(Into::into);
        /* Si no llega precio explícito pero sí líneas nuevas, se recalcula
         * desde las líneas (el precio de venta sigue a la composición). */
        let precio_nuevo = req
            .precio
            .or_else(|| req.lineas.as_ref().map(|lineas| sumar_lineas(lineas)));
        let result = sqlx::query(
            "UPDATE bdp_menus_locales SET \
                tipo = COALESCE($3, tipo), \
                nombre = COALESCE($4, nombre), \
                descripcion = COALESCE($5, descripcion), \
                precio = COALESCE($6, precio), \
                activo = COALESCE($7, activo), \
                updated_at = NOW() \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .bind(tipo)
        .bind(req.nombre.as_deref())
        .bind(req.descripcion.as_deref())
        .bind(precio_nuevo)
        .bind(req.activo)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        if let Some(lineas) = &req.lineas {
            sqlx::query("DELETE FROM bdp_menu_local_lineas WHERE menu_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            Self::insertar_lineas(&mut tx, id, lineas).await?;
        }
        tx.commit().await?;
        Ok(true)
    }

    /// Elimina un menú/pack local (las líneas se borran por CASCADE).
    pub async fn eliminar<'e, E>(executor: E, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let result = sqlx::query("DELETE FROM bdp_menus_locales WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /* ── Helpers privados ──────────────────────────────────────────────── */

    async fn listar_menus(
        pool: &PgPool,
        user_id: Uuid,
        params: &BdpMenuLocalListParams,
    ) -> Result<Vec<BdpMenuLocal>, sqlx::Error> {
        let mut query = format!("SELECT {COLUMNAS_MENU} FROM bdp_menus_locales WHERE user_id = $1");
        let mut args = sqlx::postgres::PgArguments::default();
        let _ = args.add(user_id);
        let mut param_idx: usize = 1;

        if let Some(tipo) = &params.tipo {
            param_idx += 1;
            let _ = write!(query, " AND tipo = ${param_idx}");
            let _ = args.add(tipo.clone());
        }
        if let Some(activo) = params.activo {
            param_idx += 1;
            let _ = write!(query, " AND activo = ${param_idx}");
            let _ = args.add(activo);
        }
        if let Some(busqueda) = &params.busqueda {
            let termino = busqueda.trim();
            if !termino.is_empty() {
                param_idx += 1;
                let _ = write!(
                    query,
                    " AND (nombre ILIKE ${param_idx} OR COALESCE(descripcion, '') ILIKE ${param_idx})"
                );
                let _ = args.add(format!("%{termino}%"));
            }
        }

        query.push_str(" ORDER BY tipo, nombre");
        sqlx::query_as_with::<_, BdpMenuLocal, _>(&query, args)
            .fetch_all(pool)
            .await
    }

    async fn cargar_lineas(
        pool: &PgPool,
        menus: &[BdpMenuLocal],
    ) -> Result<Vec<BdpMenuLocalConLineas>, sqlx::Error> {
        let ids: Vec<Uuid> = menus.iter().map(|m| m.id).collect();
        let lineas = if ids.is_empty() {
            Vec::new()
        } else {
            let query = format!(
                "SELECT {COLUMNAS_LINEA} FROM bdp_menu_local_lineas WHERE menu_id = ANY($1) \
                 ORDER BY orden, created_at, id"
            );
            sqlx::query_as::<_, BdpMenuLocalLinea>(&query)
                .bind(ids.as_slice())
                .fetch_all(pool)
                .await?
        };

        let mut por_menu: HashMap<Uuid, Vec<BdpMenuLocalLinea>> = HashMap::new();
        for linea in lineas {
            por_menu.entry(linea.menu_id).or_default().push(linea);
        }

        Ok(menus
            .iter()
            .map(|m| Self::a_con_lineas(m.clone(), por_menu.remove(&m.id).unwrap_or_default()))
            .collect())
    }

    async fn lineas_de(
        pool: &PgPool,
        menu_id: Uuid,
    ) -> Result<Vec<BdpMenuLocalLinea>, sqlx::Error> {
        let query = format!(
            "SELECT {COLUMNAS_LINEA} FROM bdp_menu_local_lineas WHERE menu_id = $1 \
             ORDER BY orden, created_at, id"
        );
        sqlx::query_as::<_, BdpMenuLocalLinea>(&query)
            .bind(menu_id)
            .fetch_all(pool)
            .await
    }

    async fn insertar_lineas(
        conn: &mut sqlx::PgConnection,
        menu_id: Uuid,
        lineas: &[BdpMenuLocalLineaRequest],
    ) -> Result<(), sqlx::Error> {
        for (orden, linea) in (0_i32..).zip(lineas.iter()) {
            sqlx::query(
                "INSERT INTO bdp_menu_local_lineas \
                    (id, menu_id, articulo_codigo, descripcion, cantidad, precio_unitario, orden, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
            )
            .bind(Uuid::new_v4())
            .bind(menu_id)
            .bind(linea.articulo_codigo.as_deref())
            .bind(&linea.descripcion)
            .bind(linea.cantidad.unwrap_or(Decimal::ONE))
            .bind(linea.precio_unitario.unwrap_or(Decimal::ZERO))
            .bind(orden)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }

    fn a_con_lineas(menu: BdpMenuLocal, lineas: Vec<BdpMenuLocalLinea>) -> BdpMenuLocalConLineas {
        BdpMenuLocalConLineas {
            id: menu.id,
            user_id: menu.user_id,
            tipo: menu.tipo,
            nombre: menu.nombre,
            descripcion: menu.descripcion,
            precio: menu.precio,
            activo: menu.activo,
            created_at: menu.created_at,
            updated_at: menu.updated_at,
            lineas,
        }
    }
}

/// Precio calculado de un menú/pack: suma de `cantidad * precio_unitario`.
#[must_use]
pub fn sumar_lineas(lineas: &[BdpMenuLocalLineaRequest]) -> Decimal {
    lineas.iter().fold(Decimal::ZERO, |acc, linea| {
        acc + linea.cantidad.unwrap_or(Decimal::ONE)
            * linea.precio_unitario.unwrap_or(Decimal::ZERO)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn linea(descripcion: &str, cantidad: &str, precio: &str) -> BdpMenuLocalLineaRequest {
        BdpMenuLocalLineaRequest {
            articulo_codigo: Some("ART-001".to_string()),
            descripcion: descripcion.to_string(),
            cantidad: Some(Decimal::from_str(cantidad).unwrap()),
            precio_unitario: Some(Decimal::from_str(precio).unwrap()),
        }
    }

    #[test]
    fn sumar_lineas_calcula_precio_total() {
        let lineas = vec![
            linea("Coca-Cola", "2", "1.50"),
            linea("Hamburguesa", "1", "5.00"),
        ];
        assert_eq!(sumar_lineas(&lineas), Decimal::from_str("8.00").unwrap());
    }

    #[test]
    fn sumar_lineas_vacia_es_cero() {
        assert_eq!(sumar_lineas(&[]), Decimal::ZERO);
    }

    #[test]
    fn tipo_as_str_mapea_valores() {
        assert_eq!(BdpMenuLocalTipo::Menu.as_str(), "menu");
        assert_eq!(BdpMenuLocalTipo::Pack.as_str(), "pack");
        let desde_string: BdpMenuLocalTipo = "pack".to_string().into();
        assert_eq!(desde_string, BdpMenuLocalTipo::Pack);
    }
}
