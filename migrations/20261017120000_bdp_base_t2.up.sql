/* [10AA-3/L5.0] Base T2: las tablas que el PDF de alcance exige antes de las
 * pruebas de aceptación de la fase D (roadmap 09AA-2). El PDF nombra seis:
 * inbox, outbox, mapeo de IDs, cursor, pendientes agotados y libro de stock.
 *
 * De esas seis, **outbox ya existe**: `bdp_push_pendientes` es la cola de
 * salida transaccional (se escribe en la misma transacción que el cambio local,
 * con estados, reintentos e índice único parcial de una sola fila activa por
 * entidad y operación). No se duplica: ver la correspondencia en el plan
 * `plan-fase-d-bdp-local-2026-10-10.md`.
 *
 * Estas cinco son las que faltan. Todas llevan `user_id` como el resto del
 * esquema, y todas nacen vacías: no migran datos. */

/* --------------------------------------------------------------------------
 * 1. Buzón: avisos que el TPV envía a la web (fase D, F1).
 * `accion` no lleva CHECK a propósito: los siete valores reales los fija el
 * receptor (`POST /api/bdp/avisos/:accion`), que rechaza con 4xx lo que no
 * reconoce sin guardarlo. Un CHECK aquí solo añadiría una segunda lista que
 * mantener y podría rechazar un aviso legítimo antes de verlo.
 * -------------------------------------------------------------------------- */
CREATE TABLE IF NOT EXISTS bdp_avisos_inbox (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    accion TEXT NOT NULL,
    /* IP o terminal que avisa, tal cual llega. Sirve para saber si el aviso
     * viene del TPV esperado cuando hay varios puntos de exportación. */
    origen TEXT,
    /* evento + id de comanda + timestamp: la clave que hace idempotente la
     * absorción (un aviso repetido no puede mover nada dos veces). */
    clave_idempotencia TEXT NOT NULL,
    payload JSONB NOT NULL,
    /* SHA-256 del payload censurado, para detectar reenvíos con la misma clave
     * y cuerpo distinto. Nunca se guarda un secreto en claro. */
    payload_hash TEXT NOT NULL,
    estado VARCHAR(20) NOT NULL DEFAULT 'recibido'
        CHECK (estado IN ('recibido', 'procesando', 'procesado', 'rechazado', 'requiere_revision')),
    intentos INTEGER NOT NULL DEFAULT 0,
    ultimo_error TEXT,
    recibido_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    procesado_en TIMESTAMPTZ,
    CONSTRAINT uq_bdp_avisos_inbox_idempotencia UNIQUE (user_id, clave_idempotencia)
);

CREATE INDEX IF NOT EXISTS idx_bdp_avisos_inbox_pendientes
    ON bdp_avisos_inbox (user_id, estado, recibido_en);

/* --------------------------------------------------------------------------
 * 2. Mapeo de IDs Glory <-> BDP (evidencia de D03, D04, D05 y D25).
 * `glory_id` es texto porque las entidades de Glory mezclan UUID (venta) y
 * códigos alfanuméricos (artículo, cliente).
 * -------------------------------------------------------------------------- */
CREATE TABLE IF NOT EXISTS bdp_id_map (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    entidad VARCHAR(30) NOT NULL
        CHECK (entidad IN ('venta', 'cliente', 'articulo', 'departamento', 'familia',
                           'factura', 'pago', 'comanda', 'almacen', 'empleado')),
    glory_id TEXT NOT NULL,
    /* Puede faltar: BDP no devuelve identificador en todas las operaciones. */
    bdp_id TEXT,
    bdp_serie VARCHAR(20),
    bdp_terminal INTEGER,
    market_id TEXT,
    marketplace_order_id TEXT,
    /* Datos que permiten reconocer el registro si BDP no devolvió id (fecha,
     * importe, líneas...). Es la huella de D04 para no crear una comanda
     * duplicada cuando se pierde la respuesta. */
    huella JSONB NOT NULL DEFAULT '{}'::jsonb,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_bdp_id_map_glory UNIQUE (user_id, entidad, glory_id)
);

/* Dos entidades de Glory no pueden reclamar el mismo registro de BDP: es la
 * garantía de «una sola comanda» de D04 y del toque simultáneo de D08. */
CREATE UNIQUE INDEX IF NOT EXISTS uq_bdp_id_map_bdp
    ON bdp_id_map (user_id, entidad, bdp_id)
    WHERE bdp_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_bdp_id_map_marketplace
    ON bdp_id_map (user_id, entidad, market_id, marketplace_order_id)
    WHERE market_id IS NOT NULL;

/* --------------------------------------------------------------------------
 * 3. Cursores de lectura y exportación (evidencia de D01, D24a, D24b, D28-auto).
 * `posicion` es JSONB porque la marca de agua cambia por dominio: fecha de
 * última modificación, id del último documento, página...
 * `perfil` no puede ser NULL: en un índice único de PostgreSQL los NULL no
 * chocan entre sí y el cursor dejaría de ser único.
 * -------------------------------------------------------------------------- */
CREATE TABLE IF NOT EXISTS bdp_sync_cursor (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    dominio VARCHAR(30) NOT NULL,
    perfil TEXT NOT NULL DEFAULT '',
    posicion JSONB NOT NULL DEFAULT '{}'::jsonb,
    ultimo_exito TIMESTAMPTZ,
    ultimo_intento TIMESTAMPTZ,
    ultimo_error TEXT,
    CONSTRAINT uq_bdp_sync_cursor UNIQUE (user_id, dominio, perfil)
);

/* --------------------------------------------------------------------------
 * 4. Pendientes agotados: cola de revisión manual (evidencia de D29 y D31).
 * No es un estado más de `bdp_push_pendientes`: lo que falta es registrar
 * **quién** lo resolvió y **por qué**, y eso no cabe en la fila original.
 * -------------------------------------------------------------------------- */
CREATE TABLE IF NOT EXISTS bdp_pendientes_agotados (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    origen VARCHAR(20) NOT NULL
        CHECK (origen IN ('push', 'avisos', 'lectura', 'stock')),
    /* Fila de la que viene (`bdp_push_pendientes.id`, `bdp_avisos_inbox.id`...). */
    origen_id UUID,
    dominio VARCHAR(30) NOT NULL,
    entidad_id TEXT,
    operacion VARCHAR(30),
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    intentos INTEGER NOT NULL DEFAULT 0,
    ultimo_error TEXT,
    estado VARCHAR(20) NOT NULL DEFAULT 'pendiente_revision'
        CHECK (estado IN ('pendiente_revision', 'resuelto', 'descartado')),
    resuelto_por UUID,
    resuelto_en TIMESTAMPTZ,
    nota_resolucion TEXT,
    agotado_en TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

/* El mismo pendiente no se apunta dos veces; sí puede haber varios sin
 * `origen_id` (por ejemplo, una lectura que falló sin fila propia). */
CREATE UNIQUE INDEX IF NOT EXISTS uq_bdp_pendientes_agotados_origen
    ON bdp_pendientes_agotados (user_id, origen, origen_id)
    WHERE origen_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_bdp_pendientes_agotados_estado
    ON bdp_pendientes_agotados (user_id, estado, agotado_en);

/* --------------------------------------------------------------------------
 * 5. Libro de stock S/V (evidencia de D26, D26c-e y D27) con el bloqueo por
 * artículo y almacén que D26g declara como trabajo a implementar.
 *
 * El libro es la respuesta a que BDP **no devuelve identificador de
 * operación**: `id_local` es nuestro, y con S (stock previo), V (variación) y
 * la lectura posterior se puede decidir si reintentar. La regla de D26g
 * —«si ve S+V no reintentar; si ve S, un único reintento; si ve otro valor,
 * parar»— se resuelve comparando `stock_posterior` con `stock_esperado`.
 * -------------------------------------------------------------------------- */
CREATE TABLE IF NOT EXISTS bdp_stock_ledger (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    articulo_codigo TEXT NOT NULL,
    almacen INTEGER NOT NULL,
    /* Solo en traspasos (D26d). */
    almacen_destino INTEGER,
    operacion VARCHAR(30) NOT NULL,
    /* V: la variación pedida. `Units` es decimal en BDP y nunca se envía null. */
    variacion NUMERIC(14, 3) NOT NULL,
    /* S: stock leído antes de escribir. */
    stock_previo NUMERIC(14, 3),
    /* S + V, lo que debe valer si el movimiento salió bien. */
    stock_esperado NUMERIC(14, 3),
    /* Lectura posterior. NULL mientras no se haya releído. */
    stock_posterior NUMERIC(14, 3),
    estado VARCHAR(20) NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'confirmado', 'sin_movimiento',
                          'requiere_revision', 'rechazado')),
    /* Identificador local único de la operación: BDP no devuelve el suyo. */
    id_local TEXT NOT NULL,
    motivo_codigo INTEGER,
    origen VARCHAR(20) NOT NULL
        CHECK (origen IN ('web', 'tpv', 'reconciliacion')),
    documento_id TEXT,
    nota TEXT,
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmado_en TIMESTAMPTZ,
    CONSTRAINT uq_bdp_stock_ledger_local UNIQUE (user_id, id_local)
);

/* Bloqueo por artículo y almacén (D26g): mientras una operación esté
 * `pendiente`, no puede empezar otra sobre el mismo par. Es el cerrojo que
 * evita el doble movimiento; se libera al pasar a cualquier estado final. */
CREATE UNIQUE INDEX IF NOT EXISTS uq_bdp_stock_ledger_bloqueo
    ON bdp_stock_ledger (user_id, articulo_codigo, almacen)
    WHERE estado = 'pendiente';

CREATE INDEX IF NOT EXISTS idx_bdp_stock_ledger_articulo
    ON bdp_stock_ledger (user_id, articulo_codigo, creado_en DESC);
