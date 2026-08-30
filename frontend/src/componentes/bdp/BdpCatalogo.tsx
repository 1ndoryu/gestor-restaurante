/* [198A-1/D7] Catálogo: artículos + clasificaciones (departamentos/familias).
 * [208A-2/C1] Unificado tras la auditoría: el CRUD de artículos
 * (BdpArticleMapTable) vive aquí, en la página "Catálogo" del menú, junto a
 * departamentos/familias. Configuración → BDP queda solo con conexión,
 * mapeos y permisos (decisión D1/D6). El código de clasificación se asigna
 * secuencialmente en el backend; con BDP conectado el alta se encola; sin BDP
 * queda local (independencia). */

import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Tags, Plus, Package } from 'lucide-react';
import { toast } from 'sonner';
import BdpArticleMapTable from '@/components/bdp-article-map-table';
import { useBdpCatalogo, useCrearBdpClasificacion, type BdpCatalogoTipo } from '@/api/bdp';

type VistaCatalogo = 'articulos' | 'clasificaciones';

function Clasificaciones() {
  const queryClient = useQueryClient();
  const [tipo, setTipo] = useState<BdpCatalogoTipo>('departamento');
  const [nombre, setNombre] = useState('');
  const { data, isLoading } = useBdpCatalogo(tipo);
  const crearMutation = useCrearBdpClasificacion(queryClient);

  const crear = () => {
    if (!nombre.trim()) return;
    crearMutation.mutate(
      { tipo, nombre: nombre.trim() },
      {
        onSuccess: () => {
          toast.success(`${tipo === 'departamento' ? 'Departamento' : 'Familia'} creado`);
          setNombre('');
        },
        onError: () => toast.error('No se pudo crear la clasificación'),
      },
    );
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <Button variant={tipo === 'departamento' ? 'default' : 'outline'} onClick={() => setTipo('departamento')}>
          Departamentos
        </Button>
        <Button variant={tipo === 'familia' ? 'default' : 'outline'} onClick={() => setTipo('familia')}>
          Familias
        </Button>
      </div>

      <div className="flex items-end gap-2 max-w-md">
        <div className="flex flex-col gap-1 flex-1">
          <Label htmlFor="catalogo-nombre">Nombre</Label>
          <Input
            id="catalogo-nombre"
            value={nombre}
            onChange={(e) => setNombre(e.target.value)}
            placeholder={tipo === 'departamento' ? 'Ej: Cocina' : 'Ej: Bebidas'}
            maxLength={255}
          />
        </div>
        <Button onClick={crear} disabled={crearMutation.isPending || !nombre.trim()}>
          <Plus className="size-4 mr-1" /> Crear
        </Button>
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando…</p>
      ) : data && data.length > 0 ? (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-32"><Tags className="size-3.5 inline mr-1" />Código BDP</TableHead>
                <TableHead>Nombre</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.map((c) => (
                <TableRow key={c.id}>
                  <TableCell className="font-mono text-xs tabular-nums">{c.code}</TableCell>
                  <TableCell>{c.nombre}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">No hay {tipo === 'departamento' ? 'departamentos' : 'familias'} registrados.</p>
      )}

      <p className="text-xs text-muted-foreground">
        El código BDP se asigna automáticamente al crear. Con BDP conectado, el alta se empuja al terminal; sin BDP, queda local.
      </p>
    </div>
  );
}

function BdpCatalogo() {
  const [vista, setVista] = useState<VistaCatalogo>('articulos');

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <Button variant={vista === 'articulos' ? 'default' : 'outline'} onClick={() => setVista('articulos')}>
          <Package className="size-4 mr-1" />
          Artículos
        </Button>
        <Button variant={vista === 'clasificaciones' ? 'default' : 'outline'} onClick={() => setVista('clasificaciones')}>
          <Tags className="size-4 mr-1" />
          Departamentos y familias
        </Button>
      </div>

      {vista === 'articulos' ? (
        <BdpArticleMapTable />
      ) : (
        <Clasificaciones />
      )}
    </div>
  );
}

export default BdpCatalogo;
