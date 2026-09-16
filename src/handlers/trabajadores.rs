/* [094A-3] Handlers de trabajadores: CRUD + login.
 * Solo el propietario (sin trabajador_id en JWT) puede gestionar trabajadores.
 * Tag OpenAPI: "Trabajadores" */

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    ActualizarTrabajadorRequest, CrearTrabajadorRequest, LoginTrabajadorRequest,
    TrabajadorAuthResponse, TrabajadorResponse, SECCIONES_VALIDAS,
};
use crate::repositories::TrabajadorRepository;
use crate::services::AuthService;
use crate::AppState;

type Repo = TrabajadorRepository;

/* Helper: solo el propietario puede gestionar trabajadores */
fn require_owner(auth: &AuthUser) -> Result<(), AppError> {
    if auth.trabajador_id.is_some() {
        return Err(AppError::Forbidden(
            "Solo el propietario puede gestionar trabajadores".into(),
        ));
    }
    Ok(())
}

/* Validar que las secciones enviadas sean válidas */
fn validar_secciones(secciones: &[String]) -> Result<(), AppError> {
    for s in secciones {
        if !SECCIONES_VALIDAS.contains(&s.as_str()) {
            return Err(AppError::Validation(format!(
                "Sección inválida: '{s}'. Válidas: {}",
                SECCIONES_VALIDAS.join(", ")
            )));
        }
    }
    Ok(())
}

/* ========== CRUD ========== */

#[utoipa::path(
    get,
    path = "/api/trabajadores",
    tag = "Trabajadores",
    responses(
        (status = 200, description = "Lista de trabajadores", body = Vec<TrabajadorResponse>),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<TrabajadorResponse>>, AppError> {
    require_owner(&auth)?;
    let trabajadores = Repo::list(&state.pool, auth.user_id).await?;
    let mut result = Vec::with_capacity(trabajadores.len());
    for t in trabajadores {
        let permisos = Repo::obtener_permisos(&state.pool, t.id).await?;
        result.push(TrabajadorResponse {
            id: t.id,
            nombre: t.nombre,
            email: t.email,
            cargo: t.cargo,
            activo: t.activo,
            permisos,
            created_at: t.created_at,
        });
    }
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/trabajadores",
    tag = "Trabajadores",
    request_body = CrearTrabajadorRequest,
    responses(
        (status = 201, description = "Trabajador creado", body = TrabajadorResponse),
        (status = 401, description = "No autorizado"),
        (status = 409, description = "Email duplicado"),
        (status = 422, description = "Error de validación")
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CrearTrabajadorRequest>,
) -> Result<(StatusCode, Json<TrabajadorResponse>), AppError> {
    require_owner(&auth)?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if let Some(ref permisos) = req.permisos {
        validar_secciones(permisos)?;
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Error hasheando contraseña: {e}")))?
        .to_string();

    let trabajador = Repo::create(
        &state.pool,
        auth.user_id,
        &req.nombre,
        &req.email,
        &password_hash,
        req.cargo.as_deref().unwrap_or(""),
    )
    .await
    .map_err(|e| {
        /* [134A-31] Detectar email duplicado por SQLSTATE 23505 (unique_violation),
         * no solo por texto — los mensajes pueden estar localizados. */
        let es_duplicado = e
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .is_some_and(|c| c == "23505")
            || e.to_string().contains("duplicate key")
            || e.to_string().contains("unique");
        if es_duplicado {
            AppError::Conflict("Ya existe un trabajador con ese email".into())
        } else {
            AppError::from(e)
        }
    })?;

    if let Some(ref permisos) = req.permisos {
        Repo::set_permisos(&state.pool, trabajador.id, permisos).await?;
    } else {
        /* [169A-3] Sin `permisos` explícitos → defecto de operativa diaria.
         * `Some(vec![])` explícito significa "sin acceso a nada". */
        let defecto: Vec<String> = crate::models::PERMISOS_DEFECTO_TRABAJADOR
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        Repo::set_permisos(&state.pool, trabajador.id, &defecto).await?;
    }

    let permisos = Repo::obtener_permisos(&state.pool, trabajador.id).await?;

    Ok((
        StatusCode::CREATED,
        Json(TrabajadorResponse {
            id: trabajador.id,
            nombre: trabajador.nombre,
            email: trabajador.email,
            cargo: trabajador.cargo,
            activo: trabajador.activo,
            permisos,
            created_at: trabajador.created_at,
        }),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/trabajadores/{id}",
    tag = "Trabajadores",
    request_body = ActualizarTrabajadorRequest,
    params(("id" = Uuid, Path, description = "ID del trabajador")),
    responses(
        (status = 200, description = "Trabajador actualizado", body = TrabajadorResponse),
        (status = 401, description = "No autorizado"),
        (status = 404, description = "Trabajador no encontrado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarTrabajadorRequest>,
) -> Result<Json<TrabajadorResponse>, AppError> {
    require_owner(&auth)?;
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if let Some(ref permisos) = req.permisos {
        validar_secciones(permisos)?;
    }

    let password_hash = match req.password {
        Some(ref pw) => {
            let salt = SaltString::generate(&mut OsRng);
            Some(
                Argon2::default()
                    .hash_password(pw.as_bytes(), &salt)
                    .map_err(|e| AppError::Internal(format!("Error hasheando contraseña: {e}")))?
                    .to_string(),
            )
        }
        None => None,
    };

    let trabajador = Repo::update(
        &state.pool,
        id,
        auth.user_id,
        req.nombre.as_deref(),
        req.email.as_deref(),
        password_hash.as_deref(),
        req.cargo.as_deref(),
        req.activo,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Trabajador no encontrado".into()))?;

    if let Some(ref permisos) = req.permisos {
        Repo::set_permisos(&state.pool, trabajador.id, permisos).await?;
    }

    let permisos = Repo::obtener_permisos(&state.pool, trabajador.id).await?;

    Ok(Json(TrabajadorResponse {
        id: trabajador.id,
        nombre: trabajador.nombre,
        email: trabajador.email,
        cargo: trabajador.cargo,
        activo: trabajador.activo,
        permisos,
        created_at: trabajador.created_at,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/trabajadores/{id}",
    tag = "Trabajadores",
    params(("id" = Uuid, Path, description = "ID del trabajador")),
    responses(
        (status = 204, description = "Trabajador eliminado"),
        (status = 404, description = "Trabajador no encontrado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn eliminar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    require_owner(&auth)?;
    let deleted = Repo::delete(&state.pool, id, auth.user_id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("Trabajador no encontrado".into()))
    }
}

/* ========== Login de trabajador ========== */

#[utoipa::path(
    post,
    path = "/api/auth/login-trabajador",
    tag = "Trabajadores",
    request_body = LoginTrabajadorRequest,
    responses(
        (status = 200, description = "Login exitoso", body = TrabajadorAuthResponse),
        (status = 401, description = "Credenciales inválidas")
    )
)]
pub async fn login_trabajador(
    State(state): State<AppState>,
    Json(req): Json<LoginTrabajadorRequest>,
) -> Result<Json<TrabajadorAuthResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let trabajador = Repo::find_by_email(&state.pool, &req.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let parsed_hash = PasswordHash::new(&trabajador.password_hash)
        .map_err(|e| AppError::Internal(format!("Hash inválido: {e}")))?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized)?;

    /* [094A-3] El token tiene sub=user_id (propietario) + tid=trabajador_id.
     * [169A-3] Además firma las secciones concedidas (`secs`) para que la UI
     * filtre el menú sin endpoint extra. */
    let permisos = Repo::secciones_permitidas(&state.pool, trabajador.id).await?;

    let token = AuthService::generate_token_with_secciones(
        trabajador.user_id,
        Some(trabajador.id),
        permisos.clone(),
        &state.jwt_secret,
    )?;

    Ok(Json(TrabajadorAuthResponse {
        token,
        trabajador_id: trabajador.id,
        user_id: trabajador.user_id,
        nombre: trabajador.nombre,
        permisos,
    }))
}

/* ========== Secciones disponibles ========== */

#[utoipa::path(
    get,
    path = "/api/trabajadores/secciones",
    tag = "Trabajadores",
    responses(
        (status = 200, description = "Lista de secciones", body = Vec<String>),
        (status = 401, description = "No autorizado")
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar_secciones(_auth: AuthUser) -> Json<Vec<&'static str>> {
    Json(SECCIONES_VALIDAS.to_vec())
}

/* ========== Router ========== */

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/trabajadores", get(listar).post(crear))
        .route("/trabajadores/secciones", get(listar_secciones))
        .route("/trabajadores/:id", patch(actualizar).delete(eliminar))
        /* Login público (sin auth) — se monta en el router de auth */
        .route("/auth/login-trabajador", post(login_trabajador))
}
