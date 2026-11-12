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

/* [D-auto-push] Cadencia del barrido de la cola. Cada fila ya trae su propio
 * fail-closed; 60 s evita que un cambio local espere minutos. */
pub const SEGUNDOS_PUSH_AUTOMATICO: u64 = 60;

/* [M21] Tope de reintentos seguidos por operación (solo errores transitorios).
 * Ya no es un abandono: al llegar, la fila espera `SEGUNDOS_ESPERA_REINTENTOS`
 * y vuelve a probar. Un TPV apagado un fin de semana dejaba la cola muerta en
 * cinco minutos (cinco barridos de 60 s) sin que nadie hubiera enviado nada:
 * el fallo de `Login` también cuenta como transitorio. */
pub const REINTENTOS_MAX: i32 = 5;

/* [M21] Espera tras agotar los reintentos seguidos frente a los 60 s del
 * barrido: sigue recuperándose sola, sin martillear un TPV caído. Un envío con
 * éxito saca la fila de la cola y una re-edición local la reencola. */
pub const SEGUNDOS_ESPERA_REINTENTOS: u64 = 90;

/* [M21] Rechazo funcional (4xx) y suscripción sin activar: no se reintentan en
 * bucle, pero tampoco se quedan esperando a que alguien pulse el botón. Se
 * vuelven a probar una vez al día, que es la cadencia a la que el TPV puede
 * cambiar de opinión (se activa la suscripción, se corrige el maestro). */
pub const SEGUNDOS_ESPERA_BLOQUEO: u64 = 86_400;

/* [049A-1/S5] 'rechazado' (4xx definitivo) entra en las filas activas para que
 * el flush la vea y la salte salvo reintento manual, igual que
 * 'pendiente_suscripcion'; una re-edición local (encolar) la refresca. */
pub(super) const ESTADOS_ACTIVOS: &[&str] = &[
    ESTADO_PENDIENTE,
    ESTADO_PENDIENTE_SUSCRIPCION,
    ESTADO_ERROR,
    ESTADO_RECHAZADO,
];
