/* [199A-1/F1] Avisos de acciones del TPV hacia la web (BDP -> Glory).
 *
 * El contrato de los siete eventos viene del paquete del distribuidor
 * (`AvisosDeAcciones_Documentacion`, receptor de referencia `WebhooksServer_Test`):
 *
 *   Crear · Modificar · CambiarEstado · Borrar · Finalizar · Traspasar · TraspasarLinea
 *
 * Tres cosas que ese paquete deja claras y que mandan en este módulo:
 *
 * 1. Los **nombres de los parámetros los fija la configuración del TPV**
 *    (`Clave`/`Valor` por acción), no el receptor. Por eso aquí se leen por
 *    alias y sin distinguir mayúsculas, acentos ni separadores.
 * 2. Cada acción admite **GET (query string) o POST (cuerpo JSON)**.
 * 3. El TPV **no devuelve identificador de operación** y no hay documentación de
 *    reintentos: la idempotencia la sostenemos nosotros con
 *    (acción + comanda + timestamp), que es la clave única del buzón.
 *
 * Regla de admisión (decisión L4.4, «valor seguro por defecto»): solo se rechaza
 * con 4xx lo que no se puede ni identificar —falta `IdentificadorComanda`—; todo
 * lo demás que falte o venga raro se **guarda** con estado `requiere_revision` y
 * su nota. Un aviso nunca se pierde por un campo inesperado.
 *
 * Nada de este módulo escribe en el TPV: solo en `bdp_avisos_inbox`. */

/* Partido en tres por dominio (Sentinel `god-object-rs`): aquí las acciones y
 * `normalizar_clave`; en `campos` los campos y su admisión; en `buzon` la
 * escritura en `bdp_avisos_inbox`. Las pruebas viven aquí, con lo reexportado. */

mod buzon;
mod campos;
mod worker;

pub use buzon::{BdpAvisosService, FilaAviso, ResultadoInbox};
pub use campos::{
    censurar, clave_idempotencia, hash_payload, preparar, secreto_esperado, secreto_valido,
    AvisoPreparado, CamposAviso, RechazoAviso,
};
pub use worker::{
    procesar_lote, procesar_pendientes, segundos_automaticos, Consulta, ConsultaPedido,
    ResumenLote, LIMITE_LOTE_MAX, MAX_INTENTOS,
};

/* --------------------------------------------------------------------------
 * Acciones
 * -------------------------------------------------------------------------- */

/// Las siete acciones que el TPV puede avisar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionAviso {
    Crear,
    Modificar,
    CambiarEstado,
    Borrar,
    Finalizar,
    Traspasar,
    TraspasarLinea,
}

impl AccionAviso {
    /// Slug canónico: el que se documenta para la `Ruta Acción` del TPV.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            Self::Crear => "crear",
            Self::Modificar => "modificar",
            Self::CambiarEstado => "cambiar-estado",
            Self::Borrar => "borrar",
            Self::Finalizar => "finalizar",
            Self::Traspasar => "traspasar",
            Self::TraspasarLinea => "traspasar-linea",
        }
    }

    /// Interpreta lo que venga en la URL. La `Ruta Acción` del TPV es texto
    /// libre, así que se aceptan las variantes razonables: mayúsculas, guiones,
    /// guiones bajos, espacios, acentos y los nombres largos del distribuidor
    /// (`CambiarEstado`, `TraspasarLinea`, `Comanda creada`...).
    #[must_use]
    pub fn desde_slug(slug: &str) -> Option<Self> {
        match normalizar_clave(slug).as_str() {
            "crear" | "crearcomanda" | "comandacreada" => Some(Self::Crear),
            "modificar" | "modificarcomanda" | "comandamodificada" | "actualizar" => {
                Some(Self::Modificar)
            }
            "cambiarestado"
            | "cambiarestadocomanda"
            | "estadocambiado"
            | "comandaestadocambiado"
            | "comandaestadocambiada" => Some(Self::CambiarEstado),
            "borrar" | "borrarcomanda" | "comandaborrada" | "eliminar" => Some(Self::Borrar),
            "finalizar" | "finalizarcomanda" | "comandafinalizada" => Some(Self::Finalizar),
            "traspasar" | "traspasarcomanda" | "comandatraspasada" => Some(Self::Traspasar),
            "traspasarlinea" | "traspasarlineadecomanda" | "lineatraspasada" => {
                Some(Self::TraspasarLinea)
            }
            _ => None,
        }
    }
}

/// Minúsculas, sin acentos y sin separadores: la forma en la que se comparan
/// tanto las acciones como los nombres de los campos.
pub(super) fn normalizar_clave(texto: &str) -> String {
    texto
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            '-' | '_' | ' ' | '.' | ':' | '/' => None,
            'á' | 'à' | 'ä' | 'â' => Some('a'),
            'é' | 'è' | 'ë' | 'ê' => Some('e'),
            'í' | 'ì' | 'ï' | 'î' => Some('i'),
            'ó' | 'ò' | 'ö' | 'ô' => Some('o'),
            'ú' | 'ù' | 'ü' | 'û' => Some('u'),
            'ñ' => Some('n'),
            otro => Some(otro),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn campos(pares: &[(&str, &str)]) -> CamposAviso {
        let mut mapa = BTreeMap::new();
        for (clave, valor) in pares {
            mapa.insert((*clave).to_string(), (*valor).to_string());
        }
        CamposAviso::desde_query(&mapa)
    }

    #[test]
    fn las_siete_acciones_se_reconocen_con_sus_variantes() {
        let casos = [
            ("crear", AccionAviso::Crear),
            ("Crear", AccionAviso::Crear),
            ("Comanda creada", AccionAviso::Crear),
            ("modificar", AccionAviso::Modificar),
            ("CambiarEstado", AccionAviso::CambiarEstado),
            ("cambiar_estado", AccionAviso::CambiarEstado),
            ("cambiar-estado", AccionAviso::CambiarEstado),
            ("Comanda: Estado Cambiado", AccionAviso::CambiarEstado),
            ("borrar", AccionAviso::Borrar),
            ("Finalizar", AccionAviso::Finalizar),
            ("traspasar", AccionAviso::Traspasar),
            ("TraspasarLinea", AccionAviso::TraspasarLinea),
            ("traspasar-linea", AccionAviso::TraspasarLinea),
            ("traspasar_linea", AccionAviso::TraspasarLinea),
        ];
        assert_eq!(casos.len(), 14);
        for (slug, esperada) in casos {
            assert_eq!(
                AccionAviso::desde_slug(slug),
                Some(esperada),
                "no se reconoció {slug}"
            );
        }
        assert_eq!(AccionAviso::desde_slug("pagar"), None);
        assert_eq!(AccionAviso::desde_slug(""), None);
    }

    #[test]
    fn los_campos_se_leen_por_alias_y_sin_distinguir_forma() {
        let varios = campos(&[
            ("OrderId", "15"),
            ("FechaHora", "2026-10-17T14:00:00Z"),
            ("estado", "2"),
        ]);
        assert_eq!(varios.comanda(), Some("15"));
        assert_eq!(varios.timestamp(), Some("2026-10-17T14:00:00Z"));
        assert_eq!(varios.estado(), Some("2"));

        let con_guiones = campos(&[("identificador-comanda", " 77 ")]);
        assert_eq!(con_guiones.comanda(), Some("77"));

        let desde_json = CamposAviso::desde_json(&json!({
            "IdentificadorComanda": "15",
            "Estado": 2,
            "lineas": [{"a": 1}],
            "Nulo": null
        }));
        assert_eq!(desde_json.comanda(), Some("15"));
        assert_eq!(desde_json.estado(), Some("2"));
    }

    #[test]
    fn la_clave_de_idempotencia_no_cambia_con_la_forma_del_timestamp() {
        let referencia = clave_idempotencia(AccionAviso::Crear, "15", "2026-10-17T14:00:00Z");
        for variante in [
            "2026-10-17T14:00:00z",
            "2026-10-17T14:00:00.000Z",
            "2026-10-17T16:00:00+02:00",
            "2026-10-17T14:00:00",
            " 2026-10-17T14:00:00Z ",
        ] {
            assert_eq!(
                clave_idempotencia(AccionAviso::Crear, "15", variante),
                referencia,
                "la variante {variante} debería dar la misma clave"
            );
        }
        /* Otra comanda, otra acción u otra hora sí son avisos distintos. */
        assert_ne!(
            clave_idempotencia(AccionAviso::Crear, "16", "2026-10-17T14:00:00Z"),
            referencia
        );
        assert_ne!(
            clave_idempotencia(AccionAviso::Borrar, "15", "2026-10-17T14:00:00Z"),
            referencia
        );
        assert_ne!(
            clave_idempotencia(AccionAviso::Crear, "15", "2026-10-17T14:00:01Z"),
            referencia
        );
    }

    #[test]
    fn sin_comanda_el_aviso_se_rechaza_y_con_lo_demas_raro_se_guarda() {
        let sin_comanda = campos(&[("TimeStamp", "2026-10-17T14:00:00Z")]);
        assert!(matches!(
            preparar(AccionAviso::Crear, &sin_comanda, &json!({})),
            Err(RechazoAviso::Invalido(_))
        ));

        /* Estado fuera de rango: se guarda, marcado, y no se actúa. */
        let estado_raro = campos(&[
            ("IdentificadorComanda", "15"),
            ("TimeStamp", "2026-10-17T14:00:00Z"),
            ("Estado", "9"),
        ]);
        let aviso = preparar(AccionAviso::CambiarEstado, &estado_raro, &json!({})).unwrap();
        assert_eq!(aviso.estado, "requiere_revision");
        assert!(aviso.nota.unwrap().contains("Estado fuera de 0-3"));

        /* Aviso completo y correcto: entra limpio. */
        let correcto = campos(&[
            ("IdentificadorComanda", "15"),
            ("TimeStamp", "2026-10-17T14:00:00Z"),
            ("Estado", "1"),
        ]);
        let aviso = preparar(AccionAviso::CambiarEstado, &correcto, &json!({})).unwrap();
        assert_eq!(aviso.estado, "recibido");
        assert!(aviso.nota.is_none());

        /* Finalizar sin serie ni número también se guarda para revisión. */
        let finalizar = campos(&[
            ("IdentificadorComanda", "15"),
            ("TimeStamp", "2026-10-17T14:00:00Z"),
            ("TipoDocumento", "6"),
        ]);
        let aviso = preparar(AccionAviso::Finalizar, &finalizar, &json!({})).unwrap();
        assert_eq!(aviso.estado, "requiere_revision");
        assert!(aviso.nota.unwrap().contains("Serie"));
    }

    #[test]
    fn el_secreto_nunca_se_guarda_ni_aparece_en_la_huella() {
        let original = json!({
            "IdentificadorComanda": "15",
            "Secreto": "s3cr3t0",
            "anidado": { "Token": "abc", "otro": "visible" }
        });
        let limpio = censurar(&original);
        let texto = serde_json::to_string(&limpio).unwrap();
        assert!(!texto.contains("s3cr3t0"));
        assert!(!texto.contains("\"abc\""));
        assert!(texto.contains("visible"));
        assert_eq!(limpio["Secreto"], json!(campos::CENSURA));

        /* La huella se calcula sobre el payload ya censurado, así que dos
         * reenvíos con el mismo cuerpo dan la misma huella... */
        let misma = hash_payload(&censurar(&json!({
            "IdentificadorComanda": "15", "Secreto": "otro-distinto"
        })));
        let otra = hash_payload(&censurar(&json!({
            "IdentificadorComanda": "15", "Secreto": "s3cr3t0"
        })));
        assert_eq!(misma, otra, "el secreto no debe alterar la huella");

        /* ...y un cuerpo distinto da huella distinta. */
        assert_ne!(
            hash_payload(&censurar(&json!({"IdentificadorComanda": "15"}))),
            hash_payload(&censurar(&json!({"IdentificadorComanda": "16"})))
        );
    }

    #[test]
    fn el_secreto_se_compara_entero_y_vacio_no_vale() {
        assert!(secreto_valido("abc123", "abc123"));
        assert!(!secreto_valido("abc123", "abc124"));
        assert!(!secreto_valido("abc123", "abc12"));
        assert!(!secreto_valido("abc123", ""));
        assert!(!secreto_valido("", ""));
        assert!(!secreto_valido("", "loquesea"));
    }

    #[test]
    fn el_cuerpo_manda_sobre_la_query() {
        let query = campos(&[("IdentificadorComanda", "15"), ("Estado", "0")]);
        let cuerpo = CamposAviso::desde_json(&json!({"Estado": 1}));
        let fusion = query.fusionar(&cuerpo);
        assert_eq!(fusion.comanda(), Some("15"));
        assert_eq!(fusion.estado(), Some("1"));
    }
}
