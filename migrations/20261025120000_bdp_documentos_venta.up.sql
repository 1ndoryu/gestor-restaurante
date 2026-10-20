/* [L8/D28] Documentos de venta exportados desde BDP (ExportProfiles/Documents).
 * La clave natural es el id que asigna BDP dentro del perfil: reimportar una
 * ventana solapada no duplica filas. El JSON crudo se conserva porque los
 * campos de documento no están documentados y la lectura es tolerante. */
CREATE TABLE IF NOT EXISTS bdp_documentos_venta (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    perfil INTEGER NOT NULL CHECK (perfil > 0),
    bdp_documento_id TEXT NOT NULL CHECK (bdp_documento_id <> ''),
    fecha DATE,
    datos_bdp JSONB NOT NULL DEFAULT '{}'::jsonb,
    recibido_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_bdp_documento_venta UNIQUE (user_id, perfil, bdp_documento_id)
);
