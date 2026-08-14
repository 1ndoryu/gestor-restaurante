/* 253A-5: Repositorio de ventas — queries SQL con parámetros */

use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

use crate::models::{Venta, VentaConCliente};

/// Datos necesarios para insertar una venta en BD
pub struct NuevaVenta<'a> {
    pub user_id: Uuid,
    pub fecha: chrono::NaiveDate,
    pub comensales: Option<i32>,
    pub descripcion: &'a str,
    pub iva_porcentaje: rust_decimal::Decimal,
    pub turno: &'a str,
    pub canal: &'a str,
    pub metodo_pago: &'a str,
    pub importe_base: rust_decimal::Decimal,
    pub importe_iva: rust_decimal::Decimal,
    /* [034A-5] Relaciones opcionales */
    pub reserva_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
}

/* [283A-22] Datos para actualizar parcialmente una venta. */
pub struct ActualizarVentaData<'a> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub fecha: Option<chrono::NaiveDate>,
    pub comensales: Option<i32>,
    pub descripcion: Option<&'a str>,
    pub iva_porcentaje: Option<rust_decimal::Decimal>,
    pub turno: Option<&'a str>,
    pub canal: Option<&'a str>,
    pub metodo_pago: Option<&'a str>,
    pub importe_base: Option<rust_decimal::Decimal>,
    pub importe_iva: Option<rust_decimal::Decimal>,
}

pub struct VentaRepository;

impl VentaRepository {
    pub async fn create(pool: &PgPool, data: &NuevaVenta<'_>) -> Result<Venta, sqlx::Error> {
        Self::create_with(pool, data).await
    }

    pub async fn create_with<'e, E>(
        executor: E,
        data: &NuevaVenta<'_>,
    ) -> Result<Venta, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Venta>(
            "INSERT INTO ventas (id, user_id, fecha, comensales, descripcion, iva_porcentaje, \
             turno, canal, metodo_pago, importe_base, importe_iva, reserva_id, cliente_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
             RETURNING *",
        )
        .bind(id)
        .bind(data.user_id)
        .bind(data.fecha)
        .bind(data.comensales)
        .bind(data.descripcion)
        .bind(data.iva_porcentaje)
        .bind(data.turno)
        .bind(data.canal)
        .bind(data.metodo_pago)
        .bind(data.importe_base)
        .bind(data.importe_iva)
        .bind(data.reserva_id)
        .bind(data.cliente_id)
        .fetch_one(executor)
        .await
    }

    pub async fn find_by_id(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Venta>, sqlx::Error> {
        sqlx::query_as::<_, Venta>("SELECT * FROM ventas WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
    }

    /* [044A-8+9] Whitelist de columnas — previene SQL injection.
     * [064A-3] Añadidos filtros por columna: turno, canal, metodo_pago (multi-valor separado por coma).
     * [064A-12] Filtro estado_haddock (synced/error/pending) con CASE en SQL. */
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        page: i64,
        per_page: i64,
        desde: Option<chrono::NaiveDate>,
        hasta: Option<chrono::NaiveDate>,
        busqueda: Option<&str>,
        turno: Option<&str>,
        canal: Option<&str>,
        metodo_pago: Option<&str>,
        estado_haddock: Option<&str>,
        estado_bdp: Option<&str>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<(Vec<VentaConCliente>, i64), sqlx::Error> {
        let offset = (page - 1) * per_page;

        /* Whitelist de columnas — previene SQL injection */
        let order_col = match sort_by {
            Some("importe_base") => "v.importe_base",
            Some("turno") => "v.turno",
            Some("canal") => "v.canal",
            Some("metodo_pago") => "v.metodo_pago",
            Some("nombre_cliente") => "nombre_cliente",
            _ => "v.fecha",
        };
        let order_dir = if matches!(sort_order, Some("asc")) {
            "ASC"
        } else {
            "DESC"
        };

        let busqueda_pattern = busqueda.filter(|b| !b.is_empty()).map(|b| format!("%{b}%"));

        /* Normalizar filtros vacíos a None */
        let turno_filter = turno.filter(|t| !t.is_empty());
        let canal_filter = canal.filter(|c| !c.is_empty());
        let metodo_filter = metodo_pago.filter(|m| !m.is_empty());
        let haddock_filter = estado_haddock.filter(|h| !h.is_empty());
        let bdp_filter = estado_bdp.filter(|b| !b.is_empty());

        let query_str = format!(
            "SELECT v.id, v.user_id, v.fecha, v.comensales, v.descripcion, \
                    v.iva_porcentaje, v.turno, v.canal, v.metodo_pago, \
                    v.importe_base, v.importe_iva, v.reserva_id, v.cliente_id, \
                    CASE WHEN c.id IS NOT NULL \
                         THEN CONCAT(c.nombre, CASE WHEN c.apellidos != '' THEN CONCAT(' ', c.apellidos) ELSE '' END) \
                         ELSE NULL \
                    END AS nombre_cliente, \
                    v.created_at, v.updated_at, \
                    v.haddock_synced, v.haddock_synced_at, v.haddock_sync_error, \
                    v.bdp_synced, v.bdp_synced_at, v.bdp_sync_error, v.bdp_order_id, \
                    v.bdp_order_status, v.bdp_invoiced, \
                    v.anulada, v.anulada_at, v.anulacion_motivo, v.anulacion_usuario, \
                    v.facturada_local, v.factura_numero, v.factura_fecha \
             FROM ventas v \
             LEFT JOIN clientes c ON c.id = v.cliente_id \
             WHERE v.user_id = $1 \
             AND ($4::DATE IS NULL OR v.fecha >= $4) \
             AND ($5::DATE IS NULL OR v.fecha <= $5) \
             AND ($6::TEXT IS NULL \
                  OR v.descripcion ILIKE $6 \
                  OR v.turno ILIKE $6 \
                  OR v.canal ILIKE $6 \
                  OR c.nombre ILIKE $6 \
                  OR c.apellidos ILIKE $6 \
                  OR CONCAT(c.nombre, ' ', c.apellidos) ILIKE $6) \
             AND ($7::TEXT IS NULL OR v.turno = ANY(string_to_array($7, ','))) \
             AND ($8::TEXT IS NULL OR v.canal = ANY(string_to_array($8, ','))) \
             AND ($9::TEXT IS NULL OR v.metodo_pago = ANY(string_to_array($9, ','))) \
             AND ($10::TEXT IS NULL OR \
                  (CASE \
                     WHEN v.haddock_synced = true THEN 'synced' \
                     WHEN v.haddock_sync_error IS NOT NULL THEN 'error' \
                     ELSE 'pending' \
                   END) = ANY(string_to_array($10, ','))) \
             AND ($11::TEXT IS NULL OR \
                  (CASE \
                     WHEN v.bdp_order_status IS NOT NULL THEN v.bdp_order_status \
                     WHEN v.bdp_synced = true THEN 'synced' \
                     WHEN v.bdp_sync_error IS NOT NULL THEN 'error' \
                     ELSE 'pending' \
                   END) = ANY(string_to_array($11, ','))) \
             ORDER BY {order_col} {order_dir}, v.created_at DESC \
             LIMIT $2 OFFSET $3"
        );

        let items = sqlx::query_as::<_, VentaConCliente>(&query_str)
            .bind(user_id)
            .bind(per_page)
            .bind(offset)
            .bind(desde)
            .bind(hasta)
            .bind(busqueda_pattern.as_deref())
            .bind(turno_filter)
            .bind(canal_filter)
            .bind(metodo_filter)
            .bind(haddock_filter)
            .bind(bdp_filter)
            .fetch_all(pool)
            .await?;

        /* COUNT con los mismos filtros */
        let has_text_filter = busqueda_pattern.is_some();
        let has_column_filters = turno_filter.is_some()
            || canal_filter.is_some()
            || metodo_filter.is_some()
            || haddock_filter.is_some()
            || bdp_filter.is_some();

        let count = if has_text_filter || has_column_filters {
            let rec = sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COUNT(*) FROM ventas v \
                 LEFT JOIN clientes c ON c.id = v.cliente_id \
                 WHERE v.user_id = $1 \
                 AND ($2::DATE IS NULL OR v.fecha >= $2) \
                 AND ($3::DATE IS NULL OR v.fecha <= $3) \
                 AND ($4::TEXT IS NULL \
                      OR v.descripcion ILIKE $4 \
                      OR v.turno ILIKE $4 \
                      OR v.canal ILIKE $4 \
                      OR c.nombre ILIKE $4 \
                      OR c.apellidos ILIKE $4 \
                      OR CONCAT(c.nombre, ' ', c.apellidos) ILIKE $4) \
                 AND ($5::TEXT IS NULL OR v.turno = ANY(string_to_array($5, ','))) \
                 AND ($6::TEXT IS NULL OR v.canal = ANY(string_to_array($6, ','))) \
                 AND ($7::TEXT IS NULL OR v.metodo_pago = ANY(string_to_array($7, ','))) \
                 AND ($8::TEXT IS NULL OR \
                      (CASE \
                         WHEN v.haddock_synced = true THEN 'synced' \
                         WHEN v.haddock_sync_error IS NOT NULL THEN 'error' \
                         ELSE 'pending' \
                       END) = ANY(string_to_array($8, ','))) \
                 AND ($9::TEXT IS NULL OR \
                      (CASE \
                         WHEN v.bdp_order_status IS NOT NULL THEN v.bdp_order_status \
                         WHEN v.bdp_synced = true THEN 'synced' \
                         WHEN v.bdp_sync_error IS NOT NULL THEN 'error' \
                         ELSE 'pending' \
                       END) = ANY(string_to_array($9, ',')))",
            )
            .bind(user_id)
            .bind(desde)
            .bind(hasta)
            .bind(busqueda_pattern.as_deref())
            .bind(turno_filter)
            .bind(canal_filter)
            .bind(metodo_filter)
            .bind(haddock_filter)
            .bind(bdp_filter)
            .fetch_one(pool)
            .await?;
            rec.unwrap_or(0)
        } else {
            let rec = sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COUNT(*) FROM ventas WHERE user_id = $1 \
                 AND ($2::DATE IS NULL OR fecha >= $2) \
                 AND ($3::DATE IS NULL OR fecha <= $3)",
            )
            .bind(user_id)
            .bind(desde)
            .bind(hasta)
            .fetch_one(pool)
            .await?;
            rec.unwrap_or(0)
        };

        Ok((items, count))
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        /* [128A-1/F4/D5] Las ventas anuladas nunca se borran físicamente:
         * son histórico con motivo. Guard en el DELETE para impedirlo. */
        /* [128A-1/F4] query() dinámico (no macro): la cache offline .sqlx/
         * no contiene las columnas nuevas de la migración F4. */
        let result =
            sqlx::query("DELETE FROM ventas WHERE id = $1 AND user_id = $2 AND anulada = false")
                .bind(id)
                .bind(user_id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /* [094A-1] Buscar venta asociada a una reserva — para evitar duplicados.
     * Retorna true si ya existe al menos una venta con este reserva_id. */
    pub async fn exists_by_reserva_id(
        pool: &PgPool,
        reserva_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let rec = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM ventas WHERE reserva_id = $1)",
        )
        .bind(reserva_id)
        .fetch_one(pool)
        .await?;
        Ok(rec)
    }

    /* [094A-1] Eliminar ventas asociadas a una reserva al descompletar.
     * Solo elimina ventas auto-generadas (importe_base = 0) para no perder datos manuales. */
    pub async fn delete_by_reserva_id(
        pool: &PgPool,
        reserva_id: Uuid,
        user_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM ventas WHERE reserva_id = $1 AND user_id = $2 AND importe_base = 0",
        )
        .bind(reserva_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /* [283A-22] Actualizar parcialmente una venta — COALESCE mantiene valores existentes
     * cuando el campo no se envía (None).
     * [014A-11] Convertido a query_as! para verificación SQL en compilación. */
    pub async fn update(
        pool: &PgPool,
        data: &ActualizarVentaData<'_>,
    ) -> Result<Option<Venta>, sqlx::Error> {
        Self::update_with(pool, data).await
    }

    pub async fn update_with<'e, E>(
        executor: E,
        data: &ActualizarVentaData<'_>,
    ) -> Result<Option<Venta>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Venta>(
            "UPDATE ventas SET \
             fecha = COALESCE($3, fecha), \
             comensales = COALESCE($4, comensales), \
             descripcion = COALESCE($5, descripcion), \
             iva_porcentaje = COALESCE($6, iva_porcentaje), \
             turno = COALESCE($7, turno), \
             canal = COALESCE($8, canal), \
             metodo_pago = COALESCE($9, metodo_pago), \
             importe_base = COALESCE($10, importe_base), \
             importe_iva = COALESCE($11, importe_iva), \
             updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 \
             RETURNING *",
        )
        .bind(data.id)
        .bind(data.user_id)
        .bind(data.fecha)
        .bind(data.comensales)
        .bind(data.descripcion)
        .bind(data.iva_porcentaje)
        .bind(data.turno)
        .bind(data.canal)
        .bind(data.metodo_pago)
        .bind(data.importe_base)
        .bind(data.importe_iva)
        .fetch_optional(executor)
        .await
    }

    /// Suma de `importe_base` de ventas en un rango de fechas
    pub async fn total_periodo(
        pool: &PgPool,
        user_id: Uuid,
        desde: chrono::NaiveDate,
        hasta: chrono::NaiveDate,
        excluir_anuladas: bool,
    ) -> Result<rust_decimal::Decimal, sqlx::Error> {
        /* [128A-1/F4][F4-4] M10 parametrizado por modalidad: en
         * `credito_completo` el resumen excluye las anuladas (reversión
         * idempotente del IVA); en `estado_solo` NO hay reversión contable,
         * así que las anuladas siguen contando. El servicio (dashboard)
         * decide según `config.anulacion_modalidad`. */
        let excluye_anuladas_sql = if excluir_anuladas {
            " AND anulada = false"
        } else {
            ""
        };
        let query = format!(
            "SELECT COALESCE(SUM(importe_base), 0) as total FROM ventas \
             WHERE user_id = $1 AND fecha >= $2 AND fecha <= $3{excluye_anuladas_sql}"
        );
        let rec = sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(&query)
            .bind(user_id)
            .bind(desde)
            .bind(hasta)
            .fetch_one(pool)
            .await?;
        Ok(rec.unwrap_or_default())
    }

    /* [064A-6] Actualiza el estado de sincronización Haddock de una venta.
     * Llamado por HaddockService después de cada intento de sync. */
    pub async fn update_haddock_status(
        pool: &PgPool,
        id: Uuid,
        synced: bool,
        error_msg: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE ventas SET \
             haddock_synced = $2, \
             haddock_synced_at = CASE WHEN $2 THEN NOW() ELSE haddock_synced_at END, \
             haddock_sync_error = $3 \
             WHERE id = $1",
            id,
            synced,
            error_msg
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [065A-5] Actualiza el estado de sincronización BDP de una venta.
     * Patrón idéntico a update_haddock_status pero usando sqlx::query() sin macro
     * para evitar necesitar cargo sqlx prepare (nuevas columnas no están en cache). */
    pub async fn update_bdp_status(
        pool: &PgPool,
        id: Uuid,
        synced: bool,
        error_msg: Option<&str>,
        order_id: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE ventas SET bdp_synced = $2, bdp_synced_at = CASE WHEN $2 THEN NOW() ELSE bdp_synced_at END, bdp_sync_error = $3, bdp_order_id = CASE WHEN $2 THEN $4 ELSE bdp_order_id END WHERE id = $1",
        )
        .bind(id)
        .bind(synced)
        .bind(error_msg)
        .bind(order_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [276A-4.2] Actualiza bdp_order_status de una venta — polling/endpoint manual.
     * [F8.4] Si el status es "invoiced", también marca bdp_invoiced = true. */
    pub async fn update_bdp_order_status(
        pool: &PgPool,
        id: Uuid,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        let invoiced = status == "invoiced";
        sqlx::query(
            "UPDATE ventas SET bdp_order_status = $2, bdp_invoiced = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .bind(invoiced)
        .execute(pool)
        .await?;
        Ok(())
    }

    /* [276A-4.2] Ventas pendientes de polling BDP:
     * bdp_synced=true, bdp_order_status no final ('invoiced' ni 'error'). */
    pub async fn list_bdp_pending(pool: &PgPool, user_id: Uuid) -> Result<Vec<Venta>, sqlx::Error> {
        sqlx::query_as::<_, Venta>(
            "SELECT * FROM ventas \
             WHERE user_id = $1 \
               AND bdp_synced = TRUE \
               AND anulada = FALSE \
               AND (bdp_order_status IS NULL OR bdp_order_status NOT IN ('invoiced', 'cancelled', 'error')) \
             ORDER BY created_at ASC LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /* [AUDIT-2.11b] Ventas huérfanas: la comanda puede existir en BDP pero
     * Glory no recibió confirmación (proceso murió entre HTTP y UPDATE local).
     * Estas ventas tienen bdp_synced=false + bdp_order_id no nulo + auditoría
     * 'pendiente' o 'ambiguo' para la operación 'create_order'.
     * El polling normal NO las detecta porque filtra bdp_synced=TRUE. */
    pub async fn list_bdp_orphaned(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<Venta>, sqlx::Error> {
        sqlx::query_as::<_, Venta>(
            "SELECT v.* FROM ventas v \
             WHERE v.user_id = $1 \
               AND v.bdp_synced = FALSE \
               AND v.bdp_order_id IS NOT NULL \
               AND EXISTS ( \
                 SELECT 1 FROM bdp_audit_log a \
                 WHERE a.user_id = $1 \
                   AND a.target_entity_type = 'venta' \
                   AND a.target_entity_id = v.id \
                   AND a.operacion = 'create_order' \
                   AND a.resultado IN ('pendiente', 'ambiguo') \
               ) \
             ORDER BY v.created_at ASC LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /* [AUDIT-N2] Clientes BDP huérfanos: tienen bdp_synced=true pero auditoría
     * pendiente/ambiguo para 'create_customer'. El proceso murió entre
     * update_bdp_sync y actualizar_resultado (antes del fix N1).
     * El polling cierra la auditoría consultando ExportCustomers en BDP. */
    pub async fn list_bdp_orphaned_customers(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::Cliente>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::Cliente>(
            "SELECT c.* FROM clientes c \
             WHERE c.user_id = $1 \
               AND c.bdp_synced = TRUE \
               AND c.bdp_customer_code IS NOT NULL \
               AND EXISTS ( \
                 SELECT 1 FROM bdp_audit_log a \
                 WHERE a.user_id = $1 \
                   AND a.target_entity_type = 'cliente' \
                   AND a.target_entity_id = c.id \
                   AND a.operacion = 'create_customer' \
                   AND a.resultado IN ('pendiente', 'ambiguo') \
               ) \
             ORDER BY c.created_at ASC LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /* [128A-1/F4] Anulación local de ventas (D4, M9-M11).
     *
     * Transición de estado única: solo `anulada=false` puede pasar a
     * `anulada=true` (guard M10). Bloquea ventas facturadas (M9) y ventas
     * ya anuladas (idempotencia por guard, no por UPDATE: si ya está
     * anulada devuelve la fila sin modificarla y `resultado_previo` indica
     * el resultado de la auditoría previa).
     *
     * Auditoría obligatoria (C1): INSERT en `bdp_audit_log` con
     * `operacion='anular_venta'`, `direccion='internal'`, `resultado='exito'`,
     * `target_entity_type='venta'`, `target_entity_id`, `authorization_reason`
     * (motivo) e `idempotency_key`. Con `ON CONFLICT (user_id, idempotency_key)
     * WHERE idempotency_key IS NOT NULL DO NOTHING` se logra idempotencia
     * (doble click seguro): si la clave ya se usó, no se vuelve a anular ni a
     * auditar; se devuelve el resultado previo para que el handler decida.
     *
     * Retorna (Venta, audit_id, resultado_previo, ya_anulada).
     */
    pub async fn anular(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        motivo: Option<&str>,
        anulacion_usuario: Option<Uuid>,
        idempotency_key: Option<&str>,
    ) -> Result<(Venta, Uuid, Option<String>, bool), sqlx::Error> {
        let mut tx = pool.begin().await?;

        let venta_actual: Option<Venta> =
            sqlx::query_as::<_, Venta>("SELECT * FROM ventas WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        let Some(venta) = venta_actual else {
            return Err(sqlx::Error::RowNotFound);
        };

        /* M9: solo ventas no facturadas (ni en BDP ni factura local F6). */
        if venta.facturada_local
            || venta.bdp_invoiced
            || venta.bdp_order_status.as_deref() == Some("invoiced")
        {
            return Err(sqlx::Error::Protocol(
                "venta_facturada_no_anulable".to_string(),
            ));
        }

        let audit_payload = serde_json::json!({
            "venta_id": id,
            "motivo": motivo,
            "importe_base": venta.importe_base,
            "importe_iva": venta.importe_iva,
            "bdp_synced": venta.bdp_synced,
            "bdp_order_id": venta.bdp_order_id,
        });

        /* Idempotencia C1: si la clave ya existe, no se anula dos veces. */
        let maybe_audit_id: Option<Uuid> = sqlx::query_scalar(
            r"INSERT INTO bdp_audit_log
               (user_id, operacion, direccion, datos_enviados, resultado, origen_operacion,
                target_entity_type, target_entity_id, authorization_reason, idempotency_key)
               VALUES ($1, 'anular_venta', 'internal', $2, 'exito', 'local', 'venta', $3, $4, $5)
               ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING
               RETURNING id",
        )
        .bind(user_id)
        .bind(audit_payload)
        .bind(id)
        .bind(format!(
            "Anulación local de venta {} ({}) — operación interna, no requiere autorización BDP",
            id,
            motivo.unwrap_or("sin motivo")
        ))
        .bind(idempotency_key)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(audit_id) = maybe_audit_id else {
            /* [128A-1/F4][F4-5] La clave de idempotencia está scoped por venta:
             * si la fila previa apunta a OTRA venta, no es un reintento de esta
             * anulación sino una clave reutilizada → conflicto, nunca éxito
             * idempotente falso. */
            let (existing_id, resultado, target_entity_id): (Uuid, String, Uuid) = sqlx::query_as(
                "SELECT id, resultado, target_entity_id \
                 FROM bdp_audit_log \
                 WHERE user_id = $1 AND idempotency_key = $2",
            )
            .bind(user_id)
            .bind(idempotency_key.unwrap_or_default())
            .fetch_one(&mut *tx)
            .await?;
            if target_entity_id != id {
                tx.rollback().await?;
                return Err(sqlx::Error::Protocol(
                    "idempotency_key_otra_venta".to_string(),
                ));
            }
            /* Si ya estaba anulada, devolver la fila actual (estado consistente). */
            let venta_actual =
                sqlx::query_as::<_, Venta>("SELECT * FROM ventas WHERE id = $1 AND user_id = $2")
                    .bind(id)
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?;
            tx.commit().await?;
            let ya_anulada = venta_actual.anulada;
            return Ok((venta_actual, existing_id, Some(resultado), ya_anulada));
        };

        /* Transición única con guard: solo si aún no está anulada. */
        let venta_anulada: Option<Venta> = sqlx::query_as::<_, Venta>(
            "UPDATE ventas SET anulada = true, anulada_at = NOW(), \
             anulacion_motivo = COALESCE($3, anulacion_motivo), \
             anulacion_usuario = COALESCE($4, anulacion_usuario), \
             updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 AND anulada = false \
             RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(motivo)
        .bind(anulacion_usuario)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(venta_anulada) = venta_anulada else {
            /* Ya estaba anulada (carrera): la auditoría de esta clave se
             * insertó recién pero el guard impidió el UPDATE; revertir la
             * auditoría para no inventar un éxito sin efecto. */
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("venta_ya_anulada".to_string()));
        };

        tx.commit().await?;

        Ok((venta_anulada, audit_id, None, false))
    }

    /* [128A-1/F6] Factura local mínima (A7/D9).
     *
     * Emite una factura local: estado `facturada_local=true` (final, transición
     * única), numeración local secuencial `F-{año}-{n:04}` por usuario y
     * auditoría obligatoria con `origen_operacion='local'`. El índice parcial
     * UNIQUE(user_id, factura_numero) protege la numeración en carreras; el
     * servicio reintenta si hay colisión de número (unique violation 23505).
     *
     * Guards (M9): no se factura una venta anulada, ya facturada (local o BDP),
     * ni con pagos parciales pendientes en el ledger (si el ledger tiene filas,
     * deben cubrir el total; una venta sin pagos parciales se considera pagada
     * por `metodo_pago`, diseño A6: pago completo = venta con metodo_pago).
     *
     * Idempotencia C1: si la `idempotency_key` ya se usó, devuelve la fila
     * actual + resultado previo para que el handler decida (éxito idempotente
     * o conflicto).
     *
     * Retorna (Venta, audit_id, resultado_previo, ya_facturada).
     */
    #[allow(clippy::too_many_lines)]
    pub async fn facturar_local(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        idempotency_key: Option<&str>,
    ) -> Result<(Venta, Uuid, Option<String>, bool), sqlx::Error> {
        let mut tx = pool.begin().await?;

        /* Clave de idempotencia: `None`/`""` se normaliza a una clave única
         * generada. La auditoría usa (user_id, idempotency_key) como conflicto;
         * una clave "" reutilizada en otra venta haría que la segunda llamada
         * devolviera la auditoría de la primera sin facturar (éxito falso). */
        let key = idempotency_key.filter(|k| !k.is_empty()).map_or_else(
            || format!("factura-local-{id}-{}", Uuid::new_v4()),
            str::to_string,
        );

        let venta_actual: Option<Venta> = sqlx::query_as::<_, Venta>(
            "SELECT * FROM ventas WHERE id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(venta) = venta_actual else {
            return Err(sqlx::Error::RowNotFound);
        };

        /* M9: no facturar anuladas ni doble facturación (local o BDP). */
        if venta.anulada {
            return Err(sqlx::Error::Protocol(
                "venta_anulada_no_facturable".to_string(),
            ));
        }
        if venta.facturada_local
            || venta.bdp_invoiced
            || venta.bdp_order_status.as_deref() == Some("invoiced")
        {
            return Err(sqlx::Error::Protocol("venta_ya_facturada".to_string()));
        }

        /* Si el ledger de pagos parciales tiene filas, deben cubrir el total. */
        let (tiene_pagos, pagado): (bool, rust_decimal::Decimal) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM bdp_pagos WHERE venta_id = $1), \
                    COALESCE((SELECT SUM(amount) FROM bdp_pagos \
                              WHERE venta_id = $1 AND resultado = 'exito'), 0)",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let total = venta.importe_base + venta.importe_iva;
        if tiene_pagos && (total - pagado) > rust_decimal::Decimal::new(1, 3) {
            return Err(sqlx::Error::Protocol(
                "venta_con_pagos_pendientes".to_string(),
            ));
        }

        /* Numeración local secuencial por usuario: F-{año}-{n:04}. */
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ventas WHERE user_id = $1 AND factura_numero IS NOT NULL",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        let anio = chrono::Utc::now().format("%Y");
        let numero = format!("F-{anio}-{:04}", count + 1);

        let audit_payload = serde_json::json!({
            "venta_id": id,
            "factura_numero": numero,
            "importe_base": venta.importe_base,
            "importe_iva": venta.importe_iva,
            "total": total,
            "bdp_synced": venta.bdp_synced,
            "bdp_order_id": venta.bdp_order_id,
        });

        let maybe_audit_id: Option<Uuid> = sqlx::query_scalar(
            r"INSERT INTO bdp_audit_log
               (user_id, operacion, direccion, datos_enviados, resultado, origen_operacion,
                target_entity_type, target_entity_id, authorization_reason, idempotency_key)
               VALUES ($1, 'factura_local', 'internal', $2, 'exito', 'local', 'venta', $3, $4, $5)
               ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING
               RETURNING id",
        )
        .bind(user_id)
        .bind(audit_payload)
        .bind(id)
        .bind(format!(
            "Factura local {numero} de la venta {id} — operación interna, no requiere autorización BDP"
        ))
        .bind(&key)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(audit_id) = maybe_audit_id else {
            let (existing_id, resultado): (Uuid, String) = sqlx::query_as(
                "SELECT id, resultado FROM bdp_audit_log WHERE user_id = $1 AND idempotency_key = $2",
            )
            .bind(user_id)
            .bind(&key)
            .fetch_one(&mut *tx)
            .await?;
            let venta_actual =
                sqlx::query_as::<_, Venta>("SELECT * FROM ventas WHERE id = $1 AND user_id = $2")
                    .bind(id)
                    .bind(user_id)
                    .fetch_one(&mut *tx)
                    .await?;
            tx.commit().await?;
            let ya_facturada = venta_actual.facturada_local;
            return Ok((venta_actual, existing_id, Some(resultado), ya_facturada));
        };

        let venta_facturada: Option<Venta> = sqlx::query_as::<_, Venta>(
            "UPDATE ventas SET facturada_local = true, factura_numero = $3, \
             factura_fecha = NOW(), updated_at = NOW() \
             WHERE id = $1 AND user_id = $2 AND facturada_local = false \
             RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(&numero)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(venta_facturada) = venta_facturada else {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("venta_ya_facturada".to_string()));
        };

        tx.commit().await?;

        Ok((venta_facturada, audit_id, None, false))
    }
}
