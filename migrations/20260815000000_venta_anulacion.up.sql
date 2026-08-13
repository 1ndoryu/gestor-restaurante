/* [128A-1/F4] Anulación local de ventas — C4: migración aditiva (M15).
 *
 * Semántica de columnas en `ventas`:
 *   - `anulada`: la venta quedó anulada localmente (estado final, transición única
 *     pendiente/pagada -> anulada, M9/M10). Las anuladas nunca se borran físicamente
 *     (histórico con motivo, D5).
 *   - `anulada_at`: momento de la anulación local.
 *   - `anulacion_motivo`: obligatorio en modalidad `credito_completo`.
 *   - `anulacion_usuario`: usuario que anuló (referencia a users; NULL si se anula
 *     desde un contexto sin usuario autenticado).
 *
 * Estado "anulada_local_pendiente_bdp" (C3=b, M8) se deriva, no se almacena:
 *   anulada = true AND bdp_synced = true AND bdp_order_status NOT IN ('cancelled','invoiced').
 * El poller de reconciliación excluye esas ventas; el reintento vía CancelOrder queda
 * condicionado a una fase futura con scopes/arming ampliados (decidido en §4.7/C3).
 *
 * `anulacion_modalidad` en `configuracion_restaurante` (D4):
 *   - 'credito_completo' (default): estado anulada + motivo obligatorio + reversión de
 *     IVA idempotente + exclusión del resumen diario (M10).
 *   - 'estado_solo': solo marca estado anulada (sin reversión contable).
 */

ALTER TABLE ventas
    ADD COLUMN IF NOT EXISTS anulada BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS anulada_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS anulacion_motivo TEXT,
    ADD COLUMN IF NOT EXISTS anulacion_usuario UUID REFERENCES users(id);

CREATE INDEX IF NOT EXISTS idx_ventas_anulada
    ON ventas(anulada) WHERE anulada = true;

ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS anulacion_modalidad VARCHAR(20) NOT NULL DEFAULT 'credito_completo'
        CHECK (anulacion_modalidad IN ('credito_completo', 'estado_solo'));
