#![cfg(test)]
/* [039A-1/H-Q1-03] Tests del parseo compartido de colecciones de artículos:
 * ExportArticles (Articles/Code/Description) y GetPOSList por perfil
 * (ArticleListData/ArtCode/ArtDescription/TAVPer). */

use rust_decimal::Decimal;

use super::articulos::parse_articles_value;

use serde_json::json;

#[test]
fn parses_get_pos_list_real_shape() {
    let raw = json!({
        "ArticleListData": [{
            "ArtCode": 1001,
            "ArtDescription": "COCA-COLA",
            "Price1": 1.05,
            "TAVPer": 10.0
        }]
    });
    let items = parse_articles_value(&raw);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].art_code(), Some("1001"));
    assert_eq!(items[0].description(), "COCA-COLA");
    assert_eq!(items[0].tax1.unwrap(), Decimal::from(10));
    assert_eq!(items[0].price1.unwrap(), Decimal::new(105, 2));
}

#[test]
fn parses_export_articles_shape_and_aliases_list_key() {
    let raw = json!({
        "ArticlesListData": [{
            "Code": "A7",
            "Description": "MENU",
            "Price1": 12,
            "Active": false
        }]
    });
    let items = parse_articles_value(&raw);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].art_code(), Some("A7"));
    assert!(!items[0].active);
}

#[test]
fn empty_or_missing_collection_returns_empty() {
    assert!(parse_articles_value(&json!({"ArticleListData": []})).is_empty());
    assert!(parse_articles_value(&json!({"ErrorMessage": "x"})).is_empty());
    assert!(parse_articles_value(&json!({"ArticleListData": "no-array"})).is_empty());
}

#[test]
fn items_missing_optional_fields_parse_with_defaults() {
    let raw = json!({
        "ArticleListData": [{"ArtCode": 2, "ArtDescription": "SIN IVA"}]
    });
    let items = parse_articles_value(&raw);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].art_code(), Some("2"));
    assert!(items[0].tax1.is_none());
    assert_eq!(items[0].price1, None);
    assert!(items[0].active);
}
