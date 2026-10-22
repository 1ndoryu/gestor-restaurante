/* [D33] Editor de un mapa «origen → número» sin pedir JSON. Los mapas de BDP
 * (formas de pago, canales, IVA) se guardan como objeto JSON en la configuración;
 * antes había que escribirlo a mano, con llaves y comas, y un carácter mal puesto
 * dejaba el ajuste inservible. Aquí se editan filas y el JSON se genera. */

import { useEffect, useRef, useState } from 'react';
import { Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface MapaParejasProps {
  id: string;
  etiqueta: string;
  ayuda?: string;
  /** Texto JSON del mapa tal y como vive en la configuración. */
  valor: string;
  onCambiar: (json: string) => void;
  /** Texto de la columna izquierda («Forma de pago», «IVA %»). */
  columnaClave: string;
  /** Texto de la columna derecha («Código BDP»). */
  columnaValor: string;
}

interface Fila {
  /** Identidad estable de la fila: sin ella, borrar una fila intermedia hace
   * que React reaproveche el input de la siguiente y el foco salte. */
  id: number;
  clave: string;
  valor: string;
}

let siguienteId = 0;

/** Convierte el JSON guardado en filas. Un valor corrupto se conserva como fila
 * vacía en vez de perderse: quien lo editó ve lo que había. */
function aFilas(json: string): Fila[] {
  try {
    const valor: unknown = JSON.parse(json);
    if (typeof valor !== 'object' || valor === null) return [];
    return Object.entries(valor as Record<string, unknown>).map(([clave, codigo]) => ({
      id: siguienteId++,
      clave,
      valor: String(codigo),
    }));
  } catch {
    return [];
  }
}

/** Filas → JSON. Las incompletas se descartan sin romper el resto. */
function aJson(filas: readonly Fila[]): string {
  const mapa: Record<string, number> = {};
  for (const { clave, valor } of filas) {
    const numero = Number(valor);
    if (clave.trim() === '' || valor.trim() === '' || !Number.isFinite(numero)) continue;
    mapa[clave.trim()] = numero;
  }
  return JSON.stringify(mapa, null, 2);
}

export function MapaParejas({
  id,
  etiqueta,
  ayuda,
  valor,
  onCambiar,
  columnaClave,
  columnaValor,
}: MapaParejasProps) {
  /* Las filas viven aquí y no se recalculan del JSON en cada render: una fila
   * recién añadida está vacía y no existe en el JSON, así que derivarla del
   * valor la borraría al instante. Se resincroniza solo si el valor cambió
   * fuera (recarga, otro formulario). */
  const [filas, setFilas] = useState<Fila[]>(() => aFilas(valor));
  const emitido = useRef(valor);
  useEffect(() => {
    if (valor === emitido.current) return;
    emitido.current = valor;
    setFilas(aFilas(valor));
  }, [valor]);

  const aplicar = (siguiente: Fila[]) => {
    setFilas(siguiente);
    const json = aJson(siguiente);
    emitido.current = json;
    onCambiar(json);
  };
  const editar = (id: number, campo: 'clave' | 'valor', texto: string) => {
    aplicar(filas.map((fila) => (fila.id === id ? { ...fila, [campo]: texto } : fila)));
  };

  return (
    <div className="flex flex-col gap-2">
      <Label htmlFor={`${id}-0-clave`}>{etiqueta}</Label>
      {ayuda && <p className="text-xs text-muted-foreground">{ayuda}</p>}
      {filas.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          Sin parejas todavía. Añade la primera con el botón de abajo.
        </p>
      ) : (
        <div className="flex flex-col gap-2">
          {filas.map((fila, indice) => (
            <div key={fila.id} className="flex items-center gap-2">
              <Input
                id={`${id}-${indice}-clave`}
                aria-label={`${columnaClave} de la fila ${indice + 1}`}
                value={fila.clave}
                onChange={(e) => editar(fila.id, 'clave', e.target.value)}
                placeholder={columnaClave}
                className="max-w-52"
              />
              <span className="text-muted-foreground">→</span>
              <Input
                aria-label={`${columnaValor} de la fila ${indice + 1}`}
                inputMode="numeric"
                value={fila.valor}
                onChange={(e) => editar(fila.id, 'valor', e.target.value)}
                placeholder={columnaValor}
                className="max-w-28"
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={`Quitar la fila ${indice + 1}`}
                onClick={() => aplicar(filas.filter((otra) => otra.id !== fila.id))}
              >
                <Trash2 className="size-4" />
              </Button>
            </div>
          ))}
        </div>
      )}
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="self-start"
        onClick={() => aplicar([...filas, { id: siguienteId++, clave: '', valor: '' }])}
      >
        <Plus className="size-4 mr-1" />
        Añadir pareja
      </Button>
    </div>
  );
}

/** JSON en crudo para quien sabe lo que hace; siempre detrás de un desplegable. */
export function MapaJsonCrudo({
  id,
  etiqueta,
  valor,
  onCambiar,
}: {
  id: string;
  etiqueta: string;
  valor: string;
  onCambiar: (json: string) => void;
}) {
  return (
    <details className="rounded-md border p-3">
      <summary className="cursor-pointer text-sm text-muted-foreground">{etiqueta}</summary>
      <textarea
        id={id}
        value={valor}
        onChange={(e) => onCambiar(e.target.value)}
        rows={4}
        spellCheck={false}
        aria-label={etiqueta}
        className="mt-3 w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs"
      />
    </details>
  );
}
