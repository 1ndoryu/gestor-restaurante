pub(crate) mod ai_chat;
mod ai_prompts;
mod ai_providers;
pub(crate) mod ai_tools;
mod assignment;
mod audit;
mod auth;
pub mod bandwidth_enforcement;
mod billing_stripe;
mod chat;
mod chat_timing;
pub mod contabo;
pub mod contabo_domains;
pub mod coolify;
pub mod cpu_burst;
pub mod docker_stats;
mod domain_stripe;
pub mod email;
pub mod email_preview;
mod google_auth;
pub mod hosting_runtime;
mod hosting_runtime_backups;
mod hosting_runtime_lightweight_types;
mod hosting_stripe;
pub mod image_processing;
pub mod infrastructure;
pub mod infrastructure_metrics;
mod note;
mod notification;
mod order;
mod order_slugs;
mod payment;
mod payment_method;
mod seed;
pub mod storage_enforcement;
pub mod tc_throttle;
mod test_checkout;
pub mod vps_monitor;
mod vps_stripe;
mod wallet;

pub use ai_chat::{AiChatConfig, AiChatService, AiResponse, AiSessionContext};
pub use assignment::AssignmentService;
pub use audit::AuditService;
pub use auth::AuthService;
pub use billing_stripe::{BillingCheckoutParams, BillingStripeService};
pub use chat::ChatHub;
pub use chat_timing::{ChatTimingService, RateCheckResult, TimingEvent, TimingSessionDeps};
pub use contabo::{ContaboConfig, ContaboService, CreateInstanceParams};
pub use coolify::{CoolifyConfig, CoolifyService};
pub use domain_stripe::{DomainCheckoutParams, DomainStripeService};
pub use email::EmailService;
pub use google_auth::GoogleAuthService;
pub use hosting_runtime::{
	HostingRuntimeBackupEntry, HostingRuntimeBackupReport, HostingRuntimeDeploymentSummary,
	HostingRuntimeKind, HostingRuntimeProvisionResult, HostingRuntimeRestoreReport,
	HostingRuntimeService, HostingRuntimeUpdate,
};
pub use hosting_stripe::{CheckoutParams, HostingStripeService};
pub use note::NoteService;
pub use notification::NotificationHub;
pub use order::format_price_cents;
pub use order::OrderService;
pub use payment::PaymentService;
pub use payment_method::PaymentMethodService;
pub use seed::SeedService;
pub use test_checkout::{checkout_bypass_is_configured, is_checkout_bypass_email};
pub use vps_stripe::{vps_stripe_fee_cents, VpsCheckoutParams, VpsStripeService};
pub use wallet::WalletService;

/* [276A-4.2] Servicios BDP — sincronización Glory → BDP WebLink REST API */
pub(crate) mod bdp_order_poller;
mod bdp_sync;
mod bdp_sync_preflight;
pub(crate) mod bdp_weblink;
mod bdp_weblink_catalog;

/* Servicios de dominio del restaurante necesarios para BDP */
mod configuracion;
mod haddock;
mod venta;

pub use bdp_order_poller::BdpOrderPollerService;
pub use bdp_sync::BdpSyncService;
pub use bdp_sync_preflight::{BdpSyncDryRunResponse, BdpSyncPreflightService};
pub use configuracion::ConfiguracionService;
pub use haddock::HaddockService;
pub use venta::VentaService;
