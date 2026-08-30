/* [198A-1/F1] Cola unidireccional Glory -> BDP (bdp_push_pendientes).
 *
 * Las ediciones locales encolan una fila activa; un worker (o el botón
 * "Sincronizar a BDP") la procesa con los guards de escritura existentes. La
 * política de reintentos distingue (D2 resuelta):
 *   - error transitorio  -> reintento automático acotado (tope en config);
 *   - "Subscripción no activada" -> 'pendiente_suscripcion' SIN reintento
 *     automático (la suscripción puede no activarse nunca); solo manual.
 *
 * Concurrencia (M19): UNIQUE parcial sobre filas activas + upsert; una sola
 * fila por (user_id, dominio, entidad_id, operacion). */

use rust_decimal::Decimal;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{BdpArticleMap, ConfiguracionRestaurante};
use crate::services::bdp_weblink::{BdpWeblinkClient, BdpWeblinkError};
use crate::services::bdp_weblink_catalog::{
    BdpAddOrderTipRequest, BdpAddPointsRequest, BdpArticleData, BdpCancelOrderRequest,
    BdpCreateArticlesRequest, BdpCreateDepartmentProfilesRequest, BdpCreateFamilyRequest,
    BdpMassiveStockRequest, BdpModifyArticleRequest, BdpModifyPricesRequest, BdpOrderIdentifier,
    BdpRegularizationRequest, BdpStockInfoEntry, BdpTransferRequest,
};
use crate::services::{
    BdpBackupService, BdpWriteGuard, ConfiguracionService, ModoEfectivo, ServicioModoOperacion,
};

pub const DOMINIO_ARTICULO: &str = "articulo";
pub const DOMINIO_STOCK: &str = "stock";
pub const DOMINIO_DEPARTAMENTO: &str = "departamento";
pub const DOMINIO_FAMILIA: &str = "familia";
pub const DOMINIO_VENTA: &str = "venta";
pub const DOMINIO_CLIENTE_PUNTOS: &str = "cliente_puntos";
pub const DOMINIO_PROPINA: &str = "propina";

pub const OPERACION_CREAR: &str = "crear";
pub const OPERACION_MODIFICAR: &str = "modificar";
pub const OPERACION_PRECIOS: &str = "precios";
pub const OPERACION_REGULARIZAR: &str = "regularizar";
pub const OPERACION_TRASPASAR: &str = "traspasar";
pub const OPERACION_INVENTARIO: &str = "inventario";
pub const OPERACION_CANCELAR: &str = "cancelar";
pub const OPERACION_PUNTOS: &str = "puntos";
pub const OPERACION_PROPINA: &str = "propina";

pub const ESTADO_PENDIENTE: &str = "pendiente";
pub const ESTADO_PENDIENTE_SUSCRIPCION: &str = "pendiente_suscripcion";
pub const ESTADO_ERROR: &str = "error";
pub const ESTADO_SINCRONIZADO: &str = "sincronizado";
pub const ESTADO_DESCARTADO: &str = "descartado";

/* [M21] Tope de reintentos automáticos por operación (solo errores transitorios). */
pub const REINTENTOS_MAX: i32 = 5;

const ESTADOS_ACTIVOS: &[&str] = &[ESTADO_PENDIENTE, ESTADO_PENDIENTE_SUSCRIPCION, ESTADO_ERROR];

/// Fila pendiente de push (proyección mínima para el worker).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BdpPushPendiente {
    pub id: Uuid,
    pub dominio: String,
    pub entidad_id: String,
    pub operacion: String,
    #[sqlx(rename = "payload_json")]
    pub payload: Value,
    pub estado: String,
    pub reintentos: i32,
}

pub struct BdpPushService;

impl BdpPushService {
    /// Encola (o refresca) una operación pendiente. Upsert sobre la fila activa
    /// (M19): si ya existe una fila activa para la misma entidad+operación, se
    /// actualiza el payload y se reinicia a 'pendiente'; si no, se inserta.
    pub async fn encolar(
        pool: &PgPool,
        user_id: Uuid,
        dominio: &str,
        entidad_id: &str,
        operacion: &str,
        payload: &Value,
    ) -> Result<(), String> {
        let actualizadas = sqlx::query(
            "UPDATE bdp_push_pendientes \
             SET payload_json = $5, estado = $6, reintentos = 0, \
                 ultimo_error = NULL, updated_at = NOW() \
             WHERE user_id = $1 AND dominio = $2 AND entidad_id = $3 \
               AND operacion = $4 AND estado = ANY($7)",
        )
        .bind(user_id)
        .bind(dominio)
        .bind(entidad_id)
        .bind(operacion)
        .bind(payload)
        .bind(ESTADO_PENDIENTE)
        .bind(ESTADOS_ACTIVOS)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo refrescar push pendiente: {error}"))?;

        if actualizadas.rows_affected() > 0 {
            return Ok(());
        }

        sqlx::query(
            "INSERT INTO bdp_push_pendientes \
             (id, user_id, dominio, entidad_id, operacion, payload_json) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(dominio)
        .bind(entidad_id)
        .bind(operacion)
        .bind(payload)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo encolar push BDP: {error}"))?;
        Ok(())
    }

    /// Transición de estado tras procesar la fila. `incrementar_reintento`
    /// aplica solo a errores transitorios (no a `pendiente_suscripcion`, D2).
    #[allow(clippy::too_many_arguments)]
    pub async fn marcar_resultado(
        pool: &PgPool,
        user_id: Uuid,
        dominio: &str,
        entidad_id: &str,
        operacion: &str,
        estado: &str,
        error: Option<&str>,
        incrementar_reintento: bool,
    ) -> Result<(), String> {
        sqlx::query(
            "UPDATE bdp_push_pendientes \
             SET estado = $5, ultimo_error = $6, updated_at = NOW(), \
                 reintentos = reintentos + CASE WHEN $7 THEN 1 ELSE 0 END \
             WHERE user_id = $1 AND dominio = $2 AND entidad_id = $3 AND operacion = $4",
        )
        .bind(user_id)
        .bind(dominio)
        .bind(entidad_id)
        .bind(operacion)
        .bind(estado)
        .bind(error)
        .bind(incrementar_reintento)
        .execute(pool)
        .await
        .map_err(|error| format!("No se pudo actualizar estado de push: {error}"))?;
        Ok(())
    }

    /// Lista las filas activas pendientes. Orden por dependencia de dominio
    /// (M12): departamento y familia antes que artículo, para que el push de un
    /// artículo no falle porque su departamento aún no existe en BDP.
    pub async fn listar_pendientes(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<BdpPushPendiente>, String> {
        let rows: Vec<BdpPushPendiente> = sqlx::query_as::<_, BdpPushPendiente>(
            "SELECT id, dominio, entidad_id, operacion, payload_json, estado, reintentos \
             FROM bdp_push_pendientes \
             WHERE user_id = $1 AND estado = ANY($2) \
             ORDER BY CASE dominio \
                 WHEN 'departamento' THEN 0 \
                 WHEN 'familia' THEN 1 \
                 WHEN 'articulo' THEN 2 \
                 ELSE 3 END, created_at ASC",
        )
        .bind(user_id)
        .bind(ESTADOS_ACTIVOS)
        .fetch_all(pool)
        .await
        .map_err(|error| format!("No se pudo listar push pendientes: {error}"))?;
        Ok(rows)
    }

    /* [208A-2/C4] Visibilidad de la cola (decisión D5): proyección para la UI
     * con estado, reintentos, último error y fechas, sin el payload completo. */
    pub async fn listar_filas(
        pool: &PgPool,
        user_id: Uuid,
        limite: i64,
    ) -> Result<Vec<BdpPushFila>, String> {
        let rows: Vec<BdpPushFila> = sqlx::query_as::<_, BdpPushFila>(
            "SELECT id, dominio, entidad_id, operacion, estado, reintentos, \
                    ultimo_error, updated_at \
             FROM bdp_push_pendientes \
             WHERE user_id = $1 \
             ORDER BY updated_at DESC \
             LIMIT $2",
        )
        .bind(user_id)
        .bind(limite)
        .fetch_all(pool)
        .await
        .map_err(|error| format!("No se pudo listar la cola de sincronización: {error}"))?;
        Ok(rows)
    }

    /// Fila de la cola por id (para reintento individual); `None` si no existe
    /// o no pertenece al usuario.
    pub async fn obtener_fila(
        pool: &PgPool,
        user_id: Uuid,
        fila_id: Uuid,
    ) -> Result<Option<BdpPushFila>, String> {
        sqlx::query_as::<_, BdpPushFila>(
            "SELECT id, dominio, entidad_id, operacion, estado, reintentos, \
                    ultimo_error, updated_at \
             FROM bdp_push_pendientes \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(fila_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("No se pudo leer la fila de sincronización: {error}"))
    }

    /// Fila completa (con payload) por id, para reintentar individualmente.
    pub async fn obtener_pendiente(
        pool: &PgPool,
        user_id: Uuid,
        fila_id: Uuid,
    ) -> Result<Option<BdpPushPendiente>, String> {
        sqlx::query_as::<_, BdpPushPendiente>(
            "SELECT id, dominio, entidad_id, operacion, payload_json, estado, reintentos \
             FROM bdp_push_pendientes \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(fila_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("No se pudo leer la fila de sincronización: {error}"))
    }
}

/// [208A-2/C4] Proyección de una fila de la cola para la UI de Sincronización.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct BdpPushFila {
    pub id: Uuid,
    pub dominio: String,
    pub entidad_id: String,
    pub operacion: String,
    pub estado: String,
    pub reintentos: i32,
    pub ultimo_error: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/* ===== [198A-1/F1] Payloads de push (construidos por los handlers locales) ===== */

/// Construye el payload de `articulo/modificar` (artículo ya en BDP, editado
/// localmente). Usa el código BDP ya mapeado; si el artículo es local puro
/// (sin código BDP numérico) devuelve Err para que el handler lo deje en F3.
pub fn payload_modificar_articulo(
    config: &ConfiguracionRestaurante,
    map: &BdpArticleMap,
) -> Result<Value, String> {
    let article_data = article_data_desde_map(config, map)?;
    let req = BdpModifyArticleRequest {
        article_data: serde_json::to_value(&article_data)
            .map_err(|error| format!("No se pudo serializar artículo: {error}"))?,
        profiles_list: None,
        all_profiles: Some(true),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `articulo/crear` (artículo local nuevo). Requiere un
/// código numérico explícito (D3); el caller ya lo resolvió y lo guardó en
/// `articulo_bdp_codigo`.
pub fn payload_crear_articulo(
    config: &ConfiguracionRestaurante,
    map: &BdpArticleMap,
) -> Result<Value, String> {
    let article_data = article_data_desde_map(config, map)?;
    let req = BdpCreateArticlesRequest {
        automatic_code: false,
        article_data: serde_json::to_value(&article_data)
            .map_err(|error| format!("No se pudo serializar artículo: {error}"))?,
        profiles_list: None,
        all_profiles: Some(true),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `stock/inventario` (conteo físico por lotes, D6).
/// `articulos` son las líneas ya resueltas a código BDP + unidades contadas.
pub fn payload_inventario(
    config: &ConfiguracionRestaurante,
    articulos: Vec<BdpStockInfoEntry>,
) -> Result<Value, String> {
    let req = BdpMassiveStockRequest {
        cod_reg: config.bdp_codreg_default,
        store: config.bdp_almacen_default,
        date_reg: fecha_hoy(),
        articles_list: articulos,
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `stock/regularizar` (ajuste manual de stock).
pub fn payload_regularizacion(
    config: &ConfiguracionRestaurante,
    bdp_articulo_codigo: i64,
    delta: Decimal,
) -> Result<Value, String> {
    let req = BdpRegularizationRequest {
        article: bdp_articulo_codigo,
        sd1: String::new(),
        sd2: String::new(),
        sd3: String::new(),
        units: delta,
        cod_reg: config.bdp_codreg_default,
        store: config.bdp_almacen_default,
        date_reg: fecha_hoy(),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

fn article_data_desde_map(
    config: &ConfiguracionRestaurante,
    map: &BdpArticleMap,
) -> Result<BdpArticleData, String> {
    let art_code = map.articulo_bdp_codigo.trim().parse::<i64>().map_err(|_| {
        format!(
            "Artículo sin código BDP numérico: {}",
            map.articulo_glory_codigo
        )
    })?;
    Ok(BdpArticleData {
        art_code,
        art_description: if map.descripcion.trim().is_empty() {
            map.articulo_bdp_nombre.clone()
        } else {
            map.descripcion.clone()
        },
        dept_code: (map.departamento > 0).then_some(map.departamento),
        dept_description: None,
        tav_code: lookup_tav(config, map.iva_pct),
        tav_per: Some(map.iva_pct),
        price1: Some(map.precio_tarifa1),
        price2: None,
        price3: None,
        price4: None,
        price5: None,
        web_article: Some(true),
        is_inventoriable: Some(true),
        modifiable_price: None,
        menu_dish: None,
        extra: serde_json::Map::new(),
    })
}

/* [M13] Mapeo IVA local (%) -> TAVCode BDP. Best-effort: se lee del mapa de
 * configuración; el auto-aprendizaje del mapa queda en F3. */
fn lookup_tav(config: &ConfiguracionRestaurante, iva_pct: Decimal) -> Option<i32> {
    config
        .bdp_tav_map
        .get(iva_pct.to_string().as_str())
        .and_then(Value::as_i64)
        .and_then(|code| i32::try_from(code).ok())
}

fn fecha_hoy() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Construye el payload de `departamento/crear` (D7) con `AllProfiles=true` (D4).
pub fn payload_crear_departamento(code: i32, nombre: &str) -> Result<Value, String> {
    let req = BdpCreateDepartmentProfilesRequest {
        code,
        description: nombre.to_string(),
        short_description: nombre.to_string(),
        graph_description1: String::new(),
        graph_description2: String::new(),
        graph_description3: String::new(),
        overwrite: false,
        all_profiles: true,
        profile_list: None,
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `familia/crear` (D7).
pub fn payload_crear_familia(code: i32, nombre: &str) -> Result<Value, String> {
    let req = BdpCreateFamilyRequest {
        code,
        description: nombre.to_string(),
        overwrite: false,
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `propina/propina` (D8). `add_tip=true` suma,
/// `false` sustituye (decisión D8 resuelta: configurable por venta).
pub fn payload_propina(bdp_order_id: i64, amount: Decimal, add_tip: bool) -> Result<Value, String> {
    let req = BdpAddOrderTipRequest {
        order_identifier: BdpOrderIdentifier::by_order_id(bdp_order_id),
        amount,
        add_tip,
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `venta/cancelar` (CancelOrder, F6/D2). Usa
/// `OrderIdentifier { OrderId }` (M26: el local solo guarda `bdp_order_id`;
/// sin Room/Table/Market). Requiere `pos_id` de la configuración.
pub fn payload_cancelar(
    config: &ConfiguracionRestaurante,
    bdp_order_id: i64,
) -> Result<Value, String> {
    let req = BdpCancelOrderRequest {
        pos_id: config.bdp_pos_id,
        order_identifier: BdpOrderIdentifier::by_order_id(bdp_order_id),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/// Construye el payload de `cliente_puntos/puntos` (D9).
pub fn payload_puntos(
    bdp_customer_code: i64,
    points_added: Decimal,
    reason: &str,
) -> Result<Value, String> {
    let req = BdpAddPointsRequest {
        customer: bdp_customer_code,
        points_added,
        reason: reason.to_string(),
    };
    serde_json::to_value(&req).map_err(|error| format!("No se pudo serializar push: {error}"))
}

/* ===== [198A-1/F1] Worker de flush ===== */

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BdpPushFlushResumen {
    pub procesados: usize,
    pub sincronizados: usize,
    pub pendientes_suscripcion: usize,
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
            /* D2: bloqueo por suscripción -> solo reintento manual. */
            if pendiente.estado == ESTADO_PENDIENTE_SUSCRIPCION && !forzar_manual {
                resumen.pendientes_suscripcion += 1;
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
                Ok(_) | Err(_) => resumen.errores += 1,
            }
        }
        Ok(resumen)
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
            Ok(_) | Err(_) => resumen.errores += 1,
        }
        Ok(resumen)
    }

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
        /* 3. Auditoría + consumo del armado + cierre a solo lectura. */
        let audit_id = BdpWriteGuard::authorize(
            pool,
            user_id,
            config,
            scope,
            &pendiente.dominio,
            entity_uuid,
            "glory_entidad_id",
            &pendiente.payload,
            snapshot_pre,
            None,
        )
        .await?;

        /* 4. Dispatcher -> HTTP de escritura (el cliente valida la allowlist). */
        match Self::dispatch(
            client,
            &pendiente.dominio,
            &pendiente.operacion,
            &pendiente.payload,
        )
        .await
        {
            Ok(respuesta) => {
                BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    "exito",
                    Some(&respuesta),
                    None,
                )
                .await?;
                BdpPushService::marcar_resultado(
                    pool,
                    user_id,
                    &pendiente.dominio,
                    &pendiente.entidad_id,
                    &pendiente.operacion,
                    ESTADO_SINCRONIZADO,
                    None,
                    false,
                )
                .await?;
                Ok(ESTADO_SINCRONIZADO)
            }
            Err(error) => {
                let (estado, incrementar) = clasificar_error(&error);
                let mensaje = error.to_string();
                let resultado_audit = if es_transitorio(&error) {
                    "ambiguo"
                } else {
                    "error"
                };
                BdpBackupService::actualizar_resultado(
                    pool,
                    audit_id,
                    resultado_audit,
                    None,
                    Some(&mensaje),
                )
                .await?;
                BdpPushService::marcar_resultado(
                    pool,
                    user_id,
                    &pendiente.dominio,
                    &pendiente.entidad_id,
                    &pendiente.operacion,
                    estado,
                    Some(&mensaje),
                    incrementar,
                )
                .await?;
                Ok(estado)
            }
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

/// Identificador UUID estable derivado de (`dominio`, `entidad_id`) para arming y
/// auditoría (que exigen `Uuid`). No colisiona entre entidades ni usuarios
/// porque el namespace es determinista por (`dominio`, `entidad_id`).
fn entidad_uuid(dominio: &str, entidad_id: &str) -> Uuid {
    /* UUID determinista derivado de (dominio, entidad_id) para arming y
     * auditoría (que exigen `Uuid`). No se usa `Uuid::new_v5` porque el crate
     * solo tiene habilitadas las features `serde` y `v4`. */
    let mut hasher = Sha256::new();
    hasher.update(format!("bdp-push:{dominio}:{entidad_id}").as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

fn es_transitorio(error: &BdpWeblinkError) -> bool {
    matches!(
        error,
        BdpWeblinkError::Http(_) | BdpWeblinkError::Api { .. } | BdpWeblinkError::Throttled(_)
    )
}

fn clasificar_error(error: &BdpWeblinkError) -> (&'static str, bool) {
    if let BdpWeblinkError::Remote(mensaje) = error {
        if mensaje.trim() == "Subscripción no activada" {
            return (ESTADO_PENDIENTE_SUSCRIPCION, false);
        }
    }
    if es_transitorio(error) {
        (ESTADO_ERROR, true)
    } else {
        (ESTADO_ERROR, false)
    }
}
