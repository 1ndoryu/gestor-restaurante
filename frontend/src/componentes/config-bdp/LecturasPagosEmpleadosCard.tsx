/* [26AA-2] Lectura programada de formas de pago y empleados.
 *
 * Solo lectura sobre el TPV: encenderla no escribe nada en BDP ni cambia los
 * mapeos. Los registros que BDP deja de publicar pasan a inactivos, no se
 * borran. Apagada de fábrica; se guarda al cambiar el selector. */

import { SelectorCadencia } from './cadencias';
import { useLecturasProgramadas, type LecturaProgramada } from './useLecturasProgramadas';

const ETIQUETAS: Record<LecturaProgramada['maestro'], string> = {
  formas_pago: 'Formas de pago',
  empleados: 'Empleados',
};

function fecha(valor: string | null): string {
  return valor ? new Date(valor).toLocaleString('es-ES') : 'nunca';
}

export default function LecturasPagosEmpleadosCard() {
  const { lecturas, error, cambiar } = useLecturasProgramadas();

  return (
    <div className="flex flex-col gap-3 rounded-md border p-3">
      <div>
        <p className="text-sm font-medium">Formas de pago y empleados</p>
        <p className="mt-1 text-xs text-muted-foreground">
          Cada cuánto los relee el servidor por su cuenta. Solo lectura: no cambia ningún mapeo, y
          lo que el TPV deja de publicar queda inactivo en vez de borrarse. Se guarda al elegir.
        </p>
      </div>
      {error && (
        <p role="alert" className="text-xs text-destructive">
          {error}
        </p>
      )}
      {lecturas.map((lectura) => (
        <div key={lectura.maestro} className="flex flex-col gap-1">
          <SelectorCadencia
            id={`bdp-lectura-${lectura.maestro}`}
            etiqueta={ETIQUETAS[lectura.maestro]}
            segundos={lectura.segundos}
            onCambiar={(segundos) => void cambiar(lectura.maestro, segundos)}
          />
          <p className="text-xs text-muted-foreground">
            Última lectura correcta: {fecha(lectura.ultima_ok)} · {lectura.activos} activos,{' '}
            {lectura.inactivos} inactivos
            {lectura.resumen ? ` · ${lectura.resumen}` : ''}
          </p>
          {lectura.ultimo_error && (
            <p className="text-xs text-destructive">Último error: {lectura.ultimo_error}</p>
          )}
        </div>
      ))}
    </div>
  );
}
