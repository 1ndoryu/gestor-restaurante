use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::{BdpWeblinkClient, BdpWeblinkError};
use crate::services::bdp_weblink_catalog::{
    BdpAddOrderTipRequest, BdpAddPointsRequest, BdpCancelOrderRequest, BdpCreateArticlesRequest,
    BdpCreateDepartmentProfilesRequest, BdpCreateFamilyRequest, BdpMassiveStockRequest,
    BdpModifyArticleRequest, BdpModifyPricesRequest, BdpRegularizationRequest, BdpTransferRequest,
};
use crate::services::{
    entidad_uuid, BdpBackupService, BdpWriteGuard, ConfiguracionService, EscrituraAutorizar,
    ModoEfectivo, ServicioModoOperacion,
};

use super::clasificacion::{clasificar_error, es_transitorio};
use super::cola::{BdpPushPendiente, BdpPushService, MarcarResultado};
use super::constantes::{
    DOMINIO_ARTICULO, DOMINIO_CLIENTE_PUNTOS, DOMINIO_DEPARTAMENTO, DOMINIO_FAMILIA,
    DOMINIO_PROPINA, DOMINIO_STOCK, DOMINIO_VENTA, ESTADO_PENDIENTE_SUSCRIPCION, ESTADO_RECHAZADO,
    ESTADO_SINCRONIZADO, OPERACION_CANCELAR, OPERACION_CREAR, OPERACION_INVENTARIO,
    OPERACION_MODIFICAR, OPERACION_PRECIOS, OPERACION_PROPINA, OPERACION_PUNTOS,
    OPERACION_REGULARIZAR, OPERACION_TRASPASAR, REINTENTOS_MAX,
};
use super::enriquecimiento::enriquecer_payload_modificar_articulo;

/* ===== [198A-1/F1] Worker de flush ===== */

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BdpPushFlushResumen {
    pub procesados: usize,
    pub sincronizados: usize,
    pub pendientes_suscripcion: usize,
    pub rechazados: usize,
    pub errores: usize,
    pub omitidos_standalone: usize,
    pub omitidos_manual: usize,
}

pub struct BdpPushFlushService;

impl BdpPushFlushService {
    /// Procesa la cola de un usuario. En modo `standalone` no envía nada
    /// (no-op). En `push_modalidad = manual` solo procesa si `forzar_manual`
    /// (botón "Sincronizar a BDP"); en `automatico` procesa siempre.
    ///
    /// Cada fila respeta los guards ya existentes: `armar_push` (arming
    /// autorizado por D1), `preparar_snapshot_escritura` (backup) y `authorize`
    /// (auditoría + fail-closed). `ensure_write_target_allowed` lo aplica el
    /// propio cliente antes de cada HTTP.
    pub async fn flush(
        pool: &PgPool,
        user_id: Uuid,
        forzar_manual: bool,
    ) -> Result<BdpPushFlushResumen, String> {
        Self::flush_con_dominios(pool, user_id, forzar_manual, None).await
    }

    /// [159A-2/F1] Variante con filtro opcional por dominio (p. ej. el botón
    /// "Exportar al BDP" del Catálogo envía solo departamento/familia/artículo).
    /// `None` o vacío = toda la cola (comportamiento histórico del flush global).
    pub async fn flush_con_dominios(
        pool: &PgPool,
        user_id: Uuid,
        forzar_manual: bool,
        dominios: Option<&[String]>,
    ) -> Result<BdpPushFlushResumen, String> {
        let mut resumen = BdpPushFlushResumen::default();
        let config = ConfiguracionService::obtener(pool, user_id)
            .await
            .map_err(|error| format!("No se pudo obtener configuración: {error}"))?;

        let modo = ServicioModoOperacion::modo_efectivo_desde_config(&config);
        if modo == ModoEfectivo::Standalone {
            /* Independencia: nunca enviar nada en standalone. */
            resumen.omitidos_standalone = 1;
            return Ok(resumen);
        }
        if !forzar_manual && config.push_modalidad != "automatico" {
            resumen.omitidos_manual = 1;
            return Ok(resumen);
        }

        let pendientes = BdpPushService::listar_pendientes(pool, user_id).await?;
        let client = BdpWeblinkClient::new(&config);

        for pendiente in pendientes {
            /* [159A-2/F1] Filtro por dominio del botón Exportar por sección. */
            if !Self::dominio_en_filtro(&pendiente.dominio, dominios) {
                continue;
            }
            /* D2: bloqueo por suscripción -> solo reintento manual. */
            if pendiente.estado == ESTADO_PENDIENTE_SUSCRIPCION && !forzar_manual {
                resumen.pendientes_suscripcion += 1;
                continue;
            }
            /* [049A-1/S5] Rechazo definitivo (4xx) -> solo reintento manual. */
            if pendiente.estado == ESTADO_RECHAZADO && !forzar_manual {
                resumen.rechazados += 1;
                continue;
            }
            /* M21: no reintentar indefinidamente errores transitorios. */
            if pendiente.reintentos >= REINTENTOS_MAX {
                resumen.errores += 1;
                continue;
            }
            resumen.procesados += 1;
            match Self::procesar_uno(pool, &config, &client, user_id, &pendiente, forzar_manual)
                .await
            {
                Ok(ESTADO_SINCRONIZADO) => resumen.sincronizados += 1,
                Ok(ESTADO_PENDIENTE_SUSCRIPCION) => resumen.pendientes_suscripcion += 1,
                Ok(ESTADO_RECHAZADO) => resumen.rechazados += 1,
                Ok(_) | Err(_) => resumen.errores += 1,
            }
        }
        Ok(resumen)
    }

    /// [159A-2/F1] Predicado del filtro por dominio del flush.
    /// `None` o vacío = todo pasa (flush global histórico).
    fn dominio_en_filtro(dominio: &str, dominios: Option<&[String]>) -> bool {
        match dominios {
            None => true,
            Some(filtro) => filtro.is_empty() || filtro.iter().any(|d| d == dominio),
        }
    }

    /// [208A-2/C4] Reintento individual de una fila (decisión D5). Respeta las
    /// mismas reglas que el flush manual: en standalone no envía nada y la
    /// fila bloqueada por suscripción se reintenta (D2: solo manual). El
    /// reintento manual se permite aunque se hayan agotado los reintentos
    /// automáticos transitorios (M21 no aplica a acciones manuales).
    pub async fn reintentar_uno(
        pool: &PgPool,
        user_id: Uuid,
        fila_id: Uuid,
    ) -> Result<BdpPushFlushResumen, String> {
        let mut resumen = BdpPushFlushResumen::default();
        let config = ConfiguracionService::obtener(pool, user_id)
            .await
            .map_err(|error| format!("No se pudo obtener configuración: {error}"))?;

        let modo = ServicioModoOperacion::modo_efectivo_desde_config(&config);
        if modo == ModoEfectivo::Standalone {
            /* Independencia: nunca enviar nada en standalone. */
            resumen.omitidos_standalone = 1;
            return Ok(resumen);
        }

        let pendiente = BdpPushService::obtener_pendiente(pool, user_id, fila_id)
            .await?
            .ok_or_else(|| "Fila de push no encontrada".to_string())?;
        let client = BdpWeblinkClient::new(&config);
        resumen.procesados += 1;
        match Self::procesar_uno(pool, &config, &client, user_id, &pendiente, true).await {
            Ok(ESTADO_SINCRONIZADO) => resumen.sincronizados += 1,
            Ok(ESTADO_PENDIENTE_SUSCRIPCION) => resumen.pendientes_suscripcion += 1,
            Ok(ESTADO_RECHAZADO) => resumen.rechazados += 1,
            Ok(_) | Err(_) => resumen.errores += 1,
        }
        Ok(resumen)
    }

    /// Procesa una fila: arming, snapshot, payload final, auditoría y HTTP.
    /// Los pasos van en el orden original (guard de armado antes de la
    /// auditoría y del HTTP); no reordenar.
    async fn procesar_uno(
        pool: &PgPool,
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        user_id: Uuid,
        pendiente: &BdpPushPendiente,
        forzar_manual: bool,
    ) -> Result<&'static str, String> {
        let scope = scope_para(&pendiente.dominio, &pendiente.operacion).ok_or_else(|| {
            format!(
                "Operación no soportada: {}/{}",
                pendiente.dominio, pendiente.operacion
            )
        })?;
        let entity_uuid = entidad_uuid(&pendiente.dominio, &pendiente.entidad_id);

        /* 1. Arming autorizado por push_modalidad (fail-closed). */
        BdpWriteGuard::armar_push(
            pool,
            user_id,
            config,
            scope,
            &pendiente.dominio,
            entity_uuid,
            forzar_manual,
        )
        .await?;
        /* 2. Backup pre-write (No-op salvo add_payment/invoice). */
        let snapshot_pre = BdpBackupService::preparar_snapshot_escritura(
            pool,
            user_id,
            &pendiente.operacion,
            config,
            None,
        )
        .await?;
        /* 3. Payload final: ver `payload_final`. */
        let payload =
            payload_final(pool, user_id, config, client, pendiente, scope, entity_uuid).await?;

        /* 4. Auditoría + consumo del armado + cierre a solo lectura. */
        let audit_id = BdpWriteGuard::authorize(
            pool,
            user_id,
            config,
            EscrituraAutorizar {
                scope,
                target_entity_type: &pendiente.dominio,
                target_entity_id: entity_uuid,
                entity_json_field: "glory_entidad_id",
                datos_enviados: &payload,
                snapshot_pre_id: snapshot_pre,
                idempotency_key: None,
            },
        )
        .await?;

        /* 5. Dispatcher -> HTTP de escritura (el cliente valida la allowlist).
         * `payload` es deliberadamente el mismo valor que se auditó arriba. */
        match Self::dispatch(client, &pendiente.dominio, &pendiente.operacion, &payload).await {
            Ok(respuesta) => registrar_exito(pool, user_id, pendiente, audit_id, &respuesta).await,
            Err(error) => registrar_fallo(pool, user_id, pendiente, audit_id, &error).await,
        }
    }

    async fn dispatch(
        client: &BdpWeblinkClient<'_>,
        dominio: &str,
        operacion: &str,
        payload: &Value,
    ) -> Result<Value, BdpWeblinkError> {
        let mal = |e: serde_json::Error| {
            BdpWeblinkError::Remote(format!("payload de push inválido: {e}"))
        };
        match (dominio, operacion) {
            (DOMINIO_ARTICULO, OPERACION_CREAR) => {
                let req: BdpCreateArticlesRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.create_articles_and_update_profiles(&req).await
            }
            (DOMINIO_ARTICULO, OPERACION_MODIFICAR) => {
                let req: BdpModifyArticleRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.modify_article_and_update_profile(&req).await
            }
            (DOMINIO_ARTICULO, OPERACION_PRECIOS) => {
                let req: BdpModifyPricesRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.modify_prices_articles(&req).await
            }
            (DOMINIO_STOCK, OPERACION_REGULARIZAR) => {
                let req: BdpRegularizationRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.regularize_stock(&req).await
            }
            (DOMINIO_STOCK, OPERACION_TRASPASAR) => {
                let req: BdpTransferRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.transfer_stock(&req).await
            }
            (DOMINIO_STOCK, OPERACION_INVENTARIO) => {
                let req: BdpMassiveStockRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.update_massive_inventory(&req).await
            }
            (DOMINIO_DEPARTAMENTO, OPERACION_CREAR) => {
                let req: BdpCreateDepartmentProfilesRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.create_department_and_update_profiles(&req).await
            }
            (DOMINIO_FAMILIA, OPERACION_CREAR) => {
                let req: BdpCreateFamilyRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.create_family(&req).await
            }
            (DOMINIO_VENTA, OPERACION_CANCELAR) => {
                let req: BdpCancelOrderRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.cancel_order(&req).await
            }
            (DOMINIO_PROPINA, OPERACION_PROPINA) => {
                let req: BdpAddOrderTipRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.add_order_tip(&req).await
            }
            (DOMINIO_CLIENTE_PUNTOS, OPERACION_PUNTOS) => {
                let req: BdpAddPointsRequest =
                    serde_json::from_value(payload.clone()).map_err(mal)?;
                client.add_points(&req).await
            }
            _ => Err(BdpWeblinkError::Remote(format!(
                "operación no soportada: {dominio}/{operacion}"
            ))),
        }
    }
}

/// Paso 3 del worker: completa `articulo/modificar` antes de autorizar.
/// `ModifyArticleAndUpdateProfile` no acepta ArticleData parcial y la cola
/// conserva payloads históricos parciales; así la auditoría y el HTTP reciben
/// exactamente el mismo payload final.
async fn payload_final(
    pool: &PgPool,
    user_id: Uuid,
    config: &ConfiguracionRestaurante,
    client: &BdpWeblinkClient<'_>,
    pendiente: &BdpPushPendiente,
    scope: &str,
    entity_uuid: Uuid,
) -> Result<Value, String> {
    if pendiente.dominio != DOMINIO_ARTICULO || pendiente.operacion != OPERACION_MODIFICAR {
        return Ok(pendiente.payload.clone());
    }
    match enriquecer_payload_modificar_articulo(client, &pendiente.payload).await {
        Ok(payload) => Ok(payload),
        Err(error) => {
            /* GetArticle ocurre antes de autorizar. Si falla, compensa el armado
             * de esta fila para no dejar una escritura fantasma bloqueando el
             * siguiente reintento. */
            if let Err(cleanup_error) = BdpWriteGuard::cancelar_armado_push(
                pool,
                user_id,
                config,
                scope,
                &pendiente.dominio,
                entity_uuid,
            )
            .await
            {
                return Err(format!(
                    "{error}; además no se pudo cancelar el armado BDP: {cleanup_error}"
                ));
            }
            Err(error)
        }
    }
}

/// Paso 5 (éxito): cierra la auditoría y marca la fila como sincronizada.
async fn registrar_exito(
    pool: &PgPool,
    user_id: Uuid,
    pendiente: &BdpPushPendiente,
    audit_id: Uuid,
    respuesta: &Value,
) -> Result<&'static str, String> {
    BdpBackupService::actualizar_resultado(pool, audit_id, "exito", Some(respuesta), None).await?;
    BdpPushService::marcar_resultado(
        pool,
        user_id,
        MarcarResultado {
            dominio: &pendiente.dominio,
            entidad_id: &pendiente.entidad_id,
            operacion: &pendiente.operacion,
            estado: ESTADO_SINCRONIZADO,
            error: None,
            incrementar_reintento: false,
        },
    )
    .await?;
    Ok(ESTADO_SINCRONIZADO)
}

/// Paso 5 (error): clasifica el fallo, cierra la auditoría (`ambiguo` si es
/// transitorio, `error` si no) y registra el estado y reintentos de la fila.
async fn registrar_fallo(
    pool: &PgPool,
    user_id: Uuid,
    pendiente: &BdpPushPendiente,
    audit_id: Uuid,
    error: &BdpWeblinkError,
) -> Result<&'static str, String> {
    let (estado, incrementar) = clasificar_error(error);
    let mensaje = error.to_string();
    let resultado_audit = if es_transitorio(error) {
        "ambiguo"
    } else {
        "error"
    };
    BdpBackupService::actualizar_resultado(pool, audit_id, resultado_audit, None, Some(&mensaje))
        .await?;
    BdpPushService::marcar_resultado(
        pool,
        user_id,
        MarcarResultado {
            dominio: &pendiente.dominio,
            entidad_id: &pendiente.entidad_id,
            operacion: &pendiente.operacion,
            estado,
            error: Some(&mensaje),
            incrementar_reintento: incrementar,
        },
    )
    .await?;
    Ok(estado)
}

fn scope_para(dominio: &str, operacion: &str) -> Option<&'static str> {
    match (dominio, operacion) {
        (DOMINIO_ARTICULO, OPERACION_CREAR) => Some("create_article"),
        (DOMINIO_ARTICULO, OPERACION_MODIFICAR) => Some("modify_article"),
        (DOMINIO_ARTICULO, OPERACION_PRECIOS) => Some("modify_prices"),
        (DOMINIO_STOCK, OPERACION_REGULARIZAR) => Some("regularize_stock"),
        (DOMINIO_STOCK, OPERACION_TRASPASAR) => Some("transfer_stock"),
        (DOMINIO_STOCK, OPERACION_INVENTARIO) => Some("inventory"),
        (DOMINIO_DEPARTAMENTO, OPERACION_CREAR) => Some("create_department"),
        (DOMINIO_FAMILIA, OPERACION_CREAR) => Some("create_family"),
        (DOMINIO_VENTA, OPERACION_CANCELAR) => Some("cancel_order"),
        (DOMINIO_PROPINA, OPERACION_PROPINA) => Some("add_tip"),
        (DOMINIO_CLIENTE_PUNTOS, OPERACION_PUNTOS) => Some("add_points"),
        _ => None,
    }
}
