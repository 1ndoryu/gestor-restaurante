/* [128A-1/F1] Conmutador de modo operativo BDP (independencia total del BDP).
 * auto: decide por credenciales y disponibilidad (default, migración aditiva M15).
 * standalone: nunca se llama a BDP; proveedores locales en todas las pantallas.
 * bdp: fuerza modo BDP; si cae, degrada a standalone con aviso (M2).
 * Los valores de bdp_sync_enabled / bdp_sync_mode solo se interpretan en modo bdp (M1). */
ALTER TABLE configuracion_restaurante
    ADD COLUMN IF NOT EXISTS modo_operacion VARCHAR(10) NOT NULL DEFAULT 'auto'
        CHECK (modo_operacion IN ('auto', 'standalone', 'bdp'));
