/* [263A-16] EstadisticasNoShows — reescrito con shadcn Card + Table + Button + Input.
 * Filtro de fechas y desglose por canal. Endpoint GET /api/reservas/no-shows. */

import { useState } from 'react';
import { useNoShowStats } from '../api/generated';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';

function EstadisticasNoShows() {
  const [fechaDesde, setFechaDesde] = useState('');
  const [fechaHasta, setFechaHasta] = useState('');

  const params = {
    ...(fechaDesde ? { fecha_desde: fechaDesde } : {}),
    ...(fechaHasta ? { fecha_hasta: fechaHasta } : {}),
  };

  const { data: resp, isLoading } = useNoShowStats(params);
  const stats = resp?.status === 200 ? resp.data : null;

  const limpiarFiltros = () => {
    setFechaDesde('');
    setFechaHasta('');
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-end gap-4">
        <div className="flex flex-col gap-2">
          <Label>Desde</Label>
          <Input type="date" value={fechaDesde} onChange={e => setFechaDesde(e.target.value)} />
        </div>
        <div className="flex flex-col gap-2">
          <Label>Hasta</Label>
          <Input type="date" value={fechaHasta} onChange={e => setFechaHasta(e.target.value)} />
        </div>
        {(fechaDesde || fechaHasta) && (
          <Button variant="ghost" size="sm" onClick={limpiarFiltros}>Limpiar</Button>
        )}
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground text-center py-8">Cargando estadísticas...</p>
      ) : stats ? (
        <>
          <div className="grid gap-4 sm:grid-cols-3">
            <Card>
              <CardHeader className="pb-2"><CardTitle className="text-sm font-medium text-muted-foreground">Total reservas</CardTitle></CardHeader>
              <CardContent><span className="text-2xl font-bold">{stats.total_reservas}</span></CardContent>
            </Card>
            <Card className="border-destructive/50">
              <CardHeader className="pb-2"><CardTitle className="text-sm font-medium text-destructive">No-shows</CardTitle></CardHeader>
              <CardContent><span className="text-2xl font-bold text-destructive">{stats.total_no_shows}</span></CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2"><CardTitle className="text-sm font-medium text-muted-foreground">Ratio no-show</CardTitle></CardHeader>
              <CardContent><span className="text-2xl font-bold">{stats.ratio_porcentaje.toFixed(1)}%</span></CardContent>
            </Card>
          </div>

          {stats.por_canal.length > 0 ? (
            <div>
              <h2 className="text-lg font-semibold mb-3">Desglose por canal</h2>
              <div className="rounded-md border">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Canal</TableHead>
                      <TableHead className="text-right">Reservas</TableHead>
                      <TableHead className="text-right">No-shows</TableHead>
                      <TableHead className="text-right">Ratio</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {stats.por_canal.map((canal) => (
                      <TableRow key={canal.canal_nombre || 'sin-canal'}>
                        <TableCell>{canal.canal_nombre || 'Sin canal'}</TableCell>
                        <TableCell className="text-right">{canal.total_reservas}</TableCell>
                        <TableCell className="text-right">{canal.no_shows}</TableCell>
                        <TableCell className="text-right">
                          <Badge variant={canal.ratio_porcentaje > 20 ? 'destructive' : canal.ratio_porcentaje > 10 ? 'secondary' : 'outline'}>
                            {canal.ratio_porcentaje.toFixed(1)}%
                          </Badge>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground text-center">No hay datos de canales para el período seleccionado</p>
          )}
        </>
      ) : (
        <p className="text-sm text-muted-foreground text-center py-8">No hay datos de reservas</p>
      )}
    </div>
  );
}

export default EstadisticasNoShows;
