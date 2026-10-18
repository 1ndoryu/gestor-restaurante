import {Card, CardContent, CardHeader, CardTitle} from '@/components/ui/card';
import {AnalisisReservas} from '../../api/generated';

export function PanelAnalisis({data}: {data: AnalisisReservas}) {
    const formatMoneda = (v: string | null | undefined) => {
        if (!v) return '—';
        return new Intl.NumberFormat('es-ES', {style: 'currency', currency: 'EUR'}).format(parseFloat(v));
    };

    return (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Reservas efectivas</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{data.reservas_efectivas}</span>
                    <p className="text-xs text-muted-foreground">Sin cancelaciones ni no-shows</p>
                </CardContent>
            </Card>
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Total comensales</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{data.total_comensales}</span>
                </CardContent>
            </Card>
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Comensales/reserva</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{data.comensales_por_reserva}</span>
                </CardContent>
            </Card>
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Ticket medio/reserva</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{formatMoneda(data.ticket_medio_reserva)}</span>
                    <p className="text-xs text-muted-foreground">Ventas ÷ reservas efectivas</p>
                </CardContent>
            </Card>
            <Card>
                <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Ticket medio/persona</CardTitle>
                </CardHeader>
                <CardContent>
                    <span className="text-2xl font-bold">{formatMoneda(data.ticket_medio_persona)}</span>
                    <p className="text-xs text-muted-foreground">Ventas ÷ total comensales</p>
                </CardContent>
            </Card>
        </div>
    );
}
