/* [21AB-4] Checks de configuracion de produccion que el preflight no miraba:
 * perfil de articulos, IVA (`bdp_tav_map`), perfil de documentos, cliente por
 * defecto y tarifa del catalogo. Todos comparan contra listas REALES del TPV:
 * un codigo no se da por bueno por estar escrito en la configuracion, que es
 * justo lo que Guillermo pidio no hacer (nada inventado). */

use serde_json::Value;

use super::campos::{customer_code, number_f64, number_i64, value_array};
use super::{BdpSyncDryRunCheck, BdpSyncDryRunResponse, BdpSyncPreflightService};
use crate::models::ConfiguracionRestaurante;
use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{
    BdpExportArticlesRequest, BdpExportCustomersRequest, BdpExportDocumentsRequest,
    BdpGetPosArticlesRequest, BDP_ARTICLE_LIST_KEYS,
};

/// Paginas del perfil que se recorren buscando los IVA configurados. BDP
/// pagina de 10 en 10 (es la pagina probada contra el BDP real); 20 paginas
/// cubren de sobra los pocos IVA de una carta y acotan la espera.
const PAGINAS_IVA: i32 = 20;
/// Dias que se piden al perfil de documentos: ventana corta, solo para saber
/// si el perfil existe y responde.
const DIAS_DOCUMENTOS: i64 = 7;

impl BdpSyncPreflightService {
    /// [21AB-4] Los cinco checks de configuracion de produccion, en el orden
    /// en que se leen en la pantalla. Van juntos para que `execute` siga
    /// cabiendo en el limite de lineas y para que el conjunto se vea de una vez.
    pub(super) async fn check_configuracion(
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        Self::check_items_profile(config, client, response).await;
        Self::check_tav_map(config, client, response).await;
        Self::check_documentos_perfil(config, client, response).await;
        Self::check_default_customer(config, client, response).await;
        Self::check_catalog_price_type(config, client, response).await;
    }

    /// [21AB-4] El perfil de articulos es la lista que BDP resuelve en
    /// `CreateOrder` (`ItemsProfileId`). Se comprueba contra los perfiles que
    /// el TPV ofrece para crear articulos, no contra lo guardado en config.
    pub(super) async fn check_items_profile(
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        const ENDPOINT: &str = "/API/ProfilesLists/GetCreateArticleList";
        let listado = match client.get_profiles_create_article_list().await {
            Ok(valor) => valor,
            Err(error) => {
                response.checks.push(BdpSyncDryRunCheck::error(
                    "Perfil de articulos",
                    ENDPOINT,
                    format!("No se pudo leer la lista de perfiles: {error}"),
                ));
                return;
            }
        };
        let perfiles = value_array(&listado, &["ProfilesList", "Profiles"]);
        let existe = perfiles.is_some_and(|items| {
            items.iter().any(|item| {
                number_i64(item, &["Profile", "Id"]) == Some(i64::from(config.bdp_items_profile_id))
            })
        });
        let check = if existe {
            BdpSyncDryRunCheck::ok(
                "Perfil de articulos",
                ENDPOINT,
                "El perfil de articulos configurado existe en el TPV",
                perfiles.map(Vec::len),
                Some(config.bdp_items_profile_id.to_string()),
            )
        } else {
            BdpSyncDryRunCheck::error(
                "Perfil de articulos",
                ENDPOINT,
                format!(
                    "El perfil {} no aparece entre los perfiles del TPV",
                    config.bdp_items_profile_id
                ),
            )
        };
        response.checks.push(check);
    }

    /// [21AB-4] `bdp_tav_map` traduce IVA (%) al codigo TAV del TPV y solo se
    /// usa al crear articulos. Se valida contra los pares `TAVCode`/`TAVPer`
    /// que traen los articulos del perfil: si el par no aparece, el codigo no
    /// es real aunque este escrito en config.
    pub(super) async fn check_tav_map(
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        const ENDPOINT: &str = "/API/Articles/GetPOSList";
        let Some(mapa) = config
            .bdp_tav_map
            .as_object()
            .filter(|mapa| !mapa.is_empty())
        else {
            response.checks.push(BdpSyncDryRunCheck {
                nombre: "Mapa de IVA".into(),
                endpoint: "bdp_tav_map".into(),
                ok: true,
                mensaje: "Sin mapa de IVA: crear articulos desde la web fallara hasta mapear cada IVA a su codigo TAV.".into(),
                cantidad: Some(0),
                muestra: None,
            });
            return;
        };
        /* Lo que hay que encontrar en el TPV: (codigo TAV, porcentaje) de cada
         * entrada del mapa. Se deja de paginar en cuanto estan todos. */
        let esperados: Vec<(i32, f64)> = mapa
            .iter()
            .filter_map(|(clave, valor)| {
                Some((
                    i32::try_from(valor.as_i64()?).ok()?,
                    clave.trim().parse::<f64>().ok()?,
                ))
            })
            .collect();

        let (pares, revisados) =
            match Self::pares_iva(client, config.bdp_items_profile_id, &esperados).await {
                Ok(resultado) => resultado,
                Err(error) => {
                    response.checks.push(BdpSyncDryRunCheck::error(
                        "Mapa de IVA",
                        ENDPOINT,
                        format!("No se pudieron leer los articulos del perfil: {error}"),
                    ));
                    return;
                }
            };

        let mut problemas = Vec::new();
        let mut validos = 0;
        for (clave, valor) in mapa {
            let Some(codigo) = valor.as_i64().and_then(|code| i32::try_from(code).ok()) else {
                problemas.push(format!("{clave}: '{valor}' no es un codigo TAV numerico"));
                continue;
            };
            let Ok(porcentaje) = clave.trim().parse::<f64>() else {
                problemas.push(format!("{clave}: la clave no es un porcentaje de IVA"));
                continue;
            };
            let coincide = pares
                .iter()
                .any(|(tav, per)| *tav == codigo && (*per - porcentaje).abs() < 0.01);
            if coincide {
                validos += 1;
            } else {
                problemas.push(format!(
                    "{clave}% -> TAV {codigo}: no aparece en los {revisados} articulos revisados"
                ));
            }
        }

        let check = if problemas.is_empty() {
            BdpSyncDryRunCheck::ok(
                "Mapa de IVA",
                ENDPOINT,
                format!("{validos} IVA mapeados a codigos TAV reales del perfil"),
                Some(validos),
                None,
            )
        } else {
            BdpSyncDryRunCheck::error(
                "Mapa de IVA",
                ENDPOINT,
                format!("{} problema(s): {}", problemas.len(), problemas.join("; ")),
            )
        };
        response.checks.push(check);
    }

    /// [21AB-4] Perfil de exportacion de documentos de venta. Se pide una
    /// ventana corta con `OnlyLastMovements=false`: con `true` el TPV marca los
    /// documentos como exportados y lo que devuelve no se puede volver a leer.
    pub(super) async fn check_documentos_perfil(
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        const ENDPOINT: &str = "/API/ExportProfiles/Documents";
        let Some(perfil) = config.bdp_documentos_perfil.filter(|perfil| *perfil > 0) else {
            response.checks.push(BdpSyncDryRunCheck {
                nombre: "Perfil de documentos".into(),
                endpoint: "bdp_documentos_perfil".into(),
                ok: true,
                mensaje:
                    "Sin perfil de documentos: la lectura de documentos de venta no esta activa."
                        .into(),
                cantidad: Some(0),
                muestra: None,
            });
            return;
        };

        let hoy = chrono::Utc::now().date_naive();
        let desde = hoy - chrono::Duration::days(DIAS_DOCUMENTOS);
        let peticion = BdpExportDocumentsRequest {
            export_profile_code: perfil,
            initial_date: desde.format("%Y-%m-%d").to_string(),
            final_date: hoy.format("%Y-%m-%d").to_string(),
            only_last_movements: false,
        };
        let check = match client.export_documents(&peticion).await {
            Ok(valor) if valor.get("DocumentsLists").is_some() => BdpSyncDryRunCheck::ok(
                "Perfil de documentos",
                ENDPOINT,
                format!("El perfil {perfil} exporta documentos en el TPV"),
                valor
                    .get("DocumentsLists")
                    .and_then(Value::as_array)
                    .map(Vec::len),
                Some(format!("perfil {perfil}")),
            ),
            /* Sin la clave de lista el perfil responde otra cosa: no sirve
             * como perfil de exportacion de ventas. */
            Ok(_) => BdpSyncDryRunCheck::error(
                "Perfil de documentos",
                ENDPOINT,
                format!("El perfil {perfil} respondio sin la lista de documentos"),
            ),
            Err(error) => BdpSyncDryRunCheck::error(
                "Perfil de documentos",
                ENDPOINT,
                format!("El perfil {perfil} no sirve para exportar: {error}"),
            ),
        };
        response.checks.push(check);
    }

    /// [21AB-4] Cliente por defecto: es el `Customer.Code` que viaja en las
    /// ventas sin cliente. Se valida contra la exportacion real de clientes.
    pub(super) async fn check_default_customer(
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        const ENDPOINT: &str = "/API/Customers/Export";
        let codigo = config.bdp_default_customer_code.trim();
        if codigo.is_empty() {
            response.checks.push(BdpSyncDryRunCheck {
                nombre: "Cliente por defecto".into(),
                endpoint: "bdp_default_customer_code".into(),
                ok: true,
                mensaje: "Sin cliente por defecto: las ventas sin cliente no enviaran Customer."
                    .into(),
                cantidad: Some(0),
                muestra: None,
            });
            return;
        }
        let Ok(esperado) = codigo.parse::<i32>() else {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Cliente por defecto",
                "bdp_default_customer_code",
                format!("'{codigo}' no es un codigo de cliente numerico de BDP"),
            ));
            return;
        };

        let check = match client
            .export_customers(&BdpExportCustomersRequest::default())
            .await
        {
            Ok(valor) => {
                let clientes: Vec<Value> = value_array(&valor, &["Customers"])
                    .cloned()
                    .unwrap_or_default();
                if clientes
                    .iter()
                    .any(|cliente| customer_code(cliente) == Some(esperado))
                {
                    BdpSyncDryRunCheck::ok(
                        "Cliente por defecto",
                        ENDPOINT,
                        "El cliente por defecto existe en el TPV",
                        Some(clientes.len()),
                        Some(esperado.to_string()),
                    )
                } else {
                    BdpSyncDryRunCheck::error(
                        "Cliente por defecto",
                        ENDPOINT,
                        format!(
                            "El cliente {esperado} no aparece en el TPV ({} clientes leidos)",
                            clientes.len()
                        ),
                    )
                }
            }
            Err(error) => BdpSyncDryRunCheck::error(
                "Cliente por defecto",
                ENDPOINT,
                format!("No se pudieron leer los clientes: {error}"),
            ),
        };
        response.checks.push(check);
    }

    /// [21AB-4] Tarifa del catalogo (`TypePrice` 1-5 de `ExportArticles`). Se
    /// comprueba que la tarifa configurada devuelve precios de verdad: una
    /// tarifa sin precios deja el catalogo vacio sin dar error.
    pub(super) async fn check_catalog_price_type(
        config: &ConfiguracionRestaurante,
        client: &BdpWeblinkClient<'_>,
        response: &mut BdpSyncDryRunResponse,
    ) {
        const ENDPOINT: &str = "/API/Articles/Export";
        let tarifa = config.bdp_catalog_price_type;
        if !(1..=5).contains(&tarifa) {
            response.checks.push(BdpSyncDryRunCheck::error(
                "Tarifa del catalogo",
                "bdp_catalog_price_type",
                format!("TypePrice={tarifa}: BDP solo admite tarifas de 1 a 5"),
            ));
            return;
        }

        let check = match client
            .export_articles(&BdpExportArticlesRequest::all_web_articles(tarifa))
            .await
        {
            Ok(valor) => {
                let articulos: Vec<Value> = value_array(&valor, BDP_ARTICLE_LIST_KEYS)
                    .cloned()
                    .unwrap_or_default();
                /* `ExportArticles` devuelve la tarifa pedida en `Price`; las
                 * variantes `Price1..Price5` se aceptan por si otra version las
                 * manda con sufijo. Lo que importa es que haya precio > 0: una
                 * tarifa vacia deja el catalogo sin precios y sin error. */
                let campo = format!("Price{tarifa}");
                let campos = ["Price", "Price1", campo.as_str()];
                let con_precio = articulos
                    .iter()
                    .filter(|articulo| {
                        number_f64(articulo, &campos).is_some_and(|precio| precio > 0.0)
                    })
                    .count();
                if articulos.is_empty() {
                    BdpSyncDryRunCheck::error(
                        "Tarifa del catalogo",
                        ENDPOINT,
                        "La exportacion no devolvio articulos: el TPV no tiene articulos web",
                    )
                } else if con_precio == 0 {
                    BdpSyncDryRunCheck::error(
                        "Tarifa del catalogo",
                        ENDPOINT,
                        format!(
                            "Ninguno de los {} articulos exportados trae precio: la tarifa {tarifa} no esta configurada",
                            articulos.len()
                        ),
                    )
                } else {
                    BdpSyncDryRunCheck::ok(
                        "Tarifa del catalogo",
                        ENDPOINT,
                        format!("{con_precio} articulos con precio en la tarifa {tarifa}"),
                        Some(con_precio),
                        Some(format!("tarifa {tarifa}")),
                    )
                }
            }
            Err(error) => BdpSyncDryRunCheck::error(
                "Tarifa del catalogo",
                ENDPOINT,
                format!("No se pudo exportar el catalogo: {error}"),
            ),
        };
        response.checks.push(check);
    }

    /// Pares (`TAVCode`, `TAVPer`) de los articulos del perfil y cuantos se leyeron.
    /// Se para en la primera pagina corta, cuando se agotan las paginas o en
    /// cuanto aparecen todos los pares esperados: paginar de mas solo cuesta
    /// tiempo contra el TPV.
    async fn pares_iva(
        client: &BdpWeblinkClient<'_>,
        perfil: i32,
        esperados: &[(i32, f64)],
    ) -> Result<(Vec<(i32, f64)>, usize), String> {
        let tamano = usize::try_from(super::BDP_DRY_RUN_PAGE_SIZE).unwrap_or(0);
        let mut pares: Vec<(i32, f64)> = Vec::new();
        let mut revisados = 0usize;
        for pagina in 1..=PAGINAS_IVA {
            let mut peticion =
                BdpGetPosArticlesRequest::first_page(perfil, super::BDP_DRY_RUN_PAGE_SIZE);
            peticion.actual_page = pagina;
            let crudo = client
                .get_pos_articles(&peticion)
                .await
                .map_err(|error| error.to_string())?;
            let Some(items) = value_array(&crudo, BDP_ARTICLE_LIST_KEYS) else {
                break;
            };
            revisados += items.len();
            for item in items {
                if let (Some(codigo), Some(porcentaje)) = (
                    number_i64(item, &["TAVCode"]),
                    number_f64(item, &["TAVPer"]),
                ) {
                    let Ok(codigo) = i32::try_from(codigo) else {
                        continue;
                    };
                    if !pares
                        .iter()
                        .any(|(tav, per)| *tav == codigo && (*per - porcentaje).abs() < 0.01)
                    {
                        pares.push((codigo, porcentaje));
                    }
                }
            }
            let todos = esperados.iter().all(|(tav, per)| {
                pares
                    .iter()
                    .any(|(t, p)| t == tav && (p - per).abs() < 0.01)
            });
            if items.len() < tamano || todos {
                break;
            }
        }
        Ok((pares, revisados))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    /* [21AB-4] Cada check se prueba con su caso positivo y su caso negativo:
     * un codigo escrito en la configuracion solo vale si el TPV lo devuelve. */

    fn config_de_prueba() -> ConfiguracionRestaurante {
        let mut config = super::super::tests::config();
        config.bdp_items_profile_id = 2;
        config
    }

    async fn login_en(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/Auth/Login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "AuthSession": {"Token": "t", "ExpiresIn_InSecconds": 3540}
            })))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn perfil_de_articulos_se_valida_contra_el_tpv() {
        let server = MockServer::start().await;
        login_en(&server).await;
        Mock::given(method("POST"))
            .and(path("/API/ProfilesLists/GetCreateArticleList"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "ProfilesList": [{"Profile": 1, "ProfileName": "PERFIL 1"},
                                 {"Profile": 2, "ProfileName": "PERFIL 2"}]
            })))
            .mount(&server)
            .await;
        let mut config = config_de_prueba();
        config.bdp_base_url = server.uri();
        let client = BdpWeblinkClient::new(&config);

        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_items_profile(&config, &client, &mut response).await;
        assert!(response.checks.last().unwrap().ok, "{:?}", response.checks);

        /* Copia con otro perfil: el cliente presta `config` durante su vida. */
        let mut otro = config.clone();
        otro.bdp_items_profile_id = 7;
        let client = BdpWeblinkClient::new(&otro);
        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_items_profile(&otro, &client, &mut response).await;
        let check = response.checks.last().unwrap();
        assert!(!check.ok);
        assert!(check.mensaje.contains("perfil 7"), "{}", check.mensaje);
    }

    /* Mapa de IVA: el par (codigo TAV, porcentaje) tiene que aparecer en los
     * articulos del perfil. */
    #[tokio::test]
    async fn mapa_de_iva_se_valida_contra_los_articulos() {
        let server = MockServer::start().await;
        login_en(&server).await;
        Mock::given(method("POST"))
            .and(path("/API/Articles/GetPOSList"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "ArticleListData": [
                    {"ArtCode": 1001, "ArtDescription": "COCA", "Price1": 1.0,
                     "TAVCode": 1, "TAVPer": 10.0},
                    {"ArtCode": 1002, "ArtDescription": "VINO", "Price1": 3.0,
                     "TAVCode": 2, "TAVPer": 21.0}
                ]
            })))
            .mount(&server)
            .await;
        let mut config = config_de_prueba();
        config.bdp_base_url = server.uri();

        config.bdp_tav_map = json!({"10": 1, "21": 2});
        let client = BdpWeblinkClient::new(&config);
        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_tav_map(&config, &client, &mut response).await;
        assert!(response.checks.last().unwrap().ok, "{:?}", response.checks);

        /* Un 4 % al que se le asigna el TAV 3 no existe en ningun articulo. */
        let mut otro = config.clone();
        otro.bdp_tav_map = json!({"10": 1, "4": 3});
        let client = BdpWeblinkClient::new(&otro);
        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_tav_map(&otro, &client, &mut response).await;
        let check = response.checks.last().unwrap();
        assert!(!check.ok, "{check:?}");
        assert!(check.mensaje.contains("4%"), "{}", check.mensaje);
    }

    /* Sin mapa de IVA avisa pero no bloquea: no configurado no es un error. */
    #[tokio::test]
    async fn sin_mapa_de_iva_avisa_sin_bloquear() {
        let server = MockServer::start().await;
        let mut config = config_de_prueba();
        config.bdp_base_url = server.uri();
        config.bdp_tav_map = json!({});
        let client = BdpWeblinkClient::new(&config);
        let mut response = BdpSyncDryRunResponse::new(true);

        BdpSyncPreflightService::check_tav_map(&config, &client, &mut response).await;
        assert!(response.checks.last().unwrap().ok);
        assert!(
            server
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty(),
            "sin mapa no se llama al TPV"
        );
    }

    /* Perfil de documentos: `DocumentsLists` presente vale; el 403901 que BDP
     * devuelve para un perfil incorrecto se convierte en error. */
    #[tokio::test]
    async fn perfil_de_documentos_se_valida_con_la_respuesta_real() {
        let server = MockServer::start().await;
        login_en(&server).await;
        Mock::given(method("POST"))
            .and(path("/API/ExportProfiles/Documents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "DocumentsLists": [{"Document": 1}], "ErrorMessage": ""
            })))
            .mount(&server)
            .await;
        let mut config = config_de_prueba();
        config.bdp_base_url = server.uri();
        config.bdp_documentos_perfil = Some(1);
        let client = BdpWeblinkClient::new(&config);

        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_documentos_perfil(&config, &client, &mut response).await;
        assert!(response.checks.last().unwrap().ok, "{:?}", response.checks);

        let server = MockServer::start().await;
        login_en(&server).await;
        Mock::given(method("POST"))
            .and(path("/API/ExportProfiles/Documents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "DocumentsLists": null,
                "ErrorMessage": "[403901]-PERFIL DE EXPORTACION INCORRECTO"
            })))
            .mount(&server)
            .await;
        config.bdp_base_url = server.uri();
        let client = BdpWeblinkClient::new(&config);
        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_documentos_perfil(&config, &client, &mut response).await;
        let check = response.checks.last().unwrap();
        assert!(!check.ok);
        assert!(check.mensaje.contains("403901"), "{}", check.mensaje);
    }

    /* Cliente por defecto: se busca por codigo en la exportacion real. */
    #[tokio::test]
    async fn cliente_por_defecto_se_busca_en_el_tpv() {
        let server = MockServer::start().await;
        login_en(&server).await;
        Mock::given(method("POST"))
            .and(path("/API/Customers/Export"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "Customers": [{"Customer": 1, "CommercialName": "CONTADO"},
                              {"Code": "42", "CommercialName": "HABITUAL"}]
            })))
            .mount(&server)
            .await;
        let mut config = config_de_prueba();
        config.bdp_base_url = server.uri();
        config.bdp_default_customer_code = "42".to_string();
        let client = BdpWeblinkClient::new(&config);

        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_default_customer(&config, &client, &mut response).await;
        assert!(response.checks.last().unwrap().ok, "{:?}", response.checks);

        let mut otro = config.clone();
        otro.bdp_default_customer_code = "99".to_string();
        let client = BdpWeblinkClient::new(&otro);
        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_default_customer(&otro, &client, &mut response).await;
        let check = response.checks.last().unwrap();
        assert!(!check.ok);
        assert!(check.mensaje.contains("99"), "{}", check.mensaje);

        /* BDP usa codigos enteros: un nombre no puede pasar. */
        let mut otro = config.clone();
        otro.bdp_default_customer_code = "CONTADO".to_string();
        let client = BdpWeblinkClient::new(&otro);
        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_default_customer(&otro, &client, &mut response).await;
        assert!(!response.checks.last().unwrap().ok);
    }

    /* Tarifa del catalogo: fuera de 1-5 se rechaza sin red; una tarifa sin
     * precios deja el catalogo vacio y se reporta. */
    #[tokio::test]
    async fn tarifa_del_catalogo_se_comprueba_contra_precios_reales() {
        let server = MockServer::start().await;
        login_en(&server).await;
        Mock::given(method("POST"))
            .and(path("/API/Articles/Export"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "Articles": [{"Article": 1001, "Description": "COCA", "Price": 1.05}]
            })))
            .mount(&server)
            .await;
        let mut config = config_de_prueba();
        config.bdp_base_url = server.uri();
        let client = BdpWeblinkClient::new(&config);

        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_catalog_price_type(&config, &client, &mut response).await;
        assert!(response.checks.last().unwrap().ok, "{:?}", response.checks);

        let mut otro = config.clone();
        otro.bdp_catalog_price_type = 9;
        let client = BdpWeblinkClient::new(&otro);
        let mut response = BdpSyncDryRunResponse::new(true);
        BdpSyncPreflightService::check_catalog_price_type(&otro, &client, &mut response).await;
        let check = response.checks.last().unwrap();
        assert!(!check.ok);
        assert!(check.mensaje.contains("1 a 5"), "{}", check.mensaje);
    }

    #[tokio::test]
    async fn catalogo_sin_precios_rechaza_la_tarifa() {
        let server = MockServer::start().await;
        login_en(&server).await;
        Mock::given(method("POST"))
            .and(path("/API/Articles/Export"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ErrorMessage": "",
                "Articles": [{"Article": 1001, "Description": "COCA", "Price": 0.0}]
            })))
            .mount(&server)
            .await;
        let mut config = config_de_prueba();
        config.bdp_base_url = server.uri();
        let client = BdpWeblinkClient::new(&config);
        let mut response = BdpSyncDryRunResponse::new(true);

        BdpSyncPreflightService::check_catalog_price_type(&config, &client, &mut response).await;
        let check = response.checks.last().unwrap();
        assert!(!check.ok);
        assert!(
            check.mensaje.contains("no esta configurada"),
            "{}",
            check.mensaje
        );
    }
}
