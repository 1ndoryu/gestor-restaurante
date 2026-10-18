/* Formato de albaranes de compra: fechas, importes y etiqueta de estado.
 * Funciones puras de presentación, sin estado, para que la tabla no las redefina. */
import { format } from 'date-fns';
import { CheckCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { BdpPurchaseNote } from '@/api/bdp';

export function formatDate(value: string | null) {
  if (!value) return '—';
  try {
    return format(new Date(`${value}T00:00:00`), 'dd/MM/yyyy');
  } catch {
    return value;
  }
}

export function formatCurrency(value: string | null) {
  if (!value) return '—';
  const n = Number(value);
  if (Number.isNaN(n)) return '—';
  return new Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' }).format(n);
}

export function formatEstado(estado: BdpPurchaseNote['estado']) {
  switch (estado) {
    case 'pendiente':
      return <span className="text-xs text-muted-foreground">Pendiente</span>;
    case 'borrador':
      return <span className="text-xs text-blue-600">Borrador</span>;
    case 'conciliado':
      return (
        <Button variant="outline" size="sm" disabled className="h-7 text-xs pointer-events-none">
          <CheckCircle className="mr-1 size-3" />
          Conciliado
        </Button>
      );
    default:
      return <span className="text-xs text-muted-foreground">—</span>;
  }
}
