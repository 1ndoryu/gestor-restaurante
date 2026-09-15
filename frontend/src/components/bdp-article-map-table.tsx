/* [147A-F5.6] Tabla de mapeos artículos Glory → BDP.
 * Permite listar, crear y eliminar mapeos. Importa catálogo desde BDP (F5.7).
 * [223A-1] Tooltips con TooltipButton + confirmación para sync.
 * [237A-4] Añadida columna Stock (solo lectura, viene de sync-catalog).
 * [128A-1/F2] Catálogo local: badge de origen (local/bdp), edición (PATCH)
 * de precio, IVA, descripción y código BDP, alta de artículo local (sin
 * código BDP) y toggle de activo (M7: el import no reactiva).
 * [159A-1] Alta y edición en modales (NuevoArticuloDialog +
 * ArticuloMapEditarDialog), como "Nueva Venta": nada inline. */

import { useMemo, useState } from 'react';
import { Plus, Trash2, Package, Pencil } from 'lucide-react';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps } from '../api/generated/bdp-mapeos/bdp-mapeos';
import {
  useEliminarArticleMap,
  useActualizarArticleMap,
} from '../api/generated/bdp-mapeos/bdp-mapeos';
import type { ActualizarBdpArticleMapRequest } from '../api/generated/gestionRestauranteAPI.schemas';
import { BdpArticleCatalogActions } from './BdpArticleCatalogActions';
import NuevoArticuloDialog from '@/componentes/bdp/NuevoArticuloDialog';
import ArticuloMapEditarDialog, { type ArticuloMapEdicion } from './articulo-map-editar-dialog';

function formatPrecio(precio: string | undefined): string {
  const n = Number(precio);
  return Number.isFinite(n) && n > 0 ? `${n.toFixed(2)} €` : '—';
}

function BdpArticleMapTable() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useListarArticleMaps();
  const eliminarMutation = useEliminarArticleMap({
    mutation: {
      onSuccess: () => {
        toast.success('Mapeo eliminado');
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al eliminar mapeo'),
    },
  });
  /* [128A-1/F2] Edición de campos locales (PATCH parcial, en modal) */
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

  const [nuevoOpen, setNuevoOpen] = useState(false);
  const [editando, setEditando] = useState<ArticuloMapEdicion | null>(null);
  /* [P2.4] Búsqueda y orden efectivos (todo en cliente, sin tocar BDP) */
  const [busqueda, setBusqueda] = useState('');
  const [sortKey, setSortKey] = useState<'codigo' | 'descripcion' | 'precio' | null>(null);
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('asc');
  /* Paginación en cliente: con cientos de artículos, renderizar todo en cada
   * tecla frena el navegador; solo se pintan PAGE_SIZE filas. */
  const PAGE_SIZE = 50;
  const [pagina, setPagina] = useState(0);
  const mapeos = data?.status === 200 ? data.data : [];

  function alternarOrden(key: 'codigo' | 'descripcion' | 'precio') {
    if (sortKey !== key) {
      setSortKey(key);
      setSortOrder('asc');
    } else {
      setSortOrder((o) => (o === 'asc' ? 'desc' : 'asc'));
    }
    setPagina(0);
  }

  function cambiarBusqueda(v: string) {
    setBusqueda(v);
    setPagina(0);
  }

  const mapeosVisibles = useMemo(() => {
    const q = busqueda.trim().toLowerCase();
    const filtrados = q
      ? mapeos.filter((m) =>
          (m.articulo_glory_codigo || '').toLowerCase().includes(q) ||
          (m.descripcion || '').toLowerCase().includes(q) ||
          (m.articulo_bdp_nombre || '').toLowerCase().includes(q),
        )
      : [...mapeos];
    if (sortKey) {
      const dir = sortOrder === 'asc' ? 1 : -1;
      filtrados.sort((a, b) => {
        if (sortKey === 'precio') {
          const pa = Number(a.precio_tarifa1) || 0;
          const pb = Number(b.precio_tarifa1) || 0;
          return (pa - pb) * dir;
        }
        const av = sortKey === 'codigo'
          ? (a.articulo_glory_codigo || '')
          : (a.descripcion || a.articulo_bdp_nombre || '');
        const bv = sortKey === 'codigo'
          ? (b.articulo_glory_codigo || '')
          : (b.descripcion || b.articulo_bdp_nombre || '');
        return av.localeCompare(bv, 'es') * dir;
      });
    }
    return filtrados;
  }, [mapeos, busqueda, sortKey, sortOrder]);

  const totalPaginas = Math.max(1, Math.ceil(mapeosVisibles.length / PAGE_SIZE));
  const paginaActual = Math.min(pagina, totalPaginas - 1);
  const paginaDesde = paginaActual * PAGE_SIZE;
  const mapeosPagina = mapeosVisibles.slice(paginaDesde, paginaDesde + PAGE_SIZE);

  function flecha(key: 'codigo' | 'descripcion' | 'precio') {
    if (sortKey !== key) return null;
    return sortOrder === 'asc' ? ' ↑' : ' ↓';
  }

  function startEdicion(m: (typeof mapeos)[number]) {
    setEditando({
      id: m.id,
      articulo_bdp_codigo: m.articulo_bdp_codigo || '',
      descripcion: m.descripcion || '',
      precio_tarifa1: m.precio_tarifa1 && m.precio_tarifa1 !== '0' ? m.precio_tarifa1 : '',
      iva_pct: m.iva_pct && m.iva_pct !== '0' ? m.iva_pct : '',
    });
  }

  function guardarEdicion(ed: ArticuloMapEdicion) {
    const body: ActualizarBdpArticleMapRequest = {
      articulo_bdp_codigo: ed.articulo_bdp_codigo || null,
      descripcion: ed.descripcion || null,
      precio_tarifa1: ed.precio_tarifa1 ? String(ed.precio_tarifa1) : null,
      iva_pct: ed.iva_pct ? String(ed.iva_pct) : null,
    };
    actualizarMutation.mutate({ id: ed.id, data: body });
  }

  function toggleActivo(m: (typeof mapeos)[number]) {
    toggleMutation.mutate({ id: m.id, data: { activo: !m.activo } });
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <span className="text-sm font-medium">Mapeo artículos Aplicación Web → BDP</span>
        <div className="flex items-center gap-2">
          <Button size="sm" onClick={() => setNuevoOpen(true)}>
            <Plus className="size-3.5 mr-1" />
            Nuevo artículo
          </Button>
          <BdpArticleCatalogActions />
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <Input
          type="search"
          placeholder="Buscar por código o descripción..."
          value={busqueda}
          onChange={(e) => cambiarBusqueda(e.target.value)}
          aria-label="Buscar artículos"
          className="max-w-xs"
        />
      </div>

      {isLoading ? (
        <p className="text-xs text-muted-foreground">Cargando mapeos...</p>
      ) : (
      <>
      {mapeosVisibles.length > 0 && (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>
                  <button
                    type="button"
                    onClick={() => alternarOrden('codigo')}
                    className="cursor-pointer select-none p-0 text-left font-medium hover:underline"
                  >
                    Código Aplicación Web{flecha('codigo')}
                  </button>
                </TableHead>
                <TableHead>Código BDP</TableHead>
                <TableHead>Origen</TableHead>
                <TableHead>
                  <button
                    type="button"
                    onClick={() => alternarOrden('descripcion')}
                    className="cursor-pointer select-none p-0 text-left font-medium hover:underline"
                  >
                    Descripción{flecha('descripcion')}
                  </button>
                </TableHead>
                <TableHead>
                  <button
                    type="button"
                    onClick={() => alternarOrden('precio')}
                    className="cursor-pointer select-none p-0 text-left font-medium hover:underline"
                  >
                    Precio{flecha('precio')}
                  </button>
                </TableHead>
                <TableHead>IVA</TableHead>
                <TableHead>Familia</TableHead>
                <TableHead>Stock</TableHead>
                <TableHead>Activo</TableHead>
                <TableHead className="w-10 text-center">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mapeosPagina.map((m) => (
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
                    <div className="flex items-center justify-center gap-1">
                      <TooltipButton
                        variant="outline"
                        size="icon"
                        className="bg-muted/40 hover:bg-muted"
                        onClick={() => startEdicion(m)}
                        tooltip="Editar datos locales del artículo"
                      >
                        <Pencil className="size-3.5" />
                      </TooltipButton>
                      <TooltipButton
                        variant="outline"
                        size="icon"
                        className="bg-muted/40 hover:bg-muted"
                        onClick={() => eliminarMutation.mutate({ id: m.id })}
                        disabled={eliminarMutation.isPending}
                        tooltip="Eliminar este mapeo. No afecta al catálogo BDP."
                      >
                        <Trash2 className="size-3.5 text-destructive" />
                      </TooltipButton>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
      {mapeosVisibles.length > 0 && (
        <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
          <span className="text-xs text-muted-foreground">
            {mapeosVisibles.length} artículo{mapeosVisibles.length === 1 ? '' : 's'}
            {totalPaginas > 1 && ` · página ${paginaActual + 1} de ${totalPaginas}`}
          </span>
          {totalPaginas > 1 && (
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                disabled={paginaActual === 0}
                onClick={() => setPagina((p) => Math.max(0, p - 1))}
              >
                Anterior
              </Button>
              <Button
                size="sm"
                variant="outline"
                disabled={paginaActual >= totalPaginas - 1}
                onClick={() => setPagina((p) => Math.min(totalPaginas - 1, p + 1))}
              >
                Siguiente
              </Button>
            </div>
          )}
        </div>
      )}
      {mapeosVisibles.length === 0 && (busqueda.trim() ? (
        <p className="text-xs text-muted-foreground">Sin artículos que coincidan con la búsqueda.</p>
      ) : (
        <p className="text-xs text-muted-foreground">Sin mapeos. Añade uno manualmente o usa la sincronización enriquecida del catálogo BDP.</p>
      ))}
      </>
      )}

      <NuevoArticuloDialog open={nuevoOpen} onOpenChange={setNuevoOpen} />
      <ArticuloMapEditarDialog
        edicion={editando}
        isPending={actualizarMutation.isPending}
        onOpenChange={(open) => { if (!open) setEditando(null); }}
        onGuardar={guardarEdicion}
      />
    </div>
  );
}

export default BdpArticleMapTable;
