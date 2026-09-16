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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogClose,
} from '@/components/ui/dialog';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Tags, Plus, Package } from 'lucide-react';
import { toast } from 'sonner';
import BdpArticleMapTable from '@/components/bdp-article-map-table';
import { BdpImportExportButtons } from '@/components/bdp-import-export-buttons';
import { useBdpCatalogo, useCrearBdpClasificacion, useImportarDepartamentosBdp, type BdpCatalogoTipo } from '@/api/bdp';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';

type VistaCatalogo = 'articulos' | 'clasificaciones';

function Clasificaciones() {
  const queryClient = useQueryClient();
  const [tipo, setTipo] = useState<BdpCatalogoTipo>('departamento');
  const [nombre, setNombre] = useState('');
  const [crearOpen, setCrearOpen] = useState(false);
  const [busqueda, setBusqueda] = useState('');
  const [pagina, setPagina] = useState(0);
  const PAGE_SIZE = 50;
  const { data, isLoading } = useBdpCatalogo(tipo);
  const crearMutation = useCrearBdpClasificacion(queryClient);
  const etiqueta = tipo === 'departamento' ? 'Departamento' : 'Familia';
  /* [159A-2/F2] Importar exige BDP efectivo (es lectura BDP); crear y
   * exportar funcionan también en local (el alta queda local o se encola). */
  const { data: configResponse } = useObtenerConfiguracion();
  const configData = configResponse?.status === 200 ? configResponse.data : undefined;
  const modoEfectivoBdp = !!configData && (
    configData.modo_operacion === 'bdp'
    || (configData.modo_operacion === 'auto'
      && configData.bdp_sync_enabled
      && (configData.bdp_base_url ?? '').trim() !== '')
  );
  const importarDeptosMutation = useImportarDepartamentosBdp();

  function importarDepartamentos() {
    importarDeptosMutation.mutate(undefined, {
      onSuccess: (r) => {
        const partes = [`${r.creados} creados`, `${r.vinculados} vinculados`];
        if (r.conflictos.length > 0) partes.push(`${r.conflictos.length} conflictos (no se pisan)`);
        if (r.omitidos_fuera_rango.length > 0) partes.push(`${r.omitidos_fuera_rango.length} fuera de rango (omitidos)`);
        toast.success(`Departamentos importados del BDP: ${partes.join(', ')}`);
        queryClient.invalidateQueries({ queryKey: ['bdp-catalogo', 'departamento'] });
      },
      onError: (err: unknown) => {
        const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
        toast.error('No se pudieron importar los departamentos', { description: msg });
      },
    });
  }

  const crear = () => {
    if (!nombre.trim()) return;
    crearMutation.mutate(
      { tipo, nombre: nombre.trim() },
      {
        onSuccess: () => {
          toast.success(`${etiqueta} creado`);
          setNombre('');
          setCrearOpen(false);
        },
        onError: () => toast.error('No se pudo crear la clasificación'),
      },
    );
  };

  function cambiarTipo(nuevo: string) {
    setTipo(nuevo as BdpCatalogoTipo);
    setBusqueda('');
    setPagina(0);
  }

  const filas = data ?? [];
  const q = busqueda.trim().toLowerCase();
  const filasVisibles = q === ''
    ? filas
    : filas.filter((c) =>
      String(c.code ?? '').toLowerCase().includes(q)
      || (c.nombre ?? '').toLowerCase().includes(q),
    );
  const totalPaginas = Math.max(1, Math.ceil(filasVisibles.length / PAGE_SIZE));
  const paginaActual = Math.min(pagina, totalPaginas - 1);
  const filasPagina = filasVisibles.slice(paginaActual * PAGE_SIZE, paginaActual * PAGE_SIZE + PAGE_SIZE);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Tabs value={tipo} onValueChange={cambiarTipo}>
          <TabsList>
            <TabsTrigger value="departamento">Departamentos</TabsTrigger>
            <TabsTrigger value="familia">Familias</TabsTrigger>
          </TabsList>
        </Tabs>
        <div className="flex flex-wrap items-center gap-2">
          {/* [159A-1] Alta en modal, como "Nueva Venta": nada inline. */}
          <Button onClick={() => setCrearOpen(true)}>
            <Plus className="size-4 mr-1" /> Crear {etiqueta.toLowerCase()}
          </Button>
          {/* [159A-2/F2] Par Importar/Exportar. Las familias no tienen Importar:
           * el BDP no expone sus nombres (solo códigos en ExportArticles). */}
          <BdpImportExportButtons
            onImportar={importarDepartamentos}
            importando={importarDeptosMutation.isPending}
            importarTooltip={modoEfectivoBdp ? 'Importa departamentos desde BDP a la Aplicación Web. Crea los que falten y vincula por código; nunca pisa ediciones locales.' : 'Requiere BDP conectado (modo BDP).'}
            importarDeshabilitado={!modoEfectivoBdp}
            dominiosExportar={[tipo]}
            exportarTooltip={`Envía al BDP los ${tipo === 'departamento' ? 'departamentos' : 'familias'} locales pendientes en la cola.`}
            invalidarTrasExportar={[['bdp-catalogo', tipo]]}
            mostrarImportar={tipo === 'departamento'}
          />
        </div>
      </div>
      {tipo === 'familia' && (
        <p className="text-xs text-muted-foreground">
          Las familias se gestionan en la Aplicación Web: el BDP no publica sus nombres, solo acepta altas (Exportar al BDP).
        </p>
      )}

      <Input
        type="search"
        value={busqueda}
        onChange={(e) => { setBusqueda(e.target.value); setPagina(0); }}
        placeholder={`Buscar ${etiqueta.toLowerCase()} por código o nombre…`}
        maxLength={100}
        className="max-w-sm"
      />

      <Dialog open={crearOpen} onOpenChange={setCrearOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Crear {etiqueta.toLowerCase()}</DialogTitle>
            <DialogDescription>
              El código BDP se asigna automáticamente al crear. Con BDP conectado, el alta se empuja
              al terminal; sin BDP, queda local.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-2 py-2">
            <Label htmlFor="clasificacion-nombre">Nombre</Label>
            <Input
              id="clasificacion-nombre"
              value={nombre}
              onChange={(e) => setNombre(e.target.value)}
              placeholder={tipo === 'departamento' ? 'Ej: Cocina' : 'Ej: Bebidas'}
              maxLength={255}
            />
          </div>
          <DialogFooter>
            <DialogClose asChild>
              <Button variant="outline">Cancelar</Button>
            </DialogClose>
            <Button onClick={crear} disabled={crearMutation.isPending || !nombre.trim()}>
              <Plus className="size-3.5 mr-1" /> Crear
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando…</p>
      ) : filasVisibles.length > 0 ? (
        <>
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-32"><Tags className="size-3.5 inline mr-1" />Código BDP</TableHead>
                <TableHead>Nombre</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filasPagina.map((c) => (
                <TableRow key={c.id}>
                  <TableCell className="font-mono text-xs tabular-nums">{c.code}</TableCell>
                  <TableCell>{c.nombre}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
        {totalPaginas > 1 && (
          <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-sm">
            <span className="text-muted-foreground">
              Mostrando {filasPagina.length} de {filasVisibles.length}
              {q !== '' ? ` (filtrados de ${filas.length})` : ''} · página {paginaActual + 1} de {totalPaginas}
            </span>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={() => setPagina((p) => Math.max(0, p - 1))} disabled={paginaActual === 0}>
                Anterior
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPagina((p) => Math.min(totalPaginas - 1, p + 1))}
                disabled={paginaActual >= totalPaginas - 1}
              >
                Siguiente
              </Button>
            </div>
          </div>
        )}
        </>
      ) : (
        <p className="text-sm text-muted-foreground">
          {q !== ''
            ? `Sin resultados para «${busqueda.trim()}».`
            : `No hay ${tipo === 'departamento' ? 'departamentos' : 'familias'} registrados.`}
        </p>
      )}
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
