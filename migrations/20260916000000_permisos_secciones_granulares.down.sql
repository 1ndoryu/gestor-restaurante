/* [169A-3] Reversión: re-agrupa las hijas en la clave gruesa "marketing". */
INSERT INTO permisos_trabajador (id, trabajador_id, seccion, permitido)
SELECT gen_random_uuid(), trabajador_id, 'marketing', true
FROM permisos_trabajador
WHERE seccion IN ('campanas', 'plantillas_wa', 'recordatorios') AND permitido = true
GROUP BY trabajador_id
HAVING COUNT(*) = 3
ON CONFLICT DO NOTHING;

DELETE FROM permisos_trabajador
WHERE seccion IN ('campanas', 'plantillas_wa', 'recordatorios');
