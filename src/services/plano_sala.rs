/* [263A-14] Servicio del plano de sala: orquestra CRUD zonas/mesas/combinaciones,
 * plano completo, export/import JSON. */

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    ActualizarMesaRequest, ActualizarZonaRequest, CombinacionConMesas, CombinacionExport,
    CombinacionMesas, CrearCombinacionRequest, CrearMesaRequest, CrearZonaRequest, Mesa,
    MesaExport, MesaOcupacion, ParedExport, PlanoExport, PlanoOcupacion, PlanoSala, ReservaMesa,
    ZonaConMesas, ZonaExport, ZonaOcupacion, ZonaSala,
};
use crate::repositories::PlanoSalaRepository;

pub struct PlanoSalaService;

type Repo = PlanoSalaRepository;

impl PlanoSalaService {
    /* ========== Plano completo ========== */

    pub async fn plano_completo(pool: &PgPool, user_id: Uuid) -> Result<PlanoSala, AppError> {
        let zonas = Repo::listar_zonas(pool, user_id).await?;
        let mut zonas_con_mesas = Vec::with_capacity(zonas.len());

        for zona in zonas {
            let mesas = Repo::listar_mesas_zona(pool, zona.id).await?;
            /* [094A-7] Cargar paredes de cada zona */
            let paredes = Repo::listar_paredes_zona(pool, zona.id).await?;
            zonas_con_mesas.push(ZonaConMesas {
                zona,
                mesas,
                paredes,
            });
        }

        let combos = Repo::listar_combinaciones(pool, user_id).await?;
        let mut combinaciones = Vec::with_capacity(combos.len());

        for combo in combos {
            let mesas = Repo::mesas_de_combinacion(pool, combo.id).await?;
            combinaciones.push(CombinacionConMesas {
                combinacion: combo,
                mesas,
            });
        }

        Ok(PlanoSala {
            zonas: zonas_con_mesas,
            combinaciones,
        })
    }

    /* ========== Zonas ========== */

    pub async fn crear_zona(
        pool: &PgPool,
        user_id: Uuid,
        req: CrearZonaRequest,
    ) -> Result<ZonaSala, AppError> {
        let zona = Repo::crear_zona(
            pool,
            user_id,
            &req.nombre,
            req.orden.unwrap_or(0),
            req.ancho.unwrap_or(800),
            req.alto.unwrap_or(600),
        )
        .await?;
        Ok(zona)
    }

    pub async fn actualizar_zona(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: ActualizarZonaRequest,
    ) -> Result<ZonaSala, AppError> {
        Repo::actualizar_zona(
            pool,
            id,
            user_id,
            req.nombre.as_deref(),
            req.orden,
            req.ancho,
            req.alto,
        )
        .await
        .map_err(|_| AppError::NotFound("Zona no encontrada".into()))
    }

    pub async fn eliminar_zona(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        if !Repo::eliminar_zona(pool, id, user_id).await? {
            return Err(AppError::NotFound("Zona no encontrada".into()));
        }
        Ok(())
    }

    /* ========== Mesas ========== */

    pub async fn crear_mesa(pool: &PgPool, req: CrearMesaRequest) -> Result<Mesa, AppError> {
        /* [283A-17+283A-24] Catch unique constraint (zona_id, numero) para devolver
         * Conflict en vez de 500 cuando el número de mesa ya existe en la zona.
         * Se chequea code 23505 + fallback por mensaje para robustez ante diferencias de driver. */
        Repo::crear_mesa(pool, &req).await.map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                let is_unique = db_err.code().as_deref() == Some("23505")
                    || db_err.message().contains("duplicate key")
                    || db_err.message().contains("unique constraint");
                if is_unique {
                    return AppError::Conflict(format!(
                        "Ya existe una mesa con el número {} en esta zona",
                        req.numero
                    ));
                }
            }
            AppError::Database(e)
        })
    }

    pub async fn actualizar_mesa(
        pool: &PgPool,
        id: Uuid,
        req: ActualizarMesaRequest,
    ) -> Result<Mesa, AppError> {
        Repo::actualizar_mesa(pool, id, &req)
            .await
            .map_err(|_| AppError::NotFound("Mesa no encontrada".into()))
    }

    pub async fn eliminar_mesa(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        if !Repo::eliminar_mesa(pool, id).await? {
            return Err(AppError::NotFound("Mesa no encontrada".into()));
        }
        Ok(())
    }

    pub async fn actualizar_posiciones(
        pool: &PgPool,
        posiciones: &[(Uuid, i32, i32)],
    ) -> Result<u64, AppError> {
        let total = Repo::actualizar_posiciones(pool, posiciones).await?;
        Ok(total)
    }

    /* ========== Combinaciones ========== */

    pub async fn crear_combinacion(
        pool: &PgPool,
        user_id: Uuid,
        req: CrearCombinacionRequest,
    ) -> Result<CombinacionMesas, AppError> {
        let combo = Repo::crear_combinacion(
            pool,
            user_id,
            &req.nombre,
            req.min_personas.unwrap_or(1),
            req.max_personas,
            &req.mesa_ids,
        )
        .await?;
        Ok(combo)
    }

    pub async fn eliminar_combinacion(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if !Repo::eliminar_combinacion(pool, id, user_id).await? {
            return Err(AppError::NotFound("Combinación no encontrada".into()));
        }
        Ok(())
    }

    /* ========== Export / Import ========== */

    pub async fn exportar(pool: &PgPool, user_id: Uuid) -> Result<PlanoExport, AppError> {
        let plano = Self::plano_completo(pool, user_id).await?;

        let zonas = plano
            .zonas
            .iter()
            .map(|zc| ZonaExport {
                nombre: zc.zona.nombre.clone(),
                orden: zc.zona.orden,
                ancho: zc.zona.ancho,
                alto: zc.zona.alto,
                mesas: zc
                    .mesas
                    .iter()
                    .map(|m| MesaExport {
                        numero: m.numero,
                        pos_x: m.pos_x,
                        pos_y: m.pos_y,
                        ancho: m.ancho,
                        alto: m.alto,
                        forma: m.forma.clone(),
                        min_personas: m.min_personas,
                        max_personas: m.max_personas,
                        activa: m.activa,
                    })
                    .collect(),
                /* [027B-1] Exportar paredes de la zona */
                paredes: zc
                    .paredes
                    .iter()
                    .map(|p| ParedExport {
                        pos_x: p.pos_x,
                        pos_y: p.pos_y,
                        ancho: p.ancho,
                        alto: p.alto,
                        rotacion: p.rotacion,
                        color: p.color.clone(),
                    })
                    .collect(),
            })
            .collect();

        let combinaciones = plano
            .combinaciones
            .iter()
            .map(|cc| {
                let mesas_ref = cc
                    .mesas
                    .iter()
                    .filter_map(|m| {
                        plano
                            .zonas
                            .iter()
                            .find(|z| z.zona.id == m.zona_id)
                            .map(|z| format!("{}:{}", z.zona.nombre, m.numero))
                    })
                    .collect();

                CombinacionExport {
                    nombre: cc.combinacion.nombre.clone(),
                    min_personas: cc.combinacion.min_personas,
                    max_personas: cc.combinacion.max_personas,
                    mesas_ref,
                }
            })
            .collect();

        Ok(PlanoExport {
            version: "1.0".into(),
            zonas,
            combinaciones,
        })
    }

    #[allow(clippy::too_many_lines)]
    pub async fn importar(
        pool: &PgPool,
        user_id: Uuid,
        data: PlanoExport,
    ) -> Result<PlanoSala, AppError> {
        /* Eliminar plano actual antes de importar */
        Repo::eliminar_plano_completo(pool, user_id).await?;

        /* Crear zonas y mesas */
        for zona_exp in &data.zonas {
            let zona = Repo::crear_zona(
                pool,
                user_id,
                &zona_exp.nombre,
                zona_exp.orden,
                zona_exp.ancho,
                zona_exp.alto,
            )
            .await?;

            for mesa_exp in &zona_exp.mesas {
                let req_mesa = CrearMesaRequest {
                    zona_id: zona.id,
                    numero: mesa_exp.numero,
                    pos_x: Some(mesa_exp.pos_x),
                    pos_y: Some(mesa_exp.pos_y),
                    ancho: Some(mesa_exp.ancho),
                    alto: Some(mesa_exp.alto),
                    forma: Some(mesa_exp.forma.clone()),
                    min_personas: Some(mesa_exp.min_personas),
                    max_personas: Some(mesa_exp.max_personas),
                };
                Repo::crear_mesa(pool, &req_mesa).await?;
            }

            /* [027B-1] Restaurar paredes de la zona */
            for pared_exp in &zona_exp.paredes {
                Repo::crear_pared(
                    pool,
                    zona.id,
                    pared_exp.pos_x,
                    pared_exp.pos_y,
                    pared_exp.ancho,
                    pared_exp.alto,
                    pared_exp.rotacion,
                    &pared_exp.color,
                )
                .await?;
            }
        }

        /* Recrear combinaciones resolviendo mesas_ref (zona_nombre:numero) */
        for combo_exp in &data.combinaciones {
            let mut mesa_ids = Vec::new();
            for ref_str in &combo_exp.mesas_ref {
                if let Some((zona_nombre, num_str)) = ref_str.split_once(':') {
                    if let Ok(numero) = num_str.parse::<i32>() {
                        if let Some(mesa) =
                            Repo::buscar_mesa_por_zona_numero(pool, user_id, zona_nombre, numero)
                                .await?
                        {
                            mesa_ids.push(mesa.id);
                        }
                    }
                }
            }

            if mesa_ids.len() >= 2 {
                Repo::crear_combinacion(
                    pool,
                    user_id,
                    &combo_exp.nombre,
                    combo_exp.min_personas,
                    combo_exp.max_personas,
                    &mesa_ids,
                )
                .await?;
            }
        }

        /* Retornar plano recién importado */
        Self::plano_completo(pool, user_id).await
    }

    /* ========== Ocupación (263A-16) ========== */

    /// Devuelve el plano con ocupación de mesas para una fecha y turno opcionales.
    pub async fn plano_ocupacion(
        pool: &PgPool,
        user_id: Uuid,
        fecha: chrono::NaiveDate,
        hora_desde: Option<chrono::NaiveTime>,
        hora_hasta: Option<chrono::NaiveTime>,
    ) -> Result<PlanoOcupacion, AppError> {
        let zonas = Repo::listar_zonas(pool, user_id).await?;
        let reservas_rows =
            Repo::reservas_por_mesa(pool, user_id, fecha, hora_desde, hora_hasta).await?;
        /* [014A-9] Reservas sin mesa_id vinculado, fallback por num_mesa */
        let reservas_sin_id =
            Repo::reservas_sin_mesa_id(pool, user_id, fecha, hora_desde, hora_hasta).await?;

        /* Indexar reservas por mesa_id para O(1) lookup */
        let mut reservas_por_mesa: std::collections::HashMap<Uuid, Vec<ReservaMesa>> =
            std::collections::HashMap::new();
        for r in reservas_rows {
            reservas_por_mesa
                .entry(r.mesa_id)
                .or_default()
                .push(ReservaMesa {
                    reserva_id: r.reserva_id,
                    hora: r.hora,
                    nombre_cliente: r.nombre_cliente,
                    apellidos_cliente: r.apellidos_cliente,
                    num_personas: r.num_personas,
                    estado: r.estado,
                    telefono: r.telefono,
                });
        }

        /* [014A-9] Indexar reservas sin mesa_id por num_mesa para fallback */
        let mut reservas_por_num: std::collections::HashMap<i32, Vec<ReservaMesa>> =
            std::collections::HashMap::new();
        for r in reservas_sin_id {
            reservas_por_num
                .entry(r.num_mesa)
                .or_default()
                .push(ReservaMesa {
                    reserva_id: r.reserva_id,
                    hora: r.hora,
                    nombre_cliente: r.nombre_cliente,
                    apellidos_cliente: r.apellidos_cliente,
                    num_personas: r.num_personas,
                    estado: r.estado,
                    telefono: r.telefono,
                });
        }

        let mut zonas_ocupacion = Vec::with_capacity(zonas.len());
        for zona in zonas {
            let mesas = Repo::listar_mesas_zona(pool, zona.id).await?;
            /* [134A-12] Cargar paredes para que sean visibles en la vista de reservas */
            let paredes = Repo::listar_paredes_zona(pool, zona.id).await?;
            let mesas_ocupacion = mesas
                .into_iter()
                .map(|mesa| {
                    /* Primero buscar por mesa_id, luego fallback por num_mesa [014A-9] */
                    let mut reservas = reservas_por_mesa.remove(&mesa.id).unwrap_or_default();
                    if let Some(mut por_num) = reservas_por_num.remove(&mesa.numero) {
                        reservas.append(&mut por_num);
                    }
                    MesaOcupacion { mesa, reservas }
                })
                .collect();
            zonas_ocupacion.push(ZonaOcupacion {
                zona,
                mesas: mesas_ocupacion,
                paredes,
            });
        }

        Ok(PlanoOcupacion {
            fecha,
            zonas: zonas_ocupacion,
        })
    }
}
