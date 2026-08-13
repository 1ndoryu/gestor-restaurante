/* [128A-1/F7] Menús/packs locales (D2, §4.10).
 * Agrupaciones de artículos del catálogo local, 100% operativas sin BDP.
 * Reutilizan el patrón de líneas de `venta_lineas` (artículo + descripción +
 * cantidad + precio) en una tabla hija dedicada.
 *
 * `bdp_menus_locales`  : cabecera del menú/pack (tipo, nombre, precio, activo).
 * `bdp_menu_local_lineas`: artículos que componen el menú/pack.
 */

CREATE TABLE IF NOT EXISTS bdp_menus_locales (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    tipo VARCHAR(10) NOT NULL CHECK (tipo IN ('menu', 'pack')),
    nombre TEXT NOT NULL,
    descripcion TEXT,
    precio NUMERIC(12,2) NOT NULL DEFAULT 0,
    activo BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, tipo, nombre)
);

CREATE INDEX IF NOT EXISTS idx_bdp_menus_locales_user_tipo
    ON bdp_menus_locales(user_id, tipo);

CREATE TABLE IF NOT EXISTS bdp_menu_local_lineas (
    id UUID PRIMARY KEY,
    menu_id UUID NOT NULL REFERENCES bdp_menus_locales(id) ON DELETE CASCADE,
    articulo_codigo TEXT,
    descripcion TEXT NOT NULL,
    cantidad NUMERIC(12,3) NOT NULL DEFAULT 1,
    precio_unitario NUMERIC(12,2) NOT NULL DEFAULT 0,
    orden INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bdp_menu_local_lineas_menu
    ON bdp_menu_local_lineas(menu_id);
