/* [263A-16] DashboardReservas — reescrito con shadcn Card + Button + Tailwind.
 * [263A-25] Gráficos migrados a ChartContainer de shadcn — colores adaptativos dark/light.
 * [263A-27] Fusionado con Inicio: tab "General" = acciones rápidas + resumen económico + reservas hoy. */

import {useState} from 'react';
import {useDashboardReservas} from '../api/generated';
import {Select, SelectContent, SelectItem, SelectTrigger, SelectValue} from '@/components/ui/select';
import {Tabs, TabsList, TabsTrigger, TabsContent} from '@/components/ui/tabs';
import {AccionesRapidas} from './dashboard-reservas/AccionesRapidas';
import {PanelGeneral} from './dashboard-reservas/PanelGeneral';
import {PanelResumen} from './dashboard-reservas/PanelResumen';
import {PanelOcupacion} from './dashboard-reservas/PanelOcupacion';
import {PanelAnalisis} from './dashboard-reservas/PanelAnalisis';

function DashboardReservas() {
    const ahora = new Date();
    const [anio, setAnio] = useState(ahora.getFullYear());
    const [mes, setMes] = useState(ahora.getMonth() + 1);
    const [pestana, setPestana] = useState<string>('general');

    const {data, isLoading} = useDashboardReservas({year: anio, month: mes});
    const dashboard = data?.status === 200 ? data.data : null;

    const meses = ['Enero', 'Febrero', 'Marzo', 'Abril', 'Mayo', 'Junio', 'Julio', 'Agosto', 'Septiembre', 'Octubre', 'Noviembre', 'Diciembre'];
    /* [D-308A] Números de mes 1..12: identidad estable para las keys del select
     * (el número de mes ES la identidad del item, no un índice de render). */
    const MESES_NUM = Array.from({length: 12}, (_, i) => i + 1);

    /* [283A-12] Filtrar meses/años futuros — tarea 46 */
    const anioActual = ahora.getFullYear();
    const mesActual = ahora.getMonth() + 1;
    const aniosDisponibles = [anio - 1, anio, anio + 1].filter(a => a <= anioActual);

    return (
        <div className="flex flex-col gap-6">
            <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-3">
                    <Select value={String(mes)} onValueChange={v => setMes(Number(v))}>
                        <SelectTrigger className="w-36">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {MESES_NUM.map((mesNum) => {
                                const mesFuturo = anio === anioActual && mesNum > mesActual;
                                return (
                                    <SelectItem key={mesNum} value={String(mesNum)} disabled={mesFuturo}>
                                        {meses[mesNum - 1]}
                                    </SelectItem>
                                );
                            })}
                        </SelectContent>
                    </Select>
                    <Select value={String(anio)} onValueChange={v => {
                        const nuevoAnio = Number(v);
                        setAnio(nuevoAnio);
                        if (nuevoAnio === anioActual && mes > mesActual) setMes(mesActual);
                    }}>
                        <SelectTrigger className="w-24">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {aniosDisponibles.map(a => (
                                <SelectItem key={a} value={String(a)}>
                                    {a}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
                {/* [283A-7] Acciones rápidas reubicadas al header, alineadas a la derecha */}
                <AccionesRapidas />
            </div>

            <Tabs value={pestana} onValueChange={setPestana}>
                <TabsList variant="line">
                    <TabsTrigger value="general">General</TabsTrigger>
                    <TabsTrigger value="ocupacion">Ocupación</TabsTrigger>
                    <TabsTrigger value="analisis">Análisis</TabsTrigger>
                </TabsList>

                {/* [283A-18] General + Resumen combinados en un solo tab */}
                <TabsContent value="general" className="space-y-6">
                    <PanelGeneral anio={anio} mes={mes} />
                    {isLoading ? <p className="text-sm text-muted-foreground text-center py-8">Cargando dashboard...</p> : dashboard && <PanelResumen data={dashboard.resumen} />}
                </TabsContent>
                <TabsContent value="ocupacion" className="space-y-6">{isLoading ? <p className="text-sm text-muted-foreground text-center py-8">Cargando dashboard...</p> : dashboard && <PanelOcupacion data={dashboard.ocupacion} />}</TabsContent>
                <TabsContent value="analisis" className="space-y-6">{isLoading ? <p className="text-sm text-muted-foreground text-center py-8">Cargando dashboard...</p> : dashboard && <PanelAnalisis data={dashboard.analisis} />}</TabsContent>
            </Tabs>
        </div>
    );
}

export default DashboardReservas;
