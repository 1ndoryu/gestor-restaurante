/* [147A-F6] Editor de líneas de venta multi-item.
 * Permite al usuario añadir N artículos/servicios a una venta.
 * Cada línea se mapea a un artículo BDP via articulo_codigo + bdp_article_map.
 * Responsive: mobile ≥320px (stacked), tablet/desktop (table inline). */

import { Plus, Trash2, Check, AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { LineaVentaLocal } from '../hooks/useLineasVenta';
import { useBdpArticleMaps, type BdpArticleMapItem } from '../api/bdp';
import ArticleAutocomplete from './article-autocomplete';

interface Props {
  lineas: LineaVentaLocal[];
  onAgregar: () => void;
  onEliminar: (id: string) => void;
  onActualizar: (id: string, campo: keyof Omit<LineaVentaLocal, 'id'>, valor: string) => void;
  totalBase: number;
  totalIva: number;
  totalConDescuento: number;
  readonly?: boolean;
}

function fmtEur(n: number): string {
  return n.toLocaleString('es-ES', { style: 'currency', currency: 'EUR' });
}

function BdpMappingBadge({ codigo, maps }: { codigo: string; maps: BdpArticleMapItem[] | undefined }) {
  const m = maps?.find(x => x.articulo_glory_codigo === codigo);
  if (!codigo) return <span className="text-muted-foreground text-[10px]">—</span>;
  if (m) return <span className="inline-flex items-center gap-0.5 text-[10px] text-green-600" title={`BDP: ${m.articulo_bdp_nombre}`}><Check className="h-3 w-3" />{m.articulo_bdp_codigo}</span>;
  return <span className="inline-flex items-center gap-0.5 text-[10px] text-amber-600" title="Sin mapeo BDP"><AlertTriangle className="h-3 w-3" />genérico</span>;
}

function LineaRow({ linea, idx, onActualizar, onEliminar, maps, readonly, puedeEliminar }: {
  linea: LineaVentaLocal; idx: number;
  onActualizar: (id: string, campo: keyof Omit<LineaVentaLocal, 'id'>, valor: string) => void;
  onEliminar: (id: string) => void;
  maps: BdpArticleMapItem[] | undefined; readonly?: boolean; puedeEliminar: boolean;
}) {
  const lbl = (s: string) => <span className="text-[10px] text-muted-foreground lg:hidden">{s}</span>;
  const inp = (campo: keyof Omit<LineaVentaLocal, 'id'>, tipo = 'text', extra?: object) => (
    <Input type={tipo} step={tipo === 'number' ? '0.01' : undefined} min={tipo === 'number' ? '0' : undefined}
      value={linea[campo]} onChange={e => onActualizar(linea.id, campo, e.target.value)}
      className="h-8 text-xs" readOnly={readonly} aria-label={`${campo} línea ${idx + 1}`} {...extra} />
  );
  return (
    <div className="grid grid-cols-1 gap-2 rounded-md border p-3 sm:grid-cols-2 lg:grid-cols-[minmax(140px,1fr)_minmax(0,1fr)_70px_90px_60px_60px_80px_32px] lg:items-center">
      <div className="flex flex-col gap-1 min-w-0">{lbl('Código')}<ArticleAutocomplete valor={linea.articulo_codigo} onSelect={v => onActualizar(linea.id, 'articulo_codigo', v)} readonly={readonly} /></div>
      <div className="flex flex-col gap-1 min-w-0">{lbl('Descripción')}{inp('descripcion')}</div>
      <div className="flex flex-col gap-1 min-w-0">{lbl('Cant.')}{inp('cantidad', 'number')}</div>
      <div className="flex flex-col gap-1 min-w-0">{lbl('Precio €')}{inp('precio_unitario', 'number', { placeholder: '0.00' })}</div>
      <div className="flex flex-col gap-1 min-w-0">{lbl('IVA %')}{inp('iva_pct', 'number')}</div>
      <div className="flex flex-col gap-1 min-w-0">{lbl('Dsct %')}{inp('descuento', 'number')}</div>
      <div className="flex flex-col gap-1">{lbl('BDP')}<BdpMappingBadge codigo={linea.articulo_codigo} maps={maps} /></div>
      <div className="flex items-center justify-end lg:justify-center">
        <Button type="button" variant="ghost" size="sm" onClick={() => onEliminar(linea.id)} disabled={readonly || !puedeEliminar} className="h-8 w-8 p-0 text-destructive hover:text-destructive" aria-label={`Eliminar línea ${idx + 1}`}>
          <Trash2 className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  );
}

export default function LineasVentaEditor({ lineas, onAgregar, onEliminar, onActualizar, totalBase, totalIva, totalConDescuento, readonly }: Props) {
  const { data: articleMaps } = useBdpArticleMaps();
  if (lineas.length === 0 && readonly) return null;
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <Label className="text-sm font-medium">Artículos / Líneas</Label>
        <Button type="button" variant="outline" size="sm" onClick={onAgregar} disabled={readonly} className="h-7 gap-1 text-xs">
          <Plus className="h-3 w-3" /> Añadir línea
        </Button>
      </div>
      <div className="hidden lg:grid lg:grid-cols-[minmax(140px,1fr)_minmax(0,1fr)_70px_90px_60px_60px_80px_32px] gap-2 text-[11px] font-medium text-muted-foreground">
        <span>Código</span><span>Descripción</span><span>Cant.</span><span>Precio €</span><span>IVA %</span><span>Dsct %</span><span>BDP</span><span />
      </div>
      {lineas.map((linea, idx) => (
        <LineaRow key={linea.id} linea={linea} idx={idx} onActualizar={onActualizar} onEliminar={onEliminar} maps={articleMaps} readonly={readonly} puedeEliminar={lineas.length > 0} />
      ))}
      {lineas.length > 0 && (
        <div className="flex flex-col items-end gap-1 rounded-md border bg-muted/30 p-3 text-sm">
          <div className="flex gap-4"><span className="text-muted-foreground">Base:</span><span className="font-medium">{fmtEur(totalBase)}</span></div>
          <div className="flex gap-4"><span className="text-muted-foreground">IVA:</span><span className="font-medium">{fmtEur(totalIva)}</span></div>
          <div className="flex gap-4 text-base font-semibold"><span>Total:</span><span>{fmtEur(totalConDescuento)}</span></div>
        </div>
      )}
    </div>
  );
}
