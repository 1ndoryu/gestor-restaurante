/* Cabecera de la pestaña «Inventario»: contador, botón «Guardar conteo», selector de
 * almacén y la nota que explica a qué almacén se aplica el conteo. Sin estado propio:
 * todo llega desde BdpInventario. */
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Save, Loader2 } from 'lucide-react';
import type { AlmacenBdp } from '../bdp-stock/TraspasarStockDialog';

interface CabeceraInventarioProps {
  contados: number;
  totalCatalogo: number;
  guardando: boolean;
  almacen: string;
  almacenes: AlmacenBdp[];
  onGuardar: () => void;
  onAlmacen: (valor: string) => void;
}

export function CabeceraInventario({
  contados, totalCatalogo, guardando, almacen, almacenes, onGuardar, onAlmacen,
}: CabeceraInventarioProps) {
  return (
    <>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-muted-foreground">
          {contados} artículos contados · {totalCatalogo} en catálogo
        </p>
        <div className="flex items-center gap-2">
          <Button onClick={onGuardar} disabled={guardando || contados === 0}>
            {guardando ? <Loader2 className="size-4 animate-spin mr-1" /> : <Save className="size-4 mr-1" />}
            Guardar conteo
          </Button>
          <Label htmlFor="inventario-almacen" className="text-xs text-muted-foreground">Almacén</Label>
          <Input
            id="inventario-almacen"
            type="number"
            min={0}
            list="inventario-almacenes"
            className="w-24"
            value={almacen}
            onChange={(e) => onAlmacen(e.target.value)}
          />
          <datalist id="inventario-almacenes">
            {almacenes.map((a) => (
              <option key={a.id} value={a.id}>
                {a.nombre}
              </option>
            ))}
          </datalist>
        </div>
      </div>
      <div className="flex flex-col gap-2">
        <p className="text-xs text-muted-foreground">
          {almacen === '0'
            ? 'Almacén «General» (0): stock sin desglose, el que no se ha contado por almacén.'
            : `Conteo del almacén ${almacen}: el ajuste local y el envío a BDP usan este almacén.`}
        </p>
      </div>
    </>
  );
}
