/* Estado BDP de una venta: mapeo a la etiqueta que muestra la UI. */

import type { BdpStatus } from './tipos';

export function mapBdpStatus(
  bdpSynced?: boolean,
  bdpOrderStatus?: string | null,
  bdpSyncError?: string | null,
): BdpStatus {
  if (!bdpSynced) return 'none';
  if (bdpSyncError) return 'error';
  if (!bdpOrderStatus) return 'pending';
  switch (bdpOrderStatus) {
    case 'pending': return 'pending';
    case 'accepted': return 'accepted';
    case 'cancelled': return 'cancelled';
    case 'invoiced': return 'invoiced';
    default: return 'pending';
  }
}
