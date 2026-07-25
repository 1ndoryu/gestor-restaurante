mod api_key;
mod bdp_article_map;
pub mod bdp_pago;
mod campana;
mod canal_reserva;
mod cliente;
pub mod common;
mod configuracion;
mod dashboard;
mod dashboard_reservas;
mod digitalizacion;
mod etiqueta;
mod gasto;
mod notificacion;
mod plano_sala;
mod plantilla_whatsapp;
mod reserva;
mod trabajador;
mod user;
mod venta;
mod venta_linea;

pub use campana::{
    ActualizarCampanaRequest, Campana, CampanaDestinatario, CampanasPaginadas, CampanasQuery,
    CrearCampanaRequest, SegmentoPreview, SegmentoPreviewQuery, CANALES_VALIDOS, SEGMENTOS_VALIDOS,
};
pub use canal_reserva::{CanalReserva, CrearCanalReservaRequest};
pub use cliente::{
    ActualizarClienteRequest, Cliente, ClientesPaginados, ClientesQuery, CrearClienteRequest,
    MergeClientesRequest, MergeClientesResponse,
};
pub use configuracion::{ActualizarConfiguracionRequest, ConfiguracionRestaurante};
pub use dashboard::ResumenEconomico;
pub use dashboard_reservas::{
    AgrupacionCanal, AgrupacionDiaSemana, AgrupacionFecha, AgrupacionHora, AgrupacionTurno,
    AnalisisReservas, DashboardReservas, OcupacionReservas, ResumenReservas,
};
pub use etiqueta::{
    CategoriaEtiqueta, CrearCategoriaEtiquetaRequest, CrearEtiquetaRequest, Etiqueta,
    EtiquetaConCategoria, EtiquetasQuery,
};
pub use gasto::{
    ActualizarGastoRequest, CategoriaGasto, CrearGastoRequest, Gasto, GastosPaginados, GastosQuery,
    ProveedoresQuery, TipoDocumento,
};
pub use plano_sala::{
    ActualizarMesaRequest, ActualizarParedRequest, ActualizarPosicionesParedesRequest,
    ActualizarPosicionesRequest, ActualizarZonaRequest, CombinacionConMesas, CombinacionExport,
    CombinacionMesas, CrearCombinacionRequest, CrearMesaRequest, CrearParedRequest,
    CrearZonaRequest, Mesa, MesaExport, MesaOcupacion, ParedExport, ParedSala, PlanoExport,
    PlanoOcupacion, PlanoOcupacionQuery, PlanoSala, PosicionMesa, PosicionPared, ReservaMesa,
    ZonaConMesas, ZonaExport, ZonaOcupacion, ZonaSala,
};
pub use plantilla_whatsapp::{
    ActualizarPlantillaRequest, CrearPlantillaRequest, PlantillaWhatsapp, PlantillasPaginadas,
    PlantillasQuery, CATEGORIAS_PLANTILLA,
};
pub use reserva::{
    ActualizarReservaRequest, CrearReservaRequest, EstadoReserva, NoShowPorCanal, NoShowQuery,
    NoShowStats, Reserva, ReservasConteo, ReservasPaginadas, ReservasQuery, ResumenDiario,
    ResumenMesQuery,
};
pub use trabajador::{
    ActualizarTrabajadorRequest, CrearTrabajadorRequest, LoginTrabajadorRequest, PermisoSeccion,
    Trabajador, TrabajadorAuthResponse, TrabajadorResponse, SECCIONES_VALIDAS,
};
pub use user::{
    AuthResponse, ForgotPasswordRequest, LoginRequest, MessageResponse, RegisterRequest,
    ResetPasswordRequest, User, UserResponse, UserRole,
};
pub use venta::{
    ActualizarVentaRequest, CanalVenta, CrearVentaRequest, MetodoPago, Turno, Venta,
    VentaConCliente, VentasPaginadas, VentasQuery,
};
mod recordatorio;
pub use api_key::{
    ApiKey, ApiKeyCreatedResponse, ApiKeyResponse, CamposObligatorios, ChatbotBuscarReservasQuery,
    ChatbotCrearReservaRequest, ChatbotReservaResponse, CrearApiKeyRequest, DisponibilidadResponse,
    FranjaDisponibilidad, RestauranteInfoResponse, ZonaResumen,
};
pub use digitalizacion::{DatosDocumentoExtraidos, DigitalizarDocumentoRequest};
pub use notificacion::{Notificacion, NotificacionEvent};
pub use recordatorio::{
    ActualizarReglaRequest, CrearReglaRequest, HistorialRecordatorios, RecordatorioEnviado,
    RecordatorioEnviadoDetalle, ReglaRecordatorio, ReglasPaginadas, ReglasQuery,
    CANALES_RECORDATORIO,
};
mod integracion_marketing;
pub use integracion_marketing::{
    ActualizarIntegracionesRequest, IntegracionMarketing, IntegracionMarketingPublica,
};
mod resena;
pub use resena::{
    Resena, ResenaAdmin, ResenaPublicaResponse, ResenasPaginadas, ResenasQuery,
    ResponderResenaRequest, ResponderResenaResponse,
};
mod inactividad;
pub use bdp_article_map::{
    ActualizarBdpArticleMapRequest, BdpArticleMap, CrearBdpArticleMapRequest,
};
pub use bdp_pago::BdpPago;
pub use inactividad::{
    ActualizarReglaInactividadRequest, CrearReglaInactividadRequest, ReglaInactividad,
};
pub use venta_linea::{CrearVentaLineaRequest, VentaLinea};
