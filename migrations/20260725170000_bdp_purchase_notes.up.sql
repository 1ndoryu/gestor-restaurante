/* [247A-11] Fase 1 compras BDP: albaranes de compra (solo lectura).
 * Cache local de albaranes exportados desde BDP mediante ExportPurchaseNotes.
 * El JSONB `datos_bdp` guarda la respuesta completa para no perder campos
 * que aún no están mapeados formalmente. */

CREATE TABLE IF NOT EXISTS bdp_purchase_notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    serie TEXT NOT NULL,
    numero TEXT NOT NULL,
    fecha DATE,
    codigo_proveedor TEXT,
    nombre_proveedor TEXT,
    total NUMERIC(14,4),
    datos_bdp JSONB NOT NULL DEFAULT '{}',
    ultima_sync_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, serie, numero)
);

CREATE INDEX IF NOT EXISTS idx_bdp_purchase_notes_user ON bdp_purchase_notes(user_id);
CREATE INDEX IF NOT EXISTS idx_bdp_purchase_notes_user_fecha ON bdp_purchase_notes(user_id, fecha);
CREATE INDEX IF NOT EXISTS idx_bdp_purchase_notes_user_proveedor ON bdp_purchase_notes(user_id, codigo_proveedor);
