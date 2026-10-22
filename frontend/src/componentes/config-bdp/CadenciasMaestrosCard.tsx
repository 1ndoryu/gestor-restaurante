/* [D24-auto-2] Cadencia de los tres maestros que se refrescaban solo con botón:
 * clientes, plano de sala y departamentos.
 *
 * El alcance de la fase D promete que estas tablas viajan solas. El botón sigue
 * ahí porque a veces se quiere el cambio *ahora*; la cadencia es para el resto
 * de los días. Las tres son solo lectura sobre el TPV: encenderlas no escribe
 * nada en BDP. */

import { SelectorCadencia } from './cadencias';
import type { EstadoConfiguracion } from '@/hooks/useConfiguracion';
import type { CambiarCampo } from './tipos';

const MAESTROS = [
  {
    id: 'bdp-clientes-cadencia',
    etiqueta: 'Clientes',
    campo: 'bdp_clientes_auto_segundos',
    detalle: 'Los que están en el TPV y no en la web. Nunca sobrescribe un vínculo ya hecho.',
  },
  {
    id: 'bdp-plano-cadencia',
    etiqueta: 'Plano de sala',
    campo: 'bdp_plano_auto_segundos',
    detalle: 'Salones, zonas y mesas. Solo lectura: no mueve ni renombra mesas.',
  },
  {
    id: 'bdp-departamentos-cadencia',
    etiqueta: 'Departamentos',
    campo: 'bdp_departamentos_auto_segundos',
    detalle: 'Las familias del catálogo. El artículo trae la suya, pero no se refrescaba sola.',
  },
] as const;

interface Props {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

export default function CadenciasMaestrosCard({ config, cambiarCampo }: Props) {
  return (
    <div className="flex flex-col gap-3 rounded-md border p-3">
      <div>
        <p className="text-sm font-medium">Clientes, plano de sala y departamentos</p>
        <p className="mt-1 text-xs text-muted-foreground">
          Cada cuánto los relee el servidor por su cuenta. Se guarda con el botón de guardar de
          arriba y solo se leen cuando la integración está en modo BDP.
        </p>
      </div>
      <div className="flex flex-wrap items-end gap-3">
        {MAESTROS.map((maestro) => (
          <SelectorCadencia
            key={maestro.id}
            id={maestro.id}
            etiqueta={maestro.etiqueta}
            segundos={Number(config[maestro.campo]) || 0}
            onCambiar={(segundos) => cambiarCampo(maestro.campo, segundos)}
          />
        ))}
      </div>
      <ul className="text-xs text-muted-foreground">
        {MAESTROS.map((maestro) => (
          <li key={maestro.id}>
            <span className="font-medium">{maestro.etiqueta}:</span> {maestro.detalle}
          </li>
        ))}
      </ul>
    </div>
  );
}
