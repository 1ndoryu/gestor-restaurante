/* 263A-1: Servicio de clientes — CRM con búsqueda y paginación. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    ActualizarClienteRequest, Cliente, ClientesPaginados, ClientesQuery, CrearClienteRequest,
    MergeClientesRequest, MergeClientesResponse,
};
use crate::repositories::cliente::{ActualizarClienteData, NuevoCliente};
use crate::repositories::ClienteRepository;

pub struct ClienteService;

impl ClienteService {
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        req: CrearClienteRequest,
    ) -> Result<Cliente, AppError> {
        let data = NuevoCliente {
            user_id,
            nombre: &req.nombre,
            apellidos: req.apellidos.as_deref().unwrap_or(""),
            telefono: req.telefono.as_deref().unwrap_or(""),
            prefijo_telefono: req.prefijo_telefono.as_deref().unwrap_or("+34"),
            email: req.email.as_deref().unwrap_or(""),
            empresa: req.empresa.as_deref().unwrap_or(""),
            notas: req.notas.as_deref().unwrap_or(""),
            foto_url: req.foto_url.as_deref().unwrap_or(""),
            consentimiento_comercial_email: req.consentimiento_comercial_email.unwrap_or(false),
            consentimiento_comercial_sms: req.consentimiento_comercial_sms.unwrap_or(false),
            enviar_encuestas: req.enviar_encuestas.unwrap_or(false),
            alergias: req.alergias.as_deref().unwrap_or(""),
            preferencias_bebida: req.preferencias_bebida.as_deref().unwrap_or(""),
            preferencias_ubicacion: req.preferencias_ubicacion.as_deref().unwrap_or(""),
        };

        let cliente = ClienteRepository::create(pool, &data).await?;
        Ok(cliente)
    }

    pub async fn create_bdp_import(
        pool: &PgPool,
        user_id: Uuid,
        req: CrearClienteRequest,
        bdp_customer_code: i32,
    ) -> Result<Cliente, AppError> {
        let data = NuevoCliente {
            user_id,
            nombre: &req.nombre,
            apellidos: req.apellidos.as_deref().unwrap_or(""),
            telefono: req.telefono.as_deref().unwrap_or(""),
            prefijo_telefono: req.prefijo_telefono.as_deref().unwrap_or("+34"),
            email: req.email.as_deref().unwrap_or(""),
            empresa: req.empresa.as_deref().unwrap_or(""),
            notas: req.notas.as_deref().unwrap_or(""),
            foto_url: req.foto_url.as_deref().unwrap_or(""),
            consentimiento_comercial_email: req.consentimiento_comercial_email.unwrap_or(false),
            consentimiento_comercial_sms: req.consentimiento_comercial_sms.unwrap_or(false),
            enviar_encuestas: req.enviar_encuestas.unwrap_or(false),
            alergias: req.alergias.as_deref().unwrap_or(""),
            preferencias_bebida: req.preferencias_bebida.as_deref().unwrap_or(""),
            preferencias_ubicacion: req.preferencias_ubicacion.as_deref().unwrap_or(""),
        };
        Ok(ClienteRepository::create_bdp_import(pool, &data, bdp_customer_code).await?)
    }

    pub async fn get(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Cliente, AppError> {
        ClienteRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Cliente no encontrado".into()))
    }

    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        query: ClientesQuery,
    ) -> Result<ClientesPaginados, AppError> {
        let page = query.page;
        let per_page = query.per_page;
        let (items, total) = ClienteRepository::list(
            pool,
            user_id,
            page,
            per_page,
            query.busqueda.as_deref(),
            query.sort_by.as_deref(),
            query.sort_order.as_deref(),
        )
        .await?;
        Ok(ClientesPaginados {
            items,
            total,
            page,
            per_page,
        })
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: ActualizarClienteRequest,
    ) -> Result<Cliente, AppError> {
        let data = ActualizarClienteData {
            id,
            user_id,
            nombre: req.nombre.as_deref(),
            apellidos: req.apellidos.as_deref(),
            telefono: req.telefono.as_deref(),
            prefijo_telefono: req.prefijo_telefono.as_deref(),
            email: req.email.as_deref(),
            empresa: req.empresa.as_deref(),
            notas: req.notas.as_deref(),
            foto_url: req.foto_url.as_deref(),
            consentimiento_comercial_email: req.consentimiento_comercial_email,
            consentimiento_comercial_sms: req.consentimiento_comercial_sms,
            enviar_encuestas: req.enviar_encuestas,
            alergias: req.alergias.as_deref(),
            preferencias_bebida: req.preferencias_bebida.as_deref(),
            preferencias_ubicacion: req.preferencias_ubicacion.as_deref(),
        };

        ClienteRepository::update(pool, &data)
            .await?
            .ok_or_else(|| AppError::NotFound("Cliente no encontrado".into()))
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        if !ClienteRepository::delete(pool, id, user_id).await? {
            return Err(AppError::NotFound("Cliente no encontrado".into()));
        }
        Ok(())
    }

    /* [263A-26] Merge: absorbe origen en destino.
     * Valida que ambos existan y sean distintos antes de delegar al repo. */
    pub async fn merge(
        pool: &PgPool,
        user_id: Uuid,
        req: MergeClientesRequest,
    ) -> Result<MergeClientesResponse, AppError> {
        if req.origen_id == req.destino_id {
            return Err(AppError::Validation(
                "El cliente origen y destino no pueden ser el mismo".into(),
            ));
        }

        /* Verificar que ambos existen antes de intentar merge */
        ClienteRepository::find_by_id(pool, req.origen_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Cliente origen no encontrado".into()))?;

        ClienteRepository::find_by_id(pool, req.destino_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Cliente destino no encontrado".into()))?;

        let (cliente, reservas_migradas, etiquetas_migradas, campanas_migradas) =
            ClienteRepository::merge(pool, req.origen_id, req.destino_id, user_id).await?;

        Ok(MergeClientesResponse {
            cliente,
            reservas_migradas,
            etiquetas_migradas,
            campanas_migradas,
        })
    }
}
