mod api_key;
mod auth;
/* [147A-F6] módulos BDP — sincronización Glory → BDP WebLink REST API */
pub(crate) mod bdp_order_poller;
mod bdp_sync;
mod bdp_sync_preflight;
pub mod bdp_weblink;
pub mod bdp_weblink_catalog;
mod bdp_explorer;
mod bdp_backup;
mod campana;
mod canal_reserva;
mod chatbot;
mod cliente;
mod configuracion;
mod dashboard;
mod digitalizacion;
pub mod email;
mod etiqueta;
mod gasto;
mod haddock;
mod integracion_marketing;
mod meta_whatsapp;
mod notificacion;
mod plano_sala;
mod plantilla_whatsapp;
mod recordatorio;
mod reserva;
mod twilio;
mod venta;

pub use api_key::ApiKeyService;
pub use auth::AuthService;
pub use bdp_order_poller::BdpOrderPollerService;
pub use bdp_sync::BdpSyncService;
pub use bdp_sync::SyncTablesResult;
pub use bdp_sync_preflight::{BdpSyncDryRunCheck, BdpSyncDryRunResponse, BdpSyncPreflightService};
pub use bdp_weblink::{BdpVersionResponse, BdpWeblinkClient};
pub use bdp_weblink_catalog::{
    BdpCatalogSyncResult, BdpCreateCustomerRequest, BdpExportArticlesRequest,
    BdpExportCustomersRequest, BdpGetPricesArticlesResponse, BdpGetRoomsTablesResponse,
    BdpGetRoomTablesResponse, BdpRoomData,
};
pub use bdp_explorer::{BdpExplorerService, BdpExploracionResultado, ExploracionCategoria};
pub use bdp_backup::{BdpBackupService, BdpSnapshot, BdpAuditEntry, RestoreResult};
pub use campana::CampanaService;
pub use canal_reserva::CanalReservaService;
pub use chatbot::ChatbotService;
pub use cliente::ClienteService;
pub use configuracion::ConfiguracionService;
pub use dashboard::DashboardService;
pub use digitalizacion::DigitalizacionService;
pub use email::EmailService;
pub use etiqueta::EtiquetaService;
pub use gasto::GastoService;
pub use haddock::HaddockService;
pub use integracion_marketing::IntegracionMarketingService;
pub use meta_whatsapp::MetaWhatsappService;
pub use notificacion::NotificacionService;
pub use plano_sala::PlanoSalaService;
pub use plantilla_whatsapp::PlantillaService;
pub use recordatorio::RecordatorioService;
pub use reserva::ReservaService;
pub use twilio::TwilioService;
pub use venta::VentaService;
