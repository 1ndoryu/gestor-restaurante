import {Card, CardContent, CardHeader, CardTitle} from '@/components/ui/card';
import {useResumen, useConteoReservas} from '../../api/generated';
import {DollarSign, TrendingDown, TrendingUp, CalendarDays} from 'lucide-react';

/* [263A-27] Panel General: resumen económico + conteo reservas.
 * [283A-7] Acciones rápidas movidas al header del dashboard. */
export function PanelGeneral({anio, mes}: {anio: number; mes: number}) {
    const {data: resumen, isLoading: cargandoResumen} = useResumen({year: anio, month: mes});
    const {data: conteo, isLoading: cargandoConteo} = useConteoReservas();

    const datosResumen = resumen?.status === 200 ? resumen.data : null;
    const datosConteo = conteo?.status === 200 ? conteo.data : null;

    const formatearMoneda = (valor: string) => new Intl.NumberFormat('es-ES', {style: 'currency', currency: 'EUR'}).format(parseFloat(valor));

    return (
        <>
            {/* Resumen económico */}
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Ventas</CardTitle>
                        <TrendingUp className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold text-green-600">{cargandoResumen ? '...' : datosResumen ? formatearMoneda(datosResumen.total_ventas) : '—'}</div>
                        <p className="text-xs text-muted-foreground">Total del mes</p>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Gastos</CardTitle>
                        <TrendingDown className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold text-red-600">{cargandoResumen ? '...' : datosResumen ? formatearMoneda(datosResumen.total_gastos) : '—'}</div>
                        <p className="text-xs text-muted-foreground">Total del mes</p>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Margen</CardTitle>
                        <DollarSign className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className={`text-2xl font-bold ${datosResumen && parseFloat(datosResumen.margen) >= 0 ? 'text-green-600' : 'text-red-600'}`}>{cargandoResumen ? '...' : datosResumen ? formatearMoneda(datosResumen.margen) : '—'}</div>
                        <p className="text-xs text-muted-foreground">Ventas − Gastos</p>
                    </CardContent>
                </Card>
            </div>

            {/* Reservas */}
            <div className="grid gap-4 sm:grid-cols-2">
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Reservas hoy</CardTitle>
                        <CalendarDays className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold">{cargandoConteo ? '...' : datosConteo ? datosConteo.total_hoy : 0}</div>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2">
                        <CardTitle className="text-sm font-medium">Reservas este mes</CardTitle>
                        <CalendarDays className="size-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold">{cargandoConteo ? '...' : datosConteo ? datosConteo.total_mes : 0}</div>
                    </CardContent>
                </Card>
            </div>
        </>
    );
}
