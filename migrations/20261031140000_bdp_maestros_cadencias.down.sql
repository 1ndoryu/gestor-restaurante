/* Sin vuelta atrás de los datos: apagar las cadencias es ponerlas a 0. Aquí
 * solo se retiran las columnas. */
ALTER TABLE configuracion_restaurante
    DROP CONSTRAINT IF EXISTS ck_bdp_cadencias_maestros;
ALTER TABLE configuracion_restaurante
    DROP COLUMN IF EXISTS bdp_clientes_auto_segundos,
    DROP COLUMN IF EXISTS bdp_clientes_ultima_lectura,
    DROP COLUMN IF EXISTS bdp_plano_auto_segundos,
    DROP COLUMN IF EXISTS bdp_plano_ultima_lectura,
    DROP COLUMN IF EXISTS bdp_departamentos_auto_segundos,
    DROP COLUMN IF EXISTS bdp_departamentos_ultima_lectura;
