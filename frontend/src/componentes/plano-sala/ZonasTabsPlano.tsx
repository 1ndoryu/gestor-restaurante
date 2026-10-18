/* Pestañas de zona del plano: una por zona con su nº de mesas, y alta de zona nueva. */
import type { EstadoPlanoSala } from './tipoEstadoPlano';

interface ZonasTabsPlanoProps {
  zonas: NonNullable<EstadoPlanoSala['plano']>['zonas'] | undefined;
  zonaActiva: EstadoPlanoSala['zonaActiva'];
  onCambiarZona: EstadoPlanoSala['cambiarZona'];
  onCrearZona: EstadoPlanoSala['handleCrearZona'];
}

export function ZonasTabsPlano({ zonas, zonaActiva, onCambiarZona, onCrearZona }: ZonasTabsPlanoProps) {
  return (
    <div className="flex gap-1 flex-wrap items-center border-b border-border">
      {zonas?.map(z => (
        <button
          key={z.id}
          type="button"
          className={`relative inline-flex items-center justify-center gap-1.5 px-3 py-1.5 text-sm font-medium whitespace-nowrap transition-colors rounded-t-md ${z.id === zonaActiva ? 'text-foreground after:absolute after:inset-x-0 after:bottom-[-1px] after:h-0.5 after:bg-foreground' : 'text-muted-foreground hover:text-foreground'}`}
          onClick={() => onCambiarZona(z.id)}
        >
          {z.nombre} ({z.mesas.length})
        </button>
      ))}
      <button
        type="button"
        className="inline-flex items-center gap-1 px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors rounded-t-md"
        onClick={onCrearZona}
      >
        + Zona
      </button>
    </div>
  );
}
