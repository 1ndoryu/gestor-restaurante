/* [247A-9] Ledger local de pagos parciales BDP.
 * Cada fila representa un intento de pago individual sobre una venta.
 * El saldo pendiente se calcula como total_venta - SUM(amount WHERE resultado='exito').
 * La idempotencia_key evita doble cargo si el cliente reintenta una peticion. */
CREATE TABLE IF NOT EXISTS bdp_pagos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    venta_id UUID NOT NULL REFERENCES ventas(id) ON DELETE CASCADE,
    amount NUMERIC(10, 2) NOT NULL CHECK (amount > 0),
    tender_id INT NOT NULL,
    idempotency_key VARCHAR(255) NOT NULL UNIQUE,
    bdp_order_id BIGINT,
    bdp_payment_id VARCHAR(50),
    resultado VARCHAR(50) NOT NULL DEFAULT 'exito' CHECK (resultado IN ('exito', 'ambiguo', 'error')),
    datos_respuesta JSONB,
    error_mensaje TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bdp_pagos_venta_id ON bdp_pagos(venta_id);
CREATE INDEX IF NOT EXISTS idx_bdp_pagos_venta_resultado ON bdp_pagos(venta_id, resultado);
