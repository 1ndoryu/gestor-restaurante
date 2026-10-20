/* [D32] Operaciones que BDP no expone por API: la web no las ejecuta, y debe
 * indicar qué hacer y quién lo hace. El responsable es configurable
 * (`configuracion_restaurante.bdp_responsable_no_api`). */

use serde::Serialize;

/// Operaciones de la ficha D32 (anexo B8). Cada una se hace en el TPV de BDP.
pub const OPERACIONES_NO_API: &[&str] = &[
    "Iniciar traspaso desde la web",
    "Eliminar artículos",
    "Rectificaciones",
    "Devoluciones",
    "Apertura y cierre de caja",
    "Arqueo",
    "Entradas y salidas de efectivo",
    "PDF de factura por email",
    "Reimpresión en cocina",
    "Controles avanzados de cocina",
    "Series de facturación",
    "Cambios posteriores al pago",
];

/// Texto que se muestra al usuario cuando no hay responsable configurado.
const RESPONSABLE_PENDIENTE: &str = "pendiente de asignar";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OperacionNoApi {
    pub operacion: String,
    pub mensaje: String,
}

/// Mensaje de una operación: qué hacer en BDP y quién lo gestiona.
/// Un responsable vacío (o solo espacios) se sustituye por un aviso explícito.
pub fn mensaje_operacion(operacion: &str, responsable: &str) -> String {
    let responsable = match responsable.trim() {
        "" => RESPONSABLE_PENDIENTE,
        valor => valor,
    };
    format!(
        "«{operacion}» no se puede hacer desde la web: BDP no la expone por API. \
         Realízala en el TPV de BDP. Responsable: {responsable}."
    )
}

/// Lista completa de operaciones no expuestas con su mensaje.
pub fn listar(responsable: &str) -> Vec<OperacionNoApi> {
    OPERACIONES_NO_API
        .iter()
        .map(|operacion| OperacionNoApi {
            operacion: (*operacion).to_string(),
            mensaje: mensaje_operacion(operacion, responsable),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responsable_vacio_muestra_aviso_pendiente() {
        let msg = mensaje_operacion("Arqueo", "   ");
        assert!(msg.contains("Responsable: pendiente de asignar."));
    }

    #[test]
    fn responsable_configurado_aparece_en_el_mensaje() {
        let msg = mensaje_operacion("Arqueo", "Ana García");
        assert!(msg.contains("Responsable: Ana García."));
        assert!(msg.contains("«Arqueo»"));
    }

    #[test]
    fn listar_cubre_las_doce_operaciones_de_la_ficha() {
        let lista = listar("Ana");
        assert_eq!(lista.len(), 12);
        assert!(lista.iter().all(|op| op.mensaje.contains("Responsable: Ana.")));
    }
}
