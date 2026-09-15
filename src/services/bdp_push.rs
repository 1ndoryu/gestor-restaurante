// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
/* [198A-1/F1] Cola unidireccional Glory -> BDP (bdp_push_pendientes).
 *
 * Las ediciones locales encolan una fila activa; un worker (o el botón
 * "Sincronizar a BDP") la procesa con los guards de escritura existentes. La
 * política de reintentos distingue (D2 resuelta):
 *   - error transitorio  -> reintento automático acotado (tope en config);
 *   - "Subscripción no activada" -> 'pendiente_suscripcion' SIN reintento
 *     automático (la suscripción puede no activarse nunca); solo manual;
 *   - rechazo definitivo (HTTP 4xx: payload inválido o conflicto) ->
 *     'rechazado' SIN reintento automático (el error no va a desaparecer
 *     solo); solo manual, y solo tras corregir el dato local.
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
    BdpGetArticleRequest, BdpMassiveStockRequest, BdpModifyArticleRequest, BdpModifyPricesRequest,
    BdpOrderIdentifier, BdpRegularizationRequest, BdpStockInfoEntry, BdpTransferRequest,
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
pub const ESTADO_RECHAZADO: &str = "rechazado";
pub const ESTADO_SINCRONIZADO: &str = "sincronizado";
pub const ESTADO_DESCARTADO: &str = "descartado";

/* [M21] Tope de reintentos automáticos por operación (solo errores transitorios). */
pub const REINTENTOS_MAX: i32 = 5;

/* [049A-1/S5] 'rechazado' (4xx definitivo) entra en las filas activas para que
 * el flush la vea y la salte salvo reintento manual, igual que
 * 'pendiente_suscripcion'; una re-edición local (encolar) la refresca. */
const ESTADOS_ACTIVOS: &[&str] = &[
    ESTADO_PENDIENTE,
    ESTADO_PENDIENTE_SUSCRIPCION,
    ESTADO_ERROR,
    ESTADO_RECHAZADO,
];

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
        /* [149A-2/W-Q2.2] `activo=false` despublica el artículo en BDP
         * (`WebArticle:false`) en lugar de dejarlo visible. Es la vía de
         * neutralización de altas de prueba: BDP no expone borrado. */
        web_article: Some(map.activo),
        is_inventoriable: Some(true),
        modifiable_price: None,
        menu_dish: None,
        extra: serde_json::Map::new(),
    })
}

/* [M13] Mapeo IVA local (%) -> TAVCode BDP. Best-effort: se lee del mapa de
 * configuración; el auto-aprendizaje del mapa queda en F3.
 *
 * [321A-4] La clave del mapa se busca normalizada: `iva_pct` llega de
 * `numeric(6,2)` y `Decimal::to_string()` produce "10.00" (escala 2), pero la
 * documentación (M13) y la UI configuran el mapa con claves canónicas sin
 * decimales ("10" -> 1, "21" -> 2). Sin normalizar, el lookup fallaba siempre
 * para IVAs con escala distinta de la clave configurada. `normalize()` quita
 * los ceros finales conservando decimales significativos (10.00 -> "10",
 * 10.50 -> "10.5"); se prueba primero la clave exacta para no romper mapas
 * que ya usasen "10.00". */
fn lookup_tav(config: &ConfiguracionRestaurante, iva_pct: Decimal) -> Option<i32> {
    let clave_exacta = iva_pct.to_string();
    let clave_normalizada = iva_pct.normalize().to_string();
    let mapa = &config.bdp_tav_map;
    let resolver = |clave: &str| {
        mapa.get(clave)
            .and_then(Value::as_i64)
            .and_then(|code| i32::try_from(code).ok())
    };
    resolver(&clave_exacta).or_else(|| resolver(&clave_normalizada))
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
        /* 3. ModifyArticleAndUpdateProfile no acepta ArticleData parcial. La
         * cola conserva payloads históricos parciales, por lo que se completa
         * después del snapshot y antes de autorizar: la auditoría y el HTTP
         * reciben exactamente el mismo payload final. */
        let payload = if pendiente.dominio == DOMINIO_ARTICULO
            && pendiente.operacion == OPERACION_MODIFICAR
        {
            match enriquecer_payload_modificar_articulo(client, &pendiente.payload).await {
                Ok(payload) => payload,
                Err(error) => {
                    /* GetArticle ocurre antes de autorizar. Si falla, compensa
                     * el armado de esta fila para no dejar una escritura
                     * fantasma bloqueando el siguiente reintento. */
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
                    return Err(error);
                }
            }
        } else {
            pendiente.payload.clone()
        };

        /* 4. Auditoría + consumo del armado + cierre a solo lectura. */
        let audit_id = BdpWriteGuard::authorize(
            pool,
            user_id,
            config,
            scope,
            &pendiente.dominio,
            entity_uuid,
            "glory_entidad_id",
            &payload,
            snapshot_pre,
            None,
        )
        .await?;

        /* 5. Dispatcher -> HTTP de escritura (el cliente valida la allowlist).
         * `payload` es deliberadamente el mismo valor que se auditó arriba. */
        match Self::dispatch(client, &pendiente.dominio, &pendiente.operacion, &payload).await {
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

/* [Q2.2] Enriquecimiento de ModifyArticle: lee el ArticleData completo desde
 * BDP vía GetArticle y lo fusiona con los campos parciales del payload de la
 * cola. Esto garantiza que ModifyArticleAndUpdateProfile reciba todos los
 * campos obligatorios (solución al NullReferenceException del endpoint real).
 *
 * La fusión sobreescribe en el ArticleData completo solo los campos que el
 * payload original ya trae; el resto (perfiles, precios no tocados, flags no
 * modificados) se conservan tal cual los devuelve GetArticle. Los campos
 * `profiles_list` y `all_profiles` del payload original se mantienen sin
 * alterar. */
async fn enriquecer_payload_modificar_articulo(
    client: &BdpWeblinkClient<'_>,
    payload: &Value,
) -> Result<Value, String> {
    let req: BdpModifyArticleRequest = serde_json::from_value(payload.clone())
        .map_err(|e| format!("payload de modificar inválido: {e}"))?;

    let art_code = req
        .article_data
        .get("ArtCode")
        .and_then(Value::as_i64)
        .ok_or_else(|| "payload de modificar sin ArtCode".to_string())?;

    let get_resp = client
        .get_article(&BdpGetArticleRequest { art_code })
        .await
        .map_err(|e| format!("GetArticle falló durante enriquecimiento: {e}"))?;

    let full_data = get_resp
        .get("ArticleData")
        .ok_or_else(|| "GetArticle devolvió respuesta sin ArticleData".to_string())?
        .clone();

    let full_data = fusionar_article_data(full_data, &req.article_data)?;

    let enriquecido = BdpModifyArticleRequest {
        article_data: full_data,
        profiles_list: req.profiles_list,
        all_profiles: req.all_profiles,
    };

    serde_json::to_value(&enriquecido)
        .map_err(|e| format!("No se pudo serializar payload enriquecido: {e}"))
}

fn fusionar_article_data(mut completo: Value, parcial: &Value) -> Result<Value, String> {
    let completo_obj = completo
        .as_object_mut()
        .ok_or_else(|| "GetArticle devolvió ArticleData no objeto".to_string())?;
    let parcial_obj = parcial
        .as_object()
        .ok_or_else(|| "payload de modificar tiene ArticleData no objeto".to_string())?;

    for (key, value) in parcial_obj {
        completo_obj.insert(key.clone(), value.clone());
    }
    Ok(completo)
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
        BdpWeblinkError::Http(_) | BdpWeblinkError::Throttled(_)
    ) || matches!(
        error,
        /* [049A-1/S5] 4xx es rechazo definitivo del BDP (payload inválido,
         * conflicto de negocio): reintentarlo en bucle es inútil y puede
         * enmascarar un dato local roto. Solo 5xx es transitorio. */
        BdpWeblinkError::Api { status, .. } if *status >= 500
    )
}

fn clasificar_error(error: &BdpWeblinkError) -> (&'static str, bool) {
    if let BdpWeblinkError::Remote(mensaje) = error {
        /* [049A-1/H-W-3] Match normalizado (case-insensitive y ambas grafías
         * subscrip-/suscrip-) para no perder el bloqueo por suscripción si el
         * BDP varía el texto exacto; si se pierde, caería a reintento transitorio
         * (acotado a REINTENTOS_MAX, no desastre, pero sin la semántica manual). */
        let normalizado = mensaje.trim().to_lowercase();
        if normalizado.contains("subscripción no activada")
            || normalizado.contains("subscripcion no activada")
            || normalizado.contains("suscripción no activada")
            || normalizado.contains("suscripcion no activada")
        {
            return (ESTADO_PENDIENTE_SUSCRIPCION, false);
        }
    }
    if let BdpWeblinkError::Api { status, .. } = error {
        /* [049A-1/S5] Rechazo definitivo: 4xx no se reintenta en bucle. */
        if *status < 500 {
            return (ESTADO_RECHAZADO, false);
        }
    }
    if es_transitorio(error) {
        (ESTADO_ERROR, true)
    } else {
        (ESTADO_ERROR, false)
    }
}

#[cfg(test)]
mod clasificacion_error_tests {
    use super::*;

    #[test]
    fn suscripcion_exacta_bdp() {
        let error = BdpWeblinkError::Remote("Subscripción no activada".into());
        assert_eq!(
            clasificar_error(&error),
            (ESTADO_PENDIENTE_SUSCRIPCION, false)
        );
    }

    #[test]
    fn suscripcion_con_variacion_case_y_grafia() {
        let error = BdpWeblinkError::Remote("  SUSCRIPCION NO ACTIVADA ".into());
        assert_eq!(
            clasificar_error(&error),
            (ESTADO_PENDIENTE_SUSCRIPCION, false)
        );
    }

    #[test]
    fn suscripcion_dentro_de_mensaje_mas_largo() {
        let error = BdpWeblinkError::Remote("Error: subscripción no activada para este POS".into());
        assert_eq!(
            clasificar_error(&error),
            (ESTADO_PENDIENTE_SUSCRIPCION, false)
        );
    }

    #[test]
    fn error_remoto_ajeno_no_es_suscripcion() {
        let error = BdpWeblinkError::Remote("Otro error de negocio".into());
        assert_eq!(clasificar_error(&error), (ESTADO_ERROR, false));
    }

    #[test]
    fn error_transitorio_api_incrementa_reintento() {
        let error = BdpWeblinkError::Api {
            status: 500,
            body: "boom".into(),
        };
        assert_eq!(clasificar_error(&error), (ESTADO_ERROR, true));
    }

    #[test]
    fn error_api_422_payload_invalido_es_rechazo_definitivo() {
        let error = BdpWeblinkError::Api {
            status: 422,
            body: "el campo code es obligatorio".into(),
        };
        assert_eq!(clasificar_error(&error), (ESTADO_RECHAZADO, false));
        assert_eq!(es_transitorio(&error), false);
    }

    #[test]
    fn error_api_400_y_409_tambien_son_rechazo() {
        for status in [400, 409] {
            let error = BdpWeblinkError::Api {
                status,
                body: "conflicto de negocio".into(),
            };
            assert_eq!(clasificar_error(&error), (ESTADO_RECHAZADO, false));
            assert_eq!(es_transitorio(&error), false);
        }
    }

    #[test]
    fn error_api_5xx_sigue_siendo_transitorio() {
        let error = BdpWeblinkError::Api {
            status: 503,
            body: "servidor ocupado".into(),
        };
        assert_eq!(clasificar_error(&error), (ESTADO_ERROR, true));
        assert_eq!(es_transitorio(&error), true);
    }

    #[test]
    fn error_http_timeout_es_transitorio() {
        let error = BdpWeblinkError::Http("timeout al conectar con BDP".into());
        assert_eq!(es_transitorio(&error), true);
    }
}

#[cfg(test)]
mod article_data_merge_tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn map_fixture(activo: bool) -> BdpArticleMap {
        BdpArticleMap {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            articulo_glory_codigo: "90000003".into(),
            articulo_bdp_codigo: "90000003".into(),
            articulo_bdp_nombre: "PRUEBA".into(),
            origen: "local".into(),
            local_dirty: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            descripcion: "PRUEBA".into(),
            precio_tarifa1: Decimal::from_str("1.00").unwrap(),
            iva_pct: Decimal::from_str("10.00").unwrap(),
            departamento: 1,
            familia: 1,
            subfamilia: 1,
            activo,
            barcode: String::new(),
            ultima_sync_at: None,
            stock_actual: Decimal::ZERO,
        }
    }

    /* [149A-2/W-Q2.2] Desactivar un map despublica el artículo en BDP. */
    #[test]
    fn map_inactivo_envia_web_article_false() {
        let parcial = article_data_desde_map(
            &ConfiguracionRestaurante::default(),
            &map_fixture(false),
        )
        .unwrap();
        let v = serde_json::to_value(&parcial).unwrap();
        assert_eq!(v["WebArticle"], false);
    }

    #[test]
    fn map_activo_envia_web_article_true() {
        let parcial = article_data_desde_map(
            &ConfiguracionRestaurante::default(),
            &map_fixture(true),
        )
        .unwrap();
        let v = serde_json::to_value(&parcial).unwrap();
        assert_eq!(v["WebArticle"], true);
    }

    #[test]
    fn merge_conserva_campos_completos_y_aplica_patch_parcial() {
        let completo = serde_json::json!({
            "ArtCode": 90000001,
            "ArtDescription": "Original BDP",
            "Price5": 42.5,
            "WebArticle": true
        });
        let parcial = serde_json::json!({
            "ArtCode": 90000001,
            "ArtDescription": "Editado localmente"
        });

        let resultado = fusionar_article_data(completo, &parcial).unwrap();
        assert_eq!(resultado["ArtDescription"], "Editado localmente");
        assert_eq!(resultado["Price5"], 42.5);
        assert_eq!(resultado["WebArticle"], true);
    }

    #[test]
    fn merge_rechaza_article_data_no_objeto() {
        let error =
            fusionar_article_data(serde_json::json!([]), &serde_json::json!({})).unwrap_err();
        assert_eq!(error, "GetArticle devolvió ArticleData no objeto");
    }
}

#[cfg(test)]
mod lookup_tav_tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn config_con_map(claves: &[&str], code: i32) -> ConfiguracionRestaurante {
        let mut obj = serde_json::Map::new();
        for clave in claves {
            obj.insert(clave.to_string(), Value::from(code));
        }
        ConfiguracionRestaurante {
            bdp_tav_map: Value::Object(obj),
            ..Default::default()
        }
    }

    #[test]
    fn clave_canonica_entera_matchea_con_escala_2() {
        /* [321A-4] El caso real: iva_pct = 10.00 (numeric(6,2)) y el mapa usa
         * la clave canónica "10" (M13: 10 -> 1). */
        let config = config_con_map(&["10"], 1);
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("10.00").unwrap()),
            Some(1)
        );
    }

    #[test]
    fn clave_con_decimales_significativos_matchea_normalizada() {
        let config = config_con_map(&["10.5"], 3);
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("10.50").unwrap()),
            Some(3)
        );
    }

    #[test]
    fn clave_exacta_con_escala_2_sigue_funcionando() {
        /* Compatibilidad: un mapa configurado manualmente con "10.00". */
        let config = config_con_map(&["10.00"], 9);
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("10.00").unwrap()),
            Some(9)
        );
    }

    #[test]
    fn sin_mapa_devuelve_none() {
        let config = ConfiguracionRestaurante {
            ..Default::default()
        };
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("10.00").unwrap()),
            None
        );
    }

    #[test]
    fn iva_fuera_del_mapa_devuelve_none() {
        let config = config_con_map(&["21"], 2);
        assert_eq!(
            lookup_tav(&config, Decimal::from_str("4.00").unwrap()),
            None
        );
    }
}
