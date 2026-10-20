/* [D32] Operaciones que BDP no expone por API: la web no las ejecuta, solo dice
 * quién las hace y cómo. Responsable y procedimiento se configuran aquí, uno por
 * operación, con un responsable global de respaldo: el cliente no tiene que
 * saberlo de antemano, lo rellena cuando lo decida. */

import { AlertTriangle, Check, Loader2, RotateCcw } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useOperacionesNoApi } from './useOperacionesNoApi';

export default function OperacionesNoApiCard() {
  const {
    operaciones,
    borradores,
    global,
    setGlobal,
    cargando,
    guardando,
    guardarGlobalSeleccionado,
    pendientes,
    editar,
    deshacer,
    guardar,
    guardarGlobal,
  } = useOperacionesNoApi();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Operaciones que BDP no expone por API</CardTitle>
        <CardDescription>
          Estas operaciones se hacen en el TPV, no desde la web. Aquí se decide quién las hace y
          cómo se avisa al personal: la web solo muestra ese texto.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-2 rounded-md border border-dashed p-3">
          <Label htmlFor="bdp-responsable-global">Responsable por defecto</Label>
          <div className="flex items-center gap-2">
            <Input
              id="bdp-responsable-global"
              value={global}
              placeholder="Sin asignar: cada operación usa el suyo"
              onChange={(evento) => setGlobal(evento.target.value)}
              className="max-w-md"
            />
            <Button
              type="button"
              variant="secondary"
              onClick={guardarGlobal}
              disabled={guardando !== null}
            >
              {guardarGlobalSeleccionado ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Check className="size-4" />
              )}
              Guardar
            </Button>
          </div>
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
              const sinCambios =
                borrador.responsable === fila.responsable &&
                borrador.procedimiento === fila.procedimiento;
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
                  <div className="flex items-center gap-2">
                    <Button
                      type="button"
                      size="sm"
                      onClick={() => guardar(fila.codigo)}
                      disabled={guardando !== null || sinCambios}
                    >
                      {guardando === fila.codigo ? (
                        <Loader2 className="size-4 animate-spin" />
                      ) : (
                        <Check className="size-4" />
                      )}
                      Guardar
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      onClick={() => deshacer(fila)}
                      disabled={guardando !== null || sinCambios}
                      title="Volver a los valores guardados"
                    >
                      <RotateCcw className="size-4" />
                      Deshacer
                    </Button>
                    <span className="text-xs text-muted-foreground">{fila.mensaje}</span>
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
