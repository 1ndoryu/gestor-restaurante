/* [147A-F5.6] Tabla de mapeos artículos Glory → BDP.
 * Permite listar, crear y eliminar mapeos. Importa catálogo desde BDP (F5.7).
 * [223A-1] Tooltips con TooltipButton + confirmación para sync.
 * [237A-4] Añadida columna Stock (solo lectura, viene de sync-catalog).
 * [128A-1/F2] Catálogo local: badge de origen (local/bdp), edición inline
 * (PATCH) de precio, IVA, familia, descripción y código BDP, alta de artículo
 * local (sin código BDP) y toggle de activo (M7: el import no reactiva). */

import { useState } from 'react';
import { Plus, Trash2, Package, Pencil, X, Check } from 'lucide-react';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps } from '../api/generated/bdp-mapeos/bdp-mapeos';
import {
  useCrearArticleMap,
  useEliminarArticleMap,
  useActualizarArticleMap,
} from '../api/generated/bdp-mapeos/bdp-mapeos';
import type { ActualizarBdpArticleMapRequest } from '../api/generated/gestionRestauranteAPI.schemas';
import { BdpArticleCatalogActions } from './BdpArticleCatalogActions';

/* [128A-1/F2] Formulario de alta: el código BDP es opcional porque se puede
 * crear un artículo 100% local (origen='local'). */
interface NuevoArticulo {
  articulo_glory_codigo: string;
  articulo_bdp_codigo?: string;
  descripcion: string;
  precio_tarifa1: string;
  iva_pct: string;
}

const articuloVacio: NuevoArticulo = {
  articulo_glory_codigo: '',
  articulo_bdp_codigo: '',
  descripcion: '',
  precio_tarifa1: '',
  iva_pct: '',
};

/* [128A-1/F2] Fila en modo edición inline (PATCH parcial) */
interface Edicion {
  id: string;
  articulo_bdp_codigo: string;
  descripcion: string;
  precio_tarifa1: string;
  iva_pct: string;
}

function formatPrecio(precio: string | undefined): string {
  const n = Number(precio);
  return Number.isFinite(n) && n > 0 ? `${n.toFixed(2)} €` : '—';
}

function BdpArticleMapTable() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useListarArticleMaps();
  const crearMutation = useCrearArticleMap({
    mutation: {
      onSuccess: () => {
        toast.success('Artículo creado');
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
        setNuevo({ ...articuloVacio });
      },
      onError: () => toast.error('Error al crear artículo'),
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
  /* [128A-1/F2] Edición inline de campos locales */
  const actualizarMutation = useActualizarArticleMap({
    mutation: {
      onSuccess: () => {
        toast.success('Artículo actualizado');
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
        setEditando(null);
      },
      onError: () => toast.error('Error al actualizar artículo'),
    },
  });
  const toggleMutation = useActualizarArticleMap({
    mutation: {
      onSuccess: () => {
        toast.success('Disponibilidad actualizada');
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al cambiar disponibilidad'),
    },
  });

  const [nuevo, setNuevo] = useState<NuevoArticulo>(articuloVacio);
  const [editando, setEditando] = useState<Edicion | null>(null);
  const mapeos = data?.status === 200 ? data.data : [];

  function startEdicion(m: (typeof mapeos)[number]) {
    setEditando({
      id: m.id,
      articulo_bdp_codigo: m.articulo_bdp_codigo || '',
      descripcion: m.descripcion || '',
      precio_tarifa1: m.precio_tarifa1 && m.precio_tarifa1 !== '0' ? m.precio_tarifa1 : '',
      iva_pct: m.iva_pct && m.iva_pct !== '0' ? m.iva_pct : '',
    });
  }

  function guardarEdicion() {
    if (!editando) return;
    const body: ActualizarBdpArticleMapRequest = {
      articulo_bdp_codigo: editando.articulo_bdp_codigo || null,
      descripcion: editando.descripcion || null,
      precio_tarifa1: editando.precio_tarifa1 ? String(editando.precio_tarifa1) : null,
      iva_pct: editando.iva_pct ? String(editando.iva_pct) : null,
    };
    actualizarMutation.mutate({ id: editando.id, data: body });
  }

  function toggleActivo(m: (typeof mapeos)[number]) {
    toggleMutation.mutate({ id: m.id, data: { activo: !m.activo } });
  }

  function handleCrear() {
    if (!nuevo.articulo_glory_codigo) return;
    crearMutation.mutate({
      data: {
        articulo_glory_codigo: nuevo.articulo_glory_codigo,
        articulo_bdp_codigo: nuevo.articulo_bdp_codigo || undefined,
        descripcion: nuevo.descripcion || undefined,
        precio_tarifa1: nuevo.precio_tarifa1 ? String(nuevo.precio_tarifa1) : undefined,
        iva_pct: nuevo.iva_pct ? String(nuevo.iva_pct) : undefined,
      },
    });
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <span className="text-sm font-medium">Mapeo artículos Aplicación Web → BDP</span>
        <BdpArticleCatalogActions />
      </div>

      {isLoading ? (
        <p className="text-xs text-muted-foreground">Cargando mapeos...</p>
      ) : mapeos.length > 0 ? (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Código Aplicación Web</TableHead>
                <TableHead>Código BDP</TableHead>
                <TableHead>Origen</TableHead>
                <TableHead>Descripción</TableHead>
                <TableHead>Precio</TableHead>
                <TableHead>IVA</TableHead>
                <TableHead>Familia</TableHead>
                <TableHead>Stock</TableHead>
                <TableHead>Activo</TableHead>
                <TableHead className="w-10"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mapeos.map((m) => (
                <TableRow key={m.id}>
                  <TableCell className="font-mono text-xs">{m.articulo_glory_codigo}</TableCell>
                  <TableCell className="font-mono text-xs">
                    {m.articulo_bdp_codigo || <span className="text-muted-foreground">—</span>}
                  </TableCell>
                  <TableCell>
                    {m.origen === 'local' ? (
                      <Badge variant="secondary">local</Badge>
                    ) : (
                      <Badge variant="outline">bdp</Badge>
                    )}
                  </TableCell>
                  <TableCell className="max-w-56 truncate text-xs" title={m.descripcion}>
                    {m.descripcion || m.articulo_bdp_nombre || '—'}
                  </TableCell>
                  <TableCell className="text-xs tabular-nums">{formatPrecio(m.precio_tarifa1)}</TableCell>
                  <TableCell className="text-xs tabular-nums">
                    {m.iva_pct && m.iva_pct !== '0' ? `${Number(m.iva_pct).toFixed(0)}%` : '—'}
                  </TableCell>
                  <TableCell className="text-xs tabular-nums">
                    {m.familia ? m.familia : '—'}
                  </TableCell>
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
                    <Switch
                      size="sm"
                      checked={m.activo}
                      onCheckedChange={() => toggleActivo(m)}
                      disabled={toggleMutation.isPending}
                      aria-label={`Activar/desactivar ${m.articulo_glory_codigo}`}
                    />
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      {editando?.id === m.id ? (
                        <>
                          <TooltipButton
                            variant="ghost"
                            size="icon"
                            onClick={guardarEdicion}
                            disabled={actualizarMutation.isPending}
                            tooltip="Guardar cambios"
                          >
                            <Check className="size-3.5 text-emerald-600" />
                          </TooltipButton>
                          <TooltipButton
                            variant="ghost"
                            size="icon"
                            onClick={() => setEditando(null)}
                            tooltip="Cancelar"
                          >
                            <X className="size-3.5" />
                          </TooltipButton>
                        </>
                      ) : (
                        <>
                          <TooltipButton
                            variant="ghost"
                            size="icon"
                            onClick={() => startEdicion(m)}
                            tooltip="Editar datos locales del artículo"
                          >
                            <Pencil className="size-3.5" />
                          </TooltipButton>
                          <TooltipButton
                            variant="ghost"
                            size="icon"
                            onClick={() => eliminarMutation.mutate({ id: m.id })}
                            disabled={eliminarMutation.isPending}
                            tooltip="Eliminar este mapeo. No afecta al catálogo BDP."
                          >
                            <Trash2 className="size-3.5 text-destructive" />
                          </TooltipButton>
                        </>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
              {editando && (
                <TableRow>
                  <TableCell colSpan={10} className="bg-muted/40 p-2">
                    <div className="grid gap-2 md:grid-cols-5 items-end">
                      <div className="flex flex-col gap-1">
                        <Label className="text-xs">Código BDP</Label>
                        <Input
                          className="font-mono text-xs"
                          value={editando.articulo_bdp_codigo}
                          onChange={(e) => setEditando((p) => p && { ...p, articulo_bdp_codigo: e.target.value })}
                          placeholder="Vacío = artículo local"
                        />
                      </div>
                      <div className="flex flex-col gap-1 md:col-span-2">
                        <Label className="text-xs">Descripción</Label>
                        <Input
                          className="text-xs"
                          value={editando.descripcion}
                          onChange={(e) => setEditando((p) => p && { ...p, descripcion: e.target.value })}
                        />
                      </div>
                      <div className="flex flex-col gap-1">
                        <Label className="text-xs">Precio (€)</Label>
                        <Input
                          className="text-xs"
                          type="number"
                          min="0"
                          step="0.01"
                          value={editando.precio_tarifa1}
                          onChange={(e) => setEditando((p) => p && { ...p, precio_tarifa1: e.target.value })}
                        />
                      </div>
                      <div className="flex flex-col gap-1">
                        <Label className="text-xs">IVA (%)</Label>
                        <Input
                          className="text-xs"
                          type="number"
                          min="0"
                          step="0.01"
                          value={editando.iva_pct}
                          onChange={(e) => setEditando((p) => p && { ...p, iva_pct: e.target.value })}
                        />
                      </div>
                    </div>
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">Sin mapeos. Añade uno manualmente o usa la sincronización enriquecida del catálogo BDP.</p>
      )}

      {/* Formulario inline para nuevo mapeo */}
      <div className="grid gap-2 md:grid-cols-5 items-end">
        <div className="flex flex-col gap-1">
          <Label htmlFor="nuevo-glory-codigo" className="text-xs">Código Aplicación Web</Label>
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
            placeholder="Opcional (artículo local)"
          />
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="nuevo-descripcion" className="text-xs">Descripción</Label>
          <Input
            id="nuevo-descripcion"
            className="text-xs"
            value={nuevo.descripcion}
            onChange={(e) => setNuevo((p) => ({ ...p, descripcion: e.target.value }))}
            placeholder="Nombre/descripción"
          />
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="nuevo-precio" className="text-xs">Precio (€)</Label>
          <Input
            id="nuevo-precio"
            className="text-xs"
            type="number"
            min="0"
            step="0.01"
            value={nuevo.precio_tarifa1}
            onChange={(e) => setNuevo((p) => ({ ...p, precio_tarifa1: e.target.value }))}
          />
        </div>
        <div className="flex flex-col gap-1">
          <Label htmlFor="nuevo-iva" className="text-xs">IVA (%)</Label>
          <Input
            id="nuevo-iva"
            className="text-xs"
            type="number"
            min="0"
            step="0.01"
            value={nuevo.iva_pct}
            onChange={(e) => setNuevo((p) => ({ ...p, iva_pct: e.target.value }))}
          />
        </div>
        <Button
          size="sm"
          onClick={handleCrear}
          disabled={!nuevo.articulo_glory_codigo || crearMutation.isPending}
        >
          <Plus className="size-3.5 mr-1" />
          Añadir
        </Button>
      </div>
    </div>
  );
}

export default BdpArticleMapTable;
