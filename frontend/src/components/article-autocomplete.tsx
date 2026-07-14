/* [147A-F6] Autocomplete selector de artículos BDP.
 * Extraído de lineas-venta-editor.tsx para cumplir límite de 300 líneas.
 * Busca artículos por código/nombre en bdp_article_map. */

import { useState, useRef, useEffect, useMemo } from 'react';
import { ChevronDown } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useBdpArticleMaps, type BdpArticleMapItem } from '../api/bdp';

interface Props {
  valor: string;
  onSelect: (codigo: string) => void;
  readonly?: boolean;
}

export default function ArticleAutocomplete({ valor, onSelect, readonly }: Props) {
  const [busqueda, setBusqueda] = useState('');
  const [abierto, setAbierto] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const { data: maps } = useBdpArticleMaps();

  useEffect(() => {
    function handler(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setAbierto(false);
    }
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  const resultados = useMemo(() => {
    if (!busqueda.trim() || !maps) return [];
    const q = busqueda.toLowerCase();
    return maps
      .filter(
        m =>
          m.articulo_glory_codigo.toLowerCase().includes(q) ||
          m.articulo_bdp_nombre.toLowerCase().includes(q) ||
          m.articulo_bdp_codigo.toLowerCase().includes(q),
      )
      .slice(0, 10);
  }, [busqueda, maps]);

  const seleccionActual = maps?.find(m => m.articulo_glory_codigo === valor);

  return (
    <div ref={ref} className="relative">
      <div className="flex gap-1">
        <Input
          value={abierto ? busqueda : (seleccionActual?.articulo_glory_codigo ?? valor)}
          onChange={e => {
            setBusqueda(e.target.value);
            setAbierto(true);
            onSelect(e.target.value);
          }}
          onFocus={() => {
            setBusqueda(valor || '');
            setAbierto(true);
          }}
          placeholder="Código artículo"
          className="h-8 text-xs"
          readOnly={readonly}
          aria-label="Código del artículo"
        />
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0 shrink-0"
          onClick={() => {
            setBusqueda(valor || '');
            setAbierto(!abierto);
          }}
          aria-label="Buscar artículo"
          tabIndex={-1}
        >
          <ChevronDown className="h-3 w-3" />
        </Button>
      </div>
      {abierto && resultados.length > 0 && (
        <div className="absolute z-50 mt-1 max-h-48 w-64 overflow-y-auto rounded-md border bg-popover shadow-md">
          {resultados.map((m: BdpArticleMapItem) => (
            <button
              key={m.id}
              type="button"
              className="flex w-full flex-col px-3 py-2 text-left text-xs hover:bg-accent"
              onClick={() => {
                onSelect(m.articulo_glory_codigo);
                setAbierto(false);
                setBusqueda('');
              }}
            >
              <span className="font-medium">{m.articulo_glory_codigo}</span>
              <span className="text-muted-foreground truncate">
                BDP: {m.articulo_bdp_nombre} ({m.articulo_bdp_codigo})
              </span>
            </button>
          ))}
        </div>
      )}
      {abierto && busqueda.trim() && resultados.length === 0 && (
        <div className="absolute z-50 mt-1 w-64 rounded-md border bg-popover px-3 py-2 text-xs text-muted-foreground shadow-md">
          Sin resultados — escriba el código manualmente
        </div>
      )}
    </div>
  );
}
