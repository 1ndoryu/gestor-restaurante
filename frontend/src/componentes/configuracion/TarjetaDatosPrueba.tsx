/* [D33] Datos de prueba: recargar o borrar los datos demo. Sale de «A qué se
 * conecta» —no son ajustes de conexión— y de General: el borrado toca datos y
 * necesita su propio aviso, no compartir tarjeta con interruptores inofensivos. */

import { useState } from 'react';
import { AlertTriangle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { useBdpDemoMode } from '../../hooks/useBdpDemoMode';
import { GuardadoAlInstante } from './CardSeccion';

interface TarjetaDatosPruebaProps {
  operandoSeed: boolean;
  ejecutarOperacion: (endpoint: string, descripcion: string) => Promise<void>;
}

export function TarjetaDatosPrueba({ operandoSeed, ejecutarOperacion }: TarjetaDatosPruebaProps) {
  const { demoMode, setDemoMode } = useBdpDemoMode();
  const [confirmarBorrado, setConfirmarBorrado] = useState(false);

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Datos de ejemplo</CardTitle>
          <CardDescription>Datos ficticios para probar la app sin tocar los reales</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex items-center justify-between gap-4 rounded-md border p-3">
            <div>
              <p className="text-sm font-medium">Ver las pantallas BDP con datos de ejemplo</p>
              <p className="text-xs text-muted-foreground">
                Stock, compras, explorador e historial muestran valores ficticios. No se envía nada al TPV.
              </p>
            </div>
            <Button
              variant={demoMode ? 'default' : 'outline'}
              onClick={() => setDemoMode(!demoMode)}
            >
              {demoMode ? 'Quitar los datos de ejemplo' : 'Ver con datos de ejemplo'}
            </Button>
          </div>
          <GuardadoAlInstante />
          <div className="border-t pt-4">
            <p className="mb-1 text-sm font-medium">Contenido de prueba de la cuenta</p>
            <p className="mb-3 text-xs text-muted-foreground">
              «Recargar» deja la cuenta con los datos demo otra vez. «Borrar» vacía los datos y
              conserva la cuenta y el acceso.
            </p>
            <div className="flex flex-wrap gap-3">
              <Button
                variant="outline"
                disabled={operandoSeed}
                onClick={() => ejecutarOperacion('admin/seed', 'Datos de prueba cargados')}
              >
                {operandoSeed ? 'Procesando…' : 'Recargar datos de prueba'}
              </Button>
              <Button variant="destructive" disabled={operandoSeed} onClick={() => setConfirmarBorrado(true)}>
                Borrar todos los datos
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Borrar no se deshace: el aviso dice qué queda (la cuenta) y qué no. */}
      <Dialog open={confirmarBorrado} onOpenChange={setConfirmarBorrado}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertTriangle className="size-4 text-destructive" />
              Borrar todos los datos
            </DialogTitle>
            <DialogDescription>
              Se borran reservas, ventas, clientes, artículos y comandas de esta cuenta. La cuenta y
              la contraseña se conservan. Esta acción no se puede deshacer.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirmarBorrado(false)}>
              Cancelar
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                setConfirmarBorrado(false);
                void ejecutarOperacion('admin/reset', 'Datos eliminados');
              }}
            >
              Sí, borrar los datos
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
