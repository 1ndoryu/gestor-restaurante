/* Contratos BDP de articulos TPV, clientes, pedidos y departamentos. */

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPosArticlesRequest {
    pub art1: i64,
    pub art2: i64,
    pub dept1: i32,
    pub dept2: i32,
    pub description: String,
    pub description_query_type: i32,
    pub items_per_page: i32,
    pub actual_page: i32,
    #[serde(rename = "nField")]
    pub n_field: i32,
    #[serde(rename = "nOrder")]
    pub n_order: i32,
    pub profile_code: i32,
}

impl BdpGetPosArticlesRequest {
    #[must_use]
    pub fn first_page(profile_code: i32, items_per_page: i32) -> Self {
        Self {
            art1: 1,
            art2: 9_999_999_999_999,
            dept1: 1,
            dept2: 999,
            description: String::new(),
            description_query_type: 0,
            items_per_page,
            actual_page: 1,
            n_field: 1,
            n_order: 0,
            profile_code,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportCustomersRequest {
    pub customer1: i32,
    pub customer2: i32,
}

impl Default for BdpExportCustomersRequest {
    fn default() -> Self {
        Self {
            customer1: 1,
            customer2: 999_999,
        }
    }
}

/* [048A-8] Contrato completo de CreateCustomer según la documentación
 * oficial de BDP (WebLink REST API). El BDP real con módulo de gestión
 * devuelve NullReferenceException si faltan los campos de gestión
 * (PaymentMode, Representative, AreaCode, TAVCode, RateCode), por lo que
 * el payload debe incluir todos los campos del JSON de solicitud, incluso
 * con valores vacíos. La clave del e-mail en la solicitud es `Email`
 * (no `EMail`, que es la clave de la respuesta de ExportCustomers). */
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateCustomerRequest {
    pub code: i32,
    /// Nombre fiscal (máx. 40 caracteres; BDP recorta a 40).
    pub fiscal_name: String,
    /// Nombre comercial (máx. 40 caracteres; BDP recorta a 40).
    pub commercial_name: String,
    /// Dirección (máx. 40 caracteres; BDP recorta a 40).
    pub address: String,
    /// Código postal (máx. 10 caracteres; BDP recorta a 10).
    pub post_code: String,
    /// Población (máx. 40 caracteres; BDP recorta a 40).
    pub town: String,
    /// Provincia (máx. 40 caracteres; BDP recorta a 40).
    pub province: String,
    /// Teléfono fijo (máx. 15 caracteres; BDP recorta a 15).
    pub land_line: String,
    /// Teléfono móvil (máx. 15 caracteres; BDP recorta a 15).
    pub mobile_phone: String,
    /// Identificador fiscal (máx. 15 caracteres; BDP recorta a 15).
    pub fin: String,
    /// Tipo de documento de identificación:
    /// 1=N.I.F., 2=N.I.F. Extranjero, 3=Pasaporte, 4=ID País de Residencia,
    /// 5=Certificado Residencia, 6=Otro Documento.
    /// El BDP real exige un entero (desecha "1.0" con "not a valid integer").
    pub fin_type: i32,
    /// Correo electrónico (máx. 60 caracteres; BDP recorta a 10).
    #[serde(rename = "Email")]
    pub email: String,
    /// Porcentaje de descuento (0,00 a 99,99; más de 2 decimales se redondea).
    pub per_discount: f64,
    /// Código de forma de pago (1-99). En apps con módulo de gestión NO puede ser 0.
    pub payment_mode: i32,
    /// Código de representante (1-9999). En módulo de gestión NO puede ser 0.
    pub representative: i32,
    /// Código de zona (1-999). En módulo de gestión NO puede ser 0.
    pub area_code: i32,
    /// Código de TAV (1-99). En módulo de gestión NO puede ser 0.
    pub tav_code: i32,
    /// Código de tarifa (1-99). En módulo de gestión NO puede ser 0.
    pub rate_code: i32,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCreateOrderRequest {
    pub employee_id: i32,
    pub items_profile_id: i32,
    pub order_end_type: i32,
    pub order_operation_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<bool>,
    pub order: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpOrderIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_number: Option<i32>,
}

impl BdpOrderIdentifier {
    #[must_use]
    pub const fn by_order_id(order_id: i64) -> Self {
        Self {
            order_id: Some(order_id),
            market_id: None,
            marketplace_order_id: None,
            room_number: None,
            table_number: None,
        }
    }

    #[must_use]
    pub fn by_market(market_id: i32, marketplace_order_id: impl Into<String>) -> Self {
        Self {
            order_id: None,
            market_id: Some(market_id),
            marketplace_order_id: Some(marketplace_order_id.into()),
            room_number: None,
            table_number: None,
        }
    }

    #[must_use]
    pub const fn by_table(room_number: i32, table_number: i32) -> Self {
        Self {
            order_id: None,
            market_id: None,
            marketplace_order_id: None,
            room_number: Some(room_number),
            table_number: Some(table_number),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetOrderRequest {
    pub order_identifier: BdpOrderIdentifier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpCancelOrderRequest {
    pub pos_id: i32,
    pub order_identifier: BdpOrderIdentifier,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpOrderPayment {
    pub tender_id: i32,
    pub amount: Decimal,
    pub payment_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpAddOrderPaymentRequest {
    pub order_identifier: BdpOrderIdentifier,
    pub payment: BdpOrderPayment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub employee_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpInvoiceOrderRequest {
    pub pos_id: i32,
    pub employee_id: i32,
    pub order_identifier: BdpOrderIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpExportDepartmentsRequest {
    pub dept1: i32,
    pub dept2: i32,
    pub description: String,
    pub description_query_type: i32,
    pub items_per_page: i32,
    pub actual_page: i32,
    #[serde(rename = "nField")]
    pub n_field: i32,
    #[serde(rename = "nOrder")]
    pub n_order: i32,
}

impl Default for BdpExportDepartmentsRequest {
    fn default() -> Self {
        Self {
            dept1: 1,
            dept2: 999,
            description: String::new(),
            description_query_type: 0,
            items_per_page: 0,
            actual_page: 1,
            n_field: 1,
            n_order: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpDepartmentsExportFromProfileRequest {
    pub profile_id: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BdpGetPosRequest {
    pub id: i32,
}
