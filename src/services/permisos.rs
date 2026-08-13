/* [128A-1/F8] Permisos operativos por acción (D8, M17).
 *
 * Enforcement en backend: cada acción sensible consulta el nivel configurado
 * en `configuracion_restaurante.permisos_*` y el rol efectivo del usuario
 * (`effective_role`, consistente con `AuthUser::require_role`). La UI solo
 * refleja el permiso; el backend es la fuente de verdad (M17).
 *
 * Niveles:
 *   - 'admin' (default): solo el propietario (rol Admin).
 *   - 'admin_trabajador': Admin y Trabajador (todo el staff autenticado).
 *   - 'todos': cualquier usuario autenticado.
 */

use sqlx::PgPool;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{ConfiguracionRestaurante, UserRole};
use crate::services::ConfiguracionService;

/// Acciones sensibles protegidas por un permiso configurable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionPermiso {
    /// CRUD de catálogo y mapeos de artículos.
    CatalogoEdicion,
    /// Ajuste manual de stock local.
    StockAjuste,
    /// Gestión de albaranes de compra locales (CRUD, borrador, conciliación).
    AlbaranesGestion,
    /// Anulación local de ventas.
    AnulacionVentas,
}

impl AccionPermiso {
    /// Nombre de la columna de configuración que define el nivel.
    #[must_use]
    pub const fn columna(self) -> &'static str {
        match self {
            Self::CatalogoEdicion => "permisos_catalogo_edicion",
            Self::StockAjuste => "permisos_stock_ajuste",
            Self::AlbaranesGestion => "permisos_albaranes_gestion",
            Self::AnulacionVentas => "permisos_anulacion_ventas",
        }
    }

    /// Valor configurado para la acción en la configuración del restaurante.
    #[must_use]
    pub fn valor(self, config: &ConfiguracionRestaurante) -> &str {
        match self {
            Self::CatalogoEdicion => &config.permisos_catalogo_edicion,
            Self::StockAjuste => &config.permisos_stock_ajuste,
            Self::AlbaranesGestion => &config.permisos_albaranes_gestion,
            Self::AnulacionVentas => &config.permisos_anulacion_ventas,
        }
    }
}

/// Nivel de acceso configurado para una acción.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelPermiso {
    Admin,
    AdminTrabajador,
    Todos,
}

impl NivelPermiso {
    /// Valores aceptados por la API y la BD (CHECK en migración).
    pub const VALORES: [&'static str; 3] = ["admin", "admin_trabajador", "todos"];

    #[must_use]
    pub fn desde_valor(valor: &str) -> Self {
        match valor {
            "admin_trabajador" => Self::AdminTrabajador,
            "todos" => Self::Todos,
            /* Valor desconocido → fail-closed al nivel más restrictivo. */
            _ => Self::Admin,
        }
    }

    #[must_use]
    pub fn permite(self, role: UserRole) -> bool {
        match self {
            Self::Admin => role == UserRole::Admin,
            Self::AdminTrabajador => matches!(role, UserRole::Admin | UserRole::Trabajador),
            Self::Todos => true,
        }
    }
}

/// Decide si `user` puede ejecutar `accion` según la configuración.
#[must_use]
pub fn permiso_habilitado(
    config: &ConfiguracionRestaurante,
    accion: AccionPermiso,
    user: &AuthUser,
) -> bool {
    NivelPermiso::desde_valor(accion.valor(config)).permite(user.effective_role)
}

/// Guard de handler: carga la configuración y devuelve 403 si el usuario no
/// tiene el permiso configurado para la acción.
pub async fn verificar_permiso(
    pool: &PgPool,
    accion: AccionPermiso,
    user: &AuthUser,
) -> Result<(), AppError> {
    let config = ConfiguracionService::obtener(pool, user.user_id).await?;
    if permiso_habilitado(&config, accion, user) {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "No tienes permisos para esta acción".into(),
        ))
    }
}
