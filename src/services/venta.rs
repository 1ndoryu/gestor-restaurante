/* 253A-5: Servicio de ventas — lógica de negocio
 * [064A-5] Añadido hook post-create/update para sincronización con Haddock POS API
 * [065A-5] Añadido hook post-create/update para sincronización con BDP WebLink */

use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    ActualizarVentaRequest, AnularVentaRequest, BdpPago, CrearVentaRequest, Venta, VentasPaginadas,
};
use crate::repositories::venta::{ActualizarVentaData, NuevaVenta};
use crate::repositories::{
    BdpPagoRepository, ConfiguracionRepository, VentaLineaRepository, VentaRepository,
};

use super::{BdpSyncService, HaddockService};

pub struct VentaService;

impl VentaService {
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        req: CrearVentaRequest,
    ) -> Result<Venta, AppError> {
        let turno = serde_json::to_value(&req.turno)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "manana".into());
        let canal = serde_json::to_value(&req.canal)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "comedor".into());
        let metodo = serde_json::to_value(&req.metodo_pago)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "efectivo".into());

        let descripcion = req.descripcion.as_deref().unwrap_or("");
        let data = NuevaVenta {
            user_id,
            fecha: req.fecha,
            comensales: req.comensales,
            descripcion,
            iva_porcentaje: req.iva_porcentaje,
            turno: &turno,
            canal: &canal,
            metodo_pago: &metodo,
            importe_base: req.importe_base,
            importe_iva: req.importe_iva,
            /* [034A-5] Ventas manuales no tienen reserva ni cliente asociado */
            reserva_id: None,
            cliente_id: None,
        };

        let mut tx = pool.begin().await?;
        let venta = VentaRepository::create_with(&mut *tx, &data).await?;

        /* Venta y líneas son un único agregado: no se permite conservar una
         * cabecera sin sus líneas ni caer silenciosamente al artículo genérico. */
        if let Some(ref lineas) = req.lineas {
            VentaLineaRepository::crear_batch_conn(&mut tx, venta.id, lineas).await?;
        }
        tx.commit().await?;

        /* [064A-5] Sincronizar con Haddock en background (no bloquea la respuesta) */
        Self::spawn_haddock_sync(pool.clone(), user_id, venta.clone(), false);
        /* [065A-5] Sincronizar con BDP en background */
        Self::spawn_bdp_sync(pool.clone(), user_id, venta.clone(), false);

        Ok(venta)
    }

    pub async fn get(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Venta, AppError> {
        VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        page: i64,
        per_page: i64,
        desde: Option<chrono::NaiveDate>,
        hasta: Option<chrono::NaiveDate>,
        busqueda: Option<String>,
        turno: Option<String>,
        canal: Option<String>,
        metodo_pago: Option<String>,
        estado_haddock: Option<String>,
        estado_bdp: Option<String>,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<VentasPaginadas, AppError> {
        let (items, total) = VentaRepository::list(
            pool,
            user_id,
            page,
            per_page,
            desde,
            hasta,
            busqueda.as_deref(),
            turno.as_deref(),
            canal.as_deref(),
            metodo_pago.as_deref(),
            estado_haddock.as_deref(),
            estado_bdp.as_deref(),
            sort_by.as_deref(),
            sort_order.as_deref(),
        )
        .await?;
        Ok(VentasPaginadas {
            items,
            total,
            page,
            per_page,
        })
    }

    /* [283A-22] Actualizar parcialmente una venta.
     * Convierte enums a string igual que en create para mantener consistencia. */
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: ActualizarVentaRequest,
    ) -> Result<Venta, AppError> {
        let actual = VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;
        if actual.bdp_synced {
            return Err(AppError::Conflict(
                "La venta ya fue creada en BDP. Su edición está bloqueada porque WebLink no ofrece una actualización idempotente confirmada; concilie la comanda antes de modificarla."
                    .into(),
            ));
        }

        let turno = req.turno.as_ref().and_then(|t| {
            serde_json::to_value(t)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
        });
        let canal = req.canal.as_ref().and_then(|c| {
            serde_json::to_value(c)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
        });
        let metodo = req.metodo_pago.as_ref().and_then(|m| {
            serde_json::to_value(m)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
        });

        let data = ActualizarVentaData {
            id,
            user_id,
            fecha: req.fecha,
            comensales: req.comensales,
            descripcion: req.descripcion.as_deref(),
            iva_porcentaje: req.iva_porcentaje,
            turno: turno.as_deref(),
            canal: canal.as_deref(),
            metodo_pago: metodo.as_deref(),
            importe_base: req.importe_base,
            importe_iva: req.importe_iva,
        };

        let mut tx = pool.begin().await?;
        let venta = VentaRepository::update_with(&mut *tx, &data)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

        if let Some(ref lineas) = req.lineas {
            VentaLineaRepository::reemplazar_conn(&mut tx, venta.id, lineas).await?;
        }
        tx.commit().await?;

        /* [064A-5] Re-sincronizar con Haddock tras actualización */
        Self::spawn_haddock_sync(pool.clone(), user_id, venta.clone(), true);
        /* Solo ventas aún no sincronizadas pueden llegar aquí; el reintento
         * conserva el mismo identificador estable y no intenta "actualizar"
         * una comanda remota ya creada. */
        Self::spawn_bdp_sync(pool.clone(), user_id, venta.clone(), false);

        Ok(venta)
    }

    /* [064A-5] Lanza sincronización con Haddock en un task independiente.
     * [064A-6] Ahora pasa pool para actualizar estado sync en BD.
     * [064A-7] is_update distingue create/update para prevención de duplicados.
     * No bloquea ni falla la operación principal. */
    fn spawn_haddock_sync(pool: PgPool, user_id: Uuid, venta: Venta, is_update: bool) {
        tokio::spawn(async move {
            let config = match ConfiguracionRepository::obtener_o_crear(&pool, user_id).await {
                Ok(c) => c,
                Err(e) => {
                    warn!("[064A-5] Error obteniendo config para Haddock sync: {e}");
                    return;
                }
            };
            HaddockService::sync_order(&pool, &venta, &config, is_update).await;
        });
    }

    /* [064A-8] Eliminar venta — bloqueado si sincronización Haddock o BDP está activa.
     * Haddock no tiene endpoint DELETE; BDP puede cancelar pero no confiablemente.
     * El cliente pide explícitamente bloquear.
     * [128A-1/F4/D5=A] Desbloqueo: solo ventas NO sincronizadas con BDP y NO
     * anuladas. Si Haddock sigue activo, el bloqueo permanece (M14). Si está
     * sincronizada y BDP no responde, el DELETE falla con 409 y mensaje
     * accionable. Las anuladas nunca se borran (histórico con motivo). */
    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let config = ConfiguracionRepository::obtener_o_crear(pool, user_id).await?;
        if config.haddock_sync_enabled {
            return Err(AppError::Conflict(
                "No se pueden eliminar ventas mientras la sincronización con Haddock está activa. \
                 Desactívela primero en Configuración."
                    .into(),
            ));
        }
        if config.bdp_sync_enabled {
            return Err(AppError::Conflict(
                "No se pueden eliminar ventas mientras la sincronización con BDP está activa. \
                 Desactívela primero en Configuración."
                    .into(),
            ));
        }

        let venta = VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;
        if venta.anulada {
            return Err(AppError::Conflict(
                "Las ventas anuladas nunca se eliminan: quedan como histórico con motivo (D5)."
                    .into(),
            ));
        }
        if venta.bdp_synced || venta.bdp_order_id.is_some() {
            return Err(AppError::Conflict(
                "La venta ya fue sincronizada con BDP. Para eliminarla, concilie primero la \
                 comanda (estado final en BDP) o anúlela localmente si aún no está facturada."
                    .into(),
            ));
        }
        if !VentaRepository::delete(pool, id, user_id).await? {
            return Err(AppError::NotFound(
                "Venta no encontrada tras verificación".into(),
            ));
        }
        Ok(())
    }

    /* [128A-1/F4] Anulación local de ventas (D4, M9-M11).
     *
     * Modalidades:
     *   - `credito_completo` (default): motivo obligatorio + reversión de IVA
     *     idempotente (exclusión del resumen diario en total_periodo) (M10).
     *   - `estado_solo`: solo marca el estado anulada.
     *
     * Reglas:
     *   - M9: solo ventas NO facturadas (bdp_invoiced=false y status != invoiced).
     *   - C3=b: NUNCA se llama CancelOrder; el estado "pendiente de anular en
     *     BDP" se deriva (anulada=true AND bdp_synced=true AND status no final)
     *     y el poller lo excluye (M8).
     *   - M11: liberar mesa solo si la venta es la ocupante actual. La ocupación
     *     se deriva de reservas (venta -> reserva_id -> mesa), así que si la
     *     venta tiene reserva vinculada se evalúa la liberación; si no hay
     *     vínculo de ocupación, no se toca el plano (aviso en auditoría).
     *   - Idempotencia C1: doble click seguro vía idempotency_key.
     */
    pub async fn anular(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: AnularVentaRequest,
    ) -> Result<Venta, AppError> {
        let config = ConfiguracionRepository::obtener_o_crear(pool, user_id).await?;
        let modalidad = config.anulacion_modalidad.clone();
        if modalidad != "credito_completo" && modalidad != "estado_solo" {
            return Err(AppError::Internal(format!(
                "anulacion_modalidad inválida en BD: '{modalidad}'"
            )));
        }

        /* D4/credito_completo: motivo obligatorio (M10). */
        let motivo = req.motivo.as_deref().map(str::trim);
        if modalidad == "credito_completo" && motivo.is_none_or(str::is_empty) {
            return Err(AppError::Validation(
                "En modalidad crédito completo el motivo de anulación es obligatorio.".into(),
            ));
        }

        let (venta, _audit_id, resultado_previo, _ya_anulada) = VentaRepository::anular(
            pool,
            id,
            user_id,
            motivo,
            req.anulacion_usuario,
            req.idempotency_key.as_deref(),
        )
        .await
        .map_err(Self::map_anular_error)?;

        /* Idempotencia C1: reintento con la misma clave y resultado previo
         * 'exito' es éxito idempotente; cualquier otro resultado es conflicto. */
        if let Some(resultado) = resultado_previo {
            if resultado != "exito" {
                return Err(AppError::Conflict(format!(
                    "idempotency_key ya usada (resultado previo: {resultado})"
                )));
            }
        }

        Ok(venta)
    }

    /* [128A-1/F6] Pago parcial local (A8/M13).
     * Escribe sobre el ledger existente `bdp_pagos` (fila local sin
     * `bdp_order_id`). El repo aplica guards (anulada/facturada), saldo
     * pendiente e idempotencia dentro de una transacción con lock de venta.
     * Un reintento con la misma clave es éxito idempotente solo si la fila
     * previa pertenece a la misma venta con el mismo importe. */
    pub async fn pago_parcial_local(
        pool: &PgPool,
        user_id: Uuid,
        venta_id: Uuid,
        amount: rust_decimal::Decimal,
        tender_id: i32,
        idempotency_key: Option<&str>,
    ) -> Result<(BdpPago, Option<Uuid>), AppError> {
        let key = idempotency_key.unwrap_or_default();
        let (pago, audit_id) =
            BdpPagoRepository::insertar_local(pool, user_id, venta_id, amount, tender_id, key)
                .await?;

        if audit_id.is_none() {
            /* Reintento idempotente: la clave ya se usó. */
            if pago.venta_id != venta_id {
                return Err(AppError::Conflict(
                    "idempotency_key ya usada para otra venta".into(),
                ));
            }
            if pago.amount != amount {
                return Err(AppError::Conflict(
                    "idempotency_key ya usada con otro importe".into(),
                ));
            }
        }

        Ok((pago, audit_id))
    }

    /* [128A-1/F6] Factura local mínima (A7/D9): numeración local secuencial +
     * estado `facturada` + auditoría. Reintenta ante colisión de número
     * (carrera concurrente, unique violation 23505) y mapea los guards M9 a
     * códigos HTTP adecuados. */
    pub async fn facturar_local(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        idempotency_key: Option<&str>,
    ) -> Result<Venta, AppError> {
        let mut intentos = 0;
        loop {
            intentos += 1;
            match VentaRepository::facturar_local(pool, id, user_id, idempotency_key).await {
                Ok((venta, _audit_id, resultado_previo, _ya_facturada)) => {
                    if let Some(resultado) = resultado_previo {
                        if resultado != "exito" {
                            return Err(AppError::Conflict(format!(
                                "idempotency_key ya usada (resultado previo: {resultado})"
                            )));
                        }
                    }
                    return Ok(venta);
                }
                Err(sqlx::Error::Database(ref db)) if db.is_unique_violation() && intentos < 3 => {
                    /* Carrera de numeración: reintenta con el siguiente número. */
                }
                Err(e) => return Err(Self::map_factura_local_error(e)),
            }
        }
    }

    fn map_anular_error(err: sqlx::Error) -> AppError {
        match &err {
            sqlx::Error::RowNotFound => AppError::NotFound("Venta no encontrada".into()),
            sqlx::Error::Protocol(msg) => match msg.as_str() {
                "venta_facturada_no_anulable" => {
                    AppError::Conflict("La venta está facturada y no se puede anular.".into())
                }
                "venta_ya_anulada" => AppError::Conflict("La venta ya está anulada.".into()),
                _ => AppError::Conflict(msg.clone()),
            },
            _ => AppError::Database(err),
        }
    }

    fn map_factura_local_error(err: sqlx::Error) -> AppError {
        match &err {
            sqlx::Error::RowNotFound => AppError::NotFound("Venta no encontrada".into()),
            sqlx::Error::Protocol(msg) => match msg.as_str() {
                "venta_anulada_no_facturable" => {
                    AppError::Conflict("La venta está anulada y no se puede facturar.".into())
                }
                "venta_ya_facturada" => AppError::Conflict("La venta ya está facturada.".into()),
                "venta_con_pagos_pendientes" => AppError::Validation(
                    "Quedan pagos parciales pendientes; regístralos antes de facturar.".into(),
                ),
                _ => AppError::Conflict(msg.clone()),
            },
            _ => AppError::Database(err),
        }
    }

    /* [064A-10] Retry manual de sincronización Haddock.
     * A diferencia del spawn automático, este se ejecuta sincrónicamente
     * y retorna la venta actualizada con el nuevo estado sync.
     * Falla si sync deshabilitado o token vacío. */
    pub async fn retry_haddock_sync(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Venta, AppError> {
        let venta = VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

        let config = ConfiguracionRepository::obtener_o_crear(pool, user_id).await?;
        if !config.haddock_sync_enabled {
            return Err(AppError::Validation(
                "La sincronización con Haddock no está habilitada.".into(),
            ));
        }
        if config.haddock_api_token.is_empty() {
            return Err(AppError::Validation(
                "No hay token de API de Haddock configurado.".into(),
            ));
        }

        /* Ejecuta sync sincrónicamente (is_update=false: no se editó, se reintenta) */
        HaddockService::sync_order(pool, &venta, &config, false).await;

        /* Re-leer venta con estado sync actualizado */
        VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada tras sync".into()))
    }

    /* [065A-5] Lanza sincronización con BDP en un task independiente.
     * Patrón idéntico a spawn_haddock_sync. */
    fn spawn_bdp_sync(pool: PgPool, user_id: Uuid, venta: Venta, is_update: bool) {
        tokio::spawn(async move {
            let config = match ConfiguracionRepository::obtener_o_crear(&pool, user_id).await {
                Ok(c) => c,
                Err(e) => {
                    warn!("[065A-5] Error obteniendo config para BDP sync: {e}");
                    return;
                }
            };
            BdpSyncService::sync_venta(&pool, &venta, &config, is_update, None).await;
        });
    }

    /* [065A-5] Retry manual de sincronización BDP.
     * Ejecuta sincrónicamente y retorna la venta actualizada.
     * [C1-3] `idempotency_key` se propaga al audit log si se proporciona. */
    pub async fn retry_bdp_sync(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        idempotency_key: Option<&str>,
    ) -> Result<Venta, AppError> {
        let venta = VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

        let config = ConfiguracionRepository::obtener_o_crear(pool, user_id).await?;
        if !config.bdp_sync_enabled {
            return Err(AppError::Validation(
                "La sincronización con BDP no está habilitada.".into(),
            ));
        }
        if config.bdp_sync_mode != "unidirectional" {
            return Err(AppError::Validation(
                "BDP está en modo solo lectura; no se ejecutó ninguna escritura.".into(),
            ));
        }
        if !config.bdp_auto_backup_before_write {
            return Err(AppError::Validation(
                "Escritura BDP bloqueada: auto-backup pre-write desactivado.".into(),
            ));
        }

        BdpSyncService::sync_venta(pool, &venta, &config, false, idempotency_key).await;

        VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada tras BDP sync".into()))
    }
}
