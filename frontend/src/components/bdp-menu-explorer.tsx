/* [237A-3] Explorador de menús, packs y fastfoods de BDP — solo lectura.
 * Consume los hooks generados por Orval para consultar la estructura
 * de menús, packs y modalidades de venta definidos en BDP.
 * No crea ni modifica registros. */

import { useState } from 'react';
import { Search, Loader2, ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
/* toast importado de sonner — nota: sonner es pre-existente en el proyecto pero puede no estar instalado aún */
import { toast } from 'sonner';
import {
  useGetMenuDefinition,
  useGetFastfoodDefinition,
  useGetPackDefinition,
} from '../api/generated/bdp-mapeos/bdp-mapeos';

/* Los tipos exactos no están en el schema Orval — se usan genéricos inline */
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

/* ========== Tipo de exploración ========== */

type ExploreType = 'menu' | 'fastfood' | 'pack';

const TIPOS: { value: ExploreType; label: string; desc: string }[] = [
  { value: 'menu', label: 'Menú', desc: 'Estructura de un menú definido en BDP.' },
  { value: 'fastfood', label: 'Fastfood', desc: 'Modalidad de venta rápida de BDP.' },
  { value: 'pack', label: 'Pack', desc: 'Pack agrupado de artículos en BDP.' },
];

/* ========== Renderizado de definición ========== */

function DefinitionDetail({ data }: { data: BdpDefinition }) {
  return (
    <div className="rounded-md border p-3 text-sm space-y-2">
      <p className="font-medium">{data.name ?? 'Sin nombre'}</p>
      {data.description && <p className="text-xs text-muted-foreground">{data.description}</p>}
      <div className="grid grid-cols-2 gap-2 text-xs">
        <span>Código: <Badge variant="outline">{data.code ?? '—'}</Badge></span>
        <span>Artículos: <Badge variant="secondary">{data.lines?.length ?? 0}</Badge></span>
      </div>
      {data.lines && data.lines.length > 0 && (
        <div className="mt-2">
          <p className="text-xs font-medium mb-1">Líneas:</p>
          <div className="space-y-1">
            {data.lines.map((line: BdpDefLine, i: number) => (
              <div key={i} className="flex items-center gap-2 text-xs rounded bg-muted/50 px-2 py-1">
                <ChevronRight className="size-3 text-muted-foreground" />
                <span>{line.article_name ?? line.article_code ?? `Línea ${i + 1}`}</span>
                {line.quantity != null && <Badge variant="outline" className="ml-auto">x{line.quantity}</Badge>}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/* ========== Componente principal ========== */

function BdpMenuExplorer() {
  const [tipo, setTipo] = useState<ExploreType>('menu');
  const [identificador, setIdentificador] = useState('');
  const [buscado, setBuscado] = useState(false);

  /* Hooks de consulta — solo se activan cuando hay búsqueda válida */
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

  /* Los hooks devuelven void cuando enabled=false, por eso se castea explícitamente */
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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Search className="size-4" />
          Explorar menús, packs y fastfoods
        </CardTitle>
        <CardDescription>
          Consulta la estructura de menús, packs y modalidades de venta definidos en BDP. Solo lectura — no modifica nada.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex gap-1">
            {TIPOS.map((t) => (
              <Button
                key={t.value}
                size="sm"
                variant={tipo === t.value ? 'default' : 'outline'}
                onClick={() => { setTipo(t.value); setBuscado(false); }}
              >
                {t.label}
              </Button>
            ))}
          </div>
          <div className="flex flex-col gap-1">
            <Label htmlFor="bdp-menu-id" className="text-xs">Código numérico</Label>
            <Input
              id="bdp-menu-id"
              type="number"
              min={1}
              className="w-36"
              value={identificador}
              onChange={(e) => { setIdentificador(e.target.value); setBuscado(false); }}
              placeholder="ej: 1"
            />
          </div>
          <Button size="sm" onClick={buscar} disabled={isLoading}>
            {isLoading ? <Loader2 className="size-3.5 animate-spin mr-1" /> : <Search className="size-3.5 mr-1" />}
            Consultar
          </Button>
        </div>

        <p className="text-xs text-muted-foreground">{TIPOS.find(t => t.value === tipo)?.desc}</p>

        {error && (
          <p className="text-sm text-destructive">
            {(error as { response?: { data?: { message?: string } } })?.response?.data?.message ?? 'Error al consultar. Verifica que el código existe en BDP y que la integración está activa.'}
          </p>
        )}

        {buscado && !isLoading && !error && !menuData && !fastfoodData && !packData && (
          <p className="text-sm text-muted-foreground">No se encontró un {tipo} con código {identificador}.</p>
        )}

        {menuData && <DefinitionDetail data={menuData} />}
        {fastfoodData && <DefinitionDetail data={fastfoodData} />}
        {packData && <DefinitionDetail data={packData} />}
      </CardContent>
    </Card>
  );
}

export default BdpMenuExplorer;
