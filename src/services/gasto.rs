/* 253A-5: Servicio de gastos */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    ActualizarGastoRequest, CategoriaGasto, CrearGastoRequest, Gasto, GastosPaginados,
};
use crate::repositories::gasto::{ActualizarGastoData, FiltrosGasto, NuevoGasto};
use crate::repositories::{CategoriaGastoRepository, GastoRepository};

pub struct GastoService;

impl GastoService {
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        req: CrearGastoRequest,
    ) -> Result<Gasto, AppError> {
        let tipo_doc = serde_json::to_value(&req.tipo_documento)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "ticket".into());
        /* 253A-21: metodo_pago es opcional — se almacena cadena vacía si no se proporciona */
        let metodo = req
            .metodo_pago
            .as_ref()
            .and_then(|m| serde_json::to_value(m).ok())
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        let proveedor = req.proveedor.as_deref().unwrap_or("");
        let numero_documento = req.numero_documento.as_deref().unwrap_or("");
        let data = NuevoGasto {
            user_id,
            fecha: req.fecha,
            proveedor,
            categoria_id: req.categoria_id,
            tipo_documento: &tipo_doc,
            metodo_pago: &metodo,
            numero_documento,
            recurrente: req.recurrente.unwrap_or(false),
            importe_base: req.importe_base,
            importe_iva: req.importe_iva,
        };

        let gasto = GastoRepository::create(pool, &data).await?;
        Ok(gasto)
    }

    pub async fn get(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Gasto, AppError> {
        GastoRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Gasto no encontrado".into()))
    }

    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        filtros: FiltrosGasto<'_>,
    ) -> Result<GastosPaginados, AppError> {
        let (items, total) = GastoRepository::list(pool, user_id, filtros).await?;
        Ok(GastosPaginados {
            items,
            total,
            page: filtros.page,
            per_page: filtros.per_page,
        })
    }

    /* [283A-22] Actualizar parcialmente un gasto.
     * Convierte enums a string igual que en create para mantener consistencia. */
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: ActualizarGastoRequest,
    ) -> Result<Gasto, AppError> {
        let tipo_doc = req.tipo_documento.as_ref().and_then(|t| {
            serde_json::to_value(t)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
        });
        let metodo = req.metodo_pago.as_ref().and_then(|m| {
            serde_json::to_value(m)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
        });

        let data = ActualizarGastoData {
            id,
            user_id,
            fecha: req.fecha,
            proveedor: req.proveedor.as_deref(),
            categoria_id: req.categoria_id,
            tipo_documento: tipo_doc.as_deref(),
            metodo_pago: metodo.as_deref(),
            numero_documento: req.numero_documento.as_deref(),
            recurrente: req.recurrente,
            importe_base: req.importe_base,
            importe_iva: req.importe_iva,
        };

        GastoRepository::update(pool, &data)
            .await?
            .ok_or_else(|| AppError::NotFound("Gasto no encontrado".into()))
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        if !GastoRepository::delete(pool, id, user_id).await? {
            return Err(AppError::NotFound("Gasto no encontrado".into()));
        }
        Ok(())
    }

    pub async fn categorias(pool: &PgPool) -> Result<Vec<CategoriaGasto>, AppError> {
        let cats = CategoriaGastoRepository::list_all(pool).await?;
        Ok(cats)
    }

    /* [044A-10] Proveedores únicos para autocomplete */
    pub async fn proveedores(
        pool: &PgPool,
        user_id: Uuid,
        busqueda: Option<String>,
    ) -> Result<Vec<String>, AppError> {
        let items = GastoRepository::proveedores_unicos(pool, user_id, busqueda.as_deref()).await?;
        Ok(items)
    }
}
