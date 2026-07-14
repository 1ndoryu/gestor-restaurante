/* [147A-F6] Hook para gestionar líneas de venta multi-item.
 * Extraído de useFormularioVenta para cumplir límite de 120 líneas.
 * Cada línea representa un artículo/servicio individual que se mapea a BDP. */

import { useState, useCallback, useMemo } from 'react';
import type { CrearVentaLineaRequest } from '../api/generated';

export interface LineaVentaLocal {
  id: string; /* ID local (uuid temporal para React keys) */
  articulo_codigo: string;
  descripcion: string;
  cantidad: string;
  precio_unitario: string;
  iva_pct: string;
  descuento: string;
}

const nuevaLineaId = () => Math.random().toString(36).slice(2, 10);

const lineaVacia = (): LineaVentaLocal => ({
  id: nuevaLineaId(),
  articulo_codigo: '',
  descripcion: '',
  cantidad: '1',
  precio_unitario: '',
  iva_pct: '10',
  descuento: '0',
});

export interface UseLineasVentaReturn {
  lineas: LineaVentaLocal[];
  agregarLinea: () => void;
  eliminarLinea: (id: string) => void;
  actualizarLinea: (id: string, campo: keyof Omit<LineaVentaLocal, 'id'>, valor: string) => void;
  totalBase: number;
  totalIva: number;
  totalConDescuento: number;
  lineasRequest: CrearVentaLineaRequest[];
  setLineasDesdeRequest: (lineas: CrearVentaLineaRequest[]) => void;
}

function calcularTotales(lineas: LineaVentaLocal[]) {
  let b = 0, i = 0, t = 0;
  for (const l of lineas) {
    const qty = parseFloat(l.cantidad) || 0;
    const precio = parseFloat(l.precio_unitario) || 0;
    const base = qty * precio * (1 - (parseFloat(l.descuento) || 0) / 100);
    const iva = base * ((parseFloat(l.iva_pct) || 0) / 100);
    b += base; i += iva; t += base + iva;
  }
  return { totalBase: Math.round(b * 100) / 100, totalIva: Math.round(i * 100) / 100, totalConDescuento: Math.round(t * 100) / 100 };
}

export default function useLineasVenta(ivaDefault = '10'): UseLineasVentaReturn {
  const [lineas, setLineas] = useState<LineaVentaLocal[]>(() => []);

  const agregarLinea = useCallback(() => {
    setLineas(prev => [...prev, { ...lineaVacia(), iva_pct: ivaDefault }]);
  }, [ivaDefault]);

  const eliminarLinea = useCallback((id: string) => {
    setLineas(prev => prev.filter(l => l.id !== id));
  }, []);

  const actualizarLinea = useCallback(
    (id: string, campo: keyof Omit<LineaVentaLocal, 'id'>, valor: string) => {
      setLineas(prev => prev.map(l => (l.id === id ? { ...l, [campo]: valor } : l)));
    },
    [],
  );

  const { totalBase, totalIva, totalConDescuento } = useMemo(
    () => calcularTotales(lineas),
    [lineas],
  );

  const lineasRequest: CrearVentaLineaRequest[] = useMemo(
    () =>
      lineas.map(l => ({
        articulo_codigo: l.articulo_codigo || null,
        descripcion: l.descripcion,
        cantidad: l.cantidad || null,
        precio_unitario: l.precio_unitario,
        iva_pct: l.iva_pct || null,
        descuento: l.descuento && l.descuento !== '0' ? l.descuento : null,
      })),
    [lineas],
  );

  const setLineasDesdeRequest = useCallback((reqs: CrearVentaLineaRequest[]) => {
    setLineas(
      reqs.map(r => ({
        id: nuevaLineaId(),
        articulo_codigo: r.articulo_codigo || '',
        descripcion: r.descripcion,
        cantidad: r.cantidad?.toString() || '1',
        precio_unitario: r.precio_unitario,
        iva_pct: r.iva_pct || '10',
        descuento: r.descuento || '0',
      })),
    );
  }, []);

  return { lineas, agregarLinea, eliminarLinea, actualizarLinea, totalBase, totalIva, totalConDescuento, lineasRequest, setLineasDesdeRequest };
}
