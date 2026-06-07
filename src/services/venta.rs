/* 253A-5: Servicio de ventas — lógica de negocio
 * [064A-5] Añadido hook post-create/update para sincronización con Haddock POS API
 * [065A-5] Añadido hook post-create/update para sincronización con BDP WebLink */

use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{ActualizarVentaRequest, CrearVentaRequest, Venta, VentasPaginadas};
use crate::repositories::venta::{ActualizarVentaData, NuevaVenta};
use crate::repositories::{ConfiguracionRepository, VentaRepository};

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

        let venta = VentaRepository::create(pool, &data).await?;

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

        let venta = VentaRepository::update(pool, &data)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

        /* [064A-5] Re-sincronizar con Haddock tras actualización */
        Self::spawn_haddock_sync(pool.clone(), user_id, venta.clone(), true);
        /* [065A-5] Re-sincronizar con BDP tras actualización */
        Self::spawn_bdp_sync(pool.clone(), user_id, venta.clone(), true);

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
     * El cliente pide explícitamente bloquear. */
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

        if !VentaRepository::delete(pool, id, user_id).await? {
            return Err(AppError::NotFound("Venta no encontrada".into()));
        }
        Ok(())
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
            BdpSyncService::sync_venta(&pool, &venta, &config, is_update).await;
        });
    }

    /* [065A-5] Retry manual de sincronización BDP.
     * Ejecuta sincrónicamente y retorna la venta actualizada. */
    pub async fn retry_bdp_sync(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Venta, AppError> {
        let venta = VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada".into()))?;

        let config = ConfiguracionRepository::obtener_o_crear(pool, user_id).await?;
        if !config.bdp_sync_enabled {
            return Err(AppError::Validation(
                "La sincronización con BDP no está habilitada.".into(),
            ));
        }

        BdpSyncService::sync_venta(pool, &venta, &config, false).await;

        VentaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Venta no encontrada tras BDP sync".into()))
    }
}
