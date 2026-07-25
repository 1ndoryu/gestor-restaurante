/* [BDP-EXPLORER-01] Página de explorador de menús, packs y fastfoods BDP.
 * Migración del componente embebido a una página propia con mejor UX. */

import { useState } from 'react';
import { Search, Loader2, ArrowLeft } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { toast } from 'sonner';
import {
  useGetMenuDefinition,
  useGetFastfoodDefinition,
  useGetPackDefinition,
} from '@/api/generated/bdp-mapeos/bdp-mapeos';

type ExploreType = 'menu' | 'fastfood' | 'pack';

const TIPOS: { value: ExploreType; label: string; desc: string }[] = [
  { value: 'menu', label: 'Menú', desc: 'Estructura de un menú definido en BDP.' },
  { value: 'fastfood', label: 'Fastfood', desc: 'Modalidad de venta rápida de BDP.' },
  { value: 'pack', label: 'Pack', desc: 'Pack agrupado de artículos en BDP.' },
];

interface BdpDefLine {
  article_code?: string | number;
  article_name?: string;
  quantity?: number;
}

interface BdpDefinition {
  code?: string | number;
  name?: string;
  description?: string;
  lines?: BdpDefLine[];
}

function DefinitionDetail({ data, tipo }: { data: BdpDefinition; tipo: ExploreType }) {
  return (
    <div className="rounded-md border p-3 text-sm space-y-3">
      <div className="flex items-center gap-2">
        <span className="font-medium">{data.name ?? 'Sin nombre'}</span>
        <Badge variant="outline">{TIPOS.find((t) => t.value === tipo)?.label ?? tipo}</Badge>
      </div>
      {data.description && <p className="text-xs text-muted-foreground">{data.description}</p>}
      <div className="grid grid-cols-2 gap-2 text-xs">
        <span>
          Código: <Badge variant="outline">{data.code ?? '—'}</Badge>
        </span>
        <span>
          Artículos: <Badge variant="secondary">{data.lines?.length ?? 0}</Badge>
        </span>
      </div>
      {data.lines && data.lines.length > 0 && (
        <div className="rounded-md border overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="text-xs">Artículo</TableHead>
                <TableHead className="text-xs">Código</TableHead>
                <TableHead className="text-xs text-right">Cantidad</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.lines.map((line: BdpDefLine, i: number) => (
                <TableRow key={i}>
                  <TableCell className="text-xs">{line.article_name ?? '—'}</TableCell>
                  <TableCell className="font-mono text-xs">{line.article_code ?? '—'}</TableCell>
                  <TableCell className="text-xs text-right">{line.quantity ?? '—'}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}

function BdpExplorador() {
  const navigate = useNavigate();
  const [tipo, setTipo] = useState<ExploreType>('menu');
  const [identificador, setIdentificador] = useState('');
  const [buscado, setBuscado] = useState(false);

  const idNum = Number(identificador);
  const idValido = Number.isInteger(idNum) && idNum > 0;

  const menuQuery = useGetMenuDefinition(idNum, {
    query: { enabled: buscado && tipo === 'menu' && idValido },
  });
  const fastfoodQuery = useGetFastfoodDefinition(idNum, {
    query: { enabled: buscado && tipo === 'fastfood' && idValido },
  });
  const packQuery = useGetPackDefinition(idNum, {
    query: { enabled: buscado && tipo === 'pack' && idValido },
  });

  const isLoading = menuQuery.isLoading || fastfoodQuery.isLoading || packQuery.isLoading;
  const error = menuQuery.error || fastfoodQuery.error || packQuery.error;

  const menuData = menuQuery.data && (menuQuery.data as { status?: number }).status === 200 ? (menuQuery.data as { data: BdpDefinition }).data : null;
  const fastfoodData = fastfoodQuery.data && (fastfoodQuery.data as { status?: number }).status === 200 ? (fastfoodQuery.data as { data: BdpDefinition }).data : null;
  const packData = packQuery.data && (packQuery.data as { status?: number }).status === 200 ? (packQuery.data as { data: BdpDefinition }).data : null;

  function buscar() {
    if (!idValido) {
      toast.warning('Introduce un código numérico válido');
      return;
    }
    setBuscado(true);
  }

  return (
    <div className="space-y-4 p-4 md:p-6">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="icon" onClick={() => navigate('/configuracion')}>
          <ArrowLeft className="size-4" />
        </Button>
        <h1 className="text-xl font-semibold">Explorador BDP</h1>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Search className="size-4" />
            Explorar menús, packs y fastfoods
          </CardTitle>
          <CardDescription>
            Consulta la estructura de menús, packs y modalidades de venta definidos en BDP. Solo lectura.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap items-end gap-3">
            <div className="flex gap-1">
              {TIPOS.map((t) => (
                <Button
                  key={t.value}
                  size="sm"
                  variant={tipo === t.value ? 'default' : 'outline'}
                  onClick={() => {
                    setTipo(t.value);
                    setBuscado(false);
                  }}
                >
                  {t.label}
                </Button>
              ))}
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="bdp-explorador-id" className="text-xs">
                Código numérico
              </Label>
              <Input
                id="bdp-explorador-id"
                type="number"
                min={1}
                className="w-36"
                value={identificador}
                onChange={(e) => {
                  setIdentificador(e.target.value);
                  setBuscado(false);
                }}
                placeholder="ej: 1"
              />
            </div>
            <Button size="sm" onClick={buscar} disabled={isLoading}>
              {isLoading ? <Loader2 className="size-3.5 animate-spin mr-1" /> : <Search className="size-3.5 mr-1" />}
              Consultar
            </Button>
          </div>

          <p className="text-xs text-muted-foreground">{TIPOS.find((t) => t.value === tipo)?.desc}</p>

          {error && (
            <p className="text-sm text-destructive">
              {(
                (error as { response?: { data?: { message?: string } } }).response?.data?.message ??
                'Error al consultar. Verifica que el código existe en BDP y que la integración está activa.'
              )}
            </p>
          )}

          {buscado && !isLoading && !error && !menuData && !fastfoodData && !packData && (
            <p className="text-sm text-muted-foreground">
              No se encontró un {tipo} con código {identificador}.
            </p>
          )}

          {menuData && <DefinitionDetail data={menuData} tipo="menu" />}
          {fastfoodData && <DefinitionDetail data={fastfoodData} tipo="fastfood" />}
          {packData && <DefinitionDetail data={packData} tipo="pack" />}
        </CardContent>
      </Card>
    </div>
  );
}

export default BdpExplorador;
