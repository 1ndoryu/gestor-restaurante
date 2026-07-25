/* [BDP-STOCK-01] Página de stock BDP.
 * Vista unificada y solo lectura del stock proveniente del catálogo BDP.
 * Permite filtrar por código/nombre y refrescar con sync catálogo. */

import { useState, useMemo } from 'react';
import { Search, RefreshCw, Loader2, Package, ArrowLeft } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps, useSyncCatalog } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { TooltipButton } from '@/components/ui/tooltip-button';

function BdpStock() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [filtro, setFiltro] = useState('');
  const { data, isLoading, error: listError } = useListarArticleMaps();
  const syncCatalogMutation = useSyncCatalog({
    mutation: {
      onSuccess: (resp) => {
        const d = resp as unknown as { creados?: number; actualizados?: number };
        toast.success(`Sync completado: ${d.creados ?? 0} nuevos, ${d.actualizados ?? 0} actualizados`);
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al sincronizar catálogo BDP'),
    },
  });

  const mapeos = data?.status === 200 ? data.data : [];

  const filtrados = useMemo(() => {
    const q = filtro.trim().toLowerCase();
    if (!q) return mapeos;
    return mapeos.filter(
      (m) =>
        m.articulo_glory_codigo?.toLowerCase().includes(q) ||
        m.articulo_bdp_codigo?.toLowerCase().includes(q) ||
        m.articulo_bdp_nombre?.toLowerCase().includes(q),
    );
  }, [mapeos, filtro]);

  return (
    <div className="space-y-4 p-4 md:p-6">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="icon" onClick={() => navigate('/configuracion')}>
          <ArrowLeft className="size-4" />
        </Button>
        <h1 className="text-xl font-semibold">Stock BDP</h1>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Package className="size-4" />
            Stock de artículos BDP
          </CardTitle>
          <CardDescription>
            Vista solo lectura del stock actual en BDP. Para modificarlo, hazlo directamente en el TPV/BDP.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="relative w-full sm:w-96">
              <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
              <Input
                placeholder="Filtrar por código o nombre..."
                value={filtro}
                onChange={(e) => setFiltro(e.target.value)}
                className="pl-9"
              />
            </div>
            <TooltipButton
              variant="outline"
              onClick={() => syncCatalogMutation.mutate()}
              disabled={syncCatalogMutation.isPending}
              tooltip="Importa/actualiza artículos y stock desde BDP a Glory."
            >
              {syncCatalogMutation.isPending ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
              Sync catálogo
            </TooltipButton>
          </div>

          {listError ? (
            <p className="text-sm text-destructive">
              Error al cargar el stock. Revisa que la sesión esté activa y vuelve a intentarlo.
            </p>
          ) : isLoading ? (
            <p className="text-sm text-muted-foreground">Cargando stock...</p>
          ) : filtrados.length === 0 ? (
            <p className="text-sm text-muted-foreground">No hay artículos con stock. Sincroniza el catálogo desde BDP.</p>
          ) : (
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Código Glory</TableHead>
                    <TableHead>Código BDP</TableHead>
                    <TableHead>Nombre BDP</TableHead>
                <TableHead>Precio</TableHead>
                <TableHead>Stock</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filtrados.map((m) => (
                    <TableRow key={m.id}>
                      <TableCell className="font-mono text-xs">{m.articulo_glory_codigo}</TableCell>
                      <TableCell className="font-mono text-xs">{m.articulo_bdp_codigo}</TableCell>
                      <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
                      <TableCell className="text-xs tabular-nums">
                        {m.precio_tarifa1 && m.precio_tarifa1 !== '0' ? `${Number(m.precio_tarifa1).toFixed(2)} €` : '—'}
                      </TableCell>
                      <TableCell>
                        {m.stock_actual && m.stock_actual !== '0' ? (
                          <Badge variant="secondary" className="gap-1">
                            <Package className="size-3" />
                            {m.stock_actual}
                          </Badge>
                        ) : (
                          <span className="text-xs text-muted-foreground">—</span>
                        )}
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

export default BdpStock;
