pub const DOMINIO_ARTICULO: &str = "articulo";
pub const DOMINIO_STOCK: &str = "stock";
pub const DOMINIO_DEPARTAMENTO: &str = "departamento";
pub const DOMINIO_FAMILIA: &str = "familia";
pub const DOMINIO_VENTA: &str = "venta";
pub const DOMINIO_CLIENTE_PUNTOS: &str = "cliente_puntos";
pub const DOMINIO_PROPINA: &str = "propina";

pub const OPERACION_CREAR: &str = "crear";
pub const OPERACION_MODIFICAR: &str = "modificar";
pub const OPERACION_PRECIOS: &str = "precios";
pub const OPERACION_REGULARIZAR: &str = "regularizar";
pub const OPERACION_TRASPASAR: &str = "traspasar";
pub const OPERACION_INVENTARIO: &str = "inventario";
pub const OPERACION_CANCELAR: &str = "cancelar";
pub const OPERACION_PUNTOS: &str = "puntos";
pub const OPERACION_PROPINA: &str = "propina";

pub const ESTADO_PENDIENTE: &str = "pendiente";
pub const ESTADO_PENDIENTE_SUSCRIPCION: &str = "pendiente_suscripcion";
pub const ESTADO_ERROR: &str = "error";
pub const ESTADO_RECHAZADO: &str = "rechazado";
pub const ESTADO_SINCRONIZADO: &str = "sincronizado";
pub const ESTADO_DESCARTADO: &str = "descartado";

/* [M21] Tope de reintentos automáticos por operación (solo errores transitorios). */
pub const REINTENTOS_MAX: i32 = 5;

/* [049A-1/S5] 'rechazado' (4xx definitivo) entra en las filas activas para que
 * el flush la vea y la salte salvo reintento manual, igual que
 * 'pendiente_suscripcion'; una re-edición local (encolar) la refresca. */
pub(super) const ESTADOS_ACTIVOS: &[&str] = &[
    ESTADO_PENDIENTE,
    ESTADO_PENDIENTE_SUSCRIPCION,
    ESTADO_ERROR,
    ESTADO_RECHAZADO,
];
