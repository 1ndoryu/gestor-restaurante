/* [169A-3] La clave gruesa histórica "marketing" se sustituye por grano por
 * entrada de menú (campanas, plantillas_wa, recordatorios). Los trabajadores
 * que la tuvieran concedida conservan las tres hijas; luego se borra la
 * clave vieja. Idempotente (no falla si ya no hay filas "marketing"). */
INSERT INTO permisos_trabajador (id, trabajador_id, seccion, permitido)
SELECT gen_random_uuid(), trabajador_id, hija, true
FROM permisos_trabajador
CROSS JOIN (VALUES ('campanas'), ('plantillas_wa'), ('recordatorios')) AS h(hija)
WHERE seccion = 'marketing' AND permitido = true
ON CONFLICT DO NOTHING;

DELETE FROM permisos_trabajador WHERE seccion = 'marketing';
