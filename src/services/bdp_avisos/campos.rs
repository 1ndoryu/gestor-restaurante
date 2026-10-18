/* [199A-1/F1] Campos de un aviso del TPV: lectura por alias, admisión y censura.
 * Parte de `bdp_avisos`; ver la cabecera del módulo para el contrato. */

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::{normalizar_clave, AccionAviso};

/* --------------------------------------------------------------------------
 * Campos del aviso
 * -------------------------------------------------------------------------- */

/* Alias por campo. El primero es el nombre del contrato del distribuidor; el
 * resto, lo que un integrador podría escribir en la configuración del TPV. */
const ALIAS_COMANDA: &[&str] = &[
    "IdentificadorComanda",
    "IdComanda",
    "NumeroComanda",
    "Comanda",
    "OrderId",
];
const ALIAS_ESTABLECIMIENTO: &[&str] = &[
    "IdentificadorEstablecimiento",
    "IdEstablecimiento",
    "Establecimiento",
];
const ALIAS_TIMESTAMP: &[&str] = &["TimeStamp", "FechaHora", "Fecha", "Timestamp"];
const ALIAS_ESTADO: &[&str] = &["Estado", "Status"];
const ALIAS_TIPO_DOCUMENTO: &[&str] = &["TipoDocumento", "TipoDoc"];
const ALIAS_SERIE: &[&str] = &["Serie"];
const ALIAS_NUMERO: &[&str] = &["Numero", "NumDocumento"];
const ALIAS_COMANDA_ORIGEN: &[&str] = &[
    "IdentificadorComandaOrigen",
    "IdComandaOrigen",
    "ComandaOrigen",
];
const ALIAS_LINEA_ORIGEN: &[&str] = &[
    "IdentificadorLineaComandaOrigen",
    "IdLineaComandaOrigen",
    "LineaComandaOrigen",
    "LineaOrigen",
];
const ALIAS_LINEA_DESTINO: &[&str] = &[
    "IdentificadorLineaComanda",
    "IdLineaComanda",
    "LineaComanda",
    "LineaDestino",
];
const ALIAS_SALON_ORIGEN: &[&str] = &["SalonOrigen"];
const ALIAS_SALON_DESTINO: &[&str] = &["SalonDestino"];
const ALIAS_MESA_ORIGEN: &[&str] = &["MesaOrigen"];
const ALIAS_MESA_DESTINO: &[&str] = &["MesaDestino"];
/* El secreto compartido llega como un parámetro más (el TPV admite valores
 * fijos): se acepta por cualquiera de estos nombres. */
const ALIAS_SECRETO: &[&str] = &["Secreto", "Secret", "Token", "ApiKey"];

/// Campos de un aviso con la clave normalizada.
#[derive(Debug, Default, Clone)]
pub struct CamposAviso {
    campos: BTreeMap<String, String>,
}

impl CamposAviso {
    /// Campos venidos en la query string (avisos por GET).
    #[must_use]
    pub fn desde_query(query: &BTreeMap<String, String>) -> Self {
        let mut campos = Self::default();
        for (clave, valor) in query {
            campos.insertar(clave, valor.clone());
        }
        campos
    }

    /// Campos venidos en un cuerpo JSON (avisos por POST). Los objetos y las
    /// listas anidadas se ignoran: el contrato del distribuidor es plano.
    #[must_use]
    pub fn desde_json(valor: &Value) -> Self {
        let mut campos = Self::default();
        if let Some(objeto) = valor.as_object() {
            for (clave, valor) in objeto {
                let texto = match valor {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                campos.insertar(clave, texto);
            }
        }
        campos
    }

    /// El cuerpo manda sobre la query cuando ambos traen el mismo campo.
    #[must_use]
    pub fn fusionar(mut self, otros: &Self) -> Self {
        for (clave, valor) in &otros.campos {
            self.campos.insert(clave.clone(), valor.clone());
        }
        self
    }

    fn insertar(&mut self, clave: &str, valor: String) {
        let clave = normalizar_clave(clave);
        if !clave.is_empty() {
            self.campos.insert(clave, valor);
        }
    }

    fn buscar(&self, alias: &[&str]) -> Option<&str> {
        alias
            .iter()
            .find_map(|nombre| self.campos.get(&normalizar_clave(nombre)))
            .map(|valor| valor.trim())
            .filter(|valor| !valor.is_empty())
    }

    #[must_use]
    pub fn comanda(&self) -> Option<&str> {
        self.buscar(ALIAS_COMANDA)
    }

    #[must_use]
    pub fn establecimiento(&self) -> Option<&str> {
        self.buscar(ALIAS_ESTABLECIMIENTO)
    }

    #[must_use]
    pub fn timestamp(&self) -> Option<&str> {
        self.buscar(ALIAS_TIMESTAMP)
    }

    #[must_use]
    pub fn estado(&self) -> Option<&str> {
        self.buscar(ALIAS_ESTADO)
    }

    #[must_use]
    pub fn tipo_documento(&self) -> Option<&str> {
        self.buscar(ALIAS_TIPO_DOCUMENTO)
    }

    #[must_use]
    pub fn serie(&self) -> Option<&str> {
        self.buscar(ALIAS_SERIE)
    }

    #[must_use]
    pub fn numero(&self) -> Option<&str> {
        self.buscar(ALIAS_NUMERO)
    }

    #[must_use]
    pub fn comanda_origen(&self) -> Option<&str> {
        self.buscar(ALIAS_COMANDA_ORIGEN)
    }

    #[must_use]
    pub fn linea_origen(&self) -> Option<&str> {
        self.buscar(ALIAS_LINEA_ORIGEN)
    }

    #[must_use]
    pub fn linea_destino(&self) -> Option<&str> {
        self.buscar(ALIAS_LINEA_DESTINO)
    }

    #[must_use]
    pub fn salon_origen(&self) -> Option<&str> {
        self.buscar(ALIAS_SALON_ORIGEN)
    }

    #[must_use]
    pub fn salon_destino(&self) -> Option<&str> {
        self.buscar(ALIAS_SALON_DESTINO)
    }

    #[must_use]
    pub fn mesa_origen(&self) -> Option<&str> {
        self.buscar(ALIAS_MESA_ORIGEN)
    }

    #[must_use]
    pub fn mesa_destino(&self) -> Option<&str> {
        self.buscar(ALIAS_MESA_DESTINO)
    }

    #[must_use]
    pub fn secreto(&self) -> Option<&str> {
        self.buscar(ALIAS_SECRETO)
    }
}

/* --------------------------------------------------------------------------
 * Interpretación y admisión
 * -------------------------------------------------------------------------- */

/// Lo que se guarda en el buzón, ya interpretado y censurado.
#[derive(Debug, Clone)]
pub struct AvisoPreparado {
    pub accion: AccionAviso,
    pub clave_idempotencia: String,
    /// Payload tal cual llegó, con los secretos sustituidos.
    pub payload: Value,
    pub payload_hash: String,
    /// `recibido` o `requiere_revision`.
    pub estado: &'static str,
    pub nota: Option<String>,
}

/// Por qué no se admite un aviso. `AccionDesconocida` es 404 y `Invalido` 422;
/// en ninguno de los dos casos se guarda nada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RechazoAviso {
    AccionDesconocida(String),
    Invalido(String),
}

/// Decide si un aviso se admite, con qué clave de idempotencia y con qué estado.
///
/// `cuerpo` es el payload a guardar (el JSON recibido, o la query convertida en
/// objeto cuando el aviso llega por GET).
pub fn preparar(
    accion: AccionAviso,
    campos: &CamposAviso,
    cuerpo: &Value,
) -> Result<AvisoPreparado, RechazoAviso> {
    let comanda = campos
        .comanda()
        .ok_or_else(|| {
            RechazoAviso::Invalido(
                "Falta IdentificadorComanda: sin él no hay aviso que identificar ni clave de idempotencia."
                    .to_string(),
            )
        })?
        .to_string();

    let timestamp = campos.timestamp().unwrap_or_default();
    let mut payload = censurar(cuerpo);
    canonizar_timestamp_en_payload(&mut payload);
    let payload_hash = hash_payload(&payload);
    let nota = revisar(accion, campos);

    Ok(AvisoPreparado {
        accion,
        clave_idempotencia: clave_idempotencia(accion, &comanda, timestamp),
        payload,
        payload_hash,
        estado: if nota.is_some() {
            "requiere_revision"
        } else {
            "recibido"
        },
        nota,
    })
}

/// Deja la marca de tiempo del payload en la misma forma canónica que usa la
/// clave de idempotencia. Sin esto, un reenvío del mismo aviso con la hora
/// escrita de otra manera («...T14:00:00Z» y «...T14:00:00.000Z») tendría la
/// misma clave y otra huella, y se tomaría por un conflicto en vez de por un
/// duplicado. Se conserva el mismo instante, solo cambia cómo se escribe.
fn canonizar_timestamp_en_payload(payload: &mut Value) {
    let Some(objeto) = payload.as_object_mut() else {
        return;
    };
    for (clave, valor) in objeto.iter_mut() {
        if !ALIAS_TIMESTAMP
            .iter()
            .any(|alias| normalizar_clave(alias) == normalizar_clave(clave))
        {
            continue;
        }
        if let Some(texto) = valor.as_str() {
            let canonico = timestamp_canonico(texto);
            if !canonico.is_empty() && canonico != texto {
                *valor = Value::String(canonico);
            }
        }
    }
}

/// Clave única del buzón: acción + comanda + marca de tiempo (decisión del plan
/// 199A-1). El timestamp se canoniza para que un reintento del TPV con otra
/// precisión o con la `Z` en minúscula no se cuele como aviso nuevo.
#[must_use]
pub fn clave_idempotencia(accion: AccionAviso, comanda: &str, timestamp: &str) -> String {
    format!(
        "{}|{}|{}",
        accion.slug(),
        comanda.trim(),
        timestamp_canonico(timestamp)
    )
}

/// Marca de tiempo del TPV en UTC y con precisión fija. El contrato del
/// distribuidor dice que `TimeStamp` viaja en UTC; cuando llega sin zona se
/// interpreta como UTC, no como hora local.
fn timestamp_canonico(crudo: &str) -> String {
    let texto = crudo.trim();
    if texto.is_empty() {
        return String::new();
    }
    if let Ok(fecha) = DateTime::parse_from_rfc3339(texto) {
        return fecha
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Micros, true);
    }
    for formato in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(fecha) = NaiveDateTime::parse_from_str(texto, formato) {
            return fecha.and_utc().to_rfc3339_opts(SecondsFormat::Micros, true);
        }
    }
    texto.to_string()
}

/// Motivo por el que un aviso admisible se guarda como `requiere_revision`
/// (nunca se rechaza por esto: un valor raro se registra y no se actúa).
fn revisar(accion: AccionAviso, campos: &CamposAviso) -> Option<String> {
    let mut notas: Vec<String> = Vec::new();

    if campos.timestamp().is_none() {
        notas.push("sin TimeStamp".to_string());
    }

    match accion {
        AccionAviso::CambiarEstado => match campos.estado() {
            None => notas.push("CambiarEstado sin Estado".to_string()),
            Some(valor) => {
                if !matches!(valor.parse::<i32>(), Ok(0..=3)) {
                    notas.push(format!(
                        "Estado fuera de 0-3 ({valor}); se registra sin actuar"
                    ));
                }
            }
        },
        AccionAviso::Finalizar => {
            match campos.tipo_documento() {
                None => notas.push("Finalizar sin TipoDocumento".to_string()),
                Some(valor) => {
                    if !matches!(valor.parse::<i32>(), Ok(1..=6)) {
                        notas.push(format!(
                            "TipoDocumento fuera de 1-6 ({valor}); se registra sin actuar"
                        ));
                    }
                }
            }
            if campos.serie().is_none() || campos.numero().is_none() {
                notas.push("Finalizar sin Serie o sin Numero".to_string());
            }
        }
        AccionAviso::Traspasar | AccionAviso::TraspasarLinea => {
            if campos.comanda_origen().is_none() {
                notas.push("traspaso sin comanda de origen".to_string());
            }
            for (nombre, valor) in [
                ("SalonOrigen", campos.salon_origen()),
                ("SalonDestino", campos.salon_destino()),
                ("MesaOrigen", campos.mesa_origen()),
                ("MesaDestino", campos.mesa_destino()),
            ] {
                if let Some(valor) = valor {
                    if valor.parse::<i32>().is_err() {
                        notas.push(format!("{nombre} no numérico ({valor})"));
                    }
                }
            }
        }
        AccionAviso::Crear | AccionAviso::Modificar | AccionAviso::Borrar => {}
    }

    if notas.is_empty() {
        None
    } else {
        Some(notas.join("; "))
    }
}

/* --------------------------------------------------------------------------
 * Censura y huella
 * -------------------------------------------------------------------------- */

/// Nombres de campo que nunca deben quedar escritos en claro (ni en el buzón ni
/// en los logs). El secreto compartido viaja como un parámetro más, así que hay
/// que sustituirlo antes de guardar el payload.
const CLAVES_SECRETAS: &[&str] = &[
    "secreto",
    "secret",
    "token",
    "apikey",
    "password",
    "contrasena",
    "authorization",
    "credencial",
];

pub(crate) const CENSURA: &str = "***";

/// Sustituye por `***` el valor de cualquier campo con nombre de secreto, a
/// cualquier profundidad razonable.
#[must_use]
pub fn censurar(valor: &Value) -> Value {
    censurar_en(valor, 0)
}

fn censurar_en(valor: &Value, profundidad: u8) -> Value {
    if profundidad > 8 {
        return Value::String("[demasiado anidado]".to_string());
    }
    match valor {
        Value::Object(objeto) => Value::Object(
            objeto
                .iter()
                .map(|(clave, valor)| {
                    let censurado = if CLAVES_SECRETAS.contains(&normalizar_clave(clave).as_str()) {
                        Value::String(CENSURA.to_string())
                    } else {
                        censurar_en(valor, profundidad + 1)
                    };
                    (clave.clone(), censurado)
                })
                .collect::<Map<String, Value>>(),
        ),
        Value::Array(elementos) => Value::Array(
            elementos
                .iter()
                .map(|elemento| censurar_en(elemento, profundidad + 1))
                .collect(),
        ),
        otro => otro.clone(),
    }
}

/// SHA-256 del payload censurado: detecta el reenvío de un aviso con la misma
/// clave de idempotencia y cuerpo distinto (eso sí es un problema, y se corta).
#[must_use]
pub fn hash_payload(payload: &Value) -> String {
    let bytes = serde_json::to_vec(payload).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/* --------------------------------------------------------------------------
 * Secreto compartido
 * -------------------------------------------------------------------------- */

/// Secreto esperado, del entorno. `None` = receptor sin configurar, y entonces
/// no se admite ningún aviso (fail-closed).
#[must_use]
pub fn secreto_esperado() -> Option<String> {
    std::env::var("BDP_AVISOS_SECRETO")
        .ok()
        .map(|valor| valor.trim().to_string())
        .filter(|valor| !valor.is_empty())
}

/// Comparación en tiempo constante: ni la longitud ni el contenido filtran por
/// el tiempo de respuesta. Un secreto vacío nunca vale.
#[must_use]
pub fn secreto_valido(esperado: &str, recibido: &str) -> bool {
    let (esperado, recibido) = (esperado.as_bytes(), recibido.as_bytes());
    if esperado.is_empty() || esperado.len() != recibido.len() {
        return false;
    }
    let mut diferencia = 0u8;
    for (a, b) in esperado.iter().zip(recibido) {
        diferencia |= a ^ b;
    }
    diferencia == 0
}
