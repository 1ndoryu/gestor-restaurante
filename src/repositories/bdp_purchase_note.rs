/* [247A-11] Repositorio de albaranes de compra BDP (solo lectura).
 * CRUD de cache local y upsert desde respuesta BDP. */

use rust_decimal::Decimal;
use sqlx::Arguments;
use sqlx::PgPool;
use std::fmt::Write as _;
use std::str::FromStr;
use uuid::Uuid;

use crate::models::{
    ActualizarBdpPurchaseNoteRequest, BdpPurchaseNote, BdpPurchaseNoteLineaLocal,
    BdpPurchaseNoteListParams, CrearBdpPurchaseNoteRequest,
};

pub struct BdpPurchaseNoteRepository;

/* [128A-1/F5][F5-1] Prefijo reservado para series de albaranes locales (M18):
 * ninguna serie local puede salir de este prefijo, así el UNIQUE
 * (user_id, serie, numero) no puede colisionar con importados de BDP. */
pub const SERIE_LOCAL_PREFIJO: &str = "L";

impl BdpPurchaseNoteRepository {
    /// Lista los albaranes de compra de un usuario, opcionalmente filtrados.
    pub async fn listar(
        pool: &PgPool,
        user_id: Uuid,
        params: &BdpPurchaseNoteListParams,
    ) -> Result<Vec<BdpPurchaseNote>, sqlx::Error> {
        let mut query = String::from("SELECT * FROM bdp_purchase_notes WHERE user_id = $1");
        let mut args = sqlx::postgres::PgArguments::default();
        let _ = args.add(user_id);
        let mut param_idx: usize = 1;

        if let Some(proveedor) = &params.proveedor {
            param_idx += 1;
            let _ = write!(
                query,
                " AND (codigo_proveedor ILIKE ${param_idx} OR nombre_proveedor ILIKE ${param_idx})"
            );
            let _ = args.add(format!("%{proveedor}%"));
        }
        if let Some(ref desde) = params.fecha_desde {
            param_idx += 1;
            let _ = write!(query, " AND fecha >= ${param_idx}");
            let _ = args.add(desde.clone());
        }
        if let Some(ref hasta) = params.fecha_hasta {
            param_idx += 1;
            let _ = write!(query, " AND fecha <= ${param_idx}");
            let _ = args.add(hasta.clone());
        }

        query.push_str(" ORDER BY fecha DESC NULLS LAST, serie, numero");

        sqlx::query_as_with(&query, args).fetch_all(pool).await
    }

    /// Inserta o actualiza un albarán a partir de los datos devueltos por BDP.
    /// La clave natural es (`user_id`, `serie`, `numero`).
    /// Preserva el estado local y el `gasto_id` en caso de resincronización.
    pub async fn upsert_from_bdp(
        pool: &PgPool,
        user_id: Uuid,
        note: &crate::services::bdp_weblink_catalog::BdpPurchaseNoteData,
    ) -> Result<bool, sqlx::Error> {
        let serie = note.serie_albaran.as_deref().unwrap_or("");
        let numero = note.num_albaran.as_deref().unwrap_or("");
        let codigo_proveedor = note
            .cod_proveedor
            .as_ref()
            .map(|v| v.as_str().map_or_else(|| v.to_string(), String::from));
        let nombre_proveedor = note.nom_proveedor.as_deref().unwrap_or("");
        let total = note.total_albaran;
        let fecha = note.fecha_albaran.as_deref().and_then(parse_fecha_bdp);

        /* [128A-1/F5][F5-1] El `DO UPDATE` solo toca filas importadas
         * (`origen='bdp'`): si la clave (user_id, serie, numero) ya la ocupa
         * un albarán LOCAL, el sync NUNCA pisa total/fecha/datos locales. El
         * conflicto queda sin actualizar (rows_affected=0 → no procesado). */
        let result = sqlx::query(
            "INSERT INTO bdp_purchase_notes \
                (id, user_id, serie, numero, fecha, codigo_proveedor, nombre_proveedor, total, datos_bdp, estado, ultima_sync_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pendiente', NOW(), NOW(), NOW()) \
             ON CONFLICT (user_id, serie, numero) DO UPDATE SET \
                fecha = EXCLUDED.fecha, \
                codigo_proveedor = EXCLUDED.codigo_proveedor, \
                nombre_proveedor = EXCLUDED.nombre_proveedor, \
                total = EXCLUDED.total, \
                datos_bdp = EXCLUDED.datos_bdp, \
                ultima_sync_at = NOW(), \
                updated_at = NOW() \
             WHERE bdp_purchase_notes.origen = 'bdp'",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(serie)
        .bind(numero)
        .bind(fecha)
        .bind(codigo_proveedor)
        .bind(nombre_proveedor)
        .bind(total)
        .bind(serde_json::to_value(note).unwrap_or(serde_json::json!({})))
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Obtiene un albarán por ID, validando propiedad del usuario.
    pub async fn find_by_id<'e, E>(
        executor: E,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<crate::models::BdpPurchaseNote>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        /* [287A-4] Consulta dinámica con tipo explícito para que el build
         * offline no dependa de metadatos SQLx ausentes en `.sqlx/`. */
        sqlx::query_as::<_, crate::models::BdpPurchaseNote>(
            "SELECT id, user_id, serie, numero, fecha, codigo_proveedor, nombre_proveedor, total, datos_bdp, origen, estado, gasto_id, ultima_sync_at, created_at, updated_at \
             FROM bdp_purchase_notes WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(executor)
        .await
    }

    /* [128A-1/F5] CRUD de albaranes locales (M18). Los albaranes locales usan
     * la serie reservada `L` por defecto para no chocar con el
     * UNIQUE(user_id, serie, numero) de los importados de BDP. */

    /// Crea un albarán de compra local (`origen='local'`, estado `pendiente`).
    pub async fn crear_local(
        pool: &PgPool,
        user_id: Uuid,
        req: &CrearBdpPurchaseNoteRequest,
    ) -> Result<BdpPurchaseNote, sqlx::Error> {
        /* [128A-1/F5][F5-1] M18: la serie local SIEMPRE usa el prefijo
         * reservado; si el cliente manda otra, se rechaza (spoofeable). */
        let serie = req
            .serie
            .clone()
            .unwrap_or_else(|| SERIE_LOCAL_PREFIJO.to_string());
        if !serie.starts_with(SERIE_LOCAL_PREFIJO) {
            return Err(sqlx::Error::Protocol(format!(
                "serie_local_prefijo_invalido: la serie local debe empezar por el prefijo reservado '{SERIE_LOCAL_PREFIJO}'"
            )));
        }
        let (total, datos_bdp) = construir_total_y_datos(req).map_err(sqlx::Error::Protocol)?;

        /* [128A-1/F5][F5-2] Secuencial por (user_id, serie) con reintento ante
         * colisión de carrera (23505): dos altas simultáneas no generan el
         * mismo número. Si `req.numero` viene explícito no se reintenta (el
         * duplicado es error real → 409 en el handler). */
        let mut intentos = 0;
        loop {
            intentos += 1;
            let numero = if let Some(n) = &req.numero {
                n.clone()
            } else {
                let max: Option<i32> = sqlx::query_scalar(
                    "SELECT MAX(numero::integer) FROM bdp_purchase_notes \
                     WHERE user_id = $1 AND serie = $2 AND origen = 'local' \
                       AND numero ~ '^[0-9]+$'",
                )
                .bind(user_id)
                .bind(&serie)
                .fetch_one(pool)
                .await?;
                (max.unwrap_or(0) + 1).to_string()
            };
            let fecha = req
                .fecha
                .as_deref()
                .and_then(|f| chrono::NaiveDate::parse_from_str(f, "%Y-%m-%d").ok());

            let resultado = sqlx::query_as::<_, BdpPurchaseNote>(
                "INSERT INTO bdp_purchase_notes \
                    (id, user_id, serie, numero, fecha, codigo_proveedor, nombre_proveedor, \
                     total, datos_bdp, origen, estado, ultima_sync_at, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'local', 'pendiente', NULL, NOW(), NOW()) \
                 RETURNING id, user_id, serie, numero, fecha, codigo_proveedor, nombre_proveedor, \
                           total, datos_bdp, origen, estado, gasto_id, ultima_sync_at, created_at, updated_at",
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(&serie)
            .bind(&numero)
            .bind(fecha)
            .bind(req.codigo_proveedor.as_deref())
            .bind(req.nombre_proveedor.as_deref())
            .bind(total)
            .bind(datos_bdp.clone())
            .fetch_one(pool)
            .await;

            match resultado {
                Ok(note) => return Ok(note),
                Err(sqlx::Error::Database(ref db))
                    if db.is_unique_violation() && req.numero.is_none() && intentos < 3 =>
                { /* Carrera de numeración: recalcular y reintentar. */ }
                Err(e) => return Err(e),
            }
        }
    }

    /// Actualiza un albarán local (COALESCE por campo; `datos_bdp` se recalcula
    /// solo si llegan líneas nuevas).
    pub async fn actualizar_local<'e, E>(
        executor: E,
        id: Uuid,
        user_id: Uuid,
        req: &ActualizarBdpPurchaseNoteRequest,
    ) -> Result<bool, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let (total_nuevo, datos_nuevo) =
            calcular_actualizacion(req).map_err(sqlx::Error::Protocol)?;
        let fecha = req
            .fecha
            .as_deref()
            .and_then(|f| chrono::NaiveDate::parse_from_str(f, "%Y-%m-%d").ok());
        let result = sqlx::query(
            "UPDATE bdp_purchase_notes SET \
                numero = COALESCE($3, numero), \
                fecha = COALESCE($4, fecha), \
                codigo_proveedor = COALESCE($5, codigo_proveedor), \
                nombre_proveedor = COALESCE($6, nombre_proveedor), \
                total = COALESCE($7, total), \
                datos_bdp = COALESCE($8, datos_bdp), \
                updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 AND origen = 'local'",
        )
        .bind(id)
        .bind(user_id)
        .bind(req.numero.as_deref())
        .bind(fecha)
        .bind(req.codigo_proveedor.as_deref())
        .bind(req.nombre_proveedor.as_deref())
        .bind(total_nuevo)
        .bind(datos_nuevo)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Elimina un albarán local. Solo `pendiente` o `borrador`; los conciliados
    /// no se borran (D5).
    pub async fn eliminar_local<'e, E>(
        executor: E,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let result = sqlx::query(
            "DELETE FROM bdp_purchase_notes \
             WHERE id = $1 AND user_id = $2 AND origen = 'local' \
               AND estado IN ('pendiente', 'borrador')",
        )
        .bind(id)
        .bind(user_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Extrae el desglose `(base, iva)` de un albarán local. Devuelve `None`
    /// para albaranes importados de BDP (sin `datos_bdp.lineas`).
    #[must_use]
    pub fn desglose_desde_datos(datos: &serde_json::Value) -> Option<(Decimal, Decimal)> {
        let lineas = datos.get("lineas")?.as_array()?;
        let mut base = Decimal::ZERO;
        let mut iva = Decimal::ZERO;
        let cien = Decimal::from(100);
        for linea in lineas {
            let cantidad = decimal_desde_json(linea.get("cantidad")?)?;
            let precio = decimal_desde_json(linea.get("precio_unitario")?)?;
            let iva_pct = decimal_desde_json(linea.get("iva_pct")?)?;
            let importe = cantidad * precio;
            base += importe;
            iva += importe * iva_pct / cien;
        }
        Some((base, iva))
    }

    /// Marca un albarán como borrador. Solo puede pasar desde 'pendiente'.
    pub async fn marcar_borrador<'e, E>(
        executor: E,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let result = sqlx::query(
            "UPDATE bdp_purchase_notes \
             SET estado = 'borrador', updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 AND estado = 'pendiente'",
        )
        .bind(id)
        .bind(user_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Vincula un albarán con un gasto y lo marca como conciliado.
    /// Solo puede pasar desde 'borrador'.
    pub async fn vincular_gasto<'e, E>(
        executor: E,
        id: Uuid,
        user_id: Uuid,
        gasto_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let result = sqlx::query(
            "UPDATE bdp_purchase_notes \
             SET estado = 'conciliado', gasto_id = $1, updated_at = NOW() \
             WHERE id = $2 AND user_id = $3 AND estado = 'borrador'",
        )
        .bind(gasto_id)
        .bind(id)
        .bind(user_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

/* [128A-1/F5] Cálculo de total y `datos_bdp` para albaranes locales (A10:
 * IVA por línea). Las líneas se guardan en `datos_bdp.lineas` con su
 * `importe_base` e `importe_iva` individuales, y el desglose agregado en
 * `importe_base`/`importe_iva` del propio objeto. */

/// Calcula el total y los datos JSON de un albarán local al crearlo.
pub fn construir_total_y_datos(
    req: &CrearBdpPurchaseNoteRequest,
) -> Result<(Decimal, serde_json::Value), String> {
    if let Some(lineas) = &req.lineas {
        if !lineas.is_empty() {
            return calcular_desglose(lineas, req.total);
        }
    }
    Ok((req.total.unwrap_or(Decimal::ZERO), serde_json::json!({})))
}

/// Calcula los cambios de un albarán local al actualizarlo.
/// Devuelve `(total, None)` si no llegan líneas (no tocar `datos_bdp`).
pub fn calcular_actualizacion(
    req: &ActualizarBdpPurchaseNoteRequest,
) -> Result<(Option<Decimal>, Option<serde_json::Value>), String> {
    if let Some(lineas) = &req.lineas {
        if !lineas.is_empty() {
            let (total, datos) = calcular_desglose(lineas, req.total)?;
            return Ok((Some(total), Some(datos)));
        }
    }
    Ok((req.total, None))
}

fn calcular_desglose(
    lineas: &[BdpPurchaseNoteLineaLocal],
    total_explicito: Option<Decimal>,
) -> Result<(Decimal, serde_json::Value), String> {
    let mut importe_base = Decimal::ZERO;
    let mut importe_iva = Decimal::ZERO;
    let cien = Decimal::from(100);
    let mut lineas_json = Vec::with_capacity(lineas.len());
    for linea in lineas {
        let base = linea.cantidad * linea.precio_unitario;
        let iva = base * linea.iva_pct / cien;
        importe_base += base;
        importe_iva += iva;
        lineas_json.push(serde_json::json!({
            "descripcion": linea.descripcion,
            "cantidad": linea.cantidad,
            "precio_unitario": linea.precio_unitario,
            "iva_pct": linea.iva_pct,
            "importe_base": base,
            "importe_iva": iva,
        }));
    }
    /* [128A-1/F5][F5-3] El total SIEMPRE se calcula del desglose de líneas:
     * si el cliente manda un total explícito distinto, es un descuadre (el
     * gasto de conciliación saldría por el desglose y no cuadraría con el
     * albarán). Se rechaza con mensaje accionable. */
    let total = importe_base + importe_iva;
    if let Some(explicito) = total_explicito {
        if explicito != total {
            return Err(format!(
                "El total indicado ({explicito}) no coincide con el desglose de las líneas ({total})"
            ));
        }
    }
    Ok((
        total,
        serde_json::json!({
            "lineas": lineas_json,
            "importe_base": importe_base,
            "importe_iva": importe_iva,
        }),
    ))
}

fn decimal_desde_json(value: &serde_json::Value) -> Option<Decimal> {
    match value {
        serde_json::Value::String(s) => Decimal::from_str(s).ok(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Decimal::from(i))
            } else {
                Decimal::from_str(&n.to_string()).ok()
            }
        }
        _ => None,
    }
}

/// Parsea fechas que BDP devuelve como "2021-07-27T00:00:00" u otro formato.
/// Normaliza espacios antes de intentar el parseo.
fn parse_fecha_bdp(value: &str) -> Option<chrono::NaiveDate> {
    let cleaned = value
        .split_once('T')
        .map_or(value, |(date, _)| date)
        .replace(' ', "");
    chrono::NaiveDate::parse_from_str(&cleaned, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linea(
        descripcion: &str,
        cantidad: &str,
        precio: &str,
        iva: &str,
    ) -> BdpPurchaseNoteLineaLocal {
        BdpPurchaseNoteLineaLocal {
            descripcion: descripcion.to_string(),
            cantidad: Decimal::from_str(cantidad).unwrap(),
            precio_unitario: Decimal::from_str(precio).unwrap(),
            iva_pct: Decimal::from_str(iva).unwrap(),
        }
    }

    #[test]
    fn construir_total_y_datos_calcula_base_e_iva_por_linea() {
        let req = CrearBdpPurchaseNoteRequest {
            serie: None,
            numero: None,
            fecha: None,
            codigo_proveedor: None,
            nombre_proveedor: Some("Proveedor".to_string()),
            total: None,
            lineas: Some(vec![
                linea("Tomate", "2", "10.00", "10"),
                linea("Pan", "3", "2.00", "21"),
            ]),
        };
        let (total, datos) = construir_total_y_datos(&req).expect("desglose válido");
        /* base = 20 + 6 = 26; iva = 2 + 1.26 = 3.26; total = 29.26 */
        assert_eq!(total, Decimal::from_str("29.26").unwrap());
        assert_eq!(
            datos["importe_base"],
            serde_json::json!(Decimal::from_str("26.00").unwrap())
        );
        assert_eq!(
            datos["importe_iva"],
            serde_json::json!(Decimal::from_str("3.26").unwrap())
        );
        assert_eq!(datos["lineas"].as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn construir_total_y_datos_rechaza_total_discrepante() {
        let req = CrearBdpPurchaseNoteRequest {
            serie: None,
            numero: None,
            fecha: None,
            codigo_proveedor: None,
            nombre_proveedor: Some("Proveedor".to_string()),
            total: Some(Decimal::from_str("30.00").unwrap()),
            lineas: Some(vec![linea("Tomate", "2", "10.00", "10")]),
        };
        /* Las líneas suman 22.00 (base 20 + IVA 2); el total explícito 30.00
         * no cuadra → error de validación (F5-3), nunca un descuadre silencioso. */
        let err = construir_total_y_datos(&req).expect_err("total discrepante debe fallar");
        assert!(err.contains("no coincide"), "mensaje accionable: {err}");
    }

    #[test]
    fn construir_total_y_datos_acepta_total_coincidente() {
        let req = CrearBdpPurchaseNoteRequest {
            serie: None,
            numero: None,
            fecha: None,
            codigo_proveedor: None,
            nombre_proveedor: Some("Proveedor".to_string()),
            total: Some(Decimal::from_str("22.00").unwrap()),
            lineas: Some(vec![linea("Tomate", "2", "10.00", "10")]),
        };
        let (total, _datos) = construir_total_y_datos(&req).expect("total coincide con líneas");
        assert_eq!(total, Decimal::from_str("22.00").unwrap());
    }

    #[test]
    fn construir_total_y_datos_sin_lineas_usa_total_o_cero() {
        let con_total = CrearBdpPurchaseNoteRequest {
            serie: None,
            numero: None,
            fecha: None,
            codigo_proveedor: None,
            nombre_proveedor: Some("Proveedor".to_string()),
            total: Some(Decimal::from_str("50.00").unwrap()),
            lineas: None,
        };
        let (total, datos) = construir_total_y_datos(&con_total).expect("sin líneas ok");
        assert_eq!(total, Decimal::from_str("50.00").unwrap());
        assert_eq!(datos, serde_json::json!({}));

        let sin_total = CrearBdpPurchaseNoteRequest {
            total: None,
            ..con_total
        };
        let (total, _) = construir_total_y_datos(&sin_total).expect("sin líneas ok");
        assert_eq!(total, Decimal::ZERO);
    }

    #[test]
    fn desglose_desde_datos_extrae_base_e_iva() {
        let req = CrearBdpPurchaseNoteRequest {
            serie: None,
            numero: None,
            fecha: None,
            codigo_proveedor: None,
            nombre_proveedor: Some("Proveedor".to_string()),
            total: None,
            lineas: Some(vec![linea("Tomate", "2", "10.00", "10")]),
        };
        let (_total, datos) = construir_total_y_datos(&req).expect("desglose válido");
        let (base, iva) =
            BdpPurchaseNoteRepository::desglose_desde_datos(&datos).expect("desglose presente");
        assert_eq!(base, Decimal::from_str("20.00").unwrap());
        assert_eq!(iva, Decimal::from_str("2.00").unwrap());
    }

    #[test]
    fn desglose_desde_datos_devuelve_none_sin_lineas() {
        assert!(BdpPurchaseNoteRepository::desglose_desde_datos(&serde_json::json!({})).is_none());
    }

    #[test]
    fn parse_fecha_bdp_handles_iso_date() {
        assert_eq!(
            parse_fecha_bdp("2021-07-27T00:00:00"),
            chrono::NaiveDate::from_ymd_opt(2021, 7, 27)
        );
    }

    #[test]
    fn parse_fecha_bdp_handles_date_with_spaces() {
        assert_eq!(
            parse_fecha_bdp("2021- 07 - 27T00:00:00"),
            chrono::NaiveDate::from_ymd_opt(2021, 7, 27)
        );
    }

    #[test]
    fn parse_fecha_bdp_returns_none_for_invalid() {
        assert!(parse_fecha_bdp("not-a-date").is_none());
    }
}
