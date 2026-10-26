/* [26AA-3] Permisos del modo automático por categoría.
 *
 * Cada interruptor decide qué puede escribir el modo automático en BDP. Todos
 * nacen apagados. El interruptor general de integración sigue siendo el freno:
 * apagado él, ninguna categoría escribe aunque esté encendida. */

import { Switch } from '@/components/ui/switch';
import { usePermisosAutomaticos, type PermisoAutomatico } from './usePermisosAutomaticos';

const ETIQUETAS: Record<string, string> = {
  comandas: 'Comandas',
  articulos_clientes: 'Artículos, departamentos, familias y clientes',
  propinas_puntos: 'Propinas y puntos',
  pagos: 'Pagos',
  facturas: 'Facturas',
  stock: 'Stock (regularización, traspasos e inventario)',
  camarero: 'Llamadas al camarero',
};

export default function PermisosAutomaticosCard() {
  const { permisos, error, cambiar } = usePermisosAutomaticos();

  return (
    <div className="flex flex-col gap-3 rounded-md border p-3">
      <div>
        <p className="text-sm font-medium">Permisos del modo automático</p>
        <p className="mt-1 text-xs text-muted-foreground">
          Cada categoría se enciende por separado. Están apagadas de fábrica: con todas apagadas,
          el modo automático no escribe nada en BDP. Este interruptor no sustituye al general de
          integración, que sigue siendo el freno superior. Cada cambio queda registrado.
        </p>
      </div>
      {error && (
        <p role="alert" className="text-xs text-destructive">
          {error}
        </p>
      )}
      {permisos.map((permiso: PermisoAutomatico) => (
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
    </div>
  );
}
