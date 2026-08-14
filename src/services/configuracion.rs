/* [263A-17] Servicio de configuración del restaurante.
 * Orquestra obtención y actualización de la config. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{ActualizarConfiguracionRequest, ConfiguracionRestaurante};
use crate::repositories::ConfiguracionRepository;

pub struct ConfiguracionService;

type Repo = ConfiguracionRepository;

impl ConfiguracionService {
    pub async fn obtener(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<ConfiguracionRestaurante, AppError> {
        let config = Repo::obtener_o_crear(pool, user_id).await?;
        Ok(config)
    }

    pub async fn actualizar(
        pool: &PgPool,
        user_id: Uuid,
        req: &ActualizarConfiguracionRequest,
    ) -> Result<ConfiguracionRestaurante, AppError> {
        /* [F3] Validar bdp_sync_mode si se proporciona */
        if let Some(ref mode) = req.bdp_sync_mode {
            let valid_modes = ["read_only", "unidirectional"];
            if !valid_modes.contains(&mode.as_str()) {
                return Err(AppError::Validation(format!(
                    "bdp_sync_mode inválido: '{mode}'. Valores permitidos: {}",
                    valid_modes.join(", ")
                )));
            }
        }

        /* [128A-1/F1/M1] Validar el switch maestro modo_operacion.
         * Guard de coherencia: 'standalone' y 'bdp' fuerzan la interpretación
         * de bdp_sync_enabled según el modo (M1); el valor se persiste tal cual
         * y los consumidores derivan el modo efectivo desde ServicioModoOperacion. */
        if let Some(ref modo) = req.modo_operacion {
            let valid_modos = ["auto", "standalone", "bdp"];
            if !valid_modos.contains(&modo.as_str()) {
                return Err(AppError::Validation(format!(
                    "modo_operacion inválido: '{modo}'. Valores permitidos: {}",
                    valid_modos.join(", ")
                )));
            }
        }

        /* [128A-1/F4/D4] Validar anulacion_modalidad si se proporciona. */
        if let Some(ref modalidad) = req.anulacion_modalidad {
            let valid_modalidades = ["credito_completo", "estado_solo"];
            if !valid_modalidades.contains(&modalidad.as_str()) {
                return Err(AppError::Validation(format!(
                    "anulacion_modalidad inválida: '{modalidad}'. Valores permitidos: {}",
                    valid_modalidades.join(", ")
                )));
            }
        }

        /* [128A-1/F8/D8] Validar permisos operativos si se proporcionan. */
        for (campo, valor) in [
            (
                "permisos_catalogo_edicion",
                req.permisos_catalogo_edicion.as_deref(),
            ),
            (
                "permisos_stock_ajuste",
                req.permisos_stock_ajuste.as_deref(),
            ),
            (
                "permisos_albaranes_gestion",
                req.permisos_albaranes_gestion.as_deref(),
            ),
            (
                "permisos_anulacion_ventas",
                req.permisos_anulacion_ventas.as_deref(),
            ),
            (
                "permisos_pagos_locales",
                req.permisos_pagos_locales.as_deref(),
            ),
            (
                "permisos_facturacion_local",
                req.permisos_facturacion_local.as_deref(),
            ),
        ] {
            if let Some(valor) = valor {
                if !crate::services::NivelPermiso::VALORES.contains(&valor) {
                    return Err(AppError::Validation(format!(
                        "{campo} inválido: '{valor}'. Valores permitidos: {}",
                        crate::services::NivelPermiso::VALORES.join(", ")
                    )));
                }
            }
        }

        /* Asegurar que existe antes de actualizar */
        Repo::obtener_o_crear(pool, user_id).await?;
        let config = Repo::actualizar(pool, user_id, req).await?;
        Ok(config)
    }
}
