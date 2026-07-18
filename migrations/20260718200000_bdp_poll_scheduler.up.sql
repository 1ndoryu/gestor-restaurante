ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS bdp_poll_enabled BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS bdp_poll_schedule (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    next_poll_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bdp_poll_schedule_due
    ON bdp_poll_schedule(next_poll_at);
