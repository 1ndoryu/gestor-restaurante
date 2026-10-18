mod alcance;
mod armado;
mod autorizacion;
mod consulta;

pub use alcance::{alcance_usa_uuid_propio, entidad_uuid, tipo_destino_de_alcance};
pub use autorizacion::EscrituraAutorizar;

pub struct BdpWriteGuard;
