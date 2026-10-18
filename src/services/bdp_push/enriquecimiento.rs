use serde_json::Value;

use crate::services::bdp_weblink::BdpWeblinkClient;
use crate::services::bdp_weblink_catalog::{BdpGetArticleRequest, BdpModifyArticleRequest};

/* [Q2.2] Enriquecimiento de ModifyArticle: lee el ArticleData completo desde
 * BDP vía GetArticle y lo fusiona con los campos parciales del payload de la
 * cola. Esto garantiza que ModifyArticleAndUpdateProfile reciba todos los
 * campos obligatorios (solución al NullReferenceException del endpoint real).
 *
 * La fusión sobreescribe en el ArticleData completo solo los campos que el
 * payload original ya trae; el resto (perfiles, precios no tocados, flags no
 * modificados) se conservan tal cual los devuelve GetArticle. Los campos
 * `profiles_list` y `all_profiles` del payload original se mantienen sin
 * alterar. */
pub(super) async fn enriquecer_payload_modificar_articulo(
    client: &BdpWeblinkClient<'_>,
    payload: &Value,
) -> Result<Value, String> {
    let req: BdpModifyArticleRequest = serde_json::from_value(payload.clone())
        .map_err(|e| format!("payload de modificar inválido: {e}"))?;

    let art_code = req
        .article_data
        .get("ArtCode")
        .and_then(Value::as_i64)
        .ok_or_else(|| "payload de modificar sin ArtCode".to_string())?;

    let get_resp = client
        .get_article(&BdpGetArticleRequest { art_code })
        .await
        .map_err(|e| format!("GetArticle falló durante enriquecimiento: {e}"))?;

    let full_data = get_resp
        .get("ArticleData")
        .ok_or_else(|| "GetArticle devolvió respuesta sin ArticleData".to_string())?
        .clone();

    let full_data = fusionar_article_data(full_data, &req.article_data)?;

    let enriquecido = BdpModifyArticleRequest {
        article_data: full_data,
        profiles_list: req.profiles_list,
        all_profiles: req.all_profiles,
    };

    serde_json::to_value(&enriquecido)
        .map_err(|e| format!("No se pudo serializar payload enriquecido: {e}"))
}

pub(super) fn fusionar_article_data(mut completo: Value, parcial: &Value) -> Result<Value, String> {
    let completo_obj = completo
        .as_object_mut()
        .ok_or_else(|| "GetArticle devolvió ArticleData no objeto".to_string())?;
    let parcial_obj = parcial
        .as_object()
        .ok_or_else(|| "payload de modificar tiene ArticleData no objeto".to_string())?;

    for (key, value) in parcial_obj {
        completo_obj.insert(key.clone(), value.clone());
    }
    Ok(completo)
}
