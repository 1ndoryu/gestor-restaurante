/* [263A-24] Lista de plantillas WhatsApp para Meta Business API.
 * Muestra plantillas con estado (borrador/enviada/aprobada/rechazada),
 * categoría, idioma, acciones de enviar a Meta y eliminar. */

import { usePlantillas } from '../hooks/usePlantillas';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Trash2, Send, Plus, ChevronLeft, ChevronRight, AlertCircle } from 'lucide-react';
import type { PlantillaWhatsapp } from '../api/generated/gestionRestauranteAPI.schemas';

const ETIQUETAS_CATEGORIA: Record<string, string> = {
  MARKETING: 'Marketing',
  UTILITY: 'Utilidad',
  AUTHENTICATION: 'Autenticación',
};

function badgeEstado(estado: string) {
  switch (estado) {
    case 'aprobada':
      return <Badge className="bg-green-600 hover:bg-green-700 text-white">{estado}</Badge>;
    case 'enviada':
      return <Badge variant="default">{estado}</Badge>;
    case 'rechazada':
      return <Badge variant="destructive">{estado}</Badge>;
    default:
      return <Badge variant="secondary">{estado}</Badge>;
  }
}

export default function ListaPlantillas() {
  const {
    plantillas,
    total,
    page,
    totalPages,
    filtroEstado,
    isLoading,
    setPage,
    setFiltroEstado,
    eliminarPlantilla,
    enviarAMeta,
    irANuevaPlantilla,
  } = usePlantillas();

  const handleEliminar = async (id: string) => {
    await eliminarPlantilla({ id });
  };

  const handleEnviar = (p: PlantillaWhatsapp) => {
    if (p.estado !== 'borrador') return;
    enviarAMeta({ id: p.id });
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Select
            value={filtroEstado || '__all__'}
            onValueChange={v => setFiltroEstado(v === '__all__' ? '' : v)}
          >
            <SelectTrigger className="w-48">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__all__">Todos los estados</SelectItem>
              <SelectItem value="borrador">Borrador</SelectItem>
              <SelectItem value="enviada">Enviada</SelectItem>
              <SelectItem value="aprobada">Aprobada</SelectItem>
              <SelectItem value="rechazada">Rechazada</SelectItem>
            </SelectContent>
          </Select>
          <span className="text-sm text-muted-foreground">{total} plantilla(s)</span>
        </div>
        <Button onClick={irANuevaPlantilla}>
          <Plus className="mr-1 size-4" /> Nueva Plantilla
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Plantillas WhatsApp</CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <p className="text-muted-foreground py-8 text-center">Cargando...</p>
          ) : plantillas.length === 0 ? (
            <p className="text-muted-foreground py-8 text-center">
              No hay plantillas. Crea tu primera plantilla de WhatsApp.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Nombre</TableHead>
                  <TableHead>Estado</TableHead>
                  <TableHead>Categoría</TableHead>
                  <TableHead>Idioma</TableHead>
                  <TableHead>Fecha</TableHead>
                  <TableHead className="w-24" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {plantillas.map((p: PlantillaWhatsapp) => (
                  <TableRow key={p.id}>
                    <TableCell className="font-medium">
                      <div>
                        <span>{p.nombre}</span>
                        {p.cuerpo_mensaje && (
                          <p className="text-muted-foreground text-xs mt-0.5 line-clamp-1">
                            {p.cuerpo_mensaje}
                          </p>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        {badgeEstado(p.estado)}
                        {p.estado === 'rechazada' && p.meta_razon_rechazo && (
                          <Tooltip>
                            <TooltipTrigger>
                              <AlertCircle className="size-4 text-destructive" />
                            </TooltipTrigger>
                            <TooltipContent>
                              <p className="max-w-xs">{p.meta_razon_rechazo}</p>
                            </TooltipContent>
                          </Tooltip>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline">
                        {ETIQUETAS_CATEGORIA[p.categoria] || p.categoria}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-sm">{p.idioma.toUpperCase()}</TableCell>
                    <TableCell className="text-muted-foreground text-sm">
                      {new Date(p.created_at).toLocaleDateString('es-ES')}
                    </TableCell>
                    <TableCell>
                      <div className="flex gap-1">
                        {p.estado === 'borrador' && (
                          <TooltipButton
                            variant="ghost"
                            size="icon"
                            tooltip="Enviar plantilla a Meta"
                            onClick={() => handleEnviar(p)}
                          >
                            <Send className="size-4" />
                          </TooltipButton>
                        )}
                        <TooltipButton
                          variant="ghost"
                          size="icon"
                          tooltip="Eliminar plantilla"
                          onClick={() => handleEliminar(p.id)}
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
              <Button
                variant="outline"
                size="sm"
                disabled={page <= 1}
                onClick={() => setPage(page - 1)}
              >
                <ChevronLeft className="size-4" />
              </Button>
              <span className="text-sm">
                {page} / {totalPages}
              </span>
              <Button
                variant="outline"
                size="sm"
                disabled={page >= totalPages}
                onClick={() => setPage(page + 1)}
              >
                <ChevronRight className="size-4" />
              </Button>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
