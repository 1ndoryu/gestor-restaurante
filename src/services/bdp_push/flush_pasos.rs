//! Pasos del worker de flush que no forman el bucle principal: armado del payload
//! final, cierre de la auditoría (éxito, fallo o libro de stock) y cancelación del
//! armado. Los usa `flush.rs`; viven aquí para que ese archivo no pase de 500 líneas.

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::{BdpWeblinkClient, BdpWeblinkError};
use crate::services::{BdpBackupService, BdpWriteGuard};

use super::clasificacion::{clasificar_error, es_transitorio};
use super::cola::{BdpPushPendiente, BdpPushService, MarcarResultado};
use super::constantes::{
    DOMINIO_ARTICULO, DOMINIO_CLIENTE_PUNTOS, DOMINIO_DEPARTAMENTO, DOMINIO_FAMILIA,
    DOMINIO_PROPINA, DOMINIO_STOCK, DOMINIO_VENTA, ESTADO_ERROR, ESTADO_RECHAZADO,
    ESTADO_SINCRONIZADO, OPERACION_CANCELAR, OPERACION_CREAR, OPERACION_INVENTARIO,
    OPERACION_MODIFICAR, OPERACION_PRECIOS, OPERACION_PROPINA, OPERACION_PUNTOS,
    OPERACION_REGULARIZAR, OPERACION_TRASPASAR,
};
use super::enriquecimiento::enriquecer_payload_modificar_articulo;
use super::ledger::{self, AccionLibro, LibroAbierto};

/// Paso 3 del worker: completa `articulo/modificar` antes de autorizar.
/// `ModifyArticleAndUpdateProfile` no acepta `ArticleData` parcial y la cola
/// conserva payloads históricos parciales; así la auditoría y el HTTP reciben
/// exactamente el mismo payload final.
pub(super) async fn payload_final(
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
pub(super) async fn registrar_exito(
    pool: &PgPool,
    pendiente: &BdpPushPendiente,
    audit_id: Uuid,
    respuesta: &Value,
) -> Result<&'static str, String> {
    BdpBackupService::actualizar_resultado(pool, audit_id, "exito", Some(respuesta), None).await?;
    BdpPushService::marcar_resultado(
        pool,
        MarcarResultado {
            fila_id: pendiente.id,
            marca: pendiente.updated_at,
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
///
/// [D26g] En stock el libro manda: relee `GetStock` y decide entre «ya está
/// aplicado» (se cierra como sincronizado sin reenviar), «no llegó» (un único
/// reintento) y «no se sabe» (rechazado, sin reintento, a revisión manual).
#[allow(clippy::too_many_arguments)]
pub(super) async fn registrar_fallo(
    pool: &PgPool,
    user_id: Uuid,
    client: &BdpWeblinkClient<'_>,
    pendiente: &BdpPushPendiente,
    audit_id: Uuid,
    error: &BdpWeblinkError,
    libro: Option<&LibroAbierto>,
) -> Result<&'static str, String> {
    if let Some(libro) = libro {
        return registrar_fallo_de_stock(pool, user_id, client, pendiente, audit_id, error, libro)
            .await;
    }
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
        MarcarResultado {
            fila_id: pendiente.id,
            marca: pendiente.updated_at,
            estado,
            error: Some(&mensaje),
            incrementar_reintento: incrementar,
        },
    )
    .await?;
    Ok(estado)
}

/// [D26g] Cierre de un envío de stock fallido según el dictamen del libro.
/// La auditoría guarda la lectura que lo justifica en `datos_respuesta` para
/// que el aviso se pueda revisar sin abrir la base.
pub(super) async fn registrar_fallo_de_stock(
    pool: &PgPool,
    user_id: Uuid,
    client: &BdpWeblinkClient<'_>,
    pendiente: &BdpPushPendiente,
    audit_id: Uuid,
    error: &BdpWeblinkError,
    libro: &LibroAbierto,
) -> Result<&'static str, String> {
    let (accion, nota) =
        ledger::tras_error(pool, user_id, client, libro, &error.to_string()).await?;
    let (resultado_audit, estado, incrementar) = match accion {
        /* Aplicado: el rechazo no ocurrió, la escritura sí. */
        AccionLibro::Aplicado => ("exito", ESTADO_SINCRONIZADO, false),
        /* La relectura dice que no se aplicó: queda deja de ser ambigua y el
         * guard de escritura deja pasar el reintento único. */
        AccionLibro::Reintentar => ("error", ESTADO_ERROR, true),
        /* No se puede atribuir el movimiento del stock. Pero `ambiguo` solo
         * vale cuando de verdad no sabemos si la escritura llegó: con un
         * rechazo de negocio ([200202] «no es del tipo web») o un 4xx, BDP
         * contestó que no la aplicó. Dejarlo `ambiguo` bloqueaba ese artículo
         * y almacén para siempre, porque la reconciliación del poller no
         * cubre stock y el libro ya no tiene filas que dictaminen. */
        AccionLibro::Revisar => {
            let rechazo = clasificar_error(error).0 == ESTADO_RECHAZADO;
            let audit = if rechazo || !es_transitorio(error) {
                "error"
            } else {
                "ambiguo"
            };
            let estado = if rechazo {
                ESTADO_RECHAZADO
            } else {
                ESTADO_ERROR
            };
            (audit, estado, false)
        }
    };
    BdpBackupService::actualizar_resultado(pool, audit_id, resultado_audit, None, Some(&nota))
        .await?;
    BdpPushService::marcar_resultado(
        pool,
        MarcarResultado {
            fila_id: pendiente.id,
            marca: pendiente.updated_at,
            estado,
            error: Some(&nota),
            incrementar_reintento: incrementar,
        },
    )
    .await?;
    Ok(estado)
}

/// [10AA-3] Falla antes del HTTP con el armado ya concedido: se cancela para no
/// dejar una autorización pendiente que el siguiente intento reutilizaría.
pub(super) async fn compensar_armado(
    pool: &PgPool,
    user_id: Uuid,
    config: &ConfiguracionRestaurante,
    scope: &str,
    pendiente: &BdpPushPendiente,
    entity_uuid: Uuid,
    error: String,
) -> String {
    if let Err(cleanup) = BdpWriteGuard::cancelar_armado_push(
        pool,
        user_id,
        config,
        scope,
        &pendiente.dominio,
        entity_uuid,
    )
    .await
    {
        return format!("{error}; además no se pudo cancelar el armado BDP: {cleanup}");
    }
    error
}

pub(super) fn scope_para(dominio: &str, operacion: &str) -> Option<&'static str> {
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
