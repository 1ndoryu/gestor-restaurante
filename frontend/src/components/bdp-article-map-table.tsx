/* [147A-F5.6] Tabla de mapeos artículos Glory → BDP.
 * Permite listar, crear y eliminar mapeos. Importa catálogo desde BDP (F5.7).
 * [223A-1] Tooltips con TooltipButton + confirmación para sync.
 * [237A-4] Añadida columna Stock (solo lectura, viene de sync-catalog). */

import { useState } from 'react';
import { Plus, Trash2, Loader2, RefreshCw, Package } from 'lucide-react';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps } from '../api/generated/bdp-mapeos/bdp-mapeos';
import { useCrearArticleMap, useEliminarArticleMap } from '../api/generated/bdp-mapeos/bdp-mapeos';
import { useSyncCatalog, useSyncPrices } from '../api/generated/bdp-mapeos/bdp-mapeos';

interface NuevoMapeo {
  articulo_glory_codigo: string;
  articulo_bdp_codigo: string;
  articulo_bdp_nombre: string;
}

const mapeoVacio: NuevoMapeo = {
  articulo_glory_codigo: '',
  articulo_bdp_codigo: '',
  articulo_bdp_nombre: '',
};

function BdpArticleMapTable() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useListarArticleMaps();
  const crearMutation = useCrearArticleMap({
    mutation: {
      onSuccess: () => {
        toast.success('Mapeo creado');
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
        setNuevo(mapeoVacio);
      },
      onError: () => toast.error('Error al crear mapeo'),
    },
  });
  const eliminarMutation = useEliminarArticleMap({
    mutation: {
      onSuccess: () => {
        toast.success('Mapeo eliminado');
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al eliminar mapeo'),
    },
  });

  const [nuevo, setNuevo] = useState<NuevoMapeo>(mapeoVacio);
  const [sincronizando, setSincronizando] = useState(false);
  const [actualizandoPrecios, setActualizandoPrecios] = useState(false);
  const syncCatalogMutation = useSyncCatalog({
    mutation: {
      onSuccess: (resp) => {
        const d = resp as unknown as { creados?: number; actualizados?: number; sin_cambios?: number; errores?: number };
        toast.success(`Sync completado: ${d.creados ?? 0} nuevos, ${d.actualizados ?? 0} actualizados`);
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al sincronizar catálogo BDP'),
      onSettled: () => setSincronizando(false),
    },
  });

  const syncPricesMutation = useSyncPrices({
    mutation: {
      onSuccess: (resp) => {
        const d = resp as unknown as { actualizados?: number; sin_cambios?: number };
        toast.success(`Precios actualizados: ${d.actualizados ?? 0} artículos`);
      },
      onError: () => toast.error('Error al sincronizar precios BDP'),
      onSettled: () => setActualizandoPrecios(false),
    },
  });

  const mapeos = data?.status === 200 ? data.data : [];

  function handleCrear() {
    if (!nuevo.articulo_glory_codigo || !nuevo.articulo_bdp_codigo) return;
    crearMutation.mutate({
      data: {
        articulo_glory_codigo: nuevo.articulo_glory_codigo,
        articulo_bdp_codigo: nuevo.articulo_bdp_codigo,
        articulo_bdp_nombre: nuevo.articulo_bdp_nombre || undefined,
      },
    });
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Mapeo artículos Glory → BDP</span>
        <div className="flex gap-2">
          <TooltipButton variant="default" size="sm" onClick={() => { setSincronizando(true); syncCatalogMutation.mutate(); }} disabled={sincronizando} tooltip="Importa/actualiza artículos desde BDP a Glory. Crea mapeos automáticos por código.">
            {sincronizando ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
            Sync catálogo
          </TooltipButton>
          <TooltipButton variant="outline" size="sm" onClick={() => { setActualizandoPrecios(true); syncPricesMutation.mutate(); }} disabled={actualizandoPrecios} tooltip="Actualiza los precios de los artículos mapeados desde BDP. El stock solo se actualiza con 'Sync catálogo'.">
            {actualizandoPrecios ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
            Sync precios
          </TooltipButton>
        </div>
      </div>

      {isLoading ? (
        <p className="text-xs text-muted-foreground">Cargando mapeos...</p>
      ) : mapeos.length > 0 ? (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Código Glory</TableHead>
                <TableHead>Código BDP</TableHead>
                <TableHead>Nombre BDP</TableHead>
                <TableHead>Precio</TableHead>
                <TableHead>Stock</TableHead>
                <TableHead className="w-10"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mapeos.map((m) => (
                <TableRow key={m.id}>
                  <TableCell className="font-mono text-xs">{m.articulo_glory_codigo}</TableCell>
                  <TableCell className="font-mono text-xs">{m.articulo_bdp_codigo}</TableCell>
                  <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
                  <TableCell className="text-xs tabular-nums">{m.precio_tarifa1 && m.precio_tarifa1 !== '0' ? `${Number(m.precio_tarifa1).toFixed(2)} €` : '—'}</TableCell>
                  <TableCell>
                    {m.stock_actual && m.stock_actual !== '0' ? (
                      <span className="inline-flex items-center gap-1 text-xs tabular-nums">
                        <Package className="size-3 text-muted-foreground" />
                        {m.stock_actual}
                      </span>
                    ) : (
                      <span className="text-xs text-muted-foreground">—</span>
                    )}
                  </TableCell>
                  <TableCell>
                    <TooltipButton
                      variant="ghost"
                      size="icon"
                      onClick={() => eliminarMutation.mutate({ id: m.id })}
                      disabled={eliminarMutation.isPending}
                      tooltip="Eliminar este mapeo. No afecta al catálogo BDP."
                    >
                      <Trash2 className="size-3.5 text-destructive" />
                    </TooltipButton>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">Sin mapeos. Añade uno manualmente o usa la sincronización enriquecida del catálogo BDP.</p>
      )}

      {/* Formulario inline para nuevo mapeo */}
      <div className="grid gap-2 md:grid-cols-4 items-end">
        <div className="flex flex-col gap-1">
          <Label htmlFor="nuevo-glory-codigo" className="text-xs">Código Glory</Label>
          <Input
            id="nuevo-glory-codigo"
            className="font-mono text-xs"
            value={nuevo.articulo_glory_codigo}
            onChange={(e) => setNuevo((p) => ({ ...p, articulo_glory_codigo: e.target.value }))}
            placeholder="SKU interno"
          />
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="nuevo-bdp-codigo" className="text-xs">Código BDP</Label>
          <Input
            id="nuevo-bdp-codigo"
            className="font-mono text-xs"
            value={nuevo.articulo_bdp_codigo}
            onChange={(e) => setNuevo((p) => ({ ...p, articulo_bdp_codigo: e.target.value }))}
            placeholder="Código BDP"
          />
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="nuevo-bdp-nombre" className="text-xs">Nombre BDP</Label>
          <Input
            id="nuevo-bdp-nombre"
            className="text-xs"
            value={nuevo.articulo_bdp_nombre}
            onChange={(e) => setNuevo((p) => ({ ...p, articulo_bdp_nombre: e.target.value }))}
            placeholder="Descripción (opcional)"
          />
        </div>
        <Button
          size="sm"
          onClick={handleCrear}
          disabled={!nuevo.articulo_glory_codigo || !nuevo.articulo_bdp_codigo || crearMutation.isPending}
        >
          <Plus className="size-3.5 mr-1" />
          Añadir
        </Button>
      </div>
    </div>
  );
}

export default BdpArticleMapTable;
