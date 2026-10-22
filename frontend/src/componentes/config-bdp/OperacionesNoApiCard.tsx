/* [D32] Operaciones que BDP no expone por API: la web no las ejecuta, solo dice
 * quién las hace y cómo. Responsable y procedimiento se configuran aquí, uno por
 * operación, con un responsable global de respaldo: el cliente no tiene que
 * saberlo de antemano, lo rellena cuando lo decida.
 * [D33] El hook entra de fuera y la tarjeta no trae botones: la pestaña
 * Conexiones tiene un único «Guardar» abajo a la derecha, y seis botones más
 * dentro de la tarjeta eran justo la incoherencia que se estaba corrigiendo. */

import { AlertTriangle, Loader2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { useOperacionesNoApi } from './useOperacionesNoApi';
import { AyudaTarjeta } from '../configuracion/AyudaTarjeta';

interface Props {
  operacionesNoApi: ReturnType<typeof useOperacionesNoApi>;
}

export default function OperacionesNoApiCard({ operacionesNoApi }: Props) {
  const { operaciones, borradores, global, setGlobal, cargando, pendientes, editar } =
    operacionesNoApi;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Operaciones que BDP no expone por API</CardTitle>
        <CardDescription>
          Estas operaciones se hacen en el TPV, no desde la web. Aquí se decide quién las hace y
          cómo se avisa al personal: la web solo muestra ese texto.
        </CardDescription>
        <CardAction>
          <AyudaTarjeta id="bdp-operaciones-no-api" titulo="Operaciones que BDP no expone por API" />
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-responsable-global">Responsable por defecto</Label>
          <Input
            id="bdp-responsable-global"
            value={global}
            placeholder="Sin asignar: cada operación usa el suyo"
            onChange={(evento) => setGlobal(evento.target.value)}
            className="max-w-md"
          />
          <p className="text-xs text-muted-foreground">
            Se usa en las operaciones que no tengan responsable propio. Si tampoco hay uno aquí, la
            web avisa de que está pendiente de asignar.
          </p>
        </div>

        {pendientes > 0 && (
          <div className="flex items-start gap-2 rounded-md border border-amber-500 bg-amber-50 p-3 text-sm dark:bg-amber-950/20">
            <AlertTriangle className="mt-0.5 size-4 text-amber-600" />
            <span>
              {pendientes === 1
                ? 'Una operación sigue sin responsable.'
                : `${pendientes} operaciones siguen sin responsable.`}
            </span>
          </div>
        )}

        {cargando ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Cargando operaciones...
          </div>
        ) : (
          <div className="flex flex-col divide-y rounded-md border">
            {operaciones.map((fila) => {
              const borrador = borradores[fila.codigo] ?? { responsable: '', procedimiento: '' };
              return (
                <div key={fila.codigo} className="flex flex-col gap-2 p-3">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">{fila.operacion}</span>
                    {fila.responsable_pendiente && (
                      <Badge variant="outline" className="border-amber-500 text-amber-700">
                        Sin responsable
                      </Badge>
                    )}
                  </div>
                  <div className="grid gap-2 md:grid-cols-2">
                    <Input
                      value={borrador.responsable}
                      placeholder={`Responsable (por defecto: ${global || 'sin asignar'})`}
                      onChange={(evento) => editar(fila.codigo, 'responsable', evento.target.value)}
                    />
                    <Input
                      value={borrador.procedimiento}
                      placeholder="Procedimiento (por defecto: hazla en el TPV)"
                      onChange={(evento) => editar(fila.codigo, 'procedimiento', evento.target.value)}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
