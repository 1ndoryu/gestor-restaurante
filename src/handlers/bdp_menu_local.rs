/* [128A-1/F7] Handlers de menús/packs locales (D2, §4.10).
 * CRUD local 100% operativo sin BDP. No depende de feature flags (M12): los
 * menús/packs locales son una capacidad standalone, siempre disponible.
 *
 * GET    /api/bdp/menus-locales     — listar con filtros
 * POST   /api/bdp/menus-locales     — crear
 * GET    /api/bdp/menus-locales/:id — detalle con líneas
 * PUT    /api/bdp/menus-locales/:id — actualizar (COALESCE + reemplazo líneas)
 * DELETE /api/bdp/menus-locales/:id — eliminar
 */

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarBdpMenuLocalRequest, BdpMenuLocalConLineas, BdpMenuLocalLineaRequest,
    BdpMenuLocalListParams, CrearBdpMenuLocalRequest,
};
use crate::repositories::BdpMenuLocalRepository;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bdp/menus-locales",
            get(listar_menus_locales).post(crear_menu_local),
        )
        .route(
            "/bdp/menus-locales/:id",
            get(obtener_menu_local)
                .put(actualizar_menu_local)
                .delete(eliminar_menu_local),
        )
}

/// Listar menús/packs locales con filtros opcionales.
#[utoipa::path(
    get,
    path = "/api/bdp/menus-locales",
    tag = "BDP Menús Locales",
    params(
        ("tipo" = Option<String>, Query, description = "Filtro por tipo: menu | pack"),
        ("activo" = Option<bool>, Query, description = "Filtro por estado activo"),
        ("busqueda" = Option<String>, Query, description = "Búsqueda por nombre o descripción")
    ),
    responses(
        (status = 200, description = "Lista de menús/packs locales", body = [BdpMenuLocalConLineas]),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_menus_locales(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<BdpMenuLocalListParams>,
) -> Result<Json<Vec<BdpMenuLocalConLineas>>, AppError> {
    let menus = BdpMenuLocalRepository::listar(&state.pool, auth.user_id, &params).await?;
    Ok(Json(menus))
}

/// Crear un menú/pack local (F7). Funciona sin BDP y sin gate de flags.
#[utoipa::path(
    post,
    path = "/api/bdp/menus-locales",
    tag = "BDP Menús Locales",
    request_body = CrearBdpMenuLocalRequest,
    responses(
        (status = 200, description = "Menú/pack local creado", body = BdpMenuLocalConLineas),
        (status = 400, description = "Validación fallida", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_menu_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearBdpMenuLocalRequest>,
) -> Result<Json<BdpMenuLocalConLineas>, AppError> {
    validar_tipo(&req.tipo)?;
    validar_nombre(&req.nombre)?;
    if req.precio.is_some_and(|p| p < Decimal::ZERO) {
        return Err(AppError::Validation(
            "El precio no puede ser negativo".into(),
        ));
    }
    validar_lineas(&req.lineas)?;

    let menu = BdpMenuLocalRepository::crear(&state.pool, auth.user_id, &req)
        .await
        .map_err(map_error_unique)?;
    tracing::info!(
        "[128A-1/F7] Menú/pack local {} ('{}', {}) creado por usuario {}",
        menu.id,
        menu.nombre,
        menu.tipo.as_str(),
        auth.user_id
    );
    Ok(Json(menu))
}

/// Obtener un menú/pack local con sus líneas.
#[utoipa::path(
    get,
    path = "/api/bdp/menus-locales/{id}",
    tag = "BDP Menús Locales",
    params(("id" = Uuid, Path, description = "ID del menú/pack")),
    responses(
        (status = 200, description = "Menú/pack local", body = BdpMenuLocalConLineas),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "No encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn obtener_menu_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BdpMenuLocalConLineas>, AppError> {
    let menu = BdpMenuLocalRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Menú/pack local no encontrado".into()))?;
    Ok(Json(menu))
}

/// Actualizar un menú/pack local (F7).
#[utoipa::path(
    put,
    path = "/api/bdp/menus-locales/{id}",
    tag = "BDP Menús Locales",
    request_body = ActualizarBdpMenuLocalRequest,
    params(("id" = Uuid, Path, description = "ID del menú/pack")),
    responses(
        (status = 200, description = "Menú/pack actualizado", body = BdpMenuLocalConLineas),
        (status = 400, description = "Validación fallida", body = ErrorResponse),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "No encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_menu_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarBdpMenuLocalRequest>,
) -> Result<Json<BdpMenuLocalConLineas>, AppError> {
    if let Some(ref tipo) = req.tipo {
        validar_tipo(tipo)?;
    }
    if let Some(ref nombre) = req.nombre {
        validar_nombre(nombre)?;
    }
    if req.precio.is_some_and(|p| p < Decimal::ZERO) {
        return Err(AppError::Validation(
            "El precio no puede ser negativo".into(),
        ));
    }
    if let Some(ref lineas) = req.lineas {
        validar_lineas(lineas)?;
    }

    let ok = BdpMenuLocalRepository::actualizar(&state.pool, id, auth.user_id, &req)
        .await
        .map_err(map_error_unique)?;
    if !ok {
        return Err(AppError::NotFound("Menú/pack local no encontrado".into()));
    }

    let updated = BdpMenuLocalRepository::find_by_id(&state.pool, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Menú/pack local no encontrado".into()))?;
    tracing::info!(
        "[128A-1/F7] Menú/pack local {} actualizado por usuario {}",
        id,
        auth.user_id
    );
    Ok(Json(updated))
}

/// Eliminar un menú/pack local (F7).
#[utoipa::path(
    delete,
    path = "/api/bdp/menus-locales/{id}",
    tag = "BDP Menús Locales",
    params(("id" = Uuid, Path, description = "ID del menú/pack")),
    responses(
        (status = 200, description = "Menú/pack eliminado", body = serde_json::Value),
        (status = 401, description = "No autorizado", body = ErrorResponse),
        (status = 404, description = "No encontrado", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar_menu_local(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ok = BdpMenuLocalRepository::eliminar(&state.pool, id, auth.user_id).await?;
    if !ok {
        return Err(AppError::NotFound("Menú/pack local no encontrado".into()));
    }
    tracing::info!(
        "[128A-1/F7] Menú/pack local {} eliminado por usuario {}",
        id,
        auth.user_id
    );
    Ok(Json(
        serde_json::json!({ "mensaje": "Menú/pack eliminado" }),
    ))
}

/* ── Validaciones ────────────────────────────────────────────────────── */

fn validar_tipo(tipo: &str) -> Result<(), AppError> {
    if !matches!(tipo, "menu" | "pack") {
        return Err(AppError::Validation(
            "El tipo debe ser 'menu' o 'pack'".into(),
        ));
    }
    Ok(())
}

fn validar_nombre(nombre: &str) -> Result<(), AppError> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(AppError::Validation("El nombre es obligatorio".into()));
    }
    if nombre.chars().count() > 200 {
        return Err(AppError::Validation(
            "El nombre no puede superar los 200 caracteres".into(),
        ));
    }
    Ok(())
}

fn validar_lineas(lineas: &[BdpMenuLocalLineaRequest]) -> Result<(), AppError> {
    if lineas.is_empty() {
        return Err(AppError::Validation(
            "El menú/pack debe tener al menos una línea (artículo)".into(),
        ));
    }
    for linea in lineas {
        if linea.descripcion.trim().is_empty() {
            return Err(AppError::Validation(
                "Cada línea debe tener una descripción".into(),
            ));
        }
        if linea.descripcion.chars().count() > 500 {
            return Err(AppError::Validation(
                "La descripción de una línea no puede superar 500 caracteres".into(),
            ));
        }
        if linea
            .articulo_codigo
            .as_deref()
            .is_some_and(|c| c.chars().count() > 100)
        {
            return Err(AppError::Validation(
                "El código de artículo no puede superar 100 caracteres".into(),
            ));
        }
        if linea.cantidad.is_some_and(|c| c <= Decimal::ZERO) {
            return Err(AppError::Validation(
                "La cantidad de cada línea debe ser mayor que 0".into(),
            ));
        }
        if linea.precio_unitario.is_some_and(|p| p < Decimal::ZERO) {
            return Err(AppError::Validation(
                "El precio de cada línea no puede ser negativo".into(),
            ));
        }
    }
    Ok(())
}

/// Mapea violaciones del `UNIQUE(user_id, tipo, nombre)` a un 409 legible.
fn map_error_unique(e: sqlx::Error) -> AppError {
    let es_duplicado = e
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .is_some_and(|c| c == "23505");
    if es_duplicado {
        AppError::Conflict("Ya existe un menú/pack con ese nombre y tipo".into())
    } else {
        AppError::from(e)
    }
}
