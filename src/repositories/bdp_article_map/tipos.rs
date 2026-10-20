/* Tipos públicos del mapeo BDP: resultado de upsert, error de stock y datos de import. */

use rust_decimal::Decimal;

/* [128A-1/F2] Resultado del upsert del import BDP (M6/M7).
 * - `Creado`: la fila no existía y se insertó.
 * - `Actualizado`: la fila existía y cambió.
 * - `SinCambios`: la fila existía y era idéntica.
 * - `OmitidoLocalDirty`: la fila tiene ediciones locales (`local_dirty`); el
 *   import NO la sobrescribe y el conflicto se reporta en el resultado.
 * - `OmitidoDesactivado`: la fila está desactivada localmente (`activo=false`
 *   local) y BDP la trae activa; el import NO la reactiva (M7). */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BdpArticleUpsertStatus {
    Creado,
    Actualizado,
    SinCambios,
    OmitidoLocalDirty,
    OmitidoDesactivado,
}

impl BdpArticleUpsertStatus {
    /// True si el import cambió o creó la fila (para stock y contadores).
    #[must_use]
    pub fn es_cambio(self) -> bool {
        matches!(self, Self::Creado | Self::Actualizado)
    }

    /// True si el import dejó la fila intacta por una regla local (M6/M7).
    #[must_use]
    pub fn es_omitido(self) -> bool {
        matches!(self, Self::OmitidoLocalDirty | Self::OmitidoDesactivado)
    }
}

/// [128A-1/F3] Errores de dominio del ajuste de stock local.
#[derive(Debug, thiserror::Error)]
pub enum AjusteStockError {
    #[error("Stock negativo inválido: {0}")]
    StockNegativo(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub struct BdpArticleMapRepository;

/* [157A-7] F9.1: Datos para upsert de artículo BDP enriquecido.
 * [237A-4] Añadido stock_actual. */
pub struct BdpArticleUpsertData<'a> {
    pub bdp_code: &'a str,
    pub descripcion: &'a str,
    pub precio_tarifa1: Decimal,
    pub iva_pct: Decimal,
    pub departamento: i32,
    pub familia: i32,
    pub subfamilia: i32,
    pub activo: bool,
    pub barcode: &'a str,
    /* [237A-4] Stock actual del artículo en BDP */
    pub stock_actual: Decimal,
    /* [D26d] Marca inventariable de BDP; None si la respuesta no la trae. */
    pub inventariable: Option<bool>,
}
