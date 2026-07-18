/* [147A-F6] Lógica de envío de ventas extraída de useFormularioVenta.
 * Maneja creación (multi-turno) y edición de ventas. */

import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQueryClient } from '@tanstack/react-query';
import { crearVenta, actualizarVenta, getListarVentasQueryKey, type CrearVentaRequest, type Venta } from '../api/generated';
import type { UseLineasVentaReturn } from './useLineasVenta';
import { calcularIva, type CamposVenta } from './ventaFormUtils';

export function useVentaSubmit(
  campos: CamposVenta,
  esEdicion: boolean,
  ventaInicial: Venta | undefined,
  lineasHook: UseLineasVentaReturn,
  onExito?: () => void,
) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [error, setError] = useState('');
  const [cargando, setCargando] = useState(false);

  function validarLineas(): boolean {
    const sinDatos = lineasHook.lineas.filter(l => !l.descripcion || !l.precio_unitario);
    if (sinDatos.length > 0) { setError('Todas las líneas deben tener descripción y precio'); return false; }
    return true;
  }

  function obtenerImportes(turnoIdx: number) {
    const tieneLineas = lineasHook.lineas.length > 0;
    const d = campos.detalles[campos.turnos[turnoIdx]];
    return {
      importeBase: tieneLineas ? String(lineasHook.totalBase) : d.importeBase,
      importeIva: tieneLineas ? String(lineasHook.totalIva) : calcularIva(d.importeBase, campos.ivaPorcentaje),
      metodoPago: d.metodoPago,
      tieneLineas,
    };
  }

  async function manejarEnvio(e: FormEvent) {
    e.preventDefault();
    setError('');
    if (!campos.fecha) { setError('La fecha es obligatoria'); return; }
    if (lineasHook.lineas.length > 0 && !validarLineas()) return;

    if (esEdicion) {
      const { importeBase, importeIva, metodoPago } = obtenerImportes(0);
      if (!importeBase || importeBase === '0') { setError('El importe es obligatorio'); return; }
      setCargando(true);
      try {
        await actualizarVenta(ventaInicial!.id, {
          fecha: campos.fecha, comensales: campos.comensales ? parseInt(campos.comensales, 10) : null,
          descripcion: campos.descripcion || null, iva_porcentaje: campos.ivaPorcentaje,
          turno: campos.turnos[0], canal: campos.canal, metodo_pago: metodoPago,
          importe_base: importeBase, importe_iva: importeIva,
          lineas: lineasHook.lineas.length > 0 ? lineasHook.lineasRequest : [],
        });
        await queryClient.invalidateQueries({ queryKey: getListarVentasQueryKey() });
        onExito ? onExito() : navigate('/ventas');
      } catch { setError('Error al actualizar la venta'); }
      finally { setCargando(false); }
      return;
    }

    /* Creación multi-turno */
    const sinImporte = lineasHook.lineas.length > 0 ? []
      : campos.turnos.filter(t => !campos.detalles[t].importeBase);
    if (sinImporte.length > 0) { setError('Todos los turnos seleccionados deben tener importe'); return; }
    setCargando(true);
    try {
      await Promise.all(campos.turnos.map(turno => {
        const d = campos.detalles[turno];
        const tieneLineas = lineasHook.lineas.length > 0;
        const req: CrearVentaRequest = {
          fecha: campos.fecha, comensales: campos.comensales ? parseInt(campos.comensales, 10) : null,
          descripcion: campos.descripcion || null, iva_porcentaje: campos.ivaPorcentaje,
          turno, canal: campos.canal, metodo_pago: d.metodoPago,
          importe_base: tieneLineas ? String(lineasHook.totalBase) : d.importeBase,
          importe_iva: tieneLineas ? String(lineasHook.totalIva) : calcularIva(d.importeBase, campos.ivaPorcentaje),
          lineas: tieneLineas ? lineasHook.lineasRequest : undefined,
        };
        return crearVenta(req);
      }));
      await queryClient.invalidateQueries({ queryKey: getListarVentasQueryKey() });
      onExito ? onExito() : navigate('/ventas');
    } catch { setError('Error al registrar la venta'); }
    finally { setCargando(false); }
  }

  return { error, cargando, manejarEnvio };
}
