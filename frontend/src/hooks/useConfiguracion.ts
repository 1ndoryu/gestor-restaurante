/* [263A-17] Hook para el formulario de configuración del restaurante.
 * [147A-F5.9] Refactorizado: tipos a configuracion-types.ts, sync a useConfiguracionSync.ts.
 * Cumple límite de 120 líneas (Regla 8). */

import { useState, useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  useObtenerConfiguracion,
  useActualizarConfiguracion,
  getObtenerConfiguracionQueryKey,
  getObtenerModoOperacionQueryKey,
} from '../api/generated/configuracion/configuracion';
import { useConfiguracionSync } from './useConfiguracionSync';
import type { CuerpoConfiguracionLocal, EstadoConfiguracion } from './configuracion-types';
import type { IdSeccion } from '../componentes/configuracion/secciones';

export type { EstadoConfiguracion };

/* [10AA-3] El mapa de IVA se edita como texto: un JSON mal escrito no debe tumbar
 * el guardado entero. Si no parsea se envía vacío y el backend conserva el suyo. */
const MENSAJES_VACIOS: Record<IdSeccion, string> = {
  restaurante: '', operacion: '', conexiones: '', chatbot: '', acceso: '', avanzado: '',
};

function mapaNumerico(texto: string): Record<string, number> {
  try {
    const valor: unknown = JSON.parse(texto);
    if (typeof valor !== 'object' || valor === null) return {};
    return Object.fromEntries(
      Object.entries(valor as Record<string, unknown>)
        .filter(([, codigo]) => typeof codigo === 'number'),
    ) as Record<string, number>;
  } catch {
    return {};
  }
}

export function useConfiguracion() {
  const queryClient = useQueryClient();
  const { data: datos, isLoading } = useObtenerConfiguracion();
  const mutacion = useActualizarConfiguracion();
  /* [D33] Un mensaje por grupo: el «Configuración guardada» único aparecía en
   * todas las pestañas a la vez y no decía qué se había guardado. */
  const [mensajes, setMensajes] = useState<Record<IdSeccion, string>>(MENSAJES_VACIOS);
  const [guardandoEn, setGuardandoEn] = useState<IdSeccion | null>(null);

  /* [BKP-008c] Memoize server data to prevent infinite re-render loop.
   * The object literal { status: 200, data: datos.data } was creating a new
   * reference on every render, triggering useEffect in useConfiguracionSync
   * which called setConfig → re-render → new object → infinite loop. */
  const serverData = useMemo(
    () => (datos?.status === 200 ? { status: 200, data: datos.data as unknown as Record<string, string | number | boolean> } : undefined),
    [datos],
  );
  const { config, setConfig, referencia } = useConfiguracionSync(serverData);

  const cambiarCampo = useCallback(
    <K extends keyof EstadoConfiguracion>(campo: K, valor: EstadoConfiguracion[K]) => {
      setConfig((prev) => ({ ...prev, [campo]: valor }));
    },
    [setConfig],
  );

  const guardar = useCallback(async (seccion: IdSeccion = 'restaurante') => {
    setMensajes((previos) => ({ ...previos, [seccion]: '' }));
    setGuardandoEn(seccion);
    const body: CuerpoConfiguracionLocal = {
      reserva_email_obligatorio: config.reserva_email_obligatorio,
      reserva_telefono_obligatorio: config.reserva_telefono_obligatorio,
      reserva_nombre_obligatorio: config.reserva_nombre_obligatorio,
      reserva_apellidos_obligatorio: config.reserva_apellidos_obligatorio,
      iva_por_defecto: String(config.iva_por_defecto),
      nombre_restaurante: config.nombre_restaurante,
      ...(config.groq_api_key ? { groq_api_key: config.groq_api_key } : {}),
      auto_venta_reserva: config.auto_venta_reserva,
      hora_desayuno_inicio: config.hora_desayuno_inicio,
      hora_desayuno_fin: config.hora_desayuno_fin,
      hora_comida_inicio: config.hora_comida_inicio,
      hora_comida_fin: config.hora_comida_fin,
      hora_cena_inicio: config.hora_cena_inicio,
      hora_cena_fin: config.hora_cena_fin,
      url_haddock: config.url_haddock || undefined,
      ...(config.haddock_api_token ? { haddock_api_token: config.haddock_api_token } : {}),
      haddock_sync_enabled: config.haddock_sync_enabled,
      bdp_base_url: config.bdp_base_url || undefined,
      ...(config.bdp_login ? { bdp_login: config.bdp_login } : {}),
      ...(config.bdp_password ? { bdp_password: config.bdp_password } : {}),
      ...(config.bdp_integrator_code ? { bdp_integrator_code: config.bdp_integrator_code } : {}),
      bdp_sync_enabled: config.bdp_sync_enabled,
      bdp_pos_id: config.bdp_pos_id,
      bdp_employee_id: config.bdp_employee_id,
      bdp_items_profile_id: config.bdp_items_profile_id,
      bdp_default_article_code: config.bdp_default_article_code || undefined,
      bdp_default_article_name: config.bdp_default_article_name || undefined,
      bdp_tender_map: mapaNumerico(config.bdp_tender_map),
      bdp_order_type_map: mapaNumerico(config.bdp_order_type_map),
      bdp_default_customer_code: config.bdp_default_customer_code || undefined,
      bdp_poll_interval_secs: config.bdp_poll_interval_secs,
      bdp_poll_enabled: config.bdp_poll_enabled,
      bdp_auto_sync_customers: config.bdp_auto_sync_customers,
      /* [267A-4] Feature flags BDP — se envían al backend para persistir */
      ff_bdp_auto_arm: config.ff_bdp_auto_arm,
      ff_bdp_partial_payments: config.ff_bdp_partial_payments,
      ff_bdp_cancel_order: config.ff_bdp_cancel_order,
      ff_bdp_purchase_notes_read: config.ff_bdp_purchase_notes_read,
      ff_bdp_purchase_notes_draft: config.ff_bdp_purchase_notes_draft,
      ff_bdp_purchase_notes_receive: config.ff_bdp_purchase_notes_receive,
      /* [128A-1/F4] Modalidad de anulación local de ventas */
      anulacion_modalidad: config.anulacion_modalidad,
      /* [128A-1/F8] Permisos operativos por acción (D8/M17) */
      permisos_catalogo_edicion: config.permisos_catalogo_edicion,
      permisos_stock_ajuste: config.permisos_stock_ajuste,
      permisos_albaranes_gestion: config.permisos_albaranes_gestion,
      permisos_anulacion_ventas: config.permisos_anulacion_ventas,
      permisos_pagos_locales: config.permisos_pagos_locales,
      permisos_facturacion_local: config.permisos_facturacion_local,
      /* [10AA-3] Parámetros de las escrituras a BDP (D1/D3/M13). */
      push_modalidad: config.push_modalidad,
      bdp_tav_map: mapaNumerico(config.bdp_tav_map),
      bdp_almacen_default: config.bdp_almacen_default,
      bdp_codreg_default: config.bdp_codreg_default,
      bdp_articulo_rango_inicial: config.bdp_articulo_rango_inicial,
      bdp_documentos_auto_segundos: Number(config.bdp_documentos_auto_segundos) || 0,
      ...(config.bdp_documentos_perfil ? { bdp_documentos_perfil: Number(config.bdp_documentos_perfil) } : {}),
      google_review_url: config.google_review_url || undefined,
      telefono_restaurante: config.telefono_restaurante || undefined,
      url_reservas: config.url_reservas || undefined,
    };
    try {
      await mutacion.mutateAsync({ data: body });
      await queryClient.invalidateQueries({ queryKey: getObtenerConfiguracionQueryKey() });
      /* [149A-1/P1.4] El guardado puede tocar campos que afectan al modo efectivo
       * (p. ej. bdp_sync_enabled): sin invalidar la query de modo, el badge quedaba
       * desfasado hasta recargar. Mismo patrón que site-header en sus cambios. */
      await queryClient.invalidateQueries({ queryKey: getObtenerModoOperacionQueryKey() });
      setMensajes((previos) => ({ ...previos, [seccion]: 'Guardado' }));
    } catch {
      setMensajes((previos) => ({ ...previos, [seccion]: 'Error al guardar' }));
    } finally {
      setGuardandoEn(null);
    }
  }, [config, mutacion, queryClient]);

  return {
    config, referencia, cambiarCampo, guardar, mensajes,
    cargando: isLoading, guardandoEn,
  };
}
