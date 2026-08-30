/* [247A-11] Filtros de la página de albaranes de compra BDP (proveedor,
 * fecha desde/hasta), extraídos de BdpCompras a un hook custom para mantener
 * el componente bajo el máximo de useState (protocolo usestate-excesivo).
 * Estado puro sin efecto sobre queries; el derivado `filtros` se consume en
 * el componente. */
import { useMemo, useState } from 'react';
import type { BdpPurchaseNoteFilters } from '@/api/bdp';

export function useFiltrosAlbaranes() {
  const [proveedor, setProveedor] = useState('');
  const [fechaDesde, setFechaDesde] = useState('');
  const [fechaHasta, setFechaHasta] = useState('');

  const filtros = useMemo<BdpPurchaseNoteFilters>(
    () => ({
      proveedor: proveedor || undefined,
      fecha_desde: fechaDesde || undefined,
      fecha_hasta: fechaHasta || undefined,
    }),
    [proveedor, fechaDesde, fechaHasta],
  );

  return { proveedor, setProveedor, fechaDesde, setFechaDesde, fechaHasta, setFechaHasta, filtros };
}