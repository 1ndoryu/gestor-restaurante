/* 253A-5: Servicio de reservas
263A-6: Filtros turno/estado, num_mesa, apellidos_cliente, resumen mensual
283A-4: Validación de colisiones — impide crear/actualizar reservas que excedan
capacidad (mesas/personas) o que dupliquen una mesa ya ocupada en la franja.
014A-1: Auto-venta al completar reserva (configurable).
014A-2: Upsert cliente al crear reserva. */

use chrono::{NaiveDate, NaiveTime, Timelike};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    ActualizarReservaRequest, CrearReservaRequest, NoShowStats, Reserva, ReservasConteo,
    ReservasPaginadas, ReservasQuery, ResumenDiario,
};
use crate::repositories::cliente::NuevoCliente;
use crate::repositories::reserva::{ActualizarReservaData, FiltrosReserva, NuevaReserva};
use crate::repositories::venta::NuevaVenta;
use crate::repositories::{
    ClienteRepository, ConfiguracionRepository, PlanoSalaRepository, ReservaRepository,
    VentaRepository,
};

pub struct ReservaService;

/// Duración estimada de una reserva en minutos (para calcular solapamiento)
const DURACION_RESERVA_MIN: i64 = 90;

impl ReservaService {
    /// Verifica que hay capacidad (mesas y personas) para la reserva propuesta.
    /// Si `exclude_id` es Some, excluye esa reserva del cálculo (para updates).
    /// Si `mesa_id` es Some, también verifica que esa mesa concreta está libre.
    pub async fn validar_disponibilidad(
        pool: &PgPool,
        user_id: Uuid,
        fecha: NaiveDate,
        hora: NaiveTime,
        num_personas: i32,
        mesa_id: Option<Uuid>,
        exclude_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        /* Obtener mesas activas y calcular capacidad total */
        let zonas = PlanoSalaRepository::listar_zonas(pool, user_id).await?;
        let mut todas_mesas = Vec::new();
        for zona in &zonas {
            let mesas = PlanoSalaRepository::listar_mesas_zona(pool, zona.id).await?;
            for mesa in mesas {
                if mesa.activa {
                    todas_mesas.push(mesa);
                }
            }
        }

        /* Si no hay mesas configuradas, no se puede validar capacidad — dejar pasar
         * (el restaurante aún no configuró el plano de sala). */
        if todas_mesas.is_empty() {
            return Ok(());
        }

        let total_mesas: i32 = todas_mesas.len().try_into().unwrap_or(i32::MAX);
        let capacidad_total: i32 = todas_mesas.iter().map(|m| m.max_personas).sum();

        /* [303A-3] Obtener reservas activas del día — solo estados que realmente
         * ocupan mesa (confirmada, pendiente). no_show y completada no bloquean. */
        let reservas = ReservaRepository::listar_por_fecha(pool, user_id, fecha)
            .await
            .unwrap_or_default();

        /* Calcular ocupación en la franja de la nueva reserva */
        let nueva_start = i64::from(hora.hour()) * 60 + i64::from(hora.minute());
        let nueva_end = nueva_start + DURACION_RESERVA_MIN;

        let mut personas_reservadas = 0i32;
        let mut mesas_asignadas = 0i32;

        for r in &reservas {
            /* Excluir la reserva que estamos actualizando */
            if exclude_id.is_some_and(|eid| eid == r.id) {
                continue;
            }

            /* [303A-3+303A-4] completada/no_show ya se excluyen en listar_por_fecha SQL.
             * Solo contar reservas que realmente ocupan sitio. */

            let r_start = i64::from(r.hora.hour()) * 60 + i64::from(r.hora.minute());
            let r_end = r_start + DURACION_RESERVA_MIN;

            /* Dos reservas solapan si una empieza antes de que termine la otra */
            if nueva_start < r_end && nueva_end > r_start {
                personas_reservadas += r.num_personas;

                /* [303A-3] Solo contar como "mesa ocupada" si tiene mesa_id asignado.
                 * Reservas sin mesa asignada solo cuentan para capacidad de personas. */
                if r.mesa_id.is_some() {
                    mesas_asignadas += 1;
                }

                /* Si se pide una mesa concreta y ya está ocupada en esa franja */
                if let Some(pid) = mesa_id {
                    if r.mesa_id.is_some_and(|mid| mid == pid) {
                        return Err(AppError::Conflict(
                            "La mesa seleccionada ya está ocupada en esa franja horaria".into(),
                        ));
                    }
                }
            }
        }

        /* Verificar disponibilidad de mesas físicas asignadas.
         * [DataIV-2] Solo bloquear por mesas llenas si la nueva reserva pide
         * una mesa concreta.  Reservas sin mesa especifica solo se validan
         * por capacidad de personas (siguiente check). */
        if mesa_id.is_some() && mesas_asignadas >= total_mesas {
            return Err(AppError::Conflict(format!(
                "No hay mesas disponibles en la franja de {hora}. \
                     Todas las {total_mesas} mesas están asignadas a reservas."
            )));
        }

        if personas_reservadas + num_personas > capacidad_total {
            let disponible = capacidad_total - personas_reservadas;
            return Err(AppError::Conflict(format!(
                "Capacidad insuficiente en la franja de {hora}. \
                     Disponible: {disponible} personas, solicitado: {num_personas}."
            )));
        }

        Ok(())
    }

    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        req: CrearReservaRequest,
    ) -> Result<Reserva, AppError> {
        /* [283A-4] Validar disponibilidad antes de insertar */
        Self::validar_disponibilidad(
            pool,
            user_id,
            req.fecha,
            req.hora,
            req.num_personas,
            req.mesa_id,
            None,
        )
        .await?;

        let estado = req
            .estado
            .as_ref()
            .and_then(|e| {
                serde_json::to_value(e)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
            })
            .unwrap_or_else(|| "confirmada".into());

        /* [014A-2] Upsert cliente: buscar por teléfono/email y vincular o crear */
        let telefono = req.telefono.as_deref().unwrap_or("");
        let apellidos = req.apellidos_cliente.as_deref().unwrap_or("");
        let cliente_id =
            Self::upsert_cliente(pool, user_id, &req.nombre_cliente, apellidos, telefono, "").await;

        let data = NuevaReserva {
            user_id,
            fecha: req.fecha,
            hora: req.hora,
            nombre_cliente: &req.nombre_cliente,
            num_personas: req.num_personas,
            estado: &estado,
            notas: req.notas.as_deref().unwrap_or(""),
            telefono,
            num_mesa: req.num_mesa,
            apellidos_cliente: apellidos,
            canal_id: req.canal_id,
            mesa_id: req.mesa_id,
        };

        let mut reserva = ReservaRepository::create(pool, &data).await?;

        /* [014A-2] Vincular cliente_id a la reserva si se encontró/creó */
        if let Some(cid) = cliente_id {
            let _ = sqlx::query!(
                "UPDATE reservas SET cliente_id = $1 WHERE id = $2",
                cid,
                reserva.id
            )
            .execute(pool)
            .await;
            reserva.cliente_id = Some(cid);
        }

        /* [034A-1] Si la reserva se crea directamente como "completada",
         * disparar auto-venta igual que en update. */
        if estado == "completada" {
            Self::crear_venta_automatica(pool, user_id, &reserva).await;
            /* [094A-5] Actualizar ultima_visita del cliente */
            if let Some(cid) = reserva.cliente_id {
                let _ = ClienteRepository::actualizar_ultima_visita(pool, cid).await;
            }
        }

        Ok(reserva)
    }

    pub async fn get(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Reserva, AppError> {
        ReservaRepository::find_by_id(pool, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Reserva no encontrada".into()))
    }

    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        query: &ReservasQuery,
    ) -> Result<ReservasPaginadas, AppError> {
        /* 263A-6: Mapear turno a rango horario */
        let (hora_desde, hora_hasta) = Self::turno_a_horas(query.turno.as_deref());

        /* [263A-20] Normalizar filtros vacíos a None.
         * Orval puede enviar estado="" o turno="" como query param
         * en vez de omitirlo, lo que causa que el SQL busque estado = '' */
        let estado_normalizado = query
            .estado
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(String::from);
        let busqueda_normalizada = query
            .busqueda
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(String::from);

        let filtros = FiltrosReserva {
            user_id,
            page: query.page,
            per_page: query.per_page,
            /* [303A-15] Si se envía rango (fecha_desde/fecha_hasta), ignorar `fecha` exacta
             * para no producir filtros contradictorios. */
            fecha: if query.fecha_desde.is_some() || query.fecha_hasta.is_some() {
                None
            } else {
                query.fecha
            },
            fecha_desde: query.fecha_desde,
            fecha_hasta: query.fecha_hasta,
            estado: estado_normalizado,
            hora_desde,
            hora_hasta,
            busqueda: busqueda_normalizada,
            sort_by: query.sort_by.clone(),
            sort_order: query.sort_order.clone(),
        };

        let (items, total) = ReservaRepository::list(pool, &filtros).await?;
        Ok(ReservasPaginadas {
            items,
            total,
            page: query.page,
            per_page: query.per_page,
        })
    }

    /// Convierte un turno de reserva a un rango horario
    fn turno_a_horas(turno: Option<&str>) -> (Option<NaiveTime>, Option<NaiveTime>) {
        match turno {
            Some("desayuno") => (Self::hora_literal(0, 0, 0), Self::hora_literal(12, 0, 0)),
            Some("comida") => (Self::hora_literal(12, 0, 0), Self::hora_literal(18, 0, 0)),
            Some("cena") => (Self::hora_literal(18, 0, 0), Self::hora_literal(23, 59, 0)),
            _ => (None, None),
        }
    }

    /// Hora literal del turno por defecto.
    /// `from_hms_opt` solo rechaza combinaciones que no forman una hora real; los
    /// literales de este modulo si lo son, asi que se devuelve el `Option` tal cual en
    /// lugar de entrar en panico si alguno se rompiera.
    fn hora_literal(hora: u32, minuto: u32, segundo: u32) -> Option<NaiveTime> {
        NaiveTime::from_hms_opt(hora, minuto, segundo)
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        req: ActualizarReservaRequest,
    ) -> Result<Reserva, AppError> {
        /* [283A-4] Si cambian fecha, hora, num_personas o mesa_id,
         * validar que no haya colisión con el nuevo horario. */
        let necesita_validacion = req.fecha.is_some()
            || req.hora.is_some()
            || req.num_personas.is_some()
            || req.mesa_id.is_some();

        if necesita_validacion {
            let actual = Self::get(pool, id, user_id).await?;
            let fecha = req.fecha.unwrap_or(actual.fecha);
            let hora = req.hora.unwrap_or(actual.hora);
            let personas = req.num_personas.unwrap_or(actual.num_personas);
            let mesa = req.mesa_id.or(actual.mesa_id);

            Self::validar_disponibilidad(pool, user_id, fecha, hora, personas, mesa, Some(id))
                .await?;
        }

        let estado_str = req.estado.as_ref().and_then(|e| {
            serde_json::to_value(e)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
        });

        /* [034A-4] Guardar estado anterior para evitar crear ventas duplicadas
         * cuando se re-guarda una reserva que ya es "completada". */
        let reserva_antes = Self::get(pool, id, user_id).await?;
        let estado_anterior = reserva_antes.estado.clone();

        let data = ActualizarReservaData {
            id,
            user_id,
            fecha: req.fecha,
            hora: req.hora,
            nombre_cliente: req.nombre_cliente.as_deref(),
            num_personas: req.num_personas,
            estado: estado_str.as_deref(),
            notas: req.notas.as_deref(),
            telefono: req.telefono.as_deref(),
            num_mesa: req.num_mesa,
            apellidos_cliente: req.apellidos_cliente.as_deref(),
            canal_id: req.canal_id,
            mesa_id: req.mesa_id,
        };

        let reserva = ReservaRepository::update(pool, &data)
            .await?
            .ok_or_else(|| AppError::NotFound("Reserva no encontrada".into()))?;

        /* [014A-1] Solo crear venta automática si el estado CAMBIÓ a "completada"
         * (no si ya era "completada" antes). Evita ventas duplicadas. */
        let cambio_a_completada =
            estado_str.as_deref() == Some("completada") && estado_anterior != "completada";

        /* [094A-1] Si la reserva SALE de "completada" (a cualquier otro estado),
         * eliminar las ventas auto-generadas (importe_base = 0) vinculadas. */
        let salio_de_completada = estado_str.is_some()
            && estado_str.as_deref() != Some("completada")
            && estado_anterior == "completada";

        if salio_de_completada {
            match VentaRepository::delete_by_reserva_id(pool, reserva.id, user_id).await {
                Ok(n) if n > 0 => {
                    tracing::info!(
                        "[094A-1] Eliminadas {n} ventas auto-generadas al descompletar reserva {}",
                        reserva.id
                    );
                }
                Err(e) => {
                    warn!(
                        "[094A-1] Error eliminando ventas al descompletar reserva {}: {e}",
                        reserva.id
                    );
                }
                _ => {}
            }
        }

        if cambio_a_completada {
            /* [034A-2] Si la reserva no tiene cliente vinculado, intentar crearlo ahora.
             * Cubre reservas creadas antes del fix donde upsert_cliente requería tel/email. */
            if reserva.cliente_id.is_none() {
                let cid = Self::upsert_cliente(
                    pool,
                    user_id,
                    &reserva.nombre_cliente,
                    &reserva.apellidos_cliente,
                    &reserva.telefono,
                    "",
                )
                .await;
                if let Some(cid) = cid {
                    let _ = sqlx::query!(
                        "UPDATE reservas SET cliente_id = $1 WHERE id = $2",
                        cid,
                        reserva.id
                    )
                    .execute(pool)
                    .await;
                }
            }

            Self::crear_venta_automatica(pool, user_id, &reserva).await;

            /* [094A-5] Actualizar ultima_visita del cliente al completar reserva */
            if let Some(cid) = reserva.cliente_id {
                let _ = ClienteRepository::actualizar_ultima_visita(pool, cid).await;
            }
        }

        Ok(reserva)
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        if !ReservaRepository::delete(pool, id, user_id).await? {
            return Err(AppError::NotFound("Reserva no encontrada".into()));
        }
        Ok(())
    }

    /// Conteo de reservas del mes y del día (para Home)
    pub async fn conteo(pool: &PgPool, user_id: Uuid) -> Result<ReservasConteo, AppError> {
        let (total_mes, total_hoy) = ReservaRepository::conteo_actual(pool, user_id).await?;
        Ok(ReservasConteo {
            total_mes,
            total_hoy,
        })
    }

    /// Resumen diario de un mes — para la vista calendario (263A-7)
    pub async fn resumen_mensual(
        pool: &PgPool,
        user_id: Uuid,
        anio: i32,
        mes: i32,
    ) -> Result<Vec<ResumenDiario>, AppError> {
        let datos = ReservaRepository::resumen_mensual(pool, user_id, anio, mes).await?;
        Ok(datos)
    }

    /// Estadísticas de no-shows con desglose por canal (263A-8)
    pub async fn no_show_stats(
        pool: &PgPool,
        user_id: Uuid,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
    ) -> Result<NoShowStats, AppError> {
        let (total_reservas, total_no_shows) =
            ReservaRepository::no_show_totales(pool, user_id, fecha_desde, fecha_hasta).await?;

        let ratio_porcentaje = if total_reservas > 0 {
            #[allow(clippy::cast_precision_loss)] // conteos de reservas nunca excederán 2^52
            {
                (total_no_shows as f64 / total_reservas as f64) * 100.0
            }
        } else {
            0.0
        };

        let por_canal =
            ReservaRepository::no_show_por_canal(pool, user_id, fecha_desde, fecha_hasta).await?;

        Ok(NoShowStats {
            total_reservas,
            total_no_shows,
            ratio_porcentaje,
            por_canal,
        })
    }

    /* [014A-2] Buscar o crear cliente a partir de los datos de la reserva.
     * Busca por teléfono o email. Si no existe, crea un cliente nuevo.
     * Retorna None si no hay datos suficientes para identificar al cliente. */
    async fn upsert_cliente(
        pool: &PgPool,
        user_id: Uuid,
        nombre: &str,
        apellidos: &str,
        telefono: &str,
        email: &str,
    ) -> Option<Uuid> {
        /* [034A-2] Si no hay nombre, no tiene sentido crear un cliente vacío */
        if nombre.is_empty() {
            return None;
        }

        /* Si hay teléfono o email, intentar encontrar cliente existente para deduplicar */
        if !telefono.is_empty() || !email.is_empty() {
            match ClienteRepository::find_by_telefono_o_email(pool, user_id, telefono, email).await
            {
                Ok(Some(cliente)) => return Some(cliente.id),
                Ok(None) => { /* no existe, crear nuevo */ }
                Err(e) => {
                    warn!("[014A-2] Error buscando cliente: {e}");
                    return None;
                }
            }
        }

        /* Crear cliente nuevo con datos mínimos */
        let data = NuevoCliente {
            user_id,
            nombre,
            apellidos,
            telefono,
            prefijo_telefono: "",
            email,
            empresa: "",
            notas: "",
            foto_url: "",
            consentimiento_comercial_email: false,
            consentimiento_comercial_sms: false,
            enviar_encuestas: false,
            alergias: "",
            preferencias_bebida: "",
            preferencias_ubicacion: "",
        };

        match ClienteRepository::create(pool, &data).await {
            Ok(cliente) => Some(cliente.id),
            Err(e) => {
                warn!("[014A-2] Error creando cliente: {e}");
                None
            }
        }
    }

    /* [014A-1] Crear venta automáticamente cuando una reserva se marca como completada.
     * Solo si auto_venta_reserva está habilitado en la configuración.
     * [094A-1] Verifica que no exista ya una venta con este reserva_id — evita duplicados.
     * No falla la operación si algo sale mal — solo loguea el error. */
    async fn crear_venta_automatica(pool: &PgPool, user_id: Uuid, reserva: &Reserva) {
        /* [094A-1] Verificar si ya existe una venta para esta reserva — evitar duplicados */
        match VentaRepository::exists_by_reserva_id(pool, reserva.id).await {
            Ok(true) => {
                tracing::info!(
                    "[094A-1] Ya existe venta para reserva {} — saltando creación",
                    reserva.id
                );
                return;
            }
            Err(e) => {
                warn!(
                    "[094A-1] Error verificando venta existente: {e} — continuando por seguridad"
                );
            }
            _ => {}
        }

        /* Verificar si auto_venta_reserva está habilitado */
        /* Verificar si auto_venta_reserva está habilitado */
        let config = match ConfiguracionRepository::obtener_o_crear(pool, user_id).await {
            Ok(c) => c,
            Err(e) => {
                warn!("[014A-1] Error obteniendo config para auto-venta: {e}");
                return;
            }
        };

        if !config.auto_venta_reserva {
            return;
        }

        /* Determinar turno según la hora de la reserva y la config de turnos */
        let hora = reserva.hora;
        let turno = if hora >= config.hora_desayuno_inicio && hora < config.hora_desayuno_fin {
            "manana"
        } else if hora >= config.hora_comida_inicio && hora < config.hora_comida_fin {
            "mediodia"
        } else {
            "noche"
        };

        let data = NuevaVenta {
            user_id,
            fecha: reserva.fecha,
            comensales: Some(reserva.num_personas),
            descripcion: &format!(
                "Generada automáticamente desde reserva de {}",
                reserva.nombre_cliente
            ),
            iva_porcentaje: config.iva_por_defecto,
            turno,
            canal: "comedor",
            metodo_pago: "efectivo",
            importe_base: Decimal::ZERO,
            importe_iva: Decimal::ZERO,
            /* [034A-5] Vincular venta con reserva y cliente de origen */
            reserva_id: Some(reserva.id),
            cliente_id: reserva.cliente_id,
        };

        match VentaRepository::create(pool, &data).await {
            Ok(venta) => {
                /* [064A-5] Sincronizar venta automática con Haddock.
                 * [064A-6] Ahora pasa pool para tracking de estado sync.
                 * [064A-7] is_update=false — es una creación, no edición.
                 * La config ya está cargada — reutilizamos. */
                let config_clone = config.clone();
                let pool_clone = pool.clone();
                tokio::spawn(async move {
                    super::HaddockService::sync_order(&pool_clone, &venta, &config_clone, false)
                        .await;
                });
            }
            Err(e) => {
                warn!(
                    "[014A-1] Error creando venta automática para reserva {}: {e}",
                    reserva.id
                );
            }
        }
    }
}
