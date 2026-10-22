/* [D24-m] Formas de pago del BDP frente al mapeo configurado.
 *
 * `bdp_tender_map` traduce método de pago de la web → `TenderId` de BDP. Si el
 * id no existe en BDP, el fallo solo aparece al cobrar; aquí se comparan los
 * dos lados y se marca lo que no cuadra. */

import { CreditCard, Loader2, RefreshCw } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useFormasPagoBdp } from './useFormasPagoBdp';
import { AyudaTarjeta } from '../configuracion/AyudaTarjeta';

/** Parejas `método → id` del mapa; vacío si el texto no es un JSON de objetos. */
function parejasDelMapa(texto: string): Array<{ metodo: string; id: number }> {
  try {
    const valor: unknown = JSON.parse(texto || '{}');
    if (typeof valor !== 'object' || valor === null) return [];
    return Object.entries(valor as Record<string, unknown>).flatMap(([metodo, codigo]) => {
      const id = typeof codigo === 'number' ? codigo : Number(codigo);
      return Number.isInteger(id) ? [{ metodo, id }] : [];
    });
  } catch {
    return [];
  }
}

export default function FormasPagoBdpCard({ mapa }: { mapa: string }) {
  const { formas, cargando, error, consultar } = useFormasPagoBdp();
  const parejas = parejasDelMapa(mapa);
  const idsBdp = new Set((formas ?? []).map((f) => f.id));

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <CreditCard className="size-4" />
          Formas de pago del BDP
        </CardTitle>
        <CardDescription>
          Las formas de pago dadas de alta en BDP y cómo se traduce a ellas cada método de pago de
          la web. Un código que no exista en BDP hace fallar el cobro.
        </CardDescription>
        <CardAction>
          <AyudaTarjeta id="bdp-formas-pago" titulo="Formas de pago del BDP" />
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {parejas.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            El mapeo de formas de pago está vacío: los cobros se enviarán sin `TenderId`.
          </p>
        ) : (
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <span className="text-muted-foreground">Mapeo actual:</span>
            {parejas.map(({ metodo, id }) => (
              <Badge key={metodo} variant={formas && !idsBdp.has(id) ? 'destructive' : 'secondary'}>
                {metodo} → {id}
                {formas && !idsBdp.has(id) ? ' · no existe en BDP' : ''}
              </Badge>
            ))}
          </div>
        )}

        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="secondary" onClick={() => void consultar()} disabled={cargando}>
            {cargando ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
            Consultar BDP
          </Button>
          {error && <span className="text-sm text-destructive">{error}</span>}
        </div>

        {formas && (
          <div className="flex flex-col divide-y rounded-md border">
            {formas.length === 0 ? (
              <p className="p-3 text-sm text-muted-foreground">
                El BDP no devolvió ninguna forma de pago.
              </p>
            ) : (
              formas.map((forma) => (
                <div key={forma.id} className="flex items-center gap-2 p-3 text-sm">
                  <span className="font-mono tabular-nums">{forma.id}</span>
                  <span>{forma.nombre || '(sin nombre)'}</span>
                  {forma.tipo !== null && (
                    <span className="ml-auto text-xs text-muted-foreground">tipo {forma.tipo}</span>
                  )}
                </div>
              ))
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
