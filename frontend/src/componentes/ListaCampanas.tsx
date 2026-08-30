/* [263A-23] Lista de campañas de marketing.
 * Muestra campañas con estado, canales, segmento y acciones.
 * Botón "Nueva Campaña" navega al formulario de creación. */

import {useCampanas} from '../hooks/useCampanas';
import {Button} from '@/components/ui/button';import { TooltipButton } from '@/components/ui/tooltip-button';import {Badge} from '@/components/ui/badge';
import {Table, TableBody, TableCell, TableHead, TableHeader, TableRow} from '@/components/ui/table';
import {Select, SelectContent, SelectItem, SelectTrigger, SelectValue} from '@/components/ui/select';
import {Card, CardContent, CardHeader, CardTitle} from '@/components/ui/card';
import {Trash2, Send, Plus, ChevronLeft, ChevronRight} from 'lucide-react';
import type {Campana} from '../api/generated/gestionRestauranteAPI.schemas';

const ETIQUETAS_SEGMENTO: Record<string, string> = {
    habitual: 'Habituales',
    sin_1m: 'Sin venir 1 mes',
    sin_3m: 'Sin venir 3 meses',
    sin_6m: 'Sin venir 6 meses',
    sin_9m: 'Sin venir 9 meses',
    sin_1a: 'Sin venir 1 año',
    sin_mas_1a: 'Sin venir +1 año',
    todos: 'Todos'
};

const ETIQUETAS_CANAL: Record<string, string> = {
    sms: 'SMS',
    email: 'Email',
    whatsapp: 'WhatsApp'
};

function badgeEstado(estado: string) {
    switch (estado) {
        case 'enviada':
            return <Badge variant="default">{estado}</Badge>;
        case 'cancelada':
            return <Badge variant="destructive">{estado}</Badge>;
        default:
            return <Badge variant="secondary">{estado}</Badge>;
    }
}

export default function ListaCampanas() {
    const {campanas, total, page, totalPages, filtroEstado, isLoading, setPage, setFiltroEstado, eliminarCampana, enviarCampana, irANuevaCampana} = useCampanas();

    const handleEliminar = async (id: string) => {
        await eliminarCampana({id});
    };

    const handleEnviar = (c: Campana) => {
        if (c.estado !== 'borrador') return;
        enviarCampana({id: c.id});
    };

    return (
        <div className="space-y-4">
            <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                    <Select value={filtroEstado || '__all__'} onValueChange={v => setFiltroEstado(v === '__all__' ? '' : v)}>
                        <SelectTrigger className="w-48">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="__all__">Todos los estados</SelectItem>
                            <SelectItem value="borrador">Borrador</SelectItem>
                            <SelectItem value="enviada">Enviada</SelectItem>
                            <SelectItem value="cancelada">Cancelada</SelectItem>
                        </SelectContent>
                    </Select>
                    <span className="text-sm text-muted-foreground">{total} campaña(s)</span>
                </div>
                <Button onClick={irANuevaCampana}>
                    <Plus className="mr-1 size-4" /> Nueva Campaña
                </Button>
            </div>

            <Card>
                <CardHeader>
                    <CardTitle>Campañas</CardTitle>
                </CardHeader>
                <CardContent>
                    {isLoading ? (
                        <p className="text-muted-foreground py-8 text-center">Cargando...</p>
                    ) : campanas.length === 0 ? (
                        <p className="text-muted-foreground py-8 text-center">No hay campañas. Crea tu primera campaña de marketing.</p>
                    ) : (
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Nombre</TableHead>
                                    <TableHead>Estado</TableHead>
                                    <TableHead>Canales</TableHead>
                                    <TableHead>Segmento</TableHead>
                                    <TableHead className="text-right">Destinatarios</TableHead>
                                    <TableHead>Fecha</TableHead>
                                    <TableHead className="w-24 text-center">Acciones</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {campanas.map(c => (
                                    <TableRow key={c.id}>
                                        <TableCell className="font-medium">{c.nombre}</TableCell>
                                        <TableCell>{badgeEstado(c.estado)}</TableCell>
                                        <TableCell>
                                            <div className="flex gap-1">
                                                {c.canales.map(canal => (
                                                    <Badge key={canal} variant="outline">
                                                        {ETIQUETAS_CANAL[canal] || canal}
                                                    </Badge>
                                                ))}
                                            </div>
                                        </TableCell>
                                        <TableCell>{ETIQUETAS_SEGMENTO[c.segmento] || c.segmento}</TableCell>
                                        <TableCell className="text-right">{c.total_destinatarios > 0 ? `${c.total_enviados}/${c.total_destinatarios}` : '—'}</TableCell>
                                        <TableCell className="text-muted-foreground text-[13px]">{new Date(c.created_at).toLocaleDateString('es-ES')}</TableCell>
                                        <TableCell>
                                            <div className="flex justify-center gap-1">
                                                {c.estado === 'borrador' && (
                                                    <TooltipButton variant="outline" size="icon" className="bg-muted/40 hover:bg-muted" tooltip="Enviar campaña" onClick={() => handleEnviar(c)}>
                                                        <Send className="size-4" />
                                                    </TooltipButton>
                                                )}
                                                <TooltipButton variant="outline" size="icon" className="bg-muted/40 hover:bg-muted" tooltip="Eliminar campaña" onClick={() => handleEliminar(c.id)}>
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
                            <span className="text-sm">
                                {page} / {totalPages}
                            </span>
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
