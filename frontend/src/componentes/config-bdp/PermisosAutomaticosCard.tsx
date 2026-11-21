/* [26AA-3] Permisos del modo automático por categoría.
 *
 * Cada interruptor decide qué puede escribir el modo automático en BDP. Todos
 * nacen apagados. El interruptor general de integración sigue siendo el freno:
 * apagado él, ninguna categoría escribe aunque esté encendida.
 * [21AB-5] Agrupados en las etapas de puesta en marcha que pidió Guillermo, en
 * su orden, con la vuelta rápida a solo lectura y la actividad reciente. */

import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { usePermisosAutomaticos, type PermisoAutomatico } from './usePermisosAutomaticos';

/* Orden de encendido: primero se puede leer, después escribir, y cada escritura
 * se abre por su cuenta. `ETAPAS` del servidor manda en el orden; aquí solo
 * están las etiquetas y qué significan. */
const ETIQUETAS: Record<string, string> = {
  lectura: 'Lectura desde BDP (sondeos, importaciones y consultas)',
  comandas: 'Crear y anular comandas',
  modificaciones: 'Modificar comandas ya creadas',
  descuentos: 'Descuentos e invitaciones',
  pagos: 'Cobros',
  facturas: 'Tickets y facturas',
  articulos_clientes: 'Artículos, departamentos, familias y clientes',
  propinas_puntos: 'Propinas y puntos',
  stock: 'Stock (regularización, traspasos e inventario)',
  camarero: 'Llamadas al camarero',
};

const ORDEN_ETAPAS = [
  'lectura',
  'comandas',
  'modificaciones',
  'descuentos',
  'pagos',
  'facturas',
];

/* El resto de categorías son escrituras de catálogo y back office: se muestran
 * después, porque no forman parte de la puesta en marcha por etapas. */
function ordenar(permisos: PermisoAutomatico[]): PermisoAutomatico[] {
  const peso = (categoria: string) => {
    const i = ORDEN_ETAPAS.indexOf(categoria);
    return i === -1 ? ORDEN_ETAPAS.length : i;
  };
  return [...permisos].sort((a, b) => peso(a.categoria) - peso(b.categoria));
}

export default function PermisosAutomaticosCard() {
  const { permisos, actividad, error, apagando, cambiar, volverASoloLectura } =
    usePermisosAutomaticos();
  const encendidas = permisos.filter((p) => p.activo).length;

  return (
    <div className="flex flex-col gap-3 rounded-md border p-3">
      <div>
        <p className="text-sm font-medium">Permisos del modo automático</p>
        <p className="mt-1 text-xs text-muted-foreground">
          Cada etapa se enciende por separado y en este orden. Están apagadas de fábrica: con
          todas apagadas, el modo automático no escribe nada en BDP. Este interruptor no sustituye
          al general de integración, que sigue siendo el freno superior. Cada cambio queda
          registrado.
        </p>
      </div>
      {error && (
        <p role="alert" className="text-xs text-destructive">
          {error}
        </p>
      )}
      <div className="flex items-center justify-between gap-4 border-b pb-2">
        <span className="text-xs text-muted-foreground">
          {encendidas === 0 ? 'Todo apagado' : `${encendidas} de ${permisos.length} encendidas`}
        </span>
        <Button
          type="button"
          variant="destructive"
          size="sm"
          disabled={apagando || encendidas === 0}
          onClick={() => void volverASoloLectura()}
        >
          {apagando ? 'Apagando…' : 'Volver a solo lectura'}
        </Button>
      </div>
      {ordenar(permisos).map((permiso: PermisoAutomatico) => (
        <div key={permiso.categoria} className="flex items-center justify-between gap-4">
          <label htmlFor={`bdp-permiso-${permiso.categoria}`} className="text-sm">
            {ETIQUETAS[permiso.categoria] ?? permiso.categoria}
          </label>
          <Switch
            id={`bdp-permiso-${permiso.categoria}`}
            checked={permiso.activo}
            onCheckedChange={(activo) => void cambiar(permiso.categoria, activo)}
          />
        </div>
      ))}
      {actividad.length > 0 && (
        <details className="text-xs text-muted-foreground">
          <summary className="cursor-pointer">Actividad reciente</summary>
          <ul className="mt-2 flex flex-col gap-1">
            {actividad.map((cambio, i) => (
              <li key={`${cambio.cambiado_en}-${cambio.categoria}-${i}`}>
                {new Date(cambio.cambiado_en).toLocaleString()} —{' '}
                {ETIQUETAS[cambio.categoria] ?? cambio.categoria}:{' '}
                {cambio.activo ? 'encendida' : 'apagada'}
              </li>
            ))}
          </ul>
        </details>
      )}
    </div>
  );
}
