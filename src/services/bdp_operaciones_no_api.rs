/* [D32] Operaciones que BDP no expone por API: la web no las ejecuta, y debe
 * indicar qué hacer y quién lo hace. Responsable y procedimiento se configuran
 * por operación; si falta el responsable, se usa el de la configuración global
 * y, si tampoco existe, se muestra un aviso explícito. */

use std::collections::HashMap;

use serde::Serialize;

/// Operaciones de la ficha D32 (anexo B8): (código estable, texto para el usuario).
/// Cada una se hace en el TPV de BDP.
pub const OPERACIONES_NO_API: &[(&str, &str)] = &[
    ("traspaso", "Iniciar traspaso desde la web"),
    ("eliminar_articulos", "Eliminar artículos"),
    ("rectificaciones", "Rectificaciones"),
    ("devoluciones", "Devoluciones"),
    ("caja", "Apertura y cierre de caja"),
    ("arqueo", "Arqueo"),
    ("efectivo", "Entradas y salidas de efectivo"),
    ("pdf_factura_email", "PDF de factura por email"),
    ("reimpresion_cocina", "Reimpresión en cocina"),
    ("cocina_avanzada", "Controles avanzados de cocina"),
    ("series_facturacion", "Series de facturación"),
    ("cambios_posteriores_pago", "Cambios posteriores al pago"),
];

/// Procedimiento por defecto si la operación no tiene uno configurado.
pub const PROCEDIMIENTO_POR_DEFECTO: &str = "Realízala en el TPV de BDP.";

/// Texto que se muestra al usuario cuando no hay responsable configurado.
const RESPONSABLE_PENDIENTE: &str = "pendiente de asignar";

/// Límites de texto: evitan guardar bloques enormes en una pantalla de configuración.
pub const MAX_RESPONSABLE: usize = 120;
pub const MAX_PROCEDIMIENTO: usize = 500;

/// Asignación guardada para una operación (vacía = usar los valores por defecto).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Asignacion {
    pub responsable: String,
    pub procedimiento: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OperacionNoApi {
    pub codigo: String,
    pub operacion: String,
    pub responsable: String,
    pub procedimiento: String,
    /// true si ni la operación ni la configuración global tienen responsable.
    pub responsable_pendiente: bool,
    pub mensaje: String,
}

/// Valida y normaliza una asignación antes de guardarla (recorta espacios).
pub fn validar(responsable: &str, procedimiento: &str) -> Result<Asignacion, String> {
    let responsable = responsable.trim();
    let procedimiento = procedimiento.trim();
    if responsable.chars().count() > MAX_RESPONSABLE {
        return Err(format!("responsable: máximo {MAX_RESPONSABLE} caracteres"));
    }
    if procedimiento.chars().count() > MAX_PROCEDIMIENTO {
        return Err(format!(
            "procedimiento: máximo {MAX_PROCEDIMIENTO} caracteres"
        ));
    }
    Ok(Asignacion {
        responsable: responsable.to_string(),
        procedimiento: procedimiento.to_string(),
    })
}

/// Existe el código en la ficha D32.
pub fn es_codigo_valido(codigo: &str) -> bool {
    OPERACIONES_NO_API.iter().any(|(c, _)| *c == codigo)
}

/// Resuelve una operación con su asignación y el responsable global de respaldo.
pub fn resolver(
    codigo: &str,
    operacion: &str,
    responsable_global: &str,
    asignacion: Option<&Asignacion>,
) -> OperacionNoApi {
    let asignado = asignacion.map(|a| a.responsable.trim()).unwrap_or("");
    let global = responsable_global.trim();
    let responsable = if !asignado.is_empty() {
        asignado
    } else {
        global
    };
    let procedimiento = asignacion
        .map(|a| a.procedimiento.trim())
        .filter(|p| !p.is_empty())
        .unwrap_or(PROCEDIMIENTO_POR_DEFECTO);
    let pendiente = responsable.is_empty();
    let responsable_mostrado = if pendiente {
        RESPONSABLE_PENDIENTE
    } else {
        responsable
    };
    OperacionNoApi {
        codigo: codigo.to_string(),
        operacion: operacion.to_string(),
        responsable: responsable.to_string(),
        procedimiento: procedimiento.to_string(),
        responsable_pendiente: pendiente,
        mensaje: format!(
            "«{operacion}» no se puede hacer desde la web: BDP no la expone por API. \
             {procedimiento} Responsable: {responsable_mostrado}."
        ),
    }
}

/// Lista completa de operaciones no expuestas, con sus asignaciones aplicadas.
pub fn listar(
    responsable_global: &str,
    asignaciones: &HashMap<String, Asignacion>,
) -> Vec<OperacionNoApi> {
    OPERACIONES_NO_API
        .iter()
        .map(|(codigo, operacion)| {
            resolver(
                codigo,
                operacion,
                responsable_global,
                asignaciones.get(*codigo),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_responsable_en_ningun_sitio_muestra_pendiente() {
        let op = resolver("arqueo", "Arqueo", "   ", None);
        assert!(op.responsable_pendiente);
        assert!(op.mensaje.contains("Responsable: pendiente de asignar."));
    }

    #[test]
    fn responsable_de_la_operacion_gana_al_global() {
        let asignacion = Asignacion {
            responsable: "Ana García".into(),
            procedimiento: String::new(),
        };
        let op = resolver("arqueo", "Arqueo", "Luis", Some(&asignacion));
        assert!(!op.responsable_pendiente);
        assert_eq!(op.responsable, "Ana García");
        assert!(op.mensaje.contains("Responsable: Ana García."));
    }

    #[test]
    fn sin_asignacion_usa_el_responsable_global() {
        let op = resolver("caja", "Apertura y cierre de caja", "Luis", None);
        assert_eq!(op.responsable, "Luis");
        assert_eq!(op.procedimiento, PROCEDIMIENTO_POR_DEFECTO);
    }

    #[test]
    fn procedimiento_configurado_sustituye_al_por_defecto() {
        let asignacion = Asignacion {
            responsable: String::new(),
            procedimiento: "Pedir a Guillermo en TpvNet".into(),
        };
        let op = resolver("devoluciones", "Devoluciones", "", Some(&asignacion));
        assert!(op.mensaje.contains("Pedir a Guillermo en TpvNet"));
        assert!(op.responsable_pendiente);
    }

    #[test]
    fn listar_cubre_las_doce_operaciones_con_su_asignacion() {
        let mut mapa = HashMap::new();
        mapa.insert(
            "arqueo".to_string(),
            Asignacion {
                responsable: "Ana".into(),
                procedimiento: String::new(),
            },
        );
        let lista = listar("", &mapa);
        assert_eq!(lista.len(), 12);
        assert_eq!(
            lista.iter().filter(|op| !op.responsable_pendiente).count(),
            1
        );
    }

    #[test]
    fn validar_recorta_y_rechaza_textos_largos() {
        let ok = validar("  Ana  ", " Paso 1 ").unwrap();
        assert_eq!(ok.responsable, "Ana");
        assert_eq!(ok.procedimiento, "Paso 1");
        assert!(validar(&"x".repeat(MAX_RESPONSABLE + 1), "").is_err());
        assert!(validar("", &"x".repeat(MAX_PROCEDIMIENTO + 1)).is_err());
    }

    #[test]
    fn codigo_debe_estar_en_la_ficha() {
        assert!(es_codigo_valido("arqueo"));
        assert!(!es_codigo_valido("inventado"));
    }
}
