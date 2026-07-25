/* [247A-11] Página de albaranes de compra BDP — Fase 1 (solo lectura).
 * Permite consultar y sincronizar albaranes importados desde BDP.
 * No permite crear/modificar albaranes ni tocar inventario. */

import { useMemo, useState } from 'react';
import {
  Search,
  RefreshCw,
  Loader2,
  ArrowLeft,
  AlertCircle,
  Receipt,
} from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { TooltipButton } from '@/components/ui/tooltip-button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { format } from 'date-fns';
import {
  useBdpPurchaseNotes,
  useSyncBdpPurchaseNotes,
} from '@/api/bdp';

function formatDate(value: string | null) {
  if (!value) return '—';
  try {
    return format(new Date(`${value}T00:00:00`), 'dd/MM/yyyy');
  } catch {
    return value;
  }
}

function formatCurrency(value: string | null) {
  if (!value) return '—';
  const n = Number(value);
  if (Number.isNaN(n)) return '—';
  return new Intl.NumberFormat('es-ES', { style: 'currency', currency: 'EUR' }).format(n);
}

function ReadOnlyBanner() {
  return (
    <div className="rounded-lg border bg-muted/50 p-4">
      <div className="flex items-start gap-3">
        <AlertCircle className="size-5 shrink-0 text-muted-foreground" />
        <div>
          <h3 className="text-sm font-medium">Solo lectura</h3>
          <p className="text-sm text-muted-foreground">
            Esta página muestra albaranes de compra importados desde BDP. No se pueden crear ni
            modificar albaranes. El inventario se consulta en BDP y no se altera desde Glory.
          </p>
        </div>
      </div>
    </div>
  );
}

function BdpCompras() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [proveedor, setProveedor] = useState('');
  const [fechaDesde, setFechaDesde] = useState('');
  const [fechaHasta, setFechaHasta] = useState('');
  const [profileCode, setProfileCode] = useState('1');

  const filters = useMemo(
    () => ({
      proveedor: proveedor || undefined,
      fecha_desde: fechaDesde || undefined,
      fecha_hasta: fechaHasta || undefined,
    }),
    [proveedor, fechaDesde, fechaHasta],
  );

  const { data, isLoading, error } = useBdpPurchaseNotes(filters);
  const syncMutation = useSyncBdpPurchaseNotes(queryClient);

  const notes = data ?? [];

  function handleSync() {
    const code = Number(profileCode);
    if (Number.isNaN(code) || code <= 0) {
      toast.error('El perfil de exportación debe ser un número mayor que 0');
      return;
    }
    if (!fechaDesde || !fechaHasta) {
      toast.error('Indica fecha_desde y fecha_hasta para el sync');
      return;
    }
    syncMutation.mutate(
      {
        export_profile_code: code,
        fecha_desde: fechaDesde,
        fecha_hasta: fechaHasta,
      },
      {
        onSuccess: (res) => {
          toast.success(
            `Sync completado: ${res.procesados} albaranes procesados de ${res.total_bdp}`,
          );
        },
        onError: () => {
          toast.error('Error al sincronizar albaranes BDP');
        },
      },
    );
  }

  return (
    <div className="space-y-4 p-4 md:p-6">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="icon" onClick={() => navigate('/configuracion')}>
          <ArrowLeft className="size-4" />
        </Button>
        <h1 className="text-xl font-semibold">Albaranes de compra BDP</h1>
      </div>

      <ReadOnlyBanner />

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Receipt className="size-4" />
            Albaranes importados desde BDP
          </CardTitle>
          <CardDescription>
            Consulta los albaranes de compra sincronizados. Usa Sync para importar un rango de fechas
            desde BDP.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-end flex-wrap">
              <div className="relative w-full sm:w-64">
                <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
                <Input
                  placeholder="Proveedor..."
                  value={proveedor}
                  onChange={(e) => setProveedor(e.target.value)}
                  className="pl-9"
                />
              </div>
              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground">Desde</span>
                <Input
                  type="date"
                  value={fechaDesde}
                  onChange={(e) => setFechaDesde(e.target.value)}
                />
              </div>
              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground">Hasta</span>
                <Input
                  type="date"
                  value={fechaHasta}
                  onChange={(e) => setFechaHasta(e.target.value)}
                />
              </div>
            </div>
            <div className="flex items-center gap-2">
              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground">Perfil exportación</span>
                <Input
                  type="number"
                  min={1}
                  value={profileCode}
                  onChange={(e) => setProfileCode(e.target.value)}
                  className="w-24"
                />
              </div>
              <TooltipButton
                variant="outline"
                onClick={handleSync}
                disabled={syncMutation.isPending}
                tooltip="Importa/actualiza albaranes desde BDP para el rango de fechas indicado. No modifica BDP."
              >
                {syncMutation.isPending ? (
                  <Loader2 className="size-3.5 animate-spin" />
                ) : (
                  <RefreshCw className="size-3.5" />
                )}
                Sync albaranes
              </TooltipButton>
            </div>
          </div>

          {error ? (
            <p className="text-sm text-destructive">
              Error al cargar los albaranes. Revisa que la sesión esté activa y vuelve a intentarlo.
            </p>
          ) : isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="h-10 w-full animate-pulse rounded bg-muted" />
              ))}
            </div>
          ) : notes.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No hay albaranes importados. Selecciona un rango de fechas y pulsa Sync albaranes.
            </p>
          ) : (
            <div className="rounded-md border overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Fecha</TableHead>
                    <TableHead>Serie</TableHead>
                    <TableHead>Número</TableHead>
                    <TableHead>Proveedor</TableHead>
                    <TableHead className="text-right">Total</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {notes.map((note) => (
                    <TableRow key={note.id}>
                      <TableCell className="text-xs">
                        {formatDate(note.fecha)}
                      </TableCell>
                      <TableCell className="font-mono text-xs">{note.serie || '—'}</TableCell>
                      <TableCell className="font-mono text-xs">{note.numero || '—'}</TableCell>
                      <TableCell className="text-xs">
                        {note.nombre_proveedor || note.codigo_proveedor || '—'}
                      </TableCell>
                      <TableCell className="text-right text-xs tabular-nums">
                        {formatCurrency(note.total)}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default BdpCompras;
