// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [10AA-3/F2] Pruebas del worker de avisos: completa cada aviso `recibido` con el
 * pedido de BDP y decide su estado final. Usa un doble de `ConsultaPedido` para no
 * tocar la red; la base es temporal (`#[sqlx::test]`).
 *
 * Cubre: pedido encontrado, comanda inexistente (con y sin borrado), fallo
 * transitorio que reintenta, fallo agotado, aviso sin comanda numérica, lote
 * acotado, aislamiento por cuenta, las siete acciones, y los avisos abandonados
 * en `procesando` (se retoman tras caducar el reclamo). */

use std::collections::HashMap;
use std::sync::Mutex;

use glory_backend::services::{procesar_lote, Consulta, ConsultaPedido, ResumenLote, MAX_INTENTOS};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Las siete acciones que el TPV puede avisar (guía operativa, F2).
const ACCIONES: [&str; 7] = [
    "crear",
    "modificar",
    "cambiar-estado",
    "borrar",
    "finalizar",
    "traspasar",
    "traspasar-linea",
];

/// Doble del cliente BDP: responde desde un mapa y recuerda qué comandas pidió.
#[derive(Default)]
struct DobleBdp {
    respuestas: HashMap<i64, Consulta>,
    pedidas: Mutex<Vec<i64>>,
}

impl DobleBdp {
    fn con(pares: Vec<(i64, Consulta)>) -> Self {
        Self {
            respuestas: pares.into_iter().collect(),
            pedidas: Mutex::new(Vec::new()),
        }
    }

    fn pedidas(&self) -> Vec<i64> {
        self.pedidas.lock().expect("bloqueo del doble").clone()
    }
}

/* El trait exige un Future: se resuelve de forma síncrona con `ready`, sin `async`
 * (que no tendría ningún `.await` que lo justifique). */
impl ConsultaPedido for DobleBdp {
    fn pedido(&self, comanda: i64) -> impl std::future::Future<Output = Consulta> + Send {
        self.pedidas
            .lock()
            .expect("bloqueo del doble")
            .push(comanda);
        let respuesta = self
            .respuestas
            .get(&comanda)
            .cloned()
            .unwrap_or_else(|| Consulta::Fallo("sin respuesta programada".to_string()));
        std::future::ready(respuesta)
    }
}

async fn cuenta(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("worker-{user_id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario");
    user_id
}

/// Inserta un aviso `recibido` con su payload del TPV. Devuelve su id.
async fn aviso(pool: &PgPool, user_id: Uuid, accion: &str, payload: Value, intentos: i32) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO bdp_avisos_inbox
             (id, user_id, accion, origen, clave_idempotencia, payload, payload_hash,
              estado, intentos)
           VALUES ($1, $2, $3, 'prueba', $4, $5, 'hash-prueba', 'recibido', $6)",
    )
    .bind(id)
    .bind(user_id)
    .bind(accion)
    .bind(format!("{accion}|{id}"))
    .bind(&payload)
    .bind(intentos)
    .execute(pool)
    .await
    .expect("insertar aviso");
    id
}

/// Lee estado, intentos, error y si ya tiene pedido, para comprobar la decisión.
async fn fila(pool: &PgPool, id: Uuid) -> (String, i32, Option<String>, bool, bool) {
    sqlx::query_as(
        r"SELECT estado, intentos, ultimo_error,
                 pedido_bdp IS NOT NULL, procesado_en IS NOT NULL
            FROM bdp_avisos_inbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("leer aviso")
}

fn payload_comanda(comanda: &str) -> Value {
    json!({ "IdentificadorComanda": comanda, "TimeStamp": "2026-10-18T10:00:00Z" })
}

#[sqlx::test(migrations = "./migrations")]
async fn pedido_encontrado_queda_procesado_con_su_foto(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(&pool, user, "crear", payload_comanda("15"), 0).await;
    let doble = DobleBdp::con(vec![(15, Consulta::Encontrado(json!({"Lineas": 2})))]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(
        resumen,
        ResumenLote {
            procesados: 1,
            reintentos: 0,
            revision: 0
        }
    );
    let (estado, intentos, error, con_pedido, con_fecha) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert_eq!(intentos, 1);
    assert!(error.is_none());
    assert!(con_pedido, "debe guardarse la foto del pedido");
    assert!(con_fecha, "procesado_en debe quedar marcado");
    assert_eq!(doble.pedidas(), vec![15]);
}

#[sqlx::test(migrations = "./migrations")]
async fn comanda_inexistente_en_crear_va_a_revision(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(&pool, user, "crear", payload_comanda("16"), 0).await;
    let doble = DobleBdp::con(vec![(16, Consulta::NoExiste)]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.revision, 1);
    let (estado, _, error, con_pedido, _) = fila(&pool, id).await;
    assert_eq!(estado, "requiere_revision");
    assert!(error.unwrap().contains("301000"));
    assert!(!con_pedido);
}

#[sqlx::test(migrations = "./migrations")]
async fn comanda_borrada_inexistente_es_normal_y_queda_procesada(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(&pool, user, "borrar", payload_comanda("17"), 0).await;
    let doble = DobleBdp::con(vec![(17, Consulta::NoExiste)]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.procesados, 1);
    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert!(error.is_none(), "tras un borrado no hay nada que avisar");
}

#[sqlx::test(migrations = "./migrations")]
async fn fallo_transitorio_vuelve_a_recibido_y_cuenta_el_intento(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(&pool, user, "modificar", payload_comanda("18"), 0).await;
    let doble = DobleBdp::con(vec![(18, Consulta::Fallo("timeout".to_string()))]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.reintentos, 1);
    let (estado, intentos, error, _, con_fecha) = fila(&pool, id).await;
    assert_eq!(
        estado, "recibido",
        "un fallo transitorio no saca el aviso de la cola"
    );
    assert_eq!(intentos, 1);
    assert!(error.unwrap().contains("timeout"));
    assert!(!con_fecha, "procesado_en solo se marca al salir de la cola");
}

#[sqlx::test(migrations = "./migrations")]
async fn fallo_agotado_pasa_a_revision(pool: PgPool) {
    let user = cuenta(&pool).await;
    /* Ya lleva MAX_INTENTOS - 1 intentos: esta pasada es la última. */
    let id = aviso(
        &pool,
        user,
        "modificar",
        payload_comanda("19"),
        MAX_INTENTOS - 1,
    )
    .await;
    let doble = DobleBdp::con(vec![(19, Consulta::Fallo("500".to_string()))]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.revision, 1);
    let (estado, intentos, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "requiere_revision");
    assert_eq!(intentos, MAX_INTENTOS);
    assert!(error.unwrap().contains("Agotados"));
}

#[sqlx::test(migrations = "./migrations")]
async fn aviso_sin_comanda_numerica_no_llama_a_bdp(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(
        &pool,
        user,
        "crear",
        json!({"IdentificadorComanda": "abc"}),
        0,
    )
    .await;
    let doble = DobleBdp::default();

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.revision, 1);
    assert!(
        doble.pedidas().is_empty(),
        "no se inventa un identificador para BDP"
    );
    let (estado, _, _, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "requiere_revision");
}

#[sqlx::test(migrations = "./migrations")]
async fn el_lote_respeta_su_limite_y_deja_el_resto_en_cola(pool: PgPool) {
    let user = cuenta(&pool).await;
    for comanda in ["20", "21", "22"] {
        aviso(&pool, user, "crear", payload_comanda(comanda), 0).await;
    }
    let doble = DobleBdp::con(vec![
        (20, Consulta::Encontrado(json!({}))),
        (21, Consulta::Encontrado(json!({}))),
        (22, Consulta::Encontrado(json!({}))),
    ]);

    let resumen = procesar_lote(&pool, user, &doble, 2).await.expect("lote");

    assert_eq!(resumen.procesados, 2);
    let pendientes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_avisos_inbox WHERE user_id = $1 AND estado = 'recibido'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("contar");
    assert_eq!(pendientes, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn no_toca_avisos_de_otra_cuenta_ni_ya_procesados(pool: PgPool) {
    let mia = cuenta(&pool).await;
    let ajena = cuenta(&pool).await;
    let de_otra = aviso(&pool, ajena, "crear", payload_comanda("30"), 0).await;
    let ya_hecho = aviso(&pool, mia, "crear", payload_comanda("31"), 0).await;
    sqlx::query("UPDATE bdp_avisos_inbox SET estado = 'procesado' WHERE id = $1")
        .bind(ya_hecho)
        .execute(&pool)
        .await
        .expect("marcar procesado");

    let doble = DobleBdp::con(vec![(30, Consulta::Encontrado(json!({})))]);
    let resumen = procesar_lote(&pool, mia, &doble, 10).await.expect("lote");

    assert_eq!(resumen, ResumenLote::default());
    assert_eq!(doble.pedidas().len(), 0);
    let (estado, _, _, _, _) = fila(&pool, de_otra).await;
    assert_eq!(
        estado, "recibido",
        "el aviso de otra cuenta debe seguir intacto"
    );
}

/// Inserta una venta local con su `bdp_order_id`, como la deja el alta en BDP.
async fn venta_con_bdp(pool: &PgPool, user_id: Uuid, bdp_order_id: i64) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r"INSERT INTO ventas (id, user_id, fecha, descripcion, iva_porcentaje, turno, canal,
                              metodo_pago, importe_base, importe_iva, bdp_synced, bdp_order_id)
          VALUES ($1, $2, CURRENT_DATE, 'Venta de aviso', 10, 'mediodia', 'comedor',
                  'efectivo', 10.00, 1.00, true, $3)",
    )
    .bind(id)
    .bind(user_id)
    .bind(bdp_order_id)
    .execute(pool)
    .await
    .expect("insertar venta");
    id
}

/// Lee el estado BDP y la marca de factura de una venta.
async fn estado_venta(pool: &PgPool, id: Uuid) -> (Option<String>, bool) {
    sqlx::query_as("SELECT bdp_order_status, bdp_invoiced FROM ventas WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("leer venta")
}

#[sqlx::test(migrations = "./migrations")]
async fn pedido_facturado_actualiza_la_venta_de_su_comanda(pool: PgPool) {
    let mia = cuenta(&pool).await;
    let ajena = cuenta(&pool).await;
    let venta = venta_con_bdp(&pool, mia, 40).await;
    let venta_ajena = venta_con_bdp(&pool, ajena, 40).await;
    let id = aviso(&pool, mia, "finalizar", payload_comanda("40"), 0).await;
    let doble = DobleBdp::con(vec![(40, Consulta::Encontrado(json!({"Status": 3})))]);

    procesar_lote(&pool, mia, &doble, 10).await.expect("lote");

    let (estado, _, _, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert_eq!(
        estado_venta(&pool, venta).await,
        (Some("invoiced".to_string()), true),
        "la venta de la cuenta debe reflejar el estado BDP"
    );
    assert_eq!(
        estado_venta(&pool, venta_ajena).await,
        (None, false),
        "la venta de otra cuenta no se toca"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn pedido_sin_status_procesa_el_aviso_y_no_toca_la_venta(pool: PgPool) {
    let mia = cuenta(&pool).await;
    let venta = venta_con_bdp(&pool, mia, 41).await;
    let id = aviso(&pool, mia, "modificar", payload_comanda("41"), 0).await;
    let doble = DobleBdp::con(vec![(41, Consulta::Encontrado(json!({"Lineas": 2})))]);

    procesar_lote(&pool, mia, &doble, 10).await.expect("lote");

    let (estado, _, _, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert_eq!(estado_venta(&pool, venta).await, (None, false));
}

#[sqlx::test(migrations = "./migrations")]
async fn comanda_sin_venta_local_no_falla_el_lote(pool: PgPool) {
    let mia = cuenta(&pool).await;
    let id = aviso(&pool, mia, "crear", payload_comanda("42"), 0).await;
    let doble = DobleBdp::con(vec![(42, Consulta::Encontrado(json!({"Status": 1})))]);

    let resumen = procesar_lote(&pool, mia, &doble, 10).await.expect("lote");

    assert_eq!(resumen.procesados, 1);
    let (estado, _, _, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
}

/// Las siete acciones de la guía (F2): con pedido encontrado, todas quedan procesadas.
#[sqlx::test(migrations = "./migrations")]
async fn cada_una_de_las_siete_acciones_se_procesa_con_pedido(pool: PgPool) {
    let user = cuenta(&pool).await;
    let mut ids = Vec::new();
    for (i, accion) in ACCIONES.iter().enumerate() {
        let comanda = (60 + i).to_string();
        ids.push(aviso(&pool, user, accion, payload_comanda(&comanda), 0).await);
    }
    let doble = DobleBdp::con(
        (0..ACCIONES.len())
            .map(|i| (60 + i as i64, Consulta::Encontrado(json!({"Lineas": 1}))))
            .collect(),
    );

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.procesados, ACCIONES.len() as u32);
    for (accion, id) in ACCIONES.iter().zip(ids) {
        let (estado, _, _, _, _) = fila(&pool, id).await;
        assert_eq!(estado, "procesado", "acción {accion}");
    }
}

/// Comanda que BDP no conoce: en todas las acciones va a revisión salvo en `borrar`,
/// donde es el resultado esperado.
#[sqlx::test(migrations = "./migrations")]
async fn comanda_inexistente_depende_de_la_accion(pool: PgPool) {
    let user = cuenta(&pool).await;
    let mut ids = Vec::new();
    for (i, accion) in ACCIONES.iter().enumerate() {
        let comanda = (70 + i).to_string();
        ids.push(aviso(&pool, user, accion, payload_comanda(&comanda), 0).await);
    }
    let doble = DobleBdp::con(
        (0..ACCIONES.len())
            .map(|i| (70 + i as i64, Consulta::NoExiste))
            .collect(),
    );

    procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    for (accion, id) in ACCIONES.iter().zip(ids) {
        let (estado, _, _, _, _) = fila(&pool, id).await;
        let esperado = if *accion == "borrar" {
            "procesado"
        } else {
            "requiere_revision"
        };
        assert_eq!(estado, esperado, "acción {accion}");
    }
}

/// Un aviso que quedó en `procesando` tras una caída del worker se retoma cuando su
/// reclamo caducó. Antes del fix quedaba así para siempre.
#[sqlx::test(migrations = "./migrations")]
async fn aviso_abandonado_en_procesando_se_retoma(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(&pool, user, "modificar", payload_comanda("80"), 1).await;
    sqlx::query(
        "UPDATE bdp_avisos_inbox SET estado = 'procesando',
             reclamado_en = NOW() - INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .expect("simular caída");
    let doble = DobleBdp::con(vec![(80, Consulta::Encontrado(json!({})))]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.procesados, 1);
    let (estado, intentos, _, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert_eq!(intentos, 2, "el reclamo abandonado cuenta como intento");
}

/// Un reclamo reciente, aún en curso, no se roba: otro lote no lo duplica.
#[sqlx::test(migrations = "./migrations")]
async fn reclamo_reciente_en_procesando_no_se_toca(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(&pool, user, "modificar", payload_comanda("81"), 1).await;
    sqlx::query(
        "UPDATE bdp_avisos_inbox SET estado = 'procesando', reclamado_en = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .expect("simular reclamo en curso");
    let doble = DobleBdp::con(vec![(81, Consulta::Encontrado(json!({})))]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen, ResumenLote::default());
    assert!(doble.pedidas().is_empty());
}
