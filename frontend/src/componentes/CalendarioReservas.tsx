/* [263A-16] Calendario de reservas — reescrito con shadcn Button + Card + Tailwind grid.
 * Cuadrícula mensual, click en día navega a ListaReservas con fecha. */

import { useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useResumenMensual } from '../api/generated';
import { Card, CardContent } from '@/components/ui/card';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { TooltipButton } from '@/components/ui/tooltip-button';

const NOMBRES_MES = [
  'Enero', 'Febrero', 'Marzo', 'Abril', 'Mayo', 'Junio',
  'Julio', 'Agosto', 'Septiembre', 'Octubre', 'Noviembre', 'Diciembre',
];

const DIAS_SEMANA = ['Lun', 'Mar', 'Mié', 'Jue', 'Vie', 'Sáb', 'Dom'];

function CalendarioReservas() {
  const hoy = new Date();
  const [anio, setAnio] = useState(hoy.getFullYear());
  const [mes, setMes] = useState(hoy.getMonth() + 1);
  const navigate = useNavigate();

  /* [DataIV-3] refetchOnMount asegura datos frescos al navegar de vuelta al calendario */
  const { data, isLoading } = useResumenMensual({ anio, mes }, { query: { refetchOnMount: 'always' } });
  const resumen = data?.status === 200 ? data.data : [];

  const totales = useMemo(() => {
    let reservas = 0;
    let personas = 0;
    for (const d of resumen) {
      reservas += d.total_reservas;
      personas += d.total_personas;
    }
    return { reservas, personas };
  }, [resumen]);

  const mapaFechas = useMemo(() => {
    const m = new Map<string, { reservas: number; personas: number }>();
    for (const d of resumen) {
      m.set(d.fecha, { reservas: d.total_reservas, personas: d.total_personas });
    }
    return m;
  }, [resumen]);

  const diasGrid = useMemo(() => {
    const primerDia = new Date(anio, mes - 1, 1);
    const ultimoDia = new Date(anio, mes, 0).getDate();
    const offsetInicio = (primerDia.getDay() + 6) % 7;
    const celdas: (number | null)[] = [];
    for (let i = 0; i < offsetInicio; i++) celdas.push(null);
    for (let d = 1; d <= ultimoDia; d++) celdas.push(d);
    while (celdas.length % 7 !== 0) celdas.push(null);
    return celdas;
  }, [anio, mes]);

  /* [283A-11] Restringir navegación a meses no futuros (tarea 46) */
  const esMesFuturo = anio > hoy.getFullYear() || (anio === hoy.getFullYear() && mes >= hoy.getMonth() + 1);

  const mesAnterior = () => {
    if (mes === 1) { setMes(12); setAnio(anio - 1); }
    else setMes(mes - 1);
  };

  const mesSiguiente = () => {
    if (esMesFuturo) return;
    if (mes === 12) { setMes(1); setAnio(anio + 1); }
    else setMes(mes + 1);
  };

  const irADia = (dia: number) => {
    /* [283A-12] No navegar a fechas futuras (tarea 46) */
    const fecha = new Date(anio, mes - 1, dia);
    if (fecha > hoy) return;
    const fechaStr = `${anio}-${String(mes).padStart(2, '0')}-${String(dia).padStart(2, '0')}`;
    navigate(`/reservas?fecha=${fechaStr}`);
  };

  const esHoy = (dia: number) =>
    anio === hoy.getFullYear() && mes === hoy.getMonth() + 1 && dia === hoy.getDate();

  const esFuturo = (dia: number) => {
    const fecha = new Date(anio, mes - 1, dia);
    return fecha > hoy;
  };

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-muted-foreground">
        {totales.reservas} reservas · {totales.personas} personas en {NOMBRES_MES[mes - 1]}
      </p>

      {/* [283A-11] Botones mejorados con iconos Lucide + outline (tarea 37.2) */}
      <div className="flex items-center justify-between">
        <TooltipButton variant="outline" size="icon" onClick={mesAnterior} tooltip="Mes anterior">
          <ChevronLeft className="h-4 w-4" />
        </TooltipButton>
        <span className="font-medium text-lg">{NOMBRES_MES[mes - 1]} {anio}</span>
        <TooltipButton variant="outline" size="icon" onClick={mesSiguiente} disabled={esMesFuturo} tooltip="Mes siguiente">
          <ChevronRight className="h-4 w-4" />
        </TooltipButton>
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground text-center py-8">Cargando...</p>
      ) : (
        <div className="grid grid-cols-7 gap-1">
          {DIAS_SEMANA.map(d => (
            <div key={d} className="text-center text-xs font-medium text-muted-foreground py-2">{d}</div>
          ))}
          {diasGrid.map((dia, i) => {
            if (dia === null) return <div key={`v-${i}`} />;
            const fechaKey = `${anio}-${String(mes).padStart(2, '0')}-${String(dia).padStart(2, '0')}`;
            const datos = mapaFechas.get(fechaKey);
            const futuro = esFuturo(dia);
            return (
              <Card
                key={dia}
                className={`transition-colors ${futuro ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer hover:border-primary'} ${esHoy(dia) ? 'border-primary bg-primary/5' : ''} ${datos && !futuro ? 'bg-primary/10 border-primary/30' : ''}`}
                onClick={() => !futuro && irADia(dia)}
                role={futuro ? undefined : "button"}
                tabIndex={futuro ? -1 : 0}
                onKeyDown={e => !futuro && e.key === 'Enter' && irADia(dia)}
              >
                <CardContent className="p-2 text-center">
                  <span className={`text-sm ${esHoy(dia) ? 'font-bold text-primary' : ''}`}>{dia}</span>
                  {datos && (
                    <div className="mt-1 flex flex-col items-center text-[10px] text-muted-foreground">
                      <span>{datos.reservas} res.</span>
                      <span>{datos.personas} pers.</span>
                    </div>
                  )}
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

export default CalendarioReservas;
