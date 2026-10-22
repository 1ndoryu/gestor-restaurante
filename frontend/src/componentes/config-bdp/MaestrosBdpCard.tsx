/* [D24-p] Comprobación de terminal, empleado y perfil de artículos contra el BDP.
 *
 * Los tres se escriben a mano y un número inventado solo se descubre al enviar
 * una comanda. BDP publica sus listas reales, así que aquí se leen y se marca
 * el número configurado que no existe. No escribe nada: es solo lectura. */

import { Loader2, RefreshCw } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useMaestrosBdp, type MaestroBdp } from './useMaestrosBdp';

interface Props {
  terminal: number;
  empleado: number;
  perfil: number;
}

/** Qué se dice de un número configurado frente a la lista real de BDP. */
function estadoDe(valor: number, lista: MaestroBdp[]): { texto: string; roto: boolean } {
  const encontrado = lista.find((item) => item.id === valor);
  if (encontrado) return { texto: `→ ${encontrado.nombre || `id ${valor}`}`, roto: false };
  return { texto: '· no existe en BDP', roto: true };
}

export default function MaestrosBdpCard({ terminal, empleado, perfil }: Props) {
  const { maestros, cargando, error, consultar } = useMaestrosBdp();

  const filas = maestros
    ? [
        { etiqueta: 'Terminal POS', valor: terminal, lista: maestros.terminales },
        { etiqueta: 'Empleado', valor: empleado, lista: maestros.empleados },
        { etiqueta: 'Perfil artículos', valor: perfil, lista: maestros.perfiles },
      ]
    : [];

  return (
    <div className="flex flex-col gap-3 rounded-md border border-dashed p-3">
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
