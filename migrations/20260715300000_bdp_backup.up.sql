/* [BKP-001] Sistema de backup y auditoría BDP.
 * Crea tablas para snapshots (puntos de restauración) y audit log (traza inmutable).
 * Añade campos de configuración para modo de sync y retención.
 * Prioridad: seguridad de datos del cliente — BDP no tiene backup nativo. */

-- Tabla de snapshots: cada snapshot es un punto de restauración
CREATE TABLE IF NOT EXISTS bdp_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES usuarios(id),
    tipo VARCHAR(50) NOT NULL,
    /* Tipos BDP: 'completo', 'articulos', 'clientes', 'departamentos', 'salones', 'empleados', 'tenders', 'poses'
       Tipos Glory: 'glory_ventas', 'glory_clientes', 'glory_mapeos' */
    direccion VARCHAR(20) NOT NULL,
    /* 'bdp' = snapshot de datos leídos de BDP
       'glory' = snapshot de datos locales de Glory */
    trigger_tipo VARCHAR(50) NOT NULL,
    /* 'manual', 'pre_write', 'exploracion_inicial', 'scheduled' */
    datos JSONB NOT NULL,
    /* Contenido del snapshot — estructura varía por tipo */
    metadata JSONB,
    /* Info adicional: endpoints llamados, cantidad registros, warnings, etc. */
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    /* NULL = no expira automáticamente */
    notas TEXT
);

CREATE INDEX IF NOT EXISTS idx_bdp_snapshots_user ON bdp_snapshots(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bdp_snapshots_tipo ON bdp_snapshots(tipo, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bdp_snapshots_expires ON bdp_snapshots(expires_at) WHERE expires_at IS NOT NULL;

-- Tabla de auditoría: cada operación de sync queda registrada
CREATE TABLE IF NOT EXISTS bdp_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES usuarios(id),
    operacion VARCHAR(50) NOT NULL,
    /* 'create_order', 'create_customer', 'add_payment', 'invoice',
       'sync_catalog', 'sync_prices', 'sync_tables' */
    direccion VARCHAR(20) NOT NULL,
    /* 'glory_to_bdp', 'bdp_to_glory' */
    snapshot_pre_id UUID REFERENCES bdp_snapshots(id),
    /* Snapshot tomado ANTES de la operación (NULL si no se tomó) */
    datos_enviados JSONB,
    /* Lo que se envió a BDP o lo que vino de BDP */
    resultado VARCHAR(20) NOT NULL DEFAULT 'pendiente',
    /* 'exito', 'error', 'parcial', 'pendiente' */
    datos_respuesta JSONB,
    /* Respuesta de BDP o resultado de Glory */
    error_mensaje TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bdp_audit_user ON bdp_audit_log(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bdp_audit_operacion ON bdp_audit_log(operacion, created_at DESC);

-- Nuevos campos de configuración
ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_sync_mode VARCHAR(20) NOT NULL DEFAULT 'read_only';
    /* Valores: 'read_only', 'unidirectional', 'bidirectional'
       read_only = Glory solo lee de BDP, nunca escribe (DEFAULT seguro)
       unidirectional = Glory puede enviar ventas/clientes a BDP
       bidirectional = Lectura + escritura en ambas direcciones */

ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_backup_retention_days INTEGER NOT NULL DEFAULT 30;

ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_auto_backup_before_write BOOLEAN NOT NULL DEFAULT true;
