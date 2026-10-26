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
    entidad_uuid, motivo_bloqueo, BdpBackupService, BdpWriteGuard, ConfiguracionService,
    EscrituraAutorizar, ModoEfectivo, ServicioModoOperacion,
};

use super::cola::{BdpPushPendiente, BdpPushService, MarcarResultado};
use super::constantes::{
    DOMINIO_ARTICULO, DOMINIO_CLIENTE_PUNTOS, DOMINIO_DEPARTAMENTO, DOMINIO_FAMILIA,
    DOMINIO_PROPINA, DOMINIO_STOCK, DOMINIO_VENTA, ESTADO_ERROR, ESTADO_PENDIENTE_SUSCRIPCION,
    ESTADO_RECHAZADO, ESTADO_SINCRONIZADO, OPERACION_CANCELAR, OPERACION_CREAR,
    OPERACION_INVENTARIO, OPERACION_MODIFICAR, OPERACION_PRECIOS, OPERACION_PROPINA,
    OPERACION_PUNTOS, OPERACION_REGULARIZAR, OPERACION_TRASPASAR, REINTENTOS_MAX,
};
use super::flush_pasos::{
    compensar_armado, payload_final, registrar_exito, registrar_fallo, scope_para,
};
use super::ledger;
use super::reconciliacion_puntos::resolver_intento_previo;

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
    /// [26AA-3] Filas que esperan a que se encienda su categoría automática.
    pub bloqueados_permiso: usize,
}

pub struct BdpPushFlushService;

impl BdpPushFlushService {
    /// Procesa la cola de un usuario. En modo `standalone` no envía nada
    /// (no-op). En `push_modalidad = manual` solo procesa si `forzar_manual`
    /// (botón "Sincronizar a BDP"); en `automatico` procesa siempre.
    ///
    /// Cada fila respeta los guards ya existentes: `armar_push` (arming
    /// autorizado por D1), `preparar_snapshot_escritura` (backup) y `authorize`
    /// (auditoría + fail-closed). La forma de la URL de destino la valida el
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
            /* [26AA-3] Categoría apagada: la fila espera con su motivo sin gastar reintentos. */
            if let Some(motivo) =
                Self::bloqueo_por_permiso(pool, user_id, &config, &pendiente).await
            {
                let _ = BdpPushService::anotar_motivo(pool, pendiente.id, &motivo).await;
                resumen.bloqueados_permiso += 1;
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
                Ok(_) => resumen.errores += 1,
                Err(mensaje) => {
                    Self::registrar_fallo_previo(pool, &pendiente, &mensaje).await;
                    resumen.errores += 1;
                }
            }
        }
        Ok(resumen)
    }

    /// [26AA-3] Motivo por el que el modo automático no puede escribir esta fila.
    /// Solo aplica en `automatic` y a los alcances con categoría; un error al
    /// leer el permiso bloquea (fail-closed).
    async fn bloqueo_por_permiso(
        pool: &PgPool,
        user_id: Uuid,
        config: &ConfiguracionRestaurante,
        pendiente: &BdpPushPendiente,
    ) -> Option<String> {
        if config.bdp_sync_mode != "automatic" {
            return None;
        }
        let scope = scope_para(&pendiente.dominio, &pendiente.operacion)?;
        match motivo_bloqueo(pool, user_id, scope).await {
            Ok(motivo) => motivo,
            Err(error) => Some(error),
        }
    }

    /// [10AA-3] `procesar_uno` falla antes del HTTP en el armado, el snapshot o el
    /// payload. Antes ese error solo sumaba a `errores`: la fila quedaba en
    /// `pendiente` y sin `ultimo_error`, así que el panel no enseñaba nada y un
    /// rechazo real (p. ej. «no hay mapeo de IVA») era invisible. Se persiste el
    /// motivo y se cuenta el reintento; si la propia anotación falla, se sigue.
    async fn registrar_fallo_previo(pool: &PgPool, pendiente: &BdpPushPendiente, mensaje: &str) {
        tracing::warn!(
            "flush BDP: {}/{} de {} sin enviar: {mensaje}",
            pendiente.dominio,
            pendiente.operacion,
            pendiente.entidad_id
        );
        let resultado = BdpPushService::marcar_resultado(
            pool,
            MarcarResultado {
                fila_id: pendiente.id,
                marca: pendiente.updated_at,
                estado: ESTADO_ERROR,
                error: Some(mensaje),
                incrementar_reintento: true,
            },
        )
        .await;
        if let Err(error) = resultado {
            tracing::warn!("flush BDP: no se pudo anotar el fallo: {error}");
        }
    }

    /// [D-auto-push] Barrido periódico de las cuentas que enviarán solas.
    /// Equivale al botón «Exportar al BDP» (`forzar_manual = false`): respeta las
    /// reglas ya existentes —`pendiente_suscripcion` y `rechazado` solo se
    /// reintentan a mano, y `REINTENTOS_MAX` corta los errores transitorios—, así
    /// que un fallo no se convierte en un bucle de escrituras contra el TPV.
    pub async fn sincronizar_cuentas_automaticas(pool: &sqlx::PgPool) -> usize {
        /* [10AA-4] Solo las cuentas con permiso permanente: `push_modalidad` es
         * `automatico` por defecto, así que sin mirar `bdp_sync_mode` toda cuenta
         * nueva escribía en el TPV estando en `read_only`. En `unidirectional` el
         * permiso es por operación y lo gasta una persona con el botón; este
         * barrido no debe gastarlo, así que tampoco entra aquí. `armar_push`
         * vuelve a comprobarlo por si el modo cambia entre esta consulta y el
         * envío. */
        let usuarios: Vec<Uuid> = match sqlx::query_scalar(
            "SELECT user_id FROM configuracion_restaurante \
              WHERE push_modalidad = 'automatico' \
                AND bdp_sync_mode = 'automatic'",
        )
        .fetch_all(pool)
        .await
        {
            Ok(usuarios) => usuarios,
            Err(error) => {
                tracing::warn!("[D-auto-push] No se pudieron listar cuentas automáticas: {error}");
                return 0;
            }
        };

        let mut total = 0;
        for user_id in usuarios {
            /* Un fallo de una cuenta no corta a las demás: cada fila de la cola
             * guarda su propio estado y reintento. */
            match Self::flush(pool, user_id, false).await {
                Ok(resumen) => total += resumen.sincronizados,
                Err(error) => tracing::warn!("[D-auto-push] Envío de {user_id} fallido: {error}"),
            }
        }
        total
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
            Ok(_) => resumen.errores += 1,
            Err(mensaje) => {
                Self::registrar_fallo_previo(pool, &pendiente, &mensaje).await;
                resumen.errores += 1;
            }
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

        /* 0. [198A-1/D12c] Puntos con respuesta perdida: no reenviar si BDP ya lo aplicó.
         * Va antes del armado porque solo consulta (GetPoints), no escribe. */
        if pendiente.dominio == DOMINIO_CLIENTE_PUNTOS && pendiente.operacion == OPERACION_PUNTOS {
            if let Some(estado) = resolver_intento_previo(pool, user_id, client, pendiente).await? {
                return Ok(estado);
            }
        }

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

        /* 4b. [D26g] Libro de stock: S antes de escribir y bloqueo por artículo
         * y almacén. Si el par ya tiene una operación pendiente, `abrir` falla y
         * no se envía nada; el armado se compensa para que ese error no deje una
         * escritura autorizada colgando del siguiente intento. */
        let libro = match ledger::preparar(config, client, pendiente, &payload).await {
            Ok(preparado) => preparado,
            Err(error) => return Err(error),
        };
        let libro = match libro {
            None => None,
            Some(preparado) => match preparado.abrir(pool, user_id, &pendiente.operacion).await {
                Ok(libro) => Some(libro),
                Err(error) => {
                    let error = format!("{error} (fila de la cola {})", pendiente.id);
                    return Err(compensar_armado(
                        pool,
                        user_id,
                        config,
                        scope,
                        pendiente,
                        entity_uuid,
                        error,
                    )
                    .await);
                }
            },
        };

        /* 5. Dispatcher -> HTTP de escritura (el cliente valida forma de URL).
         * `payload` es deliberadamente el mismo valor que se auditó arriba. */
        match Self::dispatch(client, &pendiente.dominio, &pendiente.operacion, &payload).await {
            Ok(respuesta) => {
                /* El libro se cierra antes de dar la fila por buena: si se cae el
                 * proceso entre ambos, la huérfana se libera sola en 2 h. */
                if let Some(libro) = &libro {
                    ledger::tras_exito(pool, user_id, libro).await?;
                }
                registrar_exito(pool, pendiente, audit_id, &respuesta).await
            }
            Err(error) => {
                registrar_fallo(
                    pool,
                    user_id,
                    client,
                    pendiente,
                    audit_id,
                    &error,
                    libro.as_ref(),
                )
                .await
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

#[cfg(test)]
mod tests_permiso {
    use super::*;
    use crate::services::bdp_push::constantes::ESTADO_PENDIENTE;
    use crate::services::fijar_permiso;
    use rust_decimal::Decimal;
    use serde_json::json;

    /* Cuenta en modo BDP con escritura automática: el único filtro que queda es el permiso. */
    async fn usuario_automatico(pool: &PgPool) -> Uuid {
        let user_id = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(format!("{user_id}@test.local"))
            .bind("x")
            .execute(pool)
            .await
            .unwrap();
        ConfiguracionService::obtener(pool, user_id).await.unwrap();
        sqlx::query(
            "UPDATE configuracion_restaurante SET modo_operacion = 'bdp', bdp_sync_enabled = TRUE, \
             bdp_base_url = 'http://127.0.0.1:1', bdp_sync_mode = 'automatic', \
             push_modalidad = 'automatico' WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        user_id
    }

    /* (estado, reintentos, ultimo_error) de la fila de puntos del cliente. */
    async fn fila(pool: &PgPool, user_id: Uuid) -> (String, i32, Option<String>) {
        sqlx::query_as(
            "SELECT estado, reintentos, ultimo_error FROM bdp_push_pendientes \
             WHERE user_id = $1 AND dominio = $2",
        )
        .bind(user_id)
        .bind(DOMINIO_CLIENTE_PUNTOS)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    /* [26AA-3] Categoría apagada: la fila espera con su motivo sin gastar reintentos
     * ni enviar nada; al encenderla, el mismo flush ya la procesa. */
    #[sqlx::test(migrations = "./migrations")]
    async fn categoria_apagada_deja_la_fila_en_espera_con_motivo(pool: PgPool) {
        let user_id = usuario_automatico(&pool).await;
        BdpPushService::acumular_puntos(
            &pool,
            user_id,
            "cli-1",
            Decimal::ONE,
            &json!({ "PointsAdded": "1" }),
        )
        .await
        .unwrap();

        let resumen = BdpPushFlushService::flush(&pool, user_id, false)
            .await
            .unwrap();
        assert_eq!(resumen.bloqueados_permiso, 1);
        assert_eq!(resumen.procesados, 0);
        let (estado, reintentos, error) = fila(&pool, user_id).await;
        assert_eq!(estado, ESTADO_PENDIENTE);
        assert_eq!(reintentos, 0);
        assert!(error.unwrap().contains("propinas_puntos"));

        fijar_permiso(&pool, user_id, "propinas_puntos", true)
            .await
            .unwrap();
        let resumen = BdpPushFlushService::flush(&pool, user_id, false)
            .await
            .unwrap();
        assert_eq!(resumen.bloqueados_permiso, 0);
        assert_eq!(resumen.procesados, 1);
    }
}
