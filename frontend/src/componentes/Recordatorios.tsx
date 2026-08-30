/* [263A-25] Gestión de recordatorios automáticos de reservas.
 * Permite configurar reglas (horas antes, canal, mensaje) y ver historial.
 * Las reglas se pueden activar/desactivar con un switch. */

import { useRecordatorios } from '../hooks/useRecordatorios';
import TablaHistorial from './TablaHistorial';
import NuevaReglaDialog, { badgeCanal, formatHoras } from './NuevaReglaDialog';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Trash2, ChevronLeft, ChevronRight } from 'lucide-react';
import type { ReglaRecordatorio } from '../api/generated/gestionRestauranteAPI.schemas';

function TablaReglas() {
  const {
    reglas,
    total,
    page,
    totalPages,
    isLoading,
    setPage,
    crearRegla,
    eliminarRegla,
    toggleActiva,
  } = useRecordatorios();

  const handleEliminar = async (id: string) => {
    await eliminarRegla({ id });
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <span className="text-sm text-muted-foreground">{total} regla(s)</span>
        <NuevaReglaDialog onCrear={crearRegla} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Reglas de Recordatorio</CardTitle>
          <CardDescription>
            Cada regla define cuándo enviar un recordatorio automático antes de la reserva
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <p className="text-muted-foreground py-8 text-center">Cargando...</p>
          ) : reglas.length === 0 ? (
            <p className="text-muted-foreground py-8 text-center">
              No hay reglas configuradas. Crea tu primera regla de recordatorio.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Nombre</TableHead>
                  <TableHead>Tiempo</TableHead>
                  <TableHead>Canal</TableHead>
                  <TableHead>Activa</TableHead>
                  <TableHead className="w-16 text-center">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {reglas.map((r: ReglaRecordatorio) => (
                  <TableRow key={r.id}>
                    <TableCell className="font-medium max-w-xs">
                      <div className="overflow-hidden">
                        <span>{r.nombre}</span>
                        {r.mensaje_plantilla && (
                          <p className="text-muted-foreground text-xs mt-0.5 truncate">
                            {r.mensaje_plantilla}
                          </p>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      {formatHoras(
                        r.tipo === 'despues' ? r.horas_despues : r.horas_antes,
                        r.tipo,
                      )}
                    </TableCell>
                    <TableCell>{badgeCanal(r.canal)}</TableCell>
                    <TableCell>
                      <Switch
                        checked={r.activa}
                        onCheckedChange={() => toggleActiva(r.id, r.activa)}
                      />
                    </TableCell>
                    <TableCell>
                      <div className="flex justify-center">
                      <TooltipButton
                        variant="outline"
                        size="icon"
                        className="bg-muted/40 hover:bg-muted"
                        tooltip="Eliminar recordatorio"
                        onClick={() => handleEliminar(r.id)}
                      >
                        <Trash2 className="size-4" />
                      </TooltipButton>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}

          {totalPages > 1 && (
            <div className="flex items-center justify-center gap-2 pt-4">
              <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                <ChevronLeft className="size-4" />
              </Button>
              <span className="text-sm">{page} / {totalPages}</span>
              <Button variant="outline" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
                <ChevronRight className="size-4" />
              </Button>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default function Recordatorios() {
  return (
    <Tabs defaultValue="reglas" className="space-y-4">
      <TabsList>
        <TabsTrigger value="reglas">Reglas</TabsTrigger>
        <TabsTrigger value="historial">Historial</TabsTrigger>
      </TabsList>
      <TabsContent value="reglas">
        <TablaReglas />
      </TabsContent>
      <TabsContent value="historial">
        <TablaHistorial />
      </TabsContent>
    </Tabs>
  );
}
