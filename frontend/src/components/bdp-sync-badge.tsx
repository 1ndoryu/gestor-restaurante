/* [147A-F5.1] Badge de estado BDP WebLink — equivalente a HaddockSyncBadge.
 * Muestra estado visual (synced/error/pendiente/cancelled/invoiced) con tooltip.
 * Usa los mismos tokens de color del sistema de diseño (variables CSS). */

import { Check, X, Minus, Clock, Ban, FileText } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import type { BdpStatus } from '../api/bdp';

interface Props {
  /** true si la venta fue enviada a BDP */
  synced: boolean;
  /** Estado de la orden en BDP (pending/accepted/cancelled/invoiced) */
  orderStatus?: string | null;
  /** Error de sincronización, si lo hubo */
  syncError?: string | null;
  /** ID de la orden en BDP (para tooltip) */
  orderId?: string | null;
}

function getDisplayStatus(synced: boolean, orderStatus?: string | null, syncError?: string | null): BdpStatus {
  if (syncError) return 'error';
  if (!synced) return 'none';
  if (!orderStatus) return 'pending';
  switch (orderStatus) {
    case 'pending': return 'pending';
    case 'accepted': return 'accepted';
    case 'cancelled': return 'cancelled';
    case 'invoiced': return 'invoiced';
    default: return 'pending';
  }
}

function BdpSyncBadge({ synced, orderStatus, syncError, orderId }: Props) {
  const status = getDisplayStatus(synced, orderStatus, syncError);

  const iconMap = {
    none: <Minus className="size-3" />,
    pending: <Clock className="size-3" />,
    accepted: <Check className="size-3" />,
    cancelled: <Ban className="size-3" />,
    invoiced: <FileText className="size-3" />,
    error: <X className="size-3" />,
  };

  const variantMap: Record<BdpStatus, string> = {
    none: 'text-muted-foreground',
    pending: 'bg-amber-50 text-amber-700 border-amber-200',
    accepted: 'bg-green-50 text-green-700 border-green-200',
    cancelled: 'bg-red-50 text-red-700 border-red-200',
    invoiced: 'bg-blue-50 text-blue-700 border-blue-200',
    error: 'bg-red-50 text-red-700 border-red-200',
  };

  const labelMap: Record<BdpStatus, string> = {
    none: 'No sincronizada',
    pending: 'Esperando validación',
    accepted: 'Aceptada',
    cancelled: 'Cancelada',
    invoiced: 'Facturada',
    error: `Error: ${syncError ?? 'desconocido'}`,
  };

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Badge variant="outline" className={`${variantMap[status]} text-xs`}>
            {iconMap[status]}
          </Badge>
        </TooltipTrigger>
        <TooltipContent side="left" className="max-w-xs">
          <div className="flex flex-col gap-1">
            <span>{labelMap[status]}</span>
            {orderId && (
              <span className="text-xs text-muted-foreground">Orden: {orderId}</span>
            )}
          </div>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

export default BdpSyncBadge;
