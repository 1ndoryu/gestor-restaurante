use sha2::{Digest, Sha256};
use uuid::Uuid;

/// [10AA-3-B] Tipo de destino que exige cada alcance para poder armarse.
///
/// El worker de push pasa como `target_entity_type` el dominio literal de la
/// fila (`bdp_push.rs`), así que el armado manual tiene que usar ese mismo
/// vocabulario. Antes solo se aceptaban «cliente» y «venta»: 10 de los 15
/// alcances devolvían 422 y su escritura no salía nunca.
#[must_use]
pub fn tipo_destino_de_alcance(alcance: &str) -> Option<&'static str> {
    match alcance {
        "create_order" | "add_payment" | "invoice" | "cancel_order" => Some("venta"),
        "create_customer" => Some("cliente"),
        "create_article" | "modify_article" | "modify_prices" => Some("articulo"),
        "create_department" => Some("departamento"),
        "create_family" => Some("familia"),
        "regularize_stock" | "transfer_stock" | "inventory" => Some("stock"),
        "add_tip" => Some("propina"),
        "add_points" => Some("cliente_puntos"),
        _ => None,
    }
}

/// [10AA-3-B] Alcances cuyo destino es una entidad con UUID propio en Glory
/// (`venta`, `cliente`). En el resto el destino es una entidad identificada por
/// un código de texto (código de artículo, de cliente de puntos, de almacén…) y
/// el UUID que exigen el armado y la auditoría se deriva con [`entidad_uuid`].
#[must_use]
pub fn alcance_usa_uuid_propio(alcance: &str) -> bool {
    matches!(
        alcance,
        "create_order" | "add_payment" | "invoice" | "cancel_order" | "create_customer"
    )
}

/// Identificador UUID estable derivado de (`dominio`, `entidad_id`) para arming y
/// auditoría (que exigen `Uuid`). No colisiona entre entidades ni usuarios
/// porque el namespace es determinista por (`dominio`, `entidad_id`).
///
/// Vive aquí, y no en el worker, porque el armado manual tiene que calcular
/// exactamente el mismo valor: si las dos copias divergen, el armado deja de
/// reutilizarse y la operación autorizada muere en silencio.
#[must_use]
pub fn entidad_uuid(dominio: &str, entidad_id: &str) -> Uuid {
    /* No se usa `Uuid::new_v5` porque el crate solo tiene habilitadas las
     * features `serde` y `v4`. */
    let mut hasher = Sha256::new();
    hasher.update(format!("bdp-push:{dominio}:{entidad_id}").as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}
