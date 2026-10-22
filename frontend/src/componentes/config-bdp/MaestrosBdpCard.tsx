/* [D24-p] Comprobación de terminal, empleado y perfil de artículos contra el BDP.
 *
 * Los tres se escriben a mano y un número inventado solo se descubre al enviar
 * una comanda. BDP publica sus listas reales, así que aquí se leen y se marca
 * el número configurado que no existe. No escribe nada: es solo lectura. */

import { Loader2, RefreshCw } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useMaestrosBdp, type MaestroBdp } from './useMaestrosBdp';

/* [D24-auto] Cadencias ofrecidas para releer solas las listas de maestros; la
 * de 15 min es la recomendada porque los terminales y perfiles del TPV cambian
 * muy de vez en cuando. */
const CADENCIAS = [0, 300, 900, 1800, 3600];

/** Segundos → texto para quien no piensa en segundos (mismo criterio que la
 * lectura de documentos de venta). */
function cadenciaTexto(segundos: number): string {
  if (segundos === 0) return 'No se leen solas';
  if (segundos % 3600 === 0) return `Cada ${segundos / 3600} h`;
  if (segundos % 60 === 0) return `Cada ${segundos / 60} min`;
  return `Cada ${segundos} s`;
}

interface Props {
  terminal: number;
  empleado: number;
  perfil: number;
  cadencia: number;
  onCadencia: (segundos: number) => void;
}

/** Qué se dice de un número configurado frente a la lista real de BDP. */
function estadoDe(valor: number, lista: MaestroBdp[]): { texto: string; roto: boolean } {
  const encontrado = lista.find((item) => item.id === valor);
  if (encontrado) return { texto: `→ ${encontrado.nombre || `id ${valor}`}`, roto: false };
  return { texto: '· no existe en BDP', roto: true };
}

export default function MaestrosBdpCard({ terminal, empleado, perfil, cadencia, onCadencia }: Props) {
  const { maestros, cargando, error, consultar } = useMaestrosBdp();

  const filas = maestros
    ? [
        { etiqueta: 'Terminal POS', valor: terminal, lista: maestros.terminales },
        { etiqueta: 'Empleado', valor: empleado, lista: maestros.empleados },
        { etiqueta: 'Perfil artículos', valor: perfil, lista: maestros.perfiles },
      ]
    : [];

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <Button type="button" variant="secondary" size="sm" onClick={() => void consultar()} disabled={cargando}>
          {cargando ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
          Comprobar contra BDP
        </Button>
        <span className="text-xs text-muted-foreground">
          Lee los terminales, empleados y perfiles reales del BDP y compara con los números de arriba.
        </span>
        {error && <span className="text-sm text-destructive">{error}</span>}
      </div>

      {/* [D24-auto] Con una cadencia distinta de cero el servidor relee estas
          listas y las guarda; la comprobación de arriba vale también sin pulsar. */}
      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1">
          <Label htmlFor="maestros-cadencia" className="text-xs text-muted-foreground">
            Lectura automática
          </Label>
          <Select value={String(cadencia)} onValueChange={(valor) => onCadencia(Number(valor))}>
            <SelectTrigger id="maestros-cadencia" className="w-44">
              <SelectValue>{cadenciaTexto(cadencia)}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              {[...new Set([...CADENCIAS, cadencia])].sort((a, b) => a - b).map((segundos) => (
                <SelectItem key={segundos} value={String(segundos)}>
                  {cadenciaTexto(segundos)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <span className="pb-2 text-xs text-muted-foreground">
          Cada cuánto relee el servidor los terminales, empleados y perfiles. Se guarda con el
          botón de guardar de arriba.
        </span>
      </div>

      {maestros && (
        <div className="flex flex-col gap-2">
          {filas.map((fila) => {
            const estado = estadoDe(fila.valor, fila.lista);
            return (
              <div key={fila.etiqueta} className="flex flex-wrap items-center gap-2 text-sm">
                <span className="text-muted-foreground">{fila.etiqueta}</span>
                <span className="font-mono tabular-nums">{fila.valor}</span>
                <Badge variant={estado.roto ? 'destructive' : 'secondary'}>{estado.texto}</Badge>
                <span className="text-xs text-muted-foreground">
                  {fila.lista.length} en BDP
                  {fila.lista.length > 0 &&
                    `: ${fila.lista
                      .slice(0, 5)
                      .map((item) => `${item.id} ${item.nombre}`.trim())
                      .join(', ')}${fila.lista.length > 5 ? '…' : ''}`}
                </span>
              </div>
            );
          })}
          {maestros.errores.length > 0 && (
            <ul className="text-xs text-destructive">
              {maestros.errores.map((mensaje) => (
                <li key={mensaje}>{mensaje}</li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
