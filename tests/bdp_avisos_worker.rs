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

use glory_backend::services::{
    cuentas_con_trabajo, procesar_lote, Consulta, ConsultaPedido, ResumenLote, MAX_INTENTOS,
};
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

/// [D10] Anular en caja una comanda vinculada deja la venta anulada en la web.
/// La anulación deja motivo y fecha, y un segundo aviso no los pisa.
#[sqlx::test(migrations = "./migrations")]
async fn comanda_anulada_en_caja_anula_la_venta_local(pool: PgPool) {
    let mia = cuenta(&pool).await;
    let venta = venta_con_bdp(&pool, mia, 43).await;
    let id = aviso(&pool, mia, "cambiar-estado", payload_comanda("43"), 0).await;
    let doble = DobleBdp::con(vec![(43, Consulta::Encontrado(json!({"Status": 2})))]);

    procesar_lote(&pool, mia, &doble, 10).await.expect("lote");

    let (estado, _, _, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    let (estado_bdp, anulada, motivo, con_fecha) = anulacion(&pool, venta).await;
    assert_eq!(estado_bdp.as_deref(), Some("cancelled"));
    assert!(anulada, "la venta debe quedar anulada en la web");
    assert!(con_fecha, "la anulación debe dejar fecha");
    assert!(
        motivo.as_deref().is_some_and(|m| m.contains("TPV")),
        "el motivo debe decir de dónde viene: {motivo:?}"
    );

    /* Un segundo aviso de la misma comanda no reescribe la anulación original. */
    sqlx::query("UPDATE ventas SET anulacion_motivo = 'motivo previo' WHERE id = $1")
        .bind(venta)
        .execute(&pool)
        .await
        .expect("fijar motivo");
    let id2 = aviso(&pool, mia, "cambiar-estado", payload_comanda("43"), 0).await;
    procesar_lote(&pool, mia, &doble, 10).await.expect("lote 2");
    let (estado2, _, _, _, _) = fila(&pool, id2).await;
    assert_eq!(estado2, "procesado", "el aviso repetido también se procesa");
    let (_, anulada2, motivo2, _) = anulacion(&pool, venta).await;
    assert!(anulada2);
    assert_eq!(
        motivo2.as_deref(),
        Some("motivo previo"),
        "un aviso repetido no debe pisar la anulación ya registrada"
    );
}

/// [D10 vivo] El TPV local, al anular, **borra** la comanda: `GetOrder` contesta
/// 301000 en vez de devolverla con `Status: 2`. El aviso `cambiar-estado` con
/// Estado=2 sobre una comanda que ya no existe es una anulación coherente.
#[sqlx::test(migrations = "./migrations")]
async fn aviso_que_cancela_una_comanda_que_bdp_ya_no_tiene_anula_la_venta(pool: PgPool) {
    let mia = cuenta(&pool).await;
    let venta = venta_con_bdp(&pool, mia, 42).await;
    let payload = json!({
        "IdentificadorComanda": "42",
        "TimeStamp": "2026-10-21T22:15:00Z",
        "Estado": 2,
    });
    let id = aviso(&pool, mia, "cambiar-estado", payload, 0).await;
    let doble = DobleBdp::con(vec![(42, Consulta::NoExiste)]);

    let resumen = procesar_lote(&pool, mia, &doble, 10).await.expect("lote");

    assert_eq!(
        resumen,
        ResumenLote {
            procesados: 1,
            reintentos: 0,
            revision: 0
        },
        "la anulación no es una incidencia: es el hecho que el TPV comunica"
    );
    let (estado, _, _, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    let (estado_bdp, anulada, motivo, con_fecha) = anulacion(&pool, venta).await;
    assert_eq!(estado_bdp.as_deref(), Some("cancelled"));
    assert!(anulada && con_fecha);
    assert!(motivo.as_deref().is_some_and(|m| m.contains("TPV")));
}

/// Un aviso sin Estado=2 cuya comanda desapareció sigue siendo una incidencia: no
/// cualquier ausencia se toma por una anulación.
#[sqlx::test(migrations = "./migrations")]
async fn comanda_ausente_sin_anulacion_sigue_yendo_a_revision(pool: PgPool) {
    let mia = cuenta(&pool).await;
    let venta = venta_con_bdp(&pool, mia, 41).await;
    let id = aviso(&pool, mia, "cambiar-estado", payload_comanda("41"), 0).await;
    let doble = DobleBdp::con(vec![(41, Consulta::NoExiste)]);

    procesar_lote(&pool, mia, &doble, 10).await.expect("lote");

    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "requiere_revision");
    assert!(error.is_some());
    let (_, anulada, _, _) = anulacion(&pool, venta).await;
    assert!(!anulada, "sin anulación declarada la venta no se anula");
}

/// Lee la anulación de una venta: estado BDP, marca, motivo y si tiene fecha.
async fn anulacion(pool: &PgPool, id: Uuid) -> (Option<String>, bool, Option<String>, bool) {
    sqlx::query_as(
        "SELECT bdp_order_status, anulada, anulacion_motivo, anulada_at IS NOT NULL \
         FROM ventas WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("leer anulación")
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

/// [D29] El bucle periódico tiene que ver también las cuentas cuyo único aviso
/// quedó varado en `procesando` por una caída entre el reclamo y el commit: si
/// solo mira `recibido`, esa fila no se retoma nunca. Un reclamo reciente, en
/// cambio, no debe despertar a nadie (el otro worker sigue en él).
#[sqlx::test(migrations = "./migrations")]
async fn el_bucle_ve_la_cuenta_cuyo_aviso_quedo_en_procesando(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(&pool, user, "finalizar", payload_comanda("82"), 1).await;
    sqlx::query(
        "UPDATE bdp_avisos_inbox SET estado = 'procesando',
             reclamado_en = NOW() - INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .expect("simular caída tras el reclamo");

    let cuentas = cuentas_con_trabajo(&pool).await.expect("listar");
    assert_eq!(
        cuentas,
        vec![user],
        "la cuenta con el aviso varado debe salir"
    );

    sqlx::query("UPDATE bdp_avisos_inbox SET reclamado_en = NOW() WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("reclamo en curso");
    let cuentas = cuentas_con_trabajo(&pool).await.expect("listar");
    assert!(
        cuentas.is_empty(),
        "un reclamo reciente no es trabajo pendiente"
    );
}

/* ---------------------------------------------------------------------------
 * [D20] Aviso y documento enlazados.
 * El aviso `finalizar` trae `Serie`+`Numero` del documento emitido en caja; la
 * venta guarda su `InvoiceNumber` de BDP. Los cuatro casos: coinciden, difieren,
 * la venta aún no tenía número, y no hay venta local.
 * ------------------------------------------------------------------------- */

/// Payload de un aviso `finalizar` con el documento que emitió el TPV.
fn payload_finalizar(comanda: &str, serie: &str, numero: &str) -> Value {
    json!({
        "IdentificadorComanda": comanda,
        "Serie": serie,
        "Numero": numero,
        "TipoDocumento": 1,
        "TimeStamp": "2026-10-20T10:00:00Z",
    })
}

/// Inserta una venta con el número de factura de BDP ya anotado (o sin él).
async fn venta_con_factura(
    pool: &PgPool,
    user_id: Uuid,
    bdp_order_id: i64,
    factura: Option<&str>,
) -> Uuid {
    let id = venta_con_bdp(pool, user_id, bdp_order_id).await;
    if let Some(numero) = factura {
        sqlx::query("UPDATE ventas SET bdp_invoice_number = $2 WHERE id = $1")
            .bind(id)
            .bind(numero)
            .execute(pool)
            .await
            .expect("anotar factura");
    }
    id
}

/// Lee el número de factura anotado en una venta.
async fn factura_de(pool: &PgPool, id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT bdp_invoice_number FROM ventas WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("leer factura")
}

#[sqlx::test(migrations = "./migrations")]
async fn finalizar_con_el_mismo_documento_queda_procesado(pool: PgPool) {
    let user = cuenta(&pool).await;
    let venta = venta_con_factura(&pool, user, 90, Some("00001TB00000013")).await;
    let id = aviso(
        &pool,
        user,
        "finalizar",
        payload_finalizar("90", "00001TB", "13"),
        0,
    )
    .await;
    let doble = DobleBdp::con(vec![(90, Consulta::Encontrado(json!({"Status": 3})))]);

    procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert!(
        error.is_none(),
        "el enlace es correcto: no hay nada que avisar"
    );
    assert_eq!(
        estado_venta(&pool, venta).await,
        (Some("invoiced".to_string()), true)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn finalizar_con_documento_distinto_es_incidencia_y_no_toca_la_venta(pool: PgPool) {
    let user = cuenta(&pool).await;
    let venta = venta_con_factura(&pool, user, 91, Some("00001TB00000099")).await;
    let id = aviso(
        &pool,
        user,
        "finalizar",
        payload_finalizar("91", "00001TB", "14"),
        0,
    )
    .await;
    let doble = DobleBdp::con(vec![(91, Consulta::Encontrado(json!({"Status": 3})))]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.revision, 1);
    let (estado, _, error, con_pedido, _) = fila(&pool, id).await;
    assert_eq!(estado, "requiere_revision");
    let nota = error.expect("la incidencia debe explicarse");
    assert!(nota.contains("00001TB00000014"), "nota: {nota}");
    assert!(nota.contains("00001TB00000099"), "nota: {nota}");
    assert!(con_pedido, "la foto del pedido se guarda igual");
    assert_eq!(
        factura_de(&pool, venta).await.as_deref(),
        Some("00001TB00000099"),
        "la venta no se toca: sigue con su número"
    );
    assert_eq!(
        estado_venta(&pool, venta).await.0,
        None,
        "con incidencia no se enlaza la venta"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn finalizar_sin_numero_en_la_venta_lo_toma_del_aviso(pool: PgPool) {
    let user = cuenta(&pool).await;
    let venta = venta_con_factura(&pool, user, 92, None).await;
    let id = aviso(
        &pool,
        user,
        "finalizar",
        payload_finalizar("92", "00001TB", "7"),
        0,
    )
    .await;
    let doble = DobleBdp::con(vec![(92, Consulta::Encontrado(json!({"Status": 3})))]);

    procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert!(error.is_none());
    assert_eq!(
        factura_de(&pool, venta).await.as_deref(),
        Some("00001TB00000007"),
        "el número se rellena a ocho dígitos, como el InvoiceNumber de BDP"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn finalizar_sin_venta_local_no_es_incidencia(pool: PgPool) {
    let user = cuenta(&pool).await;
    let id = aviso(
        &pool,
        user,
        "finalizar",
        payload_finalizar("93", "00001TB", "8"),
        0,
    )
    .await;
    let doble = DobleBdp::con(vec![(93, Consulta::Encontrado(json!({"Status": 3})))]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.procesados, 1);
    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert!(error.is_none(), "la comanda pudo crearse en otro TPV");
}

#[sqlx::test(migrations = "./migrations")]
async fn finalizar_con_el_documento_ya_en_otra_venta_es_incidencia(pool: PgPool) {
    let user = cuenta(&pool).await;
    let otra = venta_con_factura(&pool, user, 94, Some("00001TB00000015")).await;
    let venta = venta_con_factura(&pool, user, 95, None).await;
    let id = aviso(
        &pool,
        user,
        "finalizar",
        payload_finalizar("95", "00001TB", "15"),
        0,
    )
    .await;
    let doble = DobleBdp::con(vec![(95, Consulta::Encontrado(json!({"Status": 3})))]);

    let resumen = procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    assert_eq!(resumen.revision, 1);
    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "requiere_revision");
    assert!(error.unwrap().contains("ya consta en otra venta"));
    assert_eq!(
        factura_de(&pool, venta).await,
        None,
        "no se duplica el documento"
    );
    assert_eq!(
        factura_de(&pool, otra).await.as_deref(),
        Some("00001TB00000015")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn finalizar_sin_serie_o_numero_no_cruza_nada(pool: PgPool) {
    let user = cuenta(&pool).await;
    let venta = venta_con_factura(&pool, user, 96, Some("00001TB00000016")).await;
    /* El receptor ya marca este caso para revisión al guardar; aquí se comprueba
     * que el worker no inventa un documento que el aviso no trae. */
    let id = aviso(&pool, user, "finalizar", payload_comanda("96"), 0).await;
    let doble = DobleBdp::con(vec![(96, Consulta::Encontrado(json!({"Status": 3})))]);

    procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert!(error.is_none());
    assert_eq!(
        factura_de(&pool, venta).await.as_deref(),
        Some("00001TB00000016"),
        "sin documento en el aviso, el de la venta se queda como estaba"
    );
}

/// El resto de acciones no declara documento: no se cruza nada.
#[sqlx::test(migrations = "./migrations")]
async fn otras_acciones_no_cruzan_documento(pool: PgPool) {
    let user = cuenta(&pool).await;
    let venta = venta_con_factura(&pool, user, 97, Some("00001TB00000017")).await;
    let id = aviso(
        &pool,
        user,
        "modificar",
        payload_finalizar("97", "00001TB", "18"),
        0,
    )
    .await;
    let doble = DobleBdp::con(vec![(97, Consulta::Encontrado(json!({"Status": 1})))]);

    procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert!(error.is_none());
    assert_eq!(
        factura_de(&pool, venta).await.as_deref(),
        Some("00001TB00000017")
    );
}

/// [28AA-6] Añade una línea local a la venta de una comanda.
async fn linea_local(pool: &PgPool, venta: Uuid, cantidad: f64) {
    sqlx::query(
        r"INSERT INTO venta_lineas (venta_id, descripcion, cantidad, precio_unitario)
          VALUES ($1, 'Línea de prueba', $2, 1.00)",
    )
    .bind(venta)
    .bind(cantidad)
    .execute(pool)
    .await
    .expect("insertar línea");
}

/// [28AA-6] Un cambio de líneas en el TPV no se copia en silencio: el aviso queda en
/// revisión con la nota de conflicto y la venta conserva su estado.
#[sqlx::test(migrations = "./migrations")]
async fn cambio_de_lineas_en_el_tpv_queda_en_revision(pool: PgPool) {
    let user = cuenta(&pool).await;
    let venta = venta_con_bdp(&pool, user, 98).await;
    linea_local(&pool, venta, 2.0).await;
    let id = aviso(&pool, user, "modificar", payload_comanda("98"), 0).await;
    let doble = DobleBdp::con(vec![(
        98,
        Consulta::Encontrado(json!({ "Order": { "Items": [{ "Units": 1.0 }, { "Units": 2.0 }] } })),
    )]);

    procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "requiere_revision");
    assert!(error.unwrap_or_default().contains("Conflicto web–TPV"));
    assert_eq!(estado_venta(&pool, venta).await, (None, false));
}

/// [28AA-6] Si las unidades cuadran, el aviso se procesa como siempre.
#[sqlx::test(migrations = "./migrations")]
async fn lineas_que_cuadran_no_generan_conflicto(pool: PgPool) {
    let user = cuenta(&pool).await;
    let venta = venta_con_bdp(&pool, user, 99).await;
    linea_local(&pool, venta, 2.0).await;
    let id = aviso(&pool, user, "modificar", payload_comanda("99"), 0).await;
    let doble = DobleBdp::con(vec![(
        99,
        Consulta::Encontrado(json!({ "Order": { "Items": [{ "Units": 1.0 }, { "Units": 1.0 }] } })),
    )]);

    procesar_lote(&pool, user, &doble, 10).await.expect("lote");

    let (estado, _, error, _, _) = fila(&pool, id).await;
    assert_eq!(estado, "procesado");
    assert!(error.is_none());
}
