#![allow(clippy::needless_for_each)] // Generado por utoipa OpenApi derive

mod admin;
mod api_keys;
mod auth;
mod bdp_article_map;
mod bdp_avisos;
mod bdp_backup;
mod bdp_catalogo;
mod bdp_customer_sync;
mod bdp_documentos;
mod bdp_guard;
mod bdp_menu_local;
mod bdp_purchase_note;
mod bdp_push;
mod bdp_stock_revision;

/* Visible para los submódulos como `crate::handlers::AppState`. */
use crate::AppState;

pub use bdp_menu_local::{
    actualizar_menu_local, crear_menu_local, eliminar_menu_local, listar_menus_locales,
    obtener_menu_local,
};
pub use bdp_purchase_note::{
    actualizar_purchase_note_local, conciliar_purchase_note, crear_purchase_note_local,
    eliminar_purchase_note_local, listar_purchase_notes, marcar_borrador_purchase_note,
    sincronizar_purchase_notes,
};
/* [128A-1/F8] Exposición para tests de integración de permisos (D8/M17). */
pub use bdp_article_map::{
    actualizar_article_map, ajustar_stock, crear_article_map, eliminar_article_map,
    registrar_inventario,
};
/* [208A-2/C3] Conteos de inventario persistidos (D3/D4) — exposición para tests. */
pub use bdp_article_map::{crear_conteo_inventario, listar_conteos_inventario};
/* [039A-1/H-P1-02] Fail-closed por modo efectivo en handlers que conectan
 * directo con BDP (login/export) — exposición para tests de regresión. */
pub use bdp_article_map::{importar_catalogo, sync_catalog, sync_stock_almacen};
pub use bdp_customer_sync::{importar_clientes_bdp, BdpCustomerImportRequest};
/* [039A-1/H-P1-03] Fail-closed por modo efectivo en explorar/backup BDP
 * (red real) — exposición para tests de regresión. */
pub use bdp_backup::{
    departamentos_del_perfil, explorar_bdp, formas_pago_bdp, snapshot_completo, snapshot_parcial,
    SnapshotParcialRequest,
};
pub use bdp_guard::exigir_modo_bdp;
/* [208A-2/C4] Cola de sincronización (D5) — exposición para tests. */
pub use bdp_avisos::{
    listar as listar_avisos_bdp, public_routes as rutas_publicas_avisos, recibir_aviso_con_secreto,
    reintentar as reintentar_aviso_bdp, routes as rutas_avisos_bdp,
    ListarQuery as ListarAvisosQuery, RespuestaAviso,
};
pub use bdp_push::{flush_manual, listar_pendientes as listar_pendientes_push, reintentar_fila};
/* [208A-2/C5] Normalización standalone+sync (H5) — exposición para tests. */
pub use configuracion::actualizar_configuracion;
pub use ventas::anular_venta;
/* [128A-1/F8-1] Exposición para tests de integración: variantes locales de
 * F6 (pagos parciales y factura local) y el DELETE de ventas, protegidos por
 * permiso operativo. */
pub use ventas::{
    eliminar_venta, factura_local, pago_parcial_local, FacturaLocalRequest, PagoLocalRequest,
};
mod campanas;
mod canales_reserva;
mod chatbot;
mod clientes;
mod configuracion;
mod dashboard;
mod errores;
mod etiquetas;
mod gastos;
mod health;
mod inactividad;
mod modo_operacion;
mod notificaciones;
mod openapi;
mod plano_sala;
mod plantillas_whatsapp;
mod rate_limit;
mod recordatorios;
mod resenas;
mod reservas;
mod router;
mod trabajadores;
mod ventas;

pub use openapi::ApiDoc;
pub use router::create_router;
