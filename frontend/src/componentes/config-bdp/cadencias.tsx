/* [D24-auto-2] Cadencias de lectura automática del BDP, en un solo sitio.
 *
 * Las usan la comprobación de terminales/empleados/perfiles y los tres maestros
 * que antes solo se refrescaban con botón (clientes, plano de sala y
 * departamentos). Una decisión, una fuente: si mañana cambian los tramos
 * ofrecidos, cambian para todas. */

import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

/** Tramos ofrecidos. 15 min es la recomendación: estos maestros cambian muy de
 * vez en cuando y cada lectura es una llamada al TPV. */
// `as const`: los importadores no pueden mutar el tramo compartido.
export const CADENCIAS = [0, 300, 900, 1800, 3600] as const;

/** Segundos → texto para quien no piensa en segundos. */
export function cadenciaTexto(segundos: number): string {
  if (segundos === 0) return 'No se leen solas';
  if (segundos % 3600 === 0) return `Cada ${segundos / 3600} h`;
  if (segundos % 60 === 0) return `Cada ${segundos / 60} min`;
  return `Cada ${segundos} s`;
}

interface Props {
  id: string;
  etiqueta: string;
  segundos: number;
  onCambiar: (segundos: number) => void;
}

/** Un selector de cadencia. El valor guardado se ofrece siempre aunque no esté
 * en los tramos, para no cambiar en silencio lo que ya tenía la cuenta. */
export function SelectorCadencia({ id, etiqueta, segundos, onCambiar }: Props) {
  return (
    <div className="flex flex-col gap-1">
      <Label htmlFor={id} className="text-xs text-muted-foreground">
        {etiqueta}
      </Label>
      <Select value={String(segundos)} onValueChange={(valor) => onCambiar(Number(valor))}>
        <SelectTrigger id={id} className="w-44">
          <SelectValue>{cadenciaTexto(segundos)}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          {[...new Set([...CADENCIAS, segundos])].sort((a, b) => a - b).map((valor) => (
            <SelectItem key={valor} value={String(valor)}>
              {cadenciaTexto(valor)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
