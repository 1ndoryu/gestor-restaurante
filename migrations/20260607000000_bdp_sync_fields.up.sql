/* [065A-5] Campos de sincronización BDP en ventas y configuración.
 * BDP sync: cada venta Glory crea una comanda pendiente en el TPV (BDP-Net).
 * bdp_default_article_code: artículo BDP genérico para ventas sin mapeo por-producto.
 * bdp_order_id: OrderId devuelto por BDP tras CreateOrder exitoso. */

-- Configuración: artículo por defecto para el mapeo Glory → BDP
ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_default_article_code TEXT NOT NULL DEFAULT 'GLORY',
    ADD COLUMN IF NOT EXISTS bdp_default_article_name TEXT NOT NULL DEFAULT 'Servicio Glory';

-- Ventas: tracking de sincronización BDP (patrón idéntico a haddock_synced)
ALTER TABLE ventas
    ADD COLUMN IF NOT EXISTS bdp_synced BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS bdp_synced_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS bdp_sync_error TEXT,
    ADD COLUMN IF NOT EXISTS bdp_order_id BIGINT;

CREATE INDEX IF NOT EXISTS idx_ventas_bdp_synced ON ventas(bdp_synced) WHERE bdp_synced = FALSE;
