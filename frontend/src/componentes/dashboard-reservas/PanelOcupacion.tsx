import type {CSSProperties} from 'react';
import {Card, CardContent, CardHeader, CardTitle} from '@/components/ui/card';
import {Table, TableBody, TableCell, TableHead, TableHeader, TableRow} from '@/components/ui/table';
import {ChartContainer, ChartTooltip, ChartTooltipContent} from '@/components/ui/chart';
import {BarChart, Bar, XAxis, YAxis, CartesianGrid} from 'recharts';
import {OcupacionReservas, AgrupacionCanal} from '../../api/generated';
import {CONFIG_SIMPLE, CONFIG_TURNO} from './constantes';

export function PanelOcupacion({data}: {data: OcupacionReservas}) {
    return (
        <>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Media personas/reserva</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.media_personas}</span>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Media reservas/día</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.media_reservas_dia}</span>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Total reservas</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.total_reservas}</span>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="pb-2">
                        <CardTitle className="text-sm font-medium text-muted-foreground">Antelación media</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <span className="text-2xl font-bold">{data.antelacion_media_dias} días</span>
                    </CardContent>
                </Card>
            </div>

            {/* [283A-18] Hora + Turno en grid de 2, Procedencia abajo en bloque propio */}
            <div className="grid gap-4 lg:grid-cols-2">
                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Distribución por hora</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={CONFIG_SIMPLE} className="h-[220px] w-full">
                            <BarChart data={data.por_hora} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="hora" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-sm">Por turno</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <ChartContainer config={CONFIG_TURNO} className="h-[220px] w-full">
                            <BarChart data={data.por_turno} margin={{left: 0, right: 0}}>
                                <CartesianGrid vertical={false} />
                                <XAxis dataKey="turno" tickLine={false} axisLine={false} fontSize={11} />
                                <YAxis allowDecimals={false} tickLine={false} axisLine={false} width={30} />
                                <ChartTooltip content={<ChartTooltipContent />} />
                                <Bar dataKey="total" fill="var(--color-total)" radius={4} />
                                <Bar dataKey="personas" fill="var(--color-personas)" radius={4} />
                            </BarChart>
                        </ChartContainer>
                    </CardContent>
                </Card>
            </div>

            <Card>
                <CardHeader>
                    <CardTitle className="text-sm">Procedencia (canal)</CardTitle>
                </CardHeader>
                <CardContent>
                    <div className="rounded-md border">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Canal</TableHead>
                                    <TableHead className="text-right">Reservas</TableHead>
                                    <TableHead className="text-right">%</TableHead>
                                    <TableHead></TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {data.por_procedencia.map((c: AgrupacionCanal) => (
                                    <TableRow key={c.canal}>
                                        <TableCell>{c.canal}</TableCell>
                                        <TableCell className="text-right">{c.total}</TableCell>
                                        <TableCell className="text-right">{c.porcentaje}%</TableCell>
                                        <TableCell>
                                            <div className="h-2 w-full max-w-[100px] rounded-full bg-muted">
                                                <div className="h-2 rounded-full bg-primary barraProgreso" style={{ '--ancho': `${c.porcentaje > 100 ? 100 : c.porcentaje}%` } as CSSProperties} />
                                            </div>
                                        </TableCell>
                                    </TableRow>
                                ))}
                            </TableBody>
                        </Table>
                    </div>
                </CardContent>
            </Card>
        </>
    );
}
