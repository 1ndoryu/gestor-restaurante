/* [D33] Las seis pestañas de Configuración, agrupadas por la pregunta que se
 * hace la usuaria y no por módulo técnico. Cada sección declara sus campos: así
 * el aviso «cambios sin guardar» aparece solo donde de verdad hay cambios, y no
 * en las cinco pestañas a la vez. */

import type { EstadoConfiguracion } from '../../hooks/configuracion-types';

export type IdSeccion = 'restaurante' | 'operacion' | 'conexiones' | 'chatbot' | 'acceso' | 'avanzado';

interface DefinicionSeccion {
  id: IdSeccion;
  titulo: string;
  descripcion: string;
  /** Campos que guarda el botón de esta sección. */
  campos: readonly (keyof EstadoConfiguracion)[];
}

/* Todo lo de BDP comparte prefijo, pero se lista para que quitar un campo del
 * formulario obligue a actualizar la sección. */
const CAMPOS_BDP = [
  'bdp_base_url', 'bdp_login', 'bdp_password', 'bdp_integrator_code', 'bdp_sync_enabled',
  'bdp_pos_id', 'bdp_employee_id', 'bdp_items_profile_id', 'bdp_default_article_code',
  'bdp_default_article_name', 'bdp_tender_map', 'bdp_order_type_map', 'bdp_default_customer_code',
  'bdp_poll_interval_secs', 'bdp_poll_enabled', 'bdp_auto_sync_customers', 'bdp_sync_mode',
  'bdp_backup_retention_days', 'bdp_auto_backup_before_write',
  'ff_bdp_auto_arm', 'ff_bdp_partial_payments', 'ff_bdp_cancel_order',
  'ff_bdp_purchase_notes_read', 'ff_bdp_purchase_notes_draft', 'ff_bdp_purchase_notes_receive',
  'push_modalidad', 'bdp_tav_map', 'bdp_almacen_default', 'bdp_codreg_default',
  'bdp_articulo_rango_inicial', 'bdp_maestros_auto_segundos',
  'bdp_clientes_auto_segundos', 'bdp_plano_auto_segundos', 'bdp_departamentos_auto_segundos',
] as const satisfies readonly (keyof EstadoConfiguracion)[];

export const SECCIONES: readonly DefinicionSeccion[] = [
  {
    id: 'restaurante',
    titulo: 'Mi restaurante',
    descripcion: 'Cómo se llama, cuánto IVA aplica y qué datos pide al reservar',
    campos: [
      'nombre_restaurante', 'iva_por_defecto',
      'reserva_nombre_obligatorio', 'reserva_apellidos_obligatorio',
      'reserva_email_obligatorio', 'reserva_telefono_obligatorio',
      'hora_desayuno_inicio', 'hora_desayuno_fin', 'hora_comida_inicio', 'hora_comida_fin',
      'hora_cena_inicio', 'hora_cena_fin',
    ],
  },
  {
    id: 'operacion',
    titulo: 'Cómo cobro y opero',
    descripcion: 'Turnos de venta, anulaciones y lo que se sincroniza con Haddock',
    campos: [
      'auto_venta_reserva', 'anulacion_modalidad',
      'url_haddock', 'haddock_api_token', 'haddock_sync_enabled',
      'google_review_url', 'telefono_restaurante', 'url_reservas',
    ],
  },
  {
    id: 'conexiones',
    titulo: 'A qué se conecta',
    descripcion: 'TPV BDP, chatbot, marketing y las claves de otros servicios',
    campos: ['groq_api_key', ...CAMPOS_BDP],
  },
  {
    id: 'chatbot',
    titulo: 'Chatbot',
    descripcion: 'Las claves con las que un chatbot externo entra a los datos',
    campos: [],
  },
  {
    id: 'acceso',
    titulo: 'Quién entra',
    descripcion: 'Qué puede hacer cada rol y cómo se protege el terminal',
    campos: [
      'permisos_catalogo_edicion', 'permisos_stock_ajuste', 'permisos_albaranes_gestion',
      'permisos_anulacion_ventas', 'permisos_pagos_locales', 'permisos_facturacion_local',
    ],
  },
  {
    id: 'avanzado',
    titulo: 'Avanzado',
    descripcion: 'Respaldos del TPV, avisos recibidos y datos de prueba',
    campos: [],
  },
];
