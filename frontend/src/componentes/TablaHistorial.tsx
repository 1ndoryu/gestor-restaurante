/* [044A-4] Extraido de Recordatorios.tsx para cumplir limite de 300 lineas.
 * Tabla de historial de recordatorios enviados con paginación. */

import { useState } from 'react';
import { useHistorialRecordatorios } from '../api/generated/recordatorios/recordatorios';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import {
  Card, CardContent, CardDescription, CardHeader, CardTitle,
} from '@/components/ui/card';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import type { RecordatorioEnviadoDetalle } from '../api/generated/gestionRestauranteAPI.schemas';

const ETIQUETAS_CANAL: Record<string, string> = {
  sms: 'SMS',
  email: 'Email',
  whatsapp: 'WhatsApp',
};

function badgeCanal(canal: string) {
  switch (canal) {
    case 'email':
      return <Badge variant="default">{ETIQUETAS_CANAL[canal] || canal}</Badge>;
    case 'whatsapp':
      return <Badge className="bg-green-600 hover:bg-green-700 text-white">{ETIQUETAS_CANAL[canal] || canal}</Badge>;
    default:
      return <Badge variant="secondary">{ETIQUETAS_CANAL[canal] || canal}</Badge>;
  }
}

export default function TablaHistorial() {
  const [page, setPage] = useState(1);
  const { data, isLoading } = useHistorialRecordatorios(
    { page, per_page: 20 },
    { query: { queryKey: ['recordatorios-historial', page] } },
  );

  const items: RecordatorioEnviadoDetalle[] = data?.data?.items ?? [];
  const total = data?.data?.total ?? 0;
  const totalPages = Math.ceil(total / 20);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Historial de Envíos</CardTitle>
        <CardDescription>
          Registro de recordatorios enviados automáticamente
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <p className="text-muted-foreground py-8 text-center">Cargando...</p>
        ) : items.length === 0 ? (
          <p className="text-muted-foreground py-8 text-center">
            Aún no se han enviado recordatorios.
          </p>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Cliente</TableHead>
                <TableHead>Reserva</TableHead>
                <TableHead>Regla</TableHead>
                <TableHead>Canal</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead>Enviado</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.map((r: RecordatorioEnviadoDetalle) => (
                <TableRow key={r.id}>
                  <TableCell className="font-medium">{r.nombre_cliente}</TableCell>
                  <TableCell className="text-[13px]">
                    {new Date(r.fecha_reserva).toLocaleDateString('es-ES')} {r.hora_reserva?.toString().slice(0, 5)}
                  </TableCell>
                  <TableCell className="text-[13px]">{r.regla_nombre}</TableCell>
                  <TableCell>{badgeCanal(r.canal)}</TableCell>
                  <TableCell>
                    <Badge variant={r.estado === 'enviado' ? 'default' : 'destructive'}>
                      {r.estado}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground text-[13px]">
                    {new Date(r.enviado_at).toLocaleString('es-ES')}
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
  );
}
