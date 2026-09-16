/* sentinel-disable-file sqlx-query-sin-macro funcion-larga-rs:
 * seed demo idempotente usa SQL dinamico para limpiar datos. */
/* [039A-1] Sin panic: todas las funciones propagan el error con `?` y `main` devuelve
 * `Result`, de modo que un fallo de siembra sale por stderr con codigo distinto de 0
 * en lugar de abortar el proceso con un panic opaco a mitad del seed. */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL requerido".to_string())?;
    let pool = PgPool::connect(&db_url).await?;

    println!("Conectado a la base de datos.");

    let email = "demo@restaurante.com";
    let password = "demo1234";

    let salt = SaltString::generate(&mut OsRng);
    /* `password_hash::Error` no implementa `std::error::Error` (el crate no activa esa
     * feature), asi que se convierte a `String` para poder propagarlo con `?`. */
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Error al hashear password demo: {e}"))?
        .to_string();

    let user_id: Uuid = sqlx::query_scalar!(
        "INSERT INTO users (email, password_hash, nombre) VALUES ($1, $2, $3) \
         ON CONFLICT (email) DO UPDATE SET password_hash = $2 \
         RETURNING id",
        email,
        &hash,
        "Demo Admin"
    )
    .fetch_one(&pool)
    .await?;

    println!("Usuario demo: {email} (id: {user_id})");

    limpiar_datos(&pool, user_id).await?;

    let hoy = Local::now().date_naive();

    seed_configuracion(&pool, user_id).await?;
    let canal_ids = seed_canales(&pool, user_id).await?;
    let clientes = seed_clientes(&pool, user_id).await?;
    let cliente_ids: Vec<Uuid> = clientes.iter().map(|c| c.id).collect();
    let zona_id = seed_zonas_mesas(&pool, user_id).await?;
    let mesa_ids = zona_id.1;
    let zona_id = zona_id.0;
    let reserva_ids = seed_reservas(&pool, user_id, hoy, &canal_ids, &clientes, &mesa_ids).await?;
    seed_ventas(&pool, user_id, hoy, &reserva_ids, &cliente_ids).await?;
    seed_gastos(&pool, user_id, hoy).await?;
    seed_etiquetas_clientes(&pool, user_id, &cliente_ids).await?;
    seed_campanas(&pool, user_id).await?;
    seed_plantillas_whatsapp(&pool, user_id).await?;
    seed_reglas_recordatorio(&pool, user_id).await?;
    seed_notificaciones(&pool, user_id).await?;
    seed_trabajadores(&pool, user_id).await?;
    seed_resenas(&pool, user_id, &reserva_ids, &cliente_ids).await?;
    seed_inactividad(&pool, user_id, &canal_ids).await?;
    seed_paredes(&pool, zona_id).await?;

    println!("\nSeed completado exitosamente.");
    Ok(())
}
/* [044A-5] Script de seed reescrito — datos breves y actualizados.
 * 14 dias (7 pasados + hoy + 6 futuros), ~6 items/dia por entidad.
 * Relaciones correctas: reservas → clientes, mesas, canales; ventas → reservas.
 * Crea: usuario demo, configuracion, canales, clientes, zona+mesas,
 *   reservas, ventas, gastos, etiquetas, campanas, plantillas, reglas, notificaciones.
 * Idempotente: limpia datos del usuario demo antes de insertar.
 * Uso: cargo run --bin seed */

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::{Duration, Local, NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

/* Limpia todos los datos del usuario demo respetando FKs */
async fn limpiar_datos(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    let sentencias = [
        "DELETE FROM combinacion_mesa_items WHERE combinacion_id IN (SELECT id FROM combinaciones_mesas WHERE user_id = $1)",
        "DELETE FROM combinaciones_mesas WHERE user_id = $1",
        "DELETE FROM campana_destinatarios WHERE campana_id IN (SELECT id FROM campanas WHERE user_id = $1)",
        "DELETE FROM recordatorios_enviados WHERE regla_id IN (SELECT id FROM reglas_recordatorio WHERE user_id = $1)",
        "DELETE FROM notificaciones WHERE user_id = $1",
        "DELETE FROM reglas_recordatorio WHERE user_id = $1",
        "DELETE FROM campanas WHERE user_id = $1",
        "DELETE FROM plantillas_whatsapp WHERE user_id = $1",
        "DELETE FROM clientes_etiquetas WHERE cliente_id IN (SELECT id FROM clientes WHERE user_id = $1)",
        "DELETE FROM reservas_etiquetas WHERE reserva_id IN (SELECT id FROM reservas WHERE user_id = $1)",
        "DELETE FROM ventas WHERE user_id = $1",
        "DELETE FROM gastos WHERE user_id = $1",
        "DELETE FROM reservas WHERE user_id = $1",
        "DELETE FROM clientes WHERE user_id = $1",
        "DELETE FROM canales_reserva WHERE user_id = $1",
        "DELETE FROM mesas WHERE zona_id IN (SELECT id FROM zonas_sala WHERE user_id = $1)",
        "DELETE FROM paredes_sala WHERE zona_id IN (SELECT id FROM zonas_sala WHERE user_id = $1)",
        "DELETE FROM zonas_sala WHERE user_id = $1",
        "DELETE FROM permisos_trabajador WHERE trabajador_id IN (SELECT id FROM trabajadores WHERE user_id = $1)",
        "DELETE FROM trabajadores WHERE user_id = $1",
        "DELETE FROM envios_inactividad WHERE regla_id IN (SELECT id FROM reglas_inactividad WHERE user_id = $1)",
        "DELETE FROM reglas_inactividad WHERE user_id = $1",
        "DELETE FROM resenas WHERE user_id = $1",
        "DELETE FROM api_keys WHERE user_id = $1",
    ];
    for sql in &sentencias {
        sqlx::query(sql).bind(user_id).execute(pool).await?;
    }
    println!("Datos anteriores limpiados.");
    Ok(())
}

/* Configuracion base del restaurante */
async fn seed_configuracion(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO configuracion_restaurante (user_id, nombre_restaurante, iva_por_defecto, \
         reserva_telefono_obligatorio, reserva_nombre_obligatorio, \
         google_review_url, telefono_restaurante, url_reservas) \
         VALUES ($1, 'Demo Restaurante', 10.00, true, true, \
         'https://g.page/r/demo-restaurante/review', '912345678', 'https://demo-restaurante.com/reservar') \
         ON CONFLICT (user_id) DO UPDATE SET \
         nombre_restaurante = 'Demo Restaurante', \
         google_review_url = 'https://g.page/r/demo-restaurante/review', \
         telefono_restaurante = '912345678', \
         url_reservas = 'https://demo-restaurante.com/reservar'",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    println!("  Configuración del restaurante creada.");
    Ok(())
}

/* 4 canales de reserva */
async fn seed_canales(pool: &PgPool, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    let nombres = ["Teléfono", "WhatsApp", "Web", "Walk-in"];
    let mut ids = Vec::with_capacity(nombres.len());
    for nombre in &nombres {
        let id: Uuid = sqlx::query_scalar!(
            "INSERT INTO canales_reserva (user_id, nombre) VALUES ($1, $2) RETURNING id",
            user_id,
            *nombre
        )
        .fetch_one(pool)
        .await?;
        ids.push(id);
    }
    println!("  {} canales insertados.", ids.len());
    Ok(ids)
}

type ClienteDemo = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

const DATOS_CLIENTES_DEMO: &[ClienteDemo] = &[
    (
        "María",
        "García López",
        "612345678",
        "maria.garcia@email.com",
        "",
        "Frutos secos",
        "Vino tinto",
        "",
    ),
    (
        "Carlos",
        "Rodríguez",
        "623456789",
        "carlos.rod@email.com",
        "Deloitte",
        "",
        "Cerveza",
        "",
    ),
    ("Ana", "Martínez", "634567890", "", "", "Celiaca", "", ""),
    (
        "Pedro",
        "Sánchez Ruiz",
        "645678901",
        "pedro.s@email.com",
        "",
        "",
        "",
        "Ventana",
    ),
    (
        "Laura",
        "Fernández",
        "656789012",
        "laura.f@email.com",
        "",
        "Lactosa",
        "",
        "",
    ),
    (
        "Javier",
        "López Torres",
        "667890123",
        "",
        "",
        "",
        "",
        "Terraza",
    ),
    (
        "Carmen",
        "Ruiz",
        "678901234",
        "carmen.ruiz@email.com",
        "",
        "Vegetariana",
        "Agua con gas",
        "",
    ),
    (
        "Miguel",
        "Torres",
        "689012345",
        "",
        "Eventos Sol S.L.",
        "",
        "",
        "Salón privado",
    ),
];

/* 8 clientes demo con datos variados */
async fn seed_clientes(pool: &PgPool, user_id: Uuid) -> Result<Vec<ClienteCreado>, sqlx::Error> {
    let mut clientes = Vec::with_capacity(DATOS_CLIENTES_DEMO.len());
    for &(nombre, apellidos, tel, email, empresa, alergias, pref_beb, pref_ubi) in
        DATOS_CLIENTES_DEMO
    {
        let id: Uuid = sqlx::query_scalar!(
            "INSERT INTO clientes (user_id, nombre, apellidos, telefono, email, empresa, \
             alergias, preferencias_bebida, preferencias_ubicacion) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
            user_id,
            nombre,
            apellidos,
            tel,
            email,
            empresa,
            alergias,
            pref_beb,
            pref_ubi
        )
        .fetch_one(pool)
        .await?;
        clientes.push(ClienteCreado {
            id,
            nombre: nombre.to_string(),
            apellidos: apellidos.to_string(),
        });
    }
    println!("  {} clientes insertados.", clientes.len());
    Ok(clientes)
}

/* `from_hms_opt` solo rechaza combinaciones que no forman una hora real; las de este
 * binario si lo son, asi que se devuelve el `Option` en lugar de entrar en panico. */
fn hora(hora: u32, minuto: u32) -> Option<NaiveTime> {
    NaiveTime::from_hms_opt(hora, minuto, 0)
}

/* 1 zona de sala + 6 mesas posicionadas. Retorna (zona_id, mesa_ids) */
async fn seed_zonas_mesas(pool: &PgPool, user_id: Uuid) -> Result<(Uuid, Vec<Uuid>), sqlx::Error> {
    let zona_id: Uuid = sqlx::query_scalar(
        "INSERT INTO zonas_sala (user_id, nombre, orden, ancho, alto) \
         VALUES ($1, 'Salón principal', 0, 800, 600) RETURNING id",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let mut mesa_ids = Vec::with_capacity(6);
    /* (numero, pos_x, pos_y, forma, min_personas, max_personas) */
    let mesas: &[(i32, i32, i32, &str, i32, i32)] = &[
        (1, 50, 50, "cuadrada", 2, 4),
        (2, 200, 50, "cuadrada", 2, 4),
        (3, 350, 50, "redonda", 4, 6),
        (4, 50, 250, "cuadrada", 2, 2),
        (5, 200, 250, "redonda", 6, 8),
        (6, 350, 250, "cuadrada", 2, 4),
    ];
    for &(num, x, y, forma, min_p, max_p) in mesas {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO mesas (zona_id, numero, pos_x, pos_y, forma, min_personas, max_personas) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(zona_id)
        .bind(num)
        .bind(x)
        .bind(y)
        .bind(forma)
        .bind(min_p)
        .bind(max_p)
        .fetch_one(pool)
        .await?;
        mesa_ids.push(id);
    }
    println!("  1 zona + {} mesas insertadas.", mesa_ids.len());
    Ok((zona_id, mesa_ids))
}

/* Datos de cada cliente creado, para vincular reservas con nombre real */
struct ClienteCreado {
    id: Uuid,
    nombre: String,
    apellidos: String,
}

/* Datos de cada reserva creada, para vincular ventas despues */
struct ReservaCreada {
    id: Uuid,
    fecha: NaiveDate,
    estado: String,
    cliente_id: Option<Uuid>,
}

/* 14 dias x 6 reservas/dia = 84 reservas.
 * Dias pasados: ~70% completada, ~15% no_show, ~15% cancelada.
 * Hoy: mix de confirmada/pendiente/completada.
 * Futuros: 60% confirmada, 30% pendiente, 10% lista_espera. */
#[allow(
    clippy::too_many_arguments,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
async fn seed_reservas(
    pool: &PgPool,
    user_id: Uuid,
    hoy: NaiveDate,
    canal_ids: &[Uuid],
    clientes: &[ClienteCreado],
    mesa_ids: &[Uuid],
) -> Result<Vec<ReservaCreada>, sqlx::Error> {
    let horas = [
        hora(13, 0),
        hora(13, 30),
        hora(14, 0),
        hora(20, 30),
        hora(21, 0),
        hora(21, 30),
    ];

    let mut reservas = Vec::new();
    let inicio = hoy - Duration::days(7);

    for dia in 0..14i64 {
        let fecha = inicio + Duration::days(dia);
        let es_pasado = fecha < hoy;

        for i in 0..6usize {
            let idx = (dia as usize * 6 + i) % 6;
            let personas = 2 + (i % 5) as i32;

            let (estado, no_show) = if es_pasado {
                match i % 7 {
                    0 => ("no_show", true),
                    6 => ("cancelada", false),
                    _ => ("completada", false),
                }
            } else if fecha == hoy {
                match i % 3 {
                    0 => ("confirmada", false),
                    1 => ("pendiente", false),
                    _ => ("completada", false),
                }
            } else {
                match i % 10 {
                    0 => ("lista_espera", false),
                    1 | 4 => ("pendiente", false),
                    _ => ("confirmada", false),
                }
            };

            /* [064A-1] Todas las reservas vinculadas a un cliente del CRM.
             * Antes solo ~50% tenían cliente_id (i % 2 == 0). Ahora todas lo tienen
             * para que la Ficha del Cliente aparezca siempre en reservas y ventas. */
            let c = &clientes[idx % clientes.len()];
            let (nombre_r, apellidos_r, cliente_id) =
                (c.nombre.as_str(), c.apellidos.as_str(), Some(c.id));

            /* Asignar mesa FK a 4 de cada 6 reservas */
            let mesa_id = if i < 4 {
                Some(mesa_ids[i % mesa_ids.len()])
            } else {
                None
            };
            let num_mesa: Option<i32> = if i < 4 { Some((i as i32) + 1) } else { None };

            let canal_id = Some(canal_ids[idx % canal_ids.len()]);
            let telefono = format!("6{:08}", 10_000_000 + dia * 100 + i as i64);
            let hora_reserva = horas[i].ok_or_else(|| {
                sqlx::Error::Protocol(format!("hora demo invalida en indice {i}"))
            })?;

            let id: Uuid = sqlx::query_scalar!(
                "INSERT INTO reservas (user_id, fecha, hora, nombre_cliente, apellidos_cliente, \
                 num_personas, estado, notas, telefono, canal_id, cliente_id, no_show, mesa_id, num_mesa) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id",
                user_id,
                fecha,
                hora_reserva,
                nombre_r,
                apellidos_r,
                personas,
                estado,
                if i == 0 { "Mesa junto a ventana" } else { "" },
                &telefono,
                canal_id,
                cliente_id,
                no_show,
                mesa_id,
                num_mesa
            )
            .fetch_one(pool)
            .await?;

            reservas.push(ReservaCreada {
                id,
                fecha,
                estado: estado.to_string(),
                cliente_id,
            });
        }
    }
    println!(
        "  {} reservas insertadas (7 pasados + hoy + 6 futuros).",
        reservas.len()
    );
    Ok(reservas)
}

/* Ventas vinculadas a reservas completadas + 2 extras sin reserva */
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
async fn seed_ventas(
    pool: &PgPool,
    user_id: Uuid,
    hoy: NaiveDate,
    reservas: &[ReservaCreada],
    cliente_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let turnos = ["manana", "mediodia", "noche"];
    let canales_venta = ["comedor", "barra", "terraza", "delivery"];
    let metodos = ["efectivo", "tarjeta", "transferencia"];
    let descripciones = [
        "Menú del día",
        "Cena grupo",
        "Tapas variadas",
        "Menú ejecutivo",
        "Comida terraza",
        "Brunch dominical",
    ];

    let mut count: u32 = 0;

    /* Una venta por cada reserva completada en fecha pasada o de hoy */
    for r in reservas
        .iter()
        .filter(|r| r.estado == "completada" && r.fecha <= hoy)
    {
        let idx = count as usize;
        let base = Decimal::from(45 + (idx * 17) % 200);
        let iva = (base * Decimal::new(10, 2)).round_dp(2);
        let comensales = 2 + (idx % 6) as i32;

        sqlx::query!(
            "INSERT INTO ventas (user_id, fecha, comensales, descripcion, iva_porcentaje, \
             turno, canal, metodo_pago, importe_base, importe_iva, reserva_id, cliente_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            user_id,
            r.fecha,
            comensales,
            descripciones[idx % descripciones.len()],
            Decimal::from(10),
            turnos[idx % turnos.len()],
            canales_venta[idx % canales_venta.len()],
            metodos[idx % metodos.len()],
            base,
            iva,
            Some(r.id),
            r.cliente_id
        )
        .execute(pool)
        .await?;
        count += 1;
    }

    /* 2 ventas extras sin reserva (walk-in / delivery) */
    for i in 0..2u32 {
        let fecha = hoy - Duration::days(i64::from(i));
        let base = Decimal::from(30 + i * 25);
        let iva = (base * Decimal::new(10, 2)).round_dp(2);

        sqlx::query!(
            "INSERT INTO ventas (user_id, fecha, comensales, descripcion, iva_porcentaje, \
             turno, canal, metodo_pago, importe_base, importe_iva, reserva_id, cliente_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, $11)",
            user_id,
            fecha,
            2 + i as i32,
            "Pedido delivery",
            Decimal::from(10),
            "mediodia",
            "delivery",
            "tarjeta",
            base,
            iva,
            Some(cliente_ids[i as usize % cliente_ids.len()])
        )
        .execute(pool)
        .await?;
        count += 1;
    }
    println!("  {count} ventas insertadas.");
    Ok(())
}

/* Gastos: 4/dia x 8 dias pasados = 32 gastos */
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_range_loop
)]
async fn seed_gastos(pool: &PgPool, user_id: Uuid, hoy: NaiveDate) -> Result<(), sqlx::Error> {
    /* Si la tabla de categorias estuviera vacia el seed seguiria con categoria NULL;
     * un fallo real del SELECT ya no se oculta con `unwrap_or_default`. */
    let categorias: Vec<Uuid> =
        sqlx::query_scalar!("SELECT id FROM categorias_gasto ORDER BY nombre")
            .fetch_all(pool)
            .await?;

    let tipos = ["factura", "albaran", "ticket"];
    let metodos = ["efectivo", "tarjeta", "transferencia"];
    let proveedores = [
        "Distribuciones López",
        "Carnes Ibéricas S.L.",
        "Pescados del Norte",
        "Verduras Eco",
    ];

    let mut count: u32 = 0;
    for dia in 1..=8i64 {
        let fecha = hoy - Duration::days(dia);
        for i in 0..4usize {
            let idx = (dia as usize - 1) * 4 + i;
            let base = Decimal::from(20 + (idx * 13) % 400);
            let iva = (base * Decimal::new(21, 2)).round_dp(2);
            let cat_id = if categorias.is_empty() {
                None
            } else {
                Some(categorias[idx % categorias.len()])
            };

            sqlx::query!(
                "INSERT INTO gastos (user_id, fecha, proveedor, categoria_id, tipo_documento, \
                 metodo_pago, numero_documento, recurrente, importe_base, importe_iva) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                user_id,
                fecha,
                proveedores[i],
                cat_id,
                tipos[idx % tipos.len()],
                metodos[idx % metodos.len()],
                format!("DOC-{:04}", count + 1),
                i == 0,
                base,
                iva
            )
            .execute(pool)
            .await?;
            count += 1;
        }
    }
    println!("  {count} gastos insertados.");
    Ok(())
}

/* Asigna etiquetas del sistema a los clientes para probar el CRM */
async fn seed_etiquetas_clientes(
    pool: &PgPool,
    user_id: Uuid,
    cliente_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let etiquetas = sqlx::query!(
        "SELECT e.id, e.nombre FROM etiquetas e \
         JOIN categorias_etiqueta ce ON ce.id = e.categoria_id \
         WHERE e.es_sistema = TRUE AND ce.aplica_a = 'cliente' \
         ORDER BY e.nombre"
    )
    .fetch_all(pool)
    .await?;

    if etiquetas.is_empty() {
        println!("  Sin etiquetas del sistema — omitiendo asignaciones.");
        return Ok(());
    }

    let mut count: u32 = 0;

    /* 1-3 etiquetas por cliente, patron determinista */
    for (ci, cliente_id) in cliente_ids.iter().enumerate() {
        let num_tags = 1 + ci % 3;
        for t in 0..num_tags {
            let etiqueta = &etiquetas[(ci + t) % etiquetas.len()];
            let resultado = sqlx::query!(
                "INSERT INTO clientes_etiquetas (cliente_id, etiqueta_id) \
                 VALUES ($1, $2) ON CONFLICT DO NOTHING",
                cliente_id,
                etiqueta.id
            )
            .execute(pool)
            .await;

            if resultado.is_ok() {
                count += 1;
            }
        }
    }
    println!("  {count} asignaciones cliente-etiqueta insertadas.");

    /* Crear etiquetas personalizadas del usuario demo si no existen */
    let user_tags: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM etiquetas WHERE user_id = $1", user_id)
            .fetch_one(pool)
            .await?
            .unwrap_or(0);

    if user_tags == 0 {
        let cat_fidelizacion: Option<Uuid> = sqlx::query_scalar!(
            "SELECT id FROM categorias_etiqueta \
             WHERE nombre = 'Fidelización' AND aplica_a = 'cliente' AND es_sistema = TRUE"
        )
        .fetch_optional(pool)
        .await?;

        if let Some(cat_id) = cat_fidelizacion {
            let custom = [
                ("Habitual fin de semana", "#2196F3"),
                ("Amigo del chef", "#9C27B0"),
            ];
            for (nombre, color) in &custom {
                sqlx::query!(
                    "INSERT INTO etiquetas (user_id, categoria_id, nombre, color) \
                     VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                    user_id,
                    cat_id,
                    *nombre,
                    *color
                )
                .execute(pool)
                .await?;
            }
            println!("  Etiquetas personalizadas del usuario creadas.");
        }
    }
    Ok(())
}

/* 3 campañas de marketing en distintos estados */
#[allow(clippy::type_complexity)]
async fn seed_campanas(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    /* (nombre, descripcion, canales, segmento, mensaje, estado, total_dest, total_env, total_fall) */
    let campanas: &[(&str, &str, &[&str], &str, &str, &str, i32, i32, i32)] = &[
        (
            "Promoción verano 2026",
            "Campaña para recuperar clientes inactivos",
            &["sms", "email"],
            "sin_3m",
            "¡Te echamos de menos! Vuelve este verano y disfruta de un 15% de descuento en tu próxima reserva.",
            "borrador",
            0, 0, 0,
        ),
        (
            "Menú degustación mayo",
            "Promoción nuevo menú degustación primavera",
            &["email"],
            "habitual",
            "Hola, hemos preparado un nuevo menú degustación de temporada. Como cliente habitual, tienes acceso prioritario.",
            "enviada",
            42, 38, 4,
        ),
        (
            "Recordatorio San Valentín",
            "Campaña cancelada — se planificó tarde",
            &["sms", "whatsapp"],
            "todos",
            "Celebra San Valentín con nosotros. Menú especial para 2 por 65€. Reservas: 912345678",
            "cancelada",
            0, 0, 0,
        ),
    ];

    for &(nombre, desc, canales, segmento, mensaje, estado, t_dest, t_env, t_fall) in campanas {
        let canales_arr: Vec<String> = canales.iter().map(|c| (*c).to_string()).collect();
        sqlx::query!(
            "INSERT INTO campanas (user_id, nombre, descripcion_interna, cuerpo_mensaje, \
             canales, segmento, estado, total_destinatarios, total_enviados, total_fallidos) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            user_id,
            nombre,
            desc,
            mensaje,
            &canales_arr,
            segmento,
            estado,
            t_dest,
            t_env,
            t_fall
        )
        .execute(pool)
        .await?;
    }
    println!("  {} campañas insertadas.", campanas.len());
    Ok(())
}

/* 3 plantillas WhatsApp en distintos estados */
#[allow(clippy::type_complexity)]
async fn seed_plantillas_whatsapp(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    /* (nombre, categoria, idioma, cuerpo, estado, meta_template_id, meta_razon_rechazo) */
    let plantillas: &[(&str, &str, &str, &str, &str, Option<&str>, Option<&str>)] = &[
        (
            "promocion_verano",
            "MARKETING",
            "es",
            "¡Hola {{1}}! Este verano disfruta de un 15% de descuento. Te esperamos. Reserva en {{2}}.",
            "aprobada",
            Some("tpl_abc123"),
            None,
        ),
        (
            "confirmacion_reserva",
            "UTILITY",
            "es",
            "Hola {{1}}, tu reserva para {{2}} personas el {{3}} a las {{4}} está confirmada. ¡Te esperamos!",
            "borrador",
            None,
            None,
        ),
        (
            "oferta_navidad",
            "MARKETING",
            "es",
            "¡Felices fiestas {{1}}! Celebra la Navidad con nuestro menú especial por 45€/persona. Reserva: {{2}}",
            "rechazada",
            Some("tpl_xyz789"),
            Some("El contenido promocional no cumple la política de precios de Meta."),
        ),
    ];

    for &(nombre, categoria, idioma, cuerpo, estado, meta_id, razon) in plantillas {
        sqlx::query!(
            "INSERT INTO plantillas_whatsapp (user_id, nombre, categoria, idioma, cuerpo_mensaje, \
             estado, meta_template_id, meta_razon_rechazo) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            user_id,
            nombre,
            categoria,
            idioma,
            cuerpo,
            estado,
            meta_id,
            razon
        )
        .execute(pool)
        .await?;
    }
    println!("  {} plantillas WhatsApp insertadas.", plantillas.len());
    Ok(())
}

/* 3 reglas de recordatorio automatico */
async fn seed_reglas_recordatorio(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    /* (nombre, horas_antes, canal, mensaje, activa) */
    let reglas: &[(&str, i32, &str, &str, bool)] = &[
        (
            "Recordatorio día anterior",
            24,
            "sms",
            "Hola {nombre}, te recordamos tu reserva mañana a las {hora}. ¡Te esperamos!",
            true,
        ),
        (
            "Confirmación 2h antes",
            2,
            "whatsapp",
            "Hola {nombre}, tu mesa para {num_personas} está lista hoy a las {hora}. ¿Confirmas asistencia?",
            true,
        ),
        (
            "Recordatorio email semanal",
            168,
            "email",
            "Hola {nombre}, tienes una reserva programada para el {fecha} a las {hora}. Si necesitas cambios contáctanos.",
            false,
        ),
    ];

    for &(nombre, horas, canal, mensaje, activa) in reglas {
        sqlx::query!(
            "INSERT INTO reglas_recordatorio (user_id, nombre, horas_antes, canal, mensaje_plantilla, activa) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            user_id,
            nombre,
            horas,
            canal,
            mensaje,
            activa
        )
        .execute(pool)
        .await?;
    }
    println!("  {} reglas de recordatorio insertadas.", reglas.len());
    Ok(())
}

/* [134A-1] 3 trabajadores demo con distintos cargos y permisos */
async fn seed_trabajadores(pool: &PgPool, user_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(b"trabajador123", &salt)
        .map_err(|e| format!("Error al hashear password de trabajador demo: {e}"))?
        .to_string();

    /* (nombre, email, cargo) */
    let trabajadores: &[(&str, &str, &str)] = &[
        ("Sara López", "sara.lopez@demo.com", "Camarera"),
        ("Tomás Ruiz", "tomas.ruiz@demo.com", "Jefe de sala"),
        ("Marta Gil", "marta.gil@demo.com", "Hostess"),
    ];

    for &(nombre, email, cargo) in trabajadores {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO trabajadores (id, user_id, nombre, email, password_hash, cargo) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(user_id)
        .bind(nombre)
        .bind(email)
        .bind(&hash)
        .bind(cargo)
        .execute(pool)
        .await?;
        /* [169A-3] Demo con el defecto de operativa diaria (el dueño lo
         * reconfigura por trabajador desde la UI). Copia local de
         * `models::PERMISOS_DEFECTO_TRABAJADOR` (el bin no enlaza el lib). */
        for seccion in [
            "dashboard",
            "ventas",
            "gastos",
            "reservas",
            "calendario",
            "clientes",
            "canales",
            "no_shows",
            "plano_sala",
        ] {
            sqlx::query(
                "INSERT INTO permisos_trabajador (id, trabajador_id, seccion, permitido) \
                 VALUES ($1, $2, $3, true)",
            )
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(seccion)
            .execute(pool)
            .await?;
        }
    }
    println!("  {} trabajadores insertados.", trabajadores.len());
    Ok(())
}

/* [134A-1] 4 reseñas demo: 2 respondidas, 1 pendiente, 1 redirigida a Google */
async fn seed_resenas(
    pool: &PgPool,
    user_id: Uuid,
    reservas: &[ReservaCreada],
    cliente_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    /* Tomar reservas completadas para vincular reseñas */
    let completadas: Vec<&ReservaCreada> = reservas
        .iter()
        .filter(|r| r.estado == "completada")
        .take(4)
        .collect();

    if completadas.is_empty() {
        println!("  Sin reservas completadas para vincular reseñas.");
        return Ok(());
    }

    /* (puntuacion: Option, comentario: Option, redirigido_google) */
    let configs: &[(Option<i16>, Option<&str>, bool)] = &[
        (
            Some(5),
            Some("Excelente servicio, volveremos sin duda."),
            true,
        ),
        (
            Some(4),
            Some("Muy buena comida, el ambiente es agradable."),
            false,
        ),
        (Some(3), None, false),
        (None, None, false),
    ];

    let mut count: u32 = 0;
    for (i, reserva) in completadas.iter().enumerate() {
        let id = Uuid::new_v4();
        let token = format!("demo-token-{i:04}");
        let (puntuacion, comentario, redirigido) = configs[i % configs.len()];
        let cliente_id = reserva.cliente_id.or_else(|| cliente_ids.get(i).copied());

        if puntuacion.is_some() {
            sqlx::query(
                "INSERT INTO resenas (id, user_id, reserva_id, cliente_id, token, \
                 puntuacion, comentario, redirigido_google, respondida_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW()) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(user_id)
            .bind(reserva.id)
            .bind(cliente_id)
            .bind(&token)
            .bind(puntuacion)
            .bind(comentario.unwrap_or("")) /* evitar NULL explícito — usar '' como el DEFAULT */
            .bind(redirigido)
            .execute(pool)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO resenas (id, user_id, reserva_id, cliente_id, token) \
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(user_id)
            .bind(reserva.id)
            .bind(cliente_id)
            .bind(&token)
            .execute(pool)
            .await?;
        }
        count += 1;
    }
    println!("  {count} reseñas insertadas.");
    Ok(())
}

/* [134A-1] 3 reglas de inactividad con distintos canales y períodos */
async fn seed_inactividad(
    pool: &PgPool,
    user_id: Uuid,
    _canal_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    /* (nombre, dias_inactividad, canal, mensaje_plantilla) */
    let reglas: &[(&str, i32, &str, &str)] = &[
        (
            "Clientes sin visita en 30 días",
            30,
            "whatsapp",
            "Hola {nombre}, te echamos de menos. ¿Te apetece reservar una mesa esta semana?",
        ),
        (
            "Clientes inactivos 60 días",
            60,
            "sms",
            "Hola {nombre}, hace tiempo que no te vemos. ¡Vuelve y disfruta de nuestro menú!",
        ),
        (
            "Reactivación trimestral",
            90,
            "email",
            "Estimado/a {nombre}, te recordamos que puedes reservar tu mesa en cualquier momento.",
        ),
    ];

    for &(nombre, dias, canal, mensaje) in reglas {
        sqlx::query(
            "INSERT INTO reglas_inactividad (user_id, nombre, dias_inactividad, canal, mensaje_plantilla) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user_id)
        .bind(nombre)
        .bind(dias)
        .bind(canal)
        .bind(mensaje)
        .execute(pool)
        .await?;
    }
    println!("  {} reglas de inactividad insertadas.", reglas.len());
    Ok(())
}

/* [134A-1] 2 paredes demo para decorar el plano de sala */
async fn seed_paredes(pool: &PgPool, zona_id: Uuid) -> Result<(), sqlx::Error> {
    /* [134A-8] Pared = bar horizontal: ancho = largo, alto = grosor (10). Rotación orienta. */
    let paredes: &[(i32, i32, i32, i32, f64, &str)] = &[
        (100, 50, 150, 10, 90.0, "#6b7280"), /* vertical (rotada 90°) */
        (50, 200, 200, 10, 0.0, "#6b7280"),  /* horizontal */
    ];

    for &(x, y, ancho, alto, rotacion, color) in paredes {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO paredes_sala (id, zona_id, pos_x, pos_y, ancho, alto, rotacion, color) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(zona_id)
        .bind(x)
        .bind(y)
        .bind(ancho)
        .bind(alto)
        .bind(rotacion)
        .bind(color)
        .execute(pool)
        .await?;
    }
    println!("  {} paredes insertadas.", paredes.len());
    Ok(())
}

/* 3 notificaciones demo para probar el panel */
async fn seed_notificaciones(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    let notis: &[(&str, &str, &str)] = &[
        (
            "nueva_reserva",
            "Nueva reserva recibida",
            "Elena Díaz ha reservado para 4 personas mañana a las 21:00.",
        ),
        (
            "no_show",
            "No-show detectado",
            "Sergio Muñoz no se presentó a su reserva de hoy a las 13:30.",
        ),
        (
            "campana_completada",
            "Campaña enviada",
            "La campaña 'Menú degustación mayo' se envió a 38 destinatarios con 4 fallos.",
        ),
    ];

    for &(tipo, titulo, mensaje) in notis {
        sqlx::query(
            "INSERT INTO notificaciones (user_id, tipo, titulo, mensaje) VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(tipo)
        .bind(titulo)
        .bind(mensaje)
        .execute(pool)
        .await?;
    }
    println!("  {} notificaciones insertadas.", notis.len());
    Ok(())
}
