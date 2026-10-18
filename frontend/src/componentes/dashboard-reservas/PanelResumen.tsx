import {Card, CardContent, CardHeader, CardTitle} from '@/components/ui/card';
import {ChartContainer, ChartTooltip, ChartTooltipContent} from '@/components/ui/chart';
import {BarChart, Bar, XAxis, YAxis, CartesianGrid, PieChart, Pie, Cell, Legend} from 'recharts';
import {ResumenReservas, AgrupacionCanal} from '../../api/generated';
import {CONFIG_SIMPLE, PIE_COLORS} from './constantes';

export function PanelResumen({data}: {data: ResumenReservas}) {
    const signoVariacion = data.variacion_porcentaje >= 0 ? '+' : '';

    return (
        <>
            <div className="grid gap-4 sm:grid-cols-2">
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Total reservas</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.total_reservas}</span>
                        <p className={`text-xs ${data.variacion_porcentaje >= 0 ? 'text-green-600' : 'text-destructive'}`}>
                            {signoVariacion}
                            {data.variacion_porcentaje}% vs mes anterior ({data.total_mes_anterior})
                        </p>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Clientes nuevos</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.clientes_nuevos}</span>
                    </CardContent>
                </Card>
            </div>

            {/* [283A-29] Charts en grid de 2 columnas */}
            <div className="grid gap-4 lg:grid-cols-2">
                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Reservas por día</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={CONFIG_SIMPLE} className="h-[220px] w-full">
                            <BarChart data={data.por_dia.map((d: {fecha: string; total: number}) => ({...d, fecha: d.fecha.slice(5)}))} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="fecha" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Por día de semana</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={CONFIG_SIMPLE} className="h-[220px] w-full">
                            <BarChart data={data.por_dia_semana} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="dia" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Distribución por canal</CardTitle>
                    </CardHeader>
                    <CardContent>
                        {data.por_canal.length > 0 ? (
                            <ChartContainer config={CONFIG_SIMPLE} className="h-[220px] w-full">
                                <PieChart margin={{left: 0, right: 0}}>
                                    <Pie data={data.por_canal} dataKey="total" nameKey="canal" cx="50%" cy="50%" outerRadius={80}>
                                        {data.por_canal.map((ag: AgrupacionCanal, i: number) => (
                                            <Cell key={ag.canal} fill={PIE_COLORS[i % PIE_COLORS.length]} />
                                        ))}
                                    </Pie>
                                    <ChartTooltip content={<ChartTooltipContent />} />
                                    <Legend />
                                </PieChart>
                            </ChartContainer>
                        ) : (
                            <p className="text-sm text-muted-foreground text-center py-8">Sin datos de canales</p>
                        )}
                    </CardContent>
                </Card>

                {/* [283A-29] Nuevo gráfico: comparación con mes anterior */}
                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Comparación mensual</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={{total: {label: 'Reservas', color: 'var(--chart-1)'}}} className="h-[220px] w-full">
                            <BarChart data={[{periodo: 'Mes anterior', total: data.total_mes_anterior}, {periodo: 'Este mes', total: data.total_reservas}]} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="periodo" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>
            </div>
        </>
    );
}
