/* [147A-F5.9] Sub-hook extraído de useConfiguracion — sincronización servidor→local.
 * Cumple límite de 120 líneas (Regla 8). */

import { useState, useEffect } from 'react';
import { DEFAULTS } from './configuracion-types';
import type { EstadoConfiguracion } from './configuracion-types';

interface ConfigServerData {
  status: number;
  data: Record<string, string | number | boolean>;
}

export function useConfiguracionSync(datos: ConfigServerData | undefined) {
  const [config, setConfig] = useState<EstadoConfiguracion>(DEFAULTS);
  /* [D33] Última copia que llegó del servidor: sin ella no se puede saber qué
   * campos están cambiados y el aviso «sin guardar» sería inventado. */
  const [referencia, setReferencia] = useState<EstadoConfiguracion | undefined>(undefined);

  useEffect(() => {
    if (!datos || datos.status !== 200) return;
    const d = datos.data;
    const siguiente: EstadoConfiguracion = {
      reserva_email_obligatorio: Boolean(d.reserva_email_obligatorio),
      reserva_telefono_obligatorio: Boolean(d.reserva_telefono_obligatorio),
      reserva_nombre_obligatorio: Boolean(d.reserva_nombre_obligatorio),
      reserva_apellidos_obligatorio: Boolean(d.reserva_apellidos_obligatorio),
      iva_por_defecto: Number(d.iva_por_defecto),
      nombre_restaurante: String(d.nombre_restaurante ?? ''),
      groq_api_key: config.groq_api_key || '',
      auto_venta_reserva: Boolean(d.auto_venta_reserva),
      hora_desayuno_inicio: String(d.hora_desayuno_inicio ?? '00:00:00'),
      hora_desayuno_fin: String(d.hora_desayuno_fin ?? '12:00:00'),
      hora_comida_inicio: String(d.hora_comida_inicio ?? '12:00:00'),
      hora_comida_fin: String(d.hora_comida_fin ?? '18:00:00'),
      hora_cena_inicio: String(d.hora_cena_inicio ?? '18:00:00'),
      hora_cena_fin: String(d.hora_cena_fin ?? '23:59:59'),
      url_haddock: String(d.url_haddock ?? ''),
      haddock_api_token: config.haddock_api_token || '',
      haddock_sync_enabled: Boolean(d.haddock_sync_enabled),
      bdp_base_url: String(d.bdp_base_url ?? ''),
      bdp_login: config.bdp_login || '',
      bdp_password: config.bdp_password || '',
      bdp_integrator_code: config.bdp_integrator_code || '',
      bdp_configurado: Boolean(d.bdp_configurado),
      bdp_sync_enabled: Boolean(d.bdp_sync_enabled ?? false),
      bdp_pos_id: Number(d.bdp_pos_id ?? 1),
      bdp_employee_id: Number(d.bdp_employee_id ?? 1),
      bdp_items_profile_id: Number(d.bdp_items_profile_id ?? 1),
      bdp_default_article_code: String(d.bdp_default_article_code ?? ''),
      bdp_default_article_name: String(d.bdp_default_article_name ?? ''),
      bdp_tender_map: typeof d.bdp_tender_map === 'object' && d.bdp_tender_map !== null
        ? JSON.stringify(d.bdp_tender_map, null, 2)
        : String(d.bdp_tender_map ?? '{}'),
      bdp_order_type_map: typeof d.bdp_order_type_map === 'object' && d.bdp_order_type_map !== null
        ? JSON.stringify(d.bdp_order_type_map, null, 2)
        : String(d.bdp_order_type_map ?? '{}'),
      bdp_default_customer_code: String(d.bdp_default_customer_code ?? ''),
      bdp_poll_interval_secs: Number(d.bdp_poll_interval_secs ?? 60),
      bdp_poll_enabled: Boolean(d.bdp_poll_enabled ?? false),
      bdp_auto_sync_customers: Boolean(d.bdp_auto_sync_customers ?? false),
      /* [267A-4] Feature flags BDP — sincronizados desde servidor */
      ff_bdp_auto_arm: Boolean(d.ff_bdp_auto_arm ?? false),
      ff_bdp_partial_payments: Boolean(d.ff_bdp_partial_payments ?? false),
      ff_bdp_cancel_order: Boolean(d.ff_bdp_cancel_order ?? false),
      ff_bdp_purchase_notes_read: Boolean(d.ff_bdp_purchase_notes_read ?? false),
      ff_bdp_purchase_notes_draft: Boolean(d.ff_bdp_purchase_notes_draft ?? false),
      ff_bdp_purchase_notes_receive: Boolean(d.ff_bdp_purchase_notes_receive ?? false),
      modo_operacion: String(d.modo_operacion ?? 'auto'),
      anulacion_modalidad: String(d.anulacion_modalidad ?? 'credito_completo'),
      /* [128A-1/F8] Permisos operativos por acción (D8/M17) */
      permisos_catalogo_edicion: String(d.permisos_catalogo_edicion ?? 'admin'),
      permisos_stock_ajuste: String(d.permisos_stock_ajuste ?? 'admin'),
      permisos_albaranes_gestion: String(d.permisos_albaranes_gestion ?? 'admin'),
      permisos_anulacion_ventas: String(d.permisos_anulacion_ventas ?? 'admin'),
      permisos_pagos_locales: String(d.permisos_pagos_locales ?? 'admin'),
      permisos_facturacion_local: String(d.permisos_facturacion_local ?? 'admin'),
      bdp_sync_mode: String(d.bdp_sync_mode ?? 'read_only'),
      bdp_backup_retention_days: Number(d.bdp_backup_retention_days ?? 30),
      bdp_auto_backup_before_write: Boolean(d.bdp_auto_backup_before_write ?? false),
      /* [10AA-3] Parámetros de escritura BDP. El mapa de IVA llega como objeto
       * JSONB y se edita como texto, igual que los mapas de tender y tipo. */
      push_modalidad: String(d.push_modalidad ?? 'automatico'),
      bdp_tav_map: typeof d.bdp_tav_map === 'object' && d.bdp_tav_map !== null
        ? JSON.stringify(d.bdp_tav_map, null, 2)
        : String(d.bdp_tav_map ?? '{}'),
      bdp_almacen_default: Number(d.bdp_almacen_default ?? 1),
      bdp_codreg_default: Number(d.bdp_codreg_default ?? 1),
      bdp_articulo_rango_inicial: Number(d.bdp_articulo_rango_inicial ?? 90000000),
      bdp_documentos_auto_segundos: Number(d.bdp_documentos_auto_segundos ?? 0),
      bdp_documentos_perfil: d.bdp_documentos_perfil ? Number(d.bdp_documentos_perfil) : null,
      bdp_maestros_auto_segundos: Number(d.bdp_maestros_auto_segundos ?? 0),
      bdp_clientes_auto_segundos: Number(d.bdp_clientes_auto_segundos ?? 0),
      bdp_plano_auto_segundos: Number(d.bdp_plano_auto_segundos ?? 0),
      bdp_departamentos_auto_segundos: Number(d.bdp_departamentos_auto_segundos ?? 0),
      google_review_url: String(d.google_review_url ?? ''),
      telefono_restaurante: String(d.telefono_restaurante ?? ''),
      url_reservas: String(d.url_reservas ?? ''),
    };
    setConfig(siguiente);
    setReferencia(siguiente);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [datos]);

  return { config, setConfig, referencia };
}
