/* 253A-5: Servicio de dashboard — resumen económico.
 * 263A-13: Dashboard de reservas Fase 2 — 3 paneles. */

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    AnalisisReservas, DashboardReservas, OcupacionReservas, ResumenEconomico, ResumenReservas,
};
use crate::repositories::{
    ConfiguracionRepository, DashboardReservasRepository, GastoRepository, VentaRepository,
};

pub struct DashboardService;

impl DashboardService {
    /// Resumen económico de un mes: total ventas, total gastos, margen
    pub async fn resumen_mes(
        pool: &PgPool,
        user_id: Uuid,
        year: i32,
        month: u32,
    ) -> Result<ResumenEconomico, AppError> {
        let desde = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::BadRequest("Fecha inválida".into()))?;
        let hasta = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| AppError::BadRequest("Fecha inválida".into()))?
        .pred_opt()
        .ok_or_else(|| AppError::Internal("Error calculando fecha".into()))?;

        /* [128A-1/F4][F4-4] La exclusión de anuladas del resumen depende de la
         * modalidad de anulación: `credito_completo` revierte el IVA
         * (excluye), `estado_solo` solo marca estado (incluye). */
        let config = ConfiguracionRepository::obtener_o_crear(pool, user_id).await?;
        let excluir_anuladas = config.anulacion_modalidad == "credito_completo";
        let total_ventas =
            VentaRepository::total_periodo(pool, user_id, desde, hasta, excluir_anuladas).await?;
        let total_gastos = GastoRepository::total_periodo(pool, user_id, desde, hasta).await?;
        let margen = total_ventas - total_gastos;

        let mes = format!("{year}-{month:02}");

        Ok(ResumenEconomico {
            total_ventas,
            total_gastos,
            margen,
            mes,
        })
    }

    /// Dashboard completo de reservas: resumen + ocupacion + analisis
    #[allow(
        clippy::too_many_lines,
        clippy::cast_precision_loss,
        clippy::items_after_statements
    )]
    pub async fn dashboard_reservas(
        pool: &PgPool,
        user_id: Uuid,
        year: i32,
        month: u32,
    ) -> Result<DashboardReservas, AppError> {
        let desde = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::BadRequest("Fecha inválida".into()))?;
        let hasta = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| AppError::BadRequest("Fecha inválida".into()))?
        .pred_opt()
        .ok_or_else(|| AppError::Internal("Error calculando fecha".into()))?;

        /* Mes anterior para comparativa */
        let (prev_year, prev_month) = if month == 1 {
            (year - 1, 12_u32)
        } else {
            (year, month - 1)
        };
        let prev_desde = chrono::NaiveDate::from_ymd_opt(prev_year, prev_month, 1)
            .ok_or_else(|| AppError::BadRequest("Fecha inválida".into()))?;
        let prev_hasta = desde
            .pred_opt()
            .ok_or_else(|| AppError::Internal("Error calculando fecha".into()))?;

        type Repo = DashboardReservasRepository;

        /* === Panel Resumen === */
        let total_reservas = Repo::total_reservas(pool, user_id, desde, hasta).await?;
        let total_mes_anterior =
            Repo::total_reservas(pool, user_id, prev_desde, prev_hasta).await?;
        let variacion_porcentaje = if total_mes_anterior > 0 {
            ((total_reservas - total_mes_anterior) as f64 / total_mes_anterior as f64 * 1000.0)
                .round()
                / 10.0
        } else if total_reservas > 0 {
            100.0
        } else {
            0.0
        };

        let por_dia = Repo::por_dia(pool, user_id, desde, hasta).await?;
        let por_dia_semana = Repo::por_dia_semana(pool, user_id, desde, hasta).await?;
        let por_canal = Repo::por_canal(pool, user_id, desde, hasta, total_reservas).await?;
        let clientes_nuevos = Repo::clientes_nuevos(pool, user_id, desde, hasta).await?;

        let resumen = ResumenReservas {
            total_reservas,
            total_mes_anterior,
            variacion_porcentaje,
            por_dia,
            por_dia_semana,
            por_canal: por_canal.clone(),
            clientes_nuevos,
        };

        /* === Panel Ocupacion === */
        let (total_ocup, _total_personas, media_personas) =
            Repo::datos_ocupacion(pool, user_id, desde, hasta).await?;
        let dias_en_mes = (hasta - desde).num_days() + 1;
        let media_reservas_dia = if dias_en_mes > 0 {
            (total_ocup as f64 / dias_en_mes as f64 * 10.0).round() / 10.0
        } else {
            0.0
        };

        let por_hora = Repo::por_hora(pool, user_id, desde, hasta).await?;
        let por_turno = Repo::por_turno(pool, user_id, desde, hasta).await?;
        let antelacion_media_dias = Repo::antelacion_media(pool, user_id, desde, hasta).await?;

        let ocupacion = OcupacionReservas {
            media_personas: (media_personas * 10.0).round() / 10.0,
            media_reservas_dia,
            total_reservas: total_ocup,
            por_hora,
            por_turno,
            por_procedencia: por_canal,
            antelacion_media_dias: (antelacion_media_dias * 10.0).round() / 10.0,
        };

        /* === Panel Analisis === */
        let (reservas_efectivas, total_comensales) =
            Repo::reservas_efectivas(pool, user_id, desde, hasta).await?;
        let comensales_por_reserva = if reservas_efectivas > 0 {
            (total_comensales as f64 / reservas_efectivas as f64 * 10.0).round() / 10.0
        } else {
            0.0
        };

        let total_ventas = Repo::total_ventas_periodo(pool, user_id, desde, hasta).await?;
        let (ticket_medio_reserva, ticket_medio_persona) = if reservas_efectivas > 0 {
            let tmr = total_ventas / Decimal::from(reservas_efectivas);
            let tmp = if total_comensales > 0 {
                total_ventas / Decimal::from(total_comensales)
            } else {
                Decimal::ZERO
            };
            (Some(tmr.round_dp(2)), Some(tmp.round_dp(2)))
        } else {
            (None, None)
        };

        let analisis = AnalisisReservas {
            reservas_efectivas,
            total_comensales,
            comensales_por_reserva,
            ticket_medio_reserva,
            ticket_medio_persona,
        };

        Ok(DashboardReservas {
            resumen,
            ocupacion,
            analisis,
        })
    }
}
