/* 253A-10: Hook useFormularioVenta — estado del formulario de ventas.
   253A-14: acepta onExito para uso en modales.
   253A-19: Refactorizado para turnos multi-select y detalles por turno.
   283A-22: soporte edición — acepta ventaInicial para pre-rellenar y usa PUT.
   [147A-F6] Integración multi-item: compone useLineasVenta + useVentaSubmit.
   Refactorizado para cumplir límite de 120 líneas (utils + submit extraídos). */

import { useState, useEffect } from 'react';
import type { Turno, MetodoPago, Venta, VentaLinea } from '../api/generated';
import { useObtenerConfiguracion } from '../api/generated/configuracion/configuracion';
import { customInstance } from '../api/axios-instance';
import useLineasVenta from './useLineasVenta';
import { useVentaSubmit } from './useVentaSubmit';
import { camposIniciales, type CamposVenta, type DetalleTurno } from './ventaFormUtils';

export type { DetalleTurno } from './ventaFormUtils';
export { calcularIva } from './ventaFormUtils';

function useFormularioVenta(onExito?: () => void, ventaInicial?: Venta) {
  const [campos, setCampos] = useState<CamposVenta>(() => camposIniciales(ventaInicial));
  const esEdicion = !!ventaInicial;
  const lineasHook = useLineasVenta(campos.ivaPorcentaje);
  const { error, cargando, manejarEnvio } = useVentaSubmit(campos, esEdicion, ventaInicial, lineasHook, onExito);

  const { data: configData } = useObtenerConfiguracion();
  useEffect(() => {
    if (!esEdicion && configData?.status === 200 && configData.data.iva_por_defecto) {
      setCampos(prev => ({ ...prev, ivaPorcentaje: String(configData.data.iva_por_defecto) }));
    }
  }, [configData, esEdicion]);

  useEffect(() => {
    if (!ventaInicial) return;
    let activo = true;
    void customInstance<{ data: VentaLinea[] }>(`/api/ventas/${ventaInicial.id}/lineas`, {
      method: 'GET',
    }).then((respuesta) => {
      if (!activo || respuesta.data.length === 0) return;
      lineasHook.setLineasDesdeRequest(respuesta.data.map((linea) => ({
        articulo_codigo: linea.articulo_codigo || null,
        descripcion: linea.descripcion,
        cantidad: linea.cantidad,
        precio_unitario: linea.precio_unitario,
        iva_pct: linea.iva_pct,
        descuento: linea.descuento,
      })));
    });
    return () => { activo = false; };
  }, [ventaInicial?.id, lineasHook.setLineasDesdeRequest]);

  function cambiarCampo<K extends keyof CamposVenta>(campo: K, valor: CamposVenta[K]) {
    setCampos(prev => ({ ...prev, [campo]: valor }));
  }

  function toggleTurno(turno: Turno) {
    if (esEdicion) return;
    setCampos(prev => {
      const nuevos = prev.turnos.includes(turno) ? prev.turnos.filter(t => t !== turno) : [...prev.turnos, turno];
      return nuevos.length > 0 ? { ...prev, turnos: nuevos } : prev;
    });
  }

  function cambiarDetalle(turno: Turno, campo: keyof DetalleTurno, valor: string | MetodoPago) {
    setCampos(prev => ({
      ...prev,
      detalles: { ...prev.detalles, [turno]: { ...prev.detalles[turno], [campo]: valor } },
    }));
  }

  return { campos, cambiarCampo, toggleTurno, cambiarDetalle, error, manejarEnvio, cargando, esEdicion, lineasHook };
}

export default useFormularioVenta;
