/* [247A-12] Fases 2 y 3 de compras BDP: ciclo de vida local del albarán.
 * estado: pendiente | borrador | conciliado
 * gasto_id: vínculo opcional con el gasto Glory generado por la reconciliación.
 * ON DELETE SET NULL: si se borra el gasto, el albarán queda como conciliado
 * huérfano; un job o trigger puede revertirlo a borrador. */

ALTER TABLE bdp_purchase_notes
ADD COLUMN IF NOT EXISTS estado VARCHAR(20) NOT NULL DEFAULT 'pendiente',
ADD COLUMN IF NOT EXISTS gasto_id UUID REFERENCES gastos(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_bdp_purchase_notes_estado ON bdp_purchase_notes(user_id, estado);
CREATE INDEX IF NOT EXISTS idx_bdp_purchase_notes_gasto ON bdp_purchase_notes(gasto_id);
