/* [189A-1] Controles de cambio de modo BDP: cada tarjeta de modo y el botón del
 * modo automático. El flujo de cambio vive en el hook compartido useFlujoModoBdp. */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useFlujoModoBdp } from '@/componentes/flujoModoBdp';
import type { EstadoConfiguracion } from '@/hooks/useConfiguracion';

/* [189A-1] Controles de cambio de modo dentro de cada tarjeta: la tarjeta del
 * modo activo muestra el badge (y, si es escritura, la vuelta a Solo lectura);
 * las demás ofrecen cambiar a su modo con el flujo compartido. */
export function ControlesTarjetaModo({
  config,
  modo,
  etiquetaActivar,
}: {
  config: EstadoConfiguracion;
  modo: 'read_only' | 'unidirectional';
  etiquetaActivar: string;
}) {
  const { activarManual, volverSoloLectura, isPending } = useFlujoModoBdp();
  const baseUrl = String(config.bdp_base_url ?? '');
  if (config.bdp_sync_mode === modo) {
    if (modo === 'read_only') {
      return (
        <div className="mt-2">
          <Badge variant="default" className="bg-emerald-600">Activo</Badge>
        </div>
      );
    }
    return (
      <div className="flex items-center gap-2 mt-2">
        <Badge variant="default" className="bg-emerald-600">Activo</Badge>
        <Button variant="outline" size="sm" disabled={isPending} onClick={volverSoloLectura}>
          {isPending ? 'Cambiando…' : 'Volver a Solo lectura'}
        </Button>
      </div>
    );
  }
  return (
    <Button
      variant="outline"
      size="sm"
      className="mt-2"
      disabled={isPending}
      onClick={modo === 'read_only' ? volverSoloLectura : () => activarManual(baseUrl)}
    >
      {isPending ? 'Cambiando…' : etiquetaActivar}
    </Button>
  );
}

/* [267A-7] Tercera tarjeta del modo de operaciones: activar/desactivar el modo
 * automático con confirmación explícita (irreversibilidad de escrituras).
 * [267A-8] Confirmación con diálogo propio: los diálogos nativos
 * (`window.confirm`/`prompt`) no funcionan en el navegador empaquetado.
 * [189A-1] Flujo delegado al hook compartido `useFlujoModoBdp`. */
export function BotonModoAutomatico({ config }: { config: EstadoConfiguracion }) {
  const { activarAutomatico, volverSoloLectura, isPending } = useFlujoModoBdp();
  const activo = config.bdp_sync_mode === 'automatic';
  const baseUrl = String(config.bdp_base_url ?? '');

  return (
    <div className={`rounded-md border p-3 transition-colors ${activo ? 'border-emerald-500 bg-emerald-50 dark:bg-emerald-950/20' : ''}`}>
      <div className="flex items-center gap-2 mb-1">
        {activo && (
          <Badge variant="default" className="bg-emerald-600">Activo</Badge>
        )}
        <p className="text-sm font-medium">Automático (Aplicación Web → BDP)</p>
      </div>
      <p className="text-xs text-muted-foreground">
        Las escrituras se envían sin confirmación por operación. Solo la activación y la vuelta a Solo lectura piden confirmación.
      </p>
      <Button
        variant={activo ? 'outline' : 'default'}
        size="sm"
        className="mt-2"
        disabled={isPending}
        onClick={activo ? volverSoloLectura : () => activarAutomatico(baseUrl)}
      >
        {isPending ? 'Cambiando…' : activo ? 'Volver a Solo lectura' : 'Activar modo automático'}
      </Button>
    </div>
  );
}
