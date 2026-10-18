// sentinel-disable-file sqlx-query-sin-macro sqlx-query-as-sin-macro
// [por que] sqlx sin feature "macros" ni DB en compile-time: query! rompe el build.
/*! [199A-1/F1] Pruebas del receptor de avisos del TPV (`/api/bdp/avisos/:accion`)
 * y de su buzón. Base temporal local; no se llama a ningún TPV.
 *
 * Cubre lo que el plan pide para F1: duplicado, aviso perdido (llega tarde y se
 * guarda igual), fuera de orden, secreto inválido, cuerpo gigante y las rutas
 * con y sin sesión.
 *
 * El secreto se pasa a `recibir_aviso_con_secreto` en vez de tocar el entorno:
 * los tests corren en paralelo y una variable de proceso los haría pelearse. */

use glory_backend::services::NotifHub;
use std::sync::Arc;
use axum::body::{to_bytes, Body};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::Json;
use glory_backend::config::AppConfig;
use glory_backend::handlers::{
    listar_avisos_bdp, recibir_aviso_con_secreto, reintentar_aviso_bdp, rutas_avisos_bdp,
    rutas_publicas_avisos, ListarAvisosQuery, RespuestaAviso,
};
use glory_backend::middleware::AuthUser;
use glory_backend::models::UserRole;
use glory_backend::repositories::ConfiguracionRepository;
use glory_backend::services::ServicioModoOperacion;
use glory_backend::AppState;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::BTreeMap;
use tower::ServiceExt;
use uuid::Uuid;

const SECRETO: &str = "secreto-de-prueba";

fn estado(pool: PgPool) -> AppState {
    let notif_hub = Arc::new(NotifHub::new());
    AppState {
        pool,
        jwt_secret: "test-secret".to_string(),
        config: AppConfig {
            database_url: "postgres://localhost".to_string(),
            jwt_secret: "test-secret".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3000,
            smtp: None,
            app_url: "http://localhost".to_string(),
            error_report_email: None,
        },
        notif_hub,
        modo_operacion: ServicioModoOperacion::default(),
        resumen_cache: Default::default(),
        listados_cache: Default::default(),
    }
}

/// Un restaurante configurado: es lo que `resolver_usuario` necesita para saber
/// a qué cuenta apuntan los avisos.
async fn restaurante(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(format!("avisos-{user_id}@example.com"))
        .bind("argon2_hash_placeholder")
        .execute(pool)
        .await
        .expect("crear usuario");
    ConfiguracionRepository::obtener_o_crear(pool, user_id)
        .await
        .expect("config por defecto");
    user_id
}

fn admin(user_id: Uuid) -> AuthUser {
    AuthUser {
        user_id,
        role: UserRole::Admin,
        effective_role: UserRole::Admin,
        impersonator: None,
        trabajador_id: None,
    }
}

/// Llama al receptor tal como lo llamaría el TPV y devuelve el código y, si el
/// aviso se ha admitido, lo que se le contestó. Los fallos se reducen a su
/// código: es lo único que miran estas pruebas.
async fn aviso(
    estado: &AppState,
    accion: &str,
    query: &[(&str, &str)],
    cuerpo: Value,
    con_secreto: bool,
) -> Result<(StatusCode, RespuestaAviso), StatusCode> {
    let mapa: BTreeMap<String, String> = query
        .iter()
        .map(|(clave, valor)| ((*clave).to_string(), (*valor).to_string()))
        .collect();
    let mut cabeceras = HeaderMap::new();
    if con_secreto {
        cabeceras.insert("x-bdp-secreto", SECRETO.parse().expect("cabecera válida"));
    }

    match recibir_aviso_con_secreto(
        State(estado.clone()),
        Path(accion.to_string()),
        Query(mapa),
        cabeceras,
        axum::body::Bytes::from(serde_json::to_vec(&cuerpo).expect("serializar")),
        Some(SECRETO.to_string()),
    )
    .await
    {
        Ok((codigo, Json(respuesta))) => Ok((codigo, respuesta)),
        Err(error) => Err(axum::response::IntoResponse::into_response(error).status()),
    }
}

fn codigo(respuesta: axum::response::Response) -> StatusCode {
    respuesta.status()
}

/// El caso normal: el aviso entra y se puede leer del buzón.
#[sqlx::test(migrations = "./migrations")]
async fn un_aviso_valido_entra_en_el_buzon_y_se_puede_listar(pool: PgPool) {
    let user_id = restaurante(&pool).await;
    let estado = estado(pool.clone());

    let (estado_http, respuesta) = aviso(
        &estado,
        "Crear",
        &[],
        json!({
            "IdentificadorComanda": "15",
            "IdentificadorEstablecimiento": "1",
            "TimeStamp": "2026-10-17T14:00:00Z",
            "Secreto": SECRETO
        }),
        true,
    )
    .await
    .expect("el aviso se admite");

    assert_eq!(estado_http, StatusCode::ACCEPTED);
    assert_eq!(respuesta.estado, "recibido");
    assert_eq!(respuesta.accion, "crear");

    let avisos = listar_avisos_bdp(
        State(estado),
        admin(user_id),
        Query(ListarAvisosQuery::default()),
    )
    .await
    .expect("listar el buzón")
    .0;
    assert_eq!(avisos.len(), 1);
    assert_eq!(avisos[0].accion, "crear");
    assert_eq!(avisos[0].estado, "recibido");
    assert_eq!(
        avisos[0].clave_idempotencia,
        "crear|15|2026-10-17T14:00:00.000000Z"
    );

    /* El secreto nunca queda escrito en el payload guardado. */
    let texto = serde_json::to_string(&avisos[0].payload).expect("serializar payload");
    assert!(
        !texto.contains(SECRETO),
        "el secreto no puede guardarse: {texto}"
    );
    assert!(texto.contains("***"));
}

/// El reenvío del mismo aviso no duplica nada (D04, D08).
#[sqlx::test(migrations = "./migrations")]
async fn el_mismo_aviso_dos_veces_no_duplica(pool: PgPool) {
    let user_id = restaurante(&pool).await;
    let estado = estado(pool.clone());
    let cuerpo = json!({
        "IdentificadorComanda": "15",
        "TimeStamp": "2026-10-17T14:00:00Z"
    });

    let (estado_http, _) = aviso(&estado, "crear", &[], cuerpo.clone(), true)
        .await
        .expect("primer envío");
    assert_eq!(estado_http, StatusCode::ACCEPTED);

    /* El mismo aviso, pero con otra forma de escribir la hora: sigue siendo el
     * mismo, porque la clave canoniza la marca de tiempo. */
    let (estado_http, respuesta) = aviso(
        &estado,
        "Crear",
        &[],
        json!({
            "IdentificadorComanda": "15",
            "TimeStamp": "2026-10-17T14:00:00.000Z"
        }),
        true,
    )
    .await
    .expect("reenvío");
    assert_eq!(estado_http, StatusCode::OK);
    assert_eq!(respuesta.estado, "duplicado");

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bdp_avisos_inbox WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("contar");
    assert_eq!(total, 1, "un reenvío no puede mover nada dos veces");
}

/// Misma clave y cuerpo distinto: no se pisa el original y se avisa con 409.
#[sqlx::test(migrations = "./migrations")]
async fn el_reenvio_con_otro_contenido_no_pisa_el_original(pool: PgPool) {
    let user_id = restaurante(&pool).await;
    let estado = estado(pool.clone());

    aviso(
        &estado,
        "cambiar-estado",
        &[],
        json!({
            "IdentificadorComanda": "15",
            "TimeStamp": "2026-10-17T14:00:00Z",
            "Estado": 1
        }),
        true,
    )
    .await
    .expect("primer envío");

    let error = aviso(
        &estado,
        "cambiar-estado",
        &[],
        json!({
            "IdentificadorComanda": "15",
            "TimeStamp": "2026-10-17T14:00:00Z",
            "Estado": 2
        }),
        true,
    )
    .await
    .expect_err("el contenido distinto se rechaza");
    assert_eq!(error, StatusCode::CONFLICT);

    /* El original sigue como estaba: estado 1. */
    let guardado: Value =
        sqlx::query_scalar("SELECT payload FROM bdp_avisos_inbox WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("leer el payload");
    assert_eq!(guardado["Estado"], json!(1));
}

/// Aviso perdido y fuera de orden: lo que llega tarde no se tira, se apunta con
/// su marca de tiempo para que el worker decida (F2).
#[sqlx::test(migrations = "./migrations")]
async fn lo_que_llega_tarde_o_desordenado_se_guarda(pool: PgPool) {
    let user_id = restaurante(&pool).await;
    let estado = estado(pool.clone());

    /* Primero el estado final, después la creación: al revés de como pasó. */
    for (accion, hora, estado_esperado) in [
        ("cambiar-estado", "2026-10-17T14:00:02Z", 3),
        ("crear", "2026-10-17T14:00:00Z", 0),
    ] {
        let (estado_http, _) = aviso(
            &estado,
            accion,
            &[],
            json!({
                "IdentificadorComanda": "15",
                "TimeStamp": hora,
                "Estado": estado_esperado
            }),
            true,
        )
        .await
        .expect("se admite aunque llegue desordenado");
        assert_eq!(estado_http, StatusCode::ACCEPTED);
    }

    let filas: Vec<(String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT accion, recibido_en FROM bdp_avisos_inbox WHERE user_id = $1 ORDER BY accion",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .expect("leer el buzón");
    assert_eq!(filas.len(), 2, "ningún aviso se pierde por llegar tarde");

    /* Las dos claves son distintas: la hora distingue el mismo pedido. */
    let claves: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT clave_idempotencia) FROM bdp_avisos_inbox WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("contar claves");
    assert_eq!(claves, 2);
}

/// Lo que no se puede ni identificar se rechaza sin guardar; lo que viene raro
/// pero identificable se guarda para revisión (decisión L4.4).
#[sqlx::test(migrations = "./migrations")]
async fn sin_comanda_se_rechaza_y_lo_raro_se_guarda_para_revision(pool: PgPool) {
    let user_id = restaurante(&pool).await;
    let estado = estado(pool.clone());

    let error = aviso(
        &estado,
        "crear",
        &[],
        json!({"TimeStamp": "2026-10-17T14:00:00Z"}),
        true,
    )
    .await
    .expect_err("sin comanda no hay aviso");
    assert_eq!(error, StatusCode::UNPROCESSABLE_ENTITY);

    let vacio: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bdp_avisos_inbox WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("contar");
    assert_eq!(vacio, 0, "un aviso rechazado no deja fila");

    /* Estado fuera del rango documentado: entra, marcado, y no se actúa. */
    let (estado_http, respuesta) = aviso(
        &estado,
        "cambiar-estado",
        &[],
        json!({
            "IdentificadorComanda": "15",
            "TimeStamp": "2026-10-17T14:00:00Z",
            "Estado": 9
        }),
        true,
    )
    .await
    .expect("se admite para revisión");
    assert_eq!(estado_http, StatusCode::ACCEPTED);
    assert_eq!(respuesta.estado, "requiere_revision");
    assert!(respuesta
        .nota
        .expect("con nota")
        .contains("Estado fuera de 0-3"));

    /* Y se puede reabrir a mano para que el worker lo vuelva a intentar. */
    let id = respuesta.id.expect("id del aviso");
    let reabierto = reintentar_aviso_bdp(State(estado.clone()), admin(user_id), Path(id))
        .await
        .expect("reabrir");
    assert_eq!(reabierto.0["estado"], json!("recibido"));

    let pendientes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bdp_avisos_inbox WHERE user_id = $1 AND estado = 'requiere_revision'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("contar pendientes");
    assert_eq!(pendientes, 0);

    /* Reabrir un aviso que ya no está pendiente no encuentra nada. */
    let error = reintentar_aviso_bdp(State(estado), admin(user_id), Path(id))
        .await
        .expect_err("ya no está pendiente");
    assert!(matches!(
        axum::response::IntoResponse::into_response(error).status(),
        StatusCode::NOT_FOUND
    ));
}

/// Secreto inválido, ausente o receptor sin configurar: no entra nada.
#[sqlx::test(migrations = "./migrations")]
async fn sin_secreto_valido_no_entra_ningun_aviso(pool: PgPool) {
    let user_id = restaurante(&pool).await;
    let estado = estado(pool.clone());
    let cuerpo = json!({
        "IdentificadorComanda": "15",
        "TimeStamp": "2026-10-17T14:00:00Z"
    });

    /* Sin cabecera ni parámetro. */
    let error = aviso(&estado, "crear", &[], cuerpo.clone(), false)
        .await
        .expect_err("sin secreto no pasa");
    assert_eq!(error, StatusCode::UNAUTHORIZED);

    /* Secreto equivocado (y de otra longitud: tampoco filtra por el tiempo). */
    let error = aviso(
        &estado,
        "crear",
        &[("Secreto", "otro-secreto-distinto")],
        cuerpo.clone(),
        false,
    )
    .await
    .expect_err("con secreto equivocado no pasa");
    assert_eq!(error, StatusCode::UNAUTHORIZED);

    /* Receptor sin secreto configurado: apagado, no abierto. */
    let error = recibir_aviso_con_secreto(
        State(estado.clone()),
        Path("crear".to_string()),
        Query(BTreeMap::new()),
        HeaderMap::new(),
        axum::body::Bytes::from(serde_json::to_vec(&cuerpo).expect("serializar")),
        None,
    )
    .await
    .expect_err("sin configurar no admite nada");
    assert_eq!(
        codigo(axum::response::IntoResponse::into_response(error)),
        StatusCode::SERVICE_UNAVAILABLE
    );

    /* El secreto también vale como parámetro del aviso, que es como lo manda el
     * TPV cuando se configura como valor fijo. */
    let (estado_http, _) = aviso(&estado, "crear", &[("Secreto", SECRETO)], cuerpo, true)
        .await
        .expect("con el secreto en el parámetro sí pasa");
    assert_eq!(estado_http, StatusCode::ACCEPTED);

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bdp_avisos_inbox WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("contar");
    assert_eq!(total, 1, "solo ha entrado el aviso con secreto válido");
}

/// Las siete acciones del distribuidor, por GET con la query string y por POST
/// con JSON. Es la matriz que el integrador configura en el TPV.
#[sqlx::test(migrations = "./migrations")]
async fn las_siete_acciones_entran_por_get_y_por_post(pool: PgPool) {
    let user_id = restaurante(&pool).await;
    let estado = estado(pool.clone());

    let acciones = [
        "Crear",
        "Modificar",
        "CambiarEstado",
        "Borrar",
        "Finalizar",
        "Traspasar",
        "TraspasarLinea",
    ];
    let esperados = [
        "crear",
        "modificar",
        "cambiar-estado",
        "borrar",
        "finalizar",
        "traspasar",
        "traspasar-linea",
    ];

    for (indice, accion) in acciones.iter().enumerate() {
        /* Por GET, todo en la query string (y sin cuerpo). */
        let mut query: BTreeMap<String, String> = BTreeMap::new();
        query.insert("IdentificadorComanda".to_string(), "15".to_string());
        query.insert("TimeStamp".to_string(), "2026-10-17T14:00:00Z".to_string());
        query.insert("Estado".to_string(), "1".to_string());
        query.insert("TipoDocumento".to_string(), "6".to_string());
        query.insert("Serie".to_string(), "A".to_string());
        query.insert("Numero".to_string(), "10".to_string());
        query.insert("IdentificadorComandaOrigen".to_string(), "14".to_string());
        query.insert("SalonOrigen".to_string(), "1".to_string());
        query.insert("SalonDestino".to_string(), "2".to_string());
        query.insert("MesaOrigen".to_string(), "3".to_string());
        query.insert("MesaDestino".to_string(), "4".to_string());
        query.insert("Secreto".to_string(), SECRETO.to_string());

        let (estado_http, respuesta) = recibir_aviso_con_secreto(
            State(estado.clone()),
            Path(accion.to_string()),
            Query(query),
            HeaderMap::new(),
            axum::body::Bytes::new(),
            Some(SECRETO.to_string()),
        )
        .await
        .expect("el aviso por GET se admite");
        assert_eq!(estado_http, StatusCode::ACCEPTED, "GET {accion}");
        assert_eq!(respuesta.accion, esperados[indice]);

        /* Y por POST, con el cuerpo JSON del contrato. */
        let (estado_http, respuesta) = aviso(
            &estado,
            accion,
            &[],
            json!({
                "IdentificadorComanda": "20",
                "IdentificadorEstablecimiento": "1",
                "TimeStamp": "2026-10-17T14:0{indice}:00Z",
                "Estado": 1,
                "TipoDocumento": 6,
                "Serie": "A",
                "Numero": 10,
                "IdentificadorComandaOrigen": 14,
                "SalonOrigen": 1,
                "SalonDestino": 2,
                "MesaOrigen": 3,
                "MesaDestino": 4
            }),
            true,
        )
        .await
        .expect("el aviso por POST se admite");
        assert_eq!(estado_http, StatusCode::ACCEPTED, "POST {accion}");
        assert_eq!(respuesta.accion, esperados[indice]);
    }

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bdp_avisos_inbox WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("contar");
    assert_eq!(total, 14, "siete acciones por dos vías");

    /* La acción desconocida se contesta sin guardar. */
    let error = aviso(
        &estado,
        "pagar",
        &[],
        json!({"IdentificadorComanda": "15"}),
        true,
    )
    .await
    .expect_err("acción desconocida");
    assert_eq!(error, StatusCode::NOT_FOUND);
}

/// El buzón es del restaurante: cada cuenta ve lo suyo y solo Admin entra.
#[sqlx::test(migrations = "./migrations")]
async fn el_buzon_es_del_restaurante_y_solo_lo_ve_admin(pool: PgPool) {
    let user_id = restaurante(&pool).await;
    let estado = estado(pool.clone());

    aviso(
        &estado,
        "crear",
        &[],
        json!({"IdentificadorComanda": "15", "TimeStamp": "2026-10-17T14:00:00Z"}),
        true,
    )
    .await
    .expect("aviso");

    /* Otro restaurante no ve nada. */
    let otro = restaurante(&pool).await;
    let ajenos = listar_avisos_bdp(
        State(estado.clone()),
        admin(otro),
        Query(ListarAvisosQuery::default()),
    )
    .await
    .expect("listar")
    .0;
    assert!(ajenos.is_empty(), "cada cuenta ve solo su buzón");

    /* Un trabajador no entra. */
    let mut trabajador = admin(user_id);
    trabajador.role = UserRole::Trabajador;
    trabajador.effective_role = UserRole::Trabajador;
    let error = listar_avisos_bdp(
        State(estado.clone()),
        trabajador,
        Query(ListarAvisosQuery::default()),
    )
    .await
    .expect_err("solo Admin");
    assert_eq!(
        codigo(axum::response::IntoResponse::into_response(error)),
        StatusCode::FORBIDDEN
    );

    /* El filtro por estado y el límite se respetan. */
    let avisos = listar_avisos_bdp(
        State(estado.clone()),
        admin(user_id),
        Query(ListarAvisosQuery {
            estado: Some("requiere_revision".to_string()),
            limite: Some(10),
        }),
    )
    .await
    .expect("listar por estado")
    .0;
    assert!(avisos.is_empty());

    let error = listar_avisos_bdp(
        State(estado),
        admin(user_id),
        Query(ListarAvisosQuery {
            estado: Some("inventado".to_string()),
            limite: None,
        }),
    )
    .await
    .expect_err("estado desconocido");
    assert_eq!(
        codigo(axum::response::IntoResponse::into_response(error)),
        StatusCode::BAD_REQUEST
    );
}

/// El receptor no sabe a quién apuntar si hay cero o varias configuraciones:
/// se para en vez de repartir avisos al azar.
#[sqlx::test(migrations = "./migrations")]
async fn sin_una_unica_cuenta_el_receptor_se_para(pool: PgPool) {
    let estado = estado(pool.clone());
    let error = aviso(
        &estado,
        "crear",
        &[],
        json!({"IdentificadorComanda": "15", "TimeStamp": "2026-10-17T14:00:00Z"}),
        true,
    )
    .await
    .expect_err("sin restaurante configurado no hay destino");
    assert_eq!(error, StatusCode::SERVICE_UNAVAILABLE);

    restaurante(&pool).await;
    restaurante(&pool).await;

    let error = aviso(
        &estado,
        "crear",
        &[],
        json!({"IdentificadorComanda": "15", "TimeStamp": "2026-10-17T14:00:00Z"}),
        true,
    )
    .await
    .expect_err("con dos configuraciones no se puede decidir");
    assert_eq!(error, StatusCode::SERVICE_UNAVAILABLE);
}

/* --------------------------------------------------------------------------
 * Router: tamaño del cuerpo y separación entre rutas públicas y con sesión
 * -------------------------------------------------------------------------- */

#[sqlx::test(migrations = "./migrations")]
async fn el_cuerpo_gigante_se_corta_antes_de_guardar(pool: PgPool) {
    let _user_id = restaurante(&pool).await;
    let app = rutas_publicas_avisos().with_state(estado(pool.clone()));

    /* Un aviso de verdad ocupa unos cientos de bytes; 16 KiB es el tope. */
    let relleno = "x".repeat(20 * 1024);
    let peticion = Request::builder()
        .method("POST")
        .uri("/bdp/avisos/crear")
        .header("content-type", "application/json")
        .header("x-bdp-secreto", SECRETO)
        .body(Body::from(
            serde_json::to_vec(&json!({
                "IdentificadorComanda": "15",
                "TimeStamp": "2026-10-17T14:00:00Z",
                "Relleno": relleno
            }))
            .expect("serializar"),
        ))
        .expect("petición");

    let respuesta = app.oneshot(peticion).await.expect("respuesta");
    assert_eq!(respuesta.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let filas: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bdp_avisos_inbox")
        .fetch_one(&pool)
        .await
        .expect("contar");
    assert_eq!(filas, 0, "un cuerpo demasiado grande no llega al buzón");
}

#[sqlx::test(migrations = "./migrations")]
async fn el_buzon_no_se_ve_sin_sesion_y_el_receptor_si(pool: PgPool) {
    let estado_app = estado(pool.clone());

    /* Sin sesión, el buzón contesta 401 (lo protege el extractor de sesión). */
    let peticion = Request::builder()
        .method("GET")
        .uri("/bdp/avisos")
        .body(Body::empty())
        .expect("petición");
    let respuesta = rutas_avisos_bdp()
        .with_state(estado_app.clone())
        .oneshot(peticion)
        .await
        .expect("respuesta");
    assert_eq!(respuesta.status(), StatusCode::UNAUTHORIZED);

    /* El receptor público, en cambio, atiende sin sesión: se identifica con el
     * secreto. Sin él, 401 (no 404: la ruta existe y está viva). El secreto se
     * pone en el entorno porque esta es la única prueba que pasa por el
     * manejador tal cual lo llama el router; el caso «sin configurar» ya está
     * cubierto con el secreto inyectado a `None`. */
    std::env::set_var("BDP_AVISOS_SECRETO", SECRETO);
    let peticion = Request::builder()
        .method("POST")
        .uri("/bdp/avisos/crear")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"IdentificadorComanda":"15"}"#))
        .expect("petición");
    let respuesta = rutas_publicas_avisos()
        .with_state(estado_app)
        .oneshot(peticion)
        .await
        .expect("respuesta");
    assert_eq!(respuesta.status(), StatusCode::UNAUTHORIZED);
    let cuerpo = to_bytes(respuesta.into_body(), 4096).await.expect("cuerpo");
    let texto = String::from_utf8_lossy(&cuerpo);
    assert!(texto.contains("unauthorized"), "respuesta: {texto}");
}
