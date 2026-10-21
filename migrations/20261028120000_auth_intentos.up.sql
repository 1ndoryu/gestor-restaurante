/* [D02] Registro de intentos de acceso al terminal: quién lo intentó, si entró y
 * por qué no. Nunca guarda la contraseña ni el hash; el email es el identificador
 * con el que se intentó entrar. Sirve para la evidencia de D02 («log de intentos»)
 * y para ver un ataque de fuerza bruta después de que el limitador lo frene. */
CREATE TABLE IF NOT EXISTS auth_intentos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    email TEXT NOT NULL,
    exito BOOLEAN NOT NULL,
    motivo TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auth_intentos_created
    ON auth_intentos (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_auth_intentos_email
    ON auth_intentos (lower(email), created_at DESC);
