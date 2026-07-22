/* [263A-16] Lista de canales — reescrita con shadcn Table + Dialog + Input.
 * CRUD canales de reserva (WhatsApp, Instagram, teléfono, web, etc.) */

import { useState } from 'react';
import { useListarCanales, useCrearCanal, useEliminarCanal, getListarCanalesQueryKey } from '../api/generated';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Input } from '@/components/ui/input';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Label } from '@/components/ui/label';
import { Trash2 } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';

function ListaCanales() {
  const [modalCrear, setModalCrear] = useState(false);
  const [nombre, setNombre] = useState('');
  const queryClient = useQueryClient();

  const { data: canalesResp, isLoading } = useListarCanales();
  const canales = canalesResp?.status === 200 ? canalesResp.data : [];

  const crearMut = useCrearCanal({
    mutation: {
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: getListarCanalesQueryKey() });
        setNombre('');
        setModalCrear(false);
      },
    },
  });

  const eliminarMut = useEliminarCanal({
    mutation: {
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: getListarCanalesQueryKey() });
      },
    },
  });

  const handleCrear = () => {
    const nombreLimpio = nombre.trim();
    if (!nombreLimpio) return;
    crearMut.mutate({ data: { nombre: nombreLimpio } });
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">{canales.length} canales configurados</p>
        <Button onClick={() => setModalCrear(true)}>+ Nuevo Canal</Button>
      </div>

      <Dialog open={modalCrear} onOpenChange={(open) => { setModalCrear(open); if (!open) setNombre(''); }}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Nuevo Canal</DialogTitle>
          </DialogHeader>
          <form onSubmit={(e) => { e.preventDefault(); handleCrear(); }} className="flex flex-col gap-4">
            <div className="flex flex-col gap-2">
              <Label>Nombre del canal</Label>
              <Input
                placeholder="Ej: WhatsApp, Instagram, Teléfono, Web..."
                value={nombre}
                onChange={(e) => setNombre(e.target.value)}
                autoFocus
              />
            </div>
            <Button type="submit" disabled={crearMut.isPending || !nombre.trim()}>
              {crearMut.isPending ? 'Creando...' : 'Crear Canal'}
            </Button>
          </form>
        </DialogContent>
      </Dialog>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : canales.length > 0 ? (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Canal</TableHead>
                <TableHead>Fecha de creación</TableHead>
                <TableHead className="w-10"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {canales.map((c) => (
                <TableRow key={c.id}>
                  <TableCell className="font-medium">{c.nombre}</TableCell>
                  <TableCell>{new Date(c.created_at).toLocaleDateString('es-ES')}</TableCell>
                  <TableCell>
                    <TooltipButton
                      variant="ghost"
                      size="icon"
                      onClick={() => eliminarMut.mutate({ id: c.id })}
                      disabled={eliminarMut.isPending}
                      tooltip="Eliminar canal"
                    >
                      <Trash2 className="size-4 text-destructive" />
                    </TooltipButton>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">No hay canales configurados. Crea uno para clasificar por dónde llegan las reservas.</p>
      )}

      {/* [303A-7] Aviso sobre integración con asistente inteligente */}
      <div className="rounded-md border border-blue-200 bg-blue-50 p-3 text-sm text-blue-800 dark:border-blue-800 dark:bg-blue-950 dark:text-blue-200">
        <strong>Nota:</strong> Si añade un canal nuevo y desea que el asistente inteligente registre automáticamente las reservas que lleguen a través de ese canal, por favor notifíquenoslo para configurarlo.
      </div>
    </div>
  );
}

export default ListaCanales;
