// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/* [BKP-003] Motor de snapshots y auditoría BDP.
 * Gestiona snapshots (puntos de restauración) y audit log (traza inmutable).
 * Pre-write snapshots: selectivos, máximo 1 llamada adicional a BDP.
 * Restauración: solo datos locales de Glory (BDP no permite delete/update via API).
 * Todas las queries usan runtime sqlx (no macros compile-time) para compatibilidad SQLX_OFFLINE.
 *
 * Fachada: la implementación vive en los módulos de bdp_backup, agrupada por responsabilidad.
 * Las firmas públicas se mantienen aquí para que los llamadores no cambien. */

mod conexion;
mod escritura;
mod lectura_bdp;
mod persistencia;
mod restauracion;
mod retencion;
mod snapshot_bdp;
mod snapshot_glory;
mod tipos;

pub use escritura::AuditoriaDirecta;
pub use tipos::{BdpAuditEntry, BdpSnapshot, RestoreResult};

pub struct BdpBackupService;
