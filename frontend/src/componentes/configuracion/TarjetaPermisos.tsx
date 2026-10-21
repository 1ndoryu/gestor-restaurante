/* [D33] Quién puede hacer cada cosa. Antes vivía dentro de «Funcionalidades
 * BDP», donde nadie lo buscaba: no son ajustes de la conexión con el TPV sino
 * de permisos. */

import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { EstadoConfiguracion } from '../../hooks/useConfiguracion';
import type { CambiarCampo } from '../config-bdp/tipos';

interface TarjetaPermisosProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

interface Permiso {
  campo: keyof EstadoConfiguracion;
  etiqueta: string;
  detalle: string;
}

const PERMISOS: readonly Permiso[] = [
  { campo: 'permisos_catalogo_edicion', etiqueta: 'Catálogo', detalle: 'Crear, editar y eliminar artículos y mapeos.' },
  { campo: 'permisos_stock_ajuste', etiqueta: 'Ajuste de stock', detalle: 'Ajustes manuales de stock local.' },
  { campo: 'permisos_albaranes_gestion', etiqueta: 'Albaranes de compra', detalle: 'Crear, editar, borrar, borrador y conciliar.' },
  { campo: 'permisos_anulacion_ventas', etiqueta: 'Anulación de ventas', detalle: 'Anular ventas locales.' },
  { campo: 'permisos_pagos_locales', etiqueta: 'Pagos parciales locales', detalle: 'Registrar pagos parciales en el ledger local.' },
  { campo: 'permisos_facturacion_local', etiqueta: 'Facturación local', detalle: 'Emitir facturas locales de ventas.' },
];

export function TarjetaPermisos({ config, cambiarCampo }: TarjetaPermisosProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Quién puede tocar cada cosa</CardTitle>
        <CardDescription>
          El servidor aplica el permiso en cada operación; esta pantalla solo lo declara.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="grid gap-3 md:grid-cols-2">
          {PERMISOS.map(({ campo, etiqueta, detalle }) => (
            <div key={campo} className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor={`permiso-${campo}`}>{etiqueta}</Label>
                <p className="text-xs text-muted-foreground">{detalle}</p>
              </div>
              <Select
                value={String(config[campo])}
                onValueChange={(v) => cambiarCampo(campo, v as EstadoConfiguracion[typeof campo])}
              >
                <SelectTrigger id={`permiso-${campo}`} className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">Solo admin</SelectItem>
                  <SelectItem value="admin_trabajador">Admin y trabajadores</SelectItem>
                  <SelectItem value="todos">Todos</SelectItem>
                </SelectContent>
              </Select>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
