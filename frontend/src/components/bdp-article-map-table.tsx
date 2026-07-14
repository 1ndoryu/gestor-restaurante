/* [147A-F5.6] Tabla de mapeos artículos Glory → BDP.
 * Permite listar, crear y eliminar mapeos. Importa catálogo desde BDP (F5.7). */

import { useState } from 'react';
import { Plus, Trash2, Download, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useListarArticleMaps } from '../api/generated/bdp-mapeos/bdp-mapeos';
import { useCrearArticleMap, useEliminarArticleMap } from '../api/generated/bdp-mapeos/bdp-mapeos';
import { useImportarCatalogo } from '../api/generated/bdp-mapeos/bdp-mapeos';

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
  const [importando, setImportando] = useState(false);
  const importarMutation = useImportarCatalogo({
    mutation: {
      onSuccess: (resp) => {
        const data = resp as unknown as { imported?: number; errors?: number; total?: number };
        toast.success(`Catálogo importado: ${data.imported ?? 0} artículos`);
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al importar catálogo BDP'),
      onSettled: () => setImportando(false),
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

  function importarCatalogo() {
    setImportando(true);
    importarMutation.mutate();
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Mapeo artículos Glory → BDP</span>
        <Button variant="outline" size="sm" onClick={importarCatalogo} disabled={importando}>
          {importando ? <Loader2 className="size-3.5 animate-spin" /> : <Download className="size-3.5" />}
          Importar catálogo
        </Button>
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
                <TableHead className="w-10"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {mapeos.map((m) => (
                <TableRow key={m.id}>
                  <TableCell className="font-mono text-xs">{m.articulo_glory_codigo}</TableCell>
                  <TableCell className="font-mono text-xs">{m.articulo_bdp_codigo}</TableCell>
                  <TableCell className="text-xs">{m.articulo_bdp_nombre || '—'}</TableCell>
                  <TableCell>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => eliminarMutation.mutate({ id: m.id })}
                      disabled={eliminarMutation.isPending}
                      title="Eliminar mapeo"
                    >
                      <Trash2 className="size-3.5 text-destructive" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">Sin mapeos. Añade manualmente o importa el catálogo BDP.</p>
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
