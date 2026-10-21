/* [D26d] Modal de traspaso de stock entre almacenes BDP (`Warehouse/Transfers`).
 *
 * El movimiento no se envía al guardar: queda encolado y sale con el próximo
 * flush autorizado. BDP identifica los almacenes por número, así que el campo
 * es numérico; debajo se listan los almacenes que ya conocemos en el stock local
 * para no tener que memorizar los códigos. BDP solo traspasa artículos
 * inventariables: si no lo es, el backend lo rechaza con un mensaje claro. */
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogClose } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { TraspasarBdpArticleStockRequest } from '@/api/bdp/tipos';
import type { BdpArticleMap } from '@/api/generated/gestionRestauranteAPI.schemas';

/** Almacén conocido por el stock local: id numérico de BDP y nombre. */
export interface AlmacenBdp {
  id: number;
  nombre: string;
}

interface DatosTraspaso {
  origen: string;
  destino: string;
  unidades: string;
  codigo: string;
}

export function TraspasarStockDialog({
  articulo,
  almacenes,
  open,
  onOpenChange,
  onGuardar,
  pending,
}: {
  articulo: BdpArticleMap | null;
  almacenes: AlmacenBdp[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onGuardar: (req: TraspasarBdpArticleStockRequest) => void;
  pending: boolean;
}) {
  const [datos, setDatos] = useState<DatosTraspaso>({ origen: '', destino: '', unidades: '', codigo: '1' });
  const [ultimoKey, setUltimoKey] = useState<string | null>(null);

  /* Se limpia al cerrar y al cambiar de artículo: un traspaso a medias de otro
   * artículo nunca debe reutilizarse. */
  const clave = `${articulo?.id ?? 'none'}|${open ? 'abierto' : 'cerrado'}`;
  if (ultimoKey !== clave) {
    setDatos({ origen: '', destino: '', unidades: '', codigo: '1' });
    setUltimoKey(clave);
  }

  const numero = (valor: string) => (valor.trim() === '' ? Number.NaN : Number(valor));
  /* BDP numera los almacenes de 1 en adelante: el 0 es la marca local de
   * «sin desglose» y el BDP lo rechaza con «EL ALMACÉN DE SALIDA 0 NO EXISTE». */
  const origenValido = Number.isInteger(numero(datos.origen)) && numero(datos.origen) >= 1;
  const destinoNum = numero(datos.destino);
  const destinoValido = Number.isInteger(destinoNum) && destinoNum >= 1 && destinoNum !== numero(datos.origen);
  const unidadesValida = Number.isFinite(numero(datos.unidades)) && numero(datos.unidades) > 0;
  const codigoValido = Number.isInteger(numero(datos.codigo)) && numero(datos.codigo) >= 0;
  const valido = origenValido && destinoValido && unidadesValida && codigoValido;

  const codigoArticulo = articulo?.articulo_glory_codigo ?? '';
  const nombreArticulo = articulo?.articulo_bdp_nombre || codigoArticulo;

  function guardar() {
    onGuardar({
      articulo_glory_codigo: codigoArticulo,
      unidades: datos.unidades.trim(),
      cod_traspaso: numero(datos.codigo),
      almacen_origen: numero(datos.origen),
      almacen_destino: destinoNum,
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Traspasar stock de {nombreArticulo}</DialogTitle>
          <DialogDescription>
            Mueve unidades entre almacenes de BDP. Queda encolado: se envía con el próximo
            envío autorizado a BDP, no al pulsar Guardar.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-2">
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-2">
              <Label htmlFor="traspaso-origen">Almacén de origen</Label>
              <Input
                id="traspaso-origen"
                type="number"
                min={1}
                placeholder="Ej: 1"
                list="almacenes-bdp"
                value={datos.origen}
                onChange={(e) => setDatos((d) => ({ ...d, origen: e.target.value }))}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="traspaso-destino">Almacén de destino</Label>
              <Input
                id="traspaso-destino"
                type="number"
                min={1}
                list="almacenes-bdp"
                placeholder="Ej: 1"
                value={datos.destino}
                onChange={(e) => setDatos((d) => ({ ...d, destino: e.target.value }))}
              />
            </div>
          </div>
          {/* Los almacenes que BDP conoce de verdad no se pueden listar por API:
              esto son los que ya aparecen en el stock local, como ayuda. */}
          <datalist id="almacenes-bdp">
            {almacenes.filter((a) => a.id >= 1).map((a) => (
              <option key={a.id} value={a.id}>
                {a.nombre}
              </option>
            ))}
          </datalist>
          <p className="text-xs text-muted-foreground">
            BDP numera los almacenes desde 1. El «General» (0) del stock local no es un almacén
            de BDP: sirve solo para el stock sin desglose.
          </p>
          {datos.destino.trim() !== '' && !destinoValido && (
            <p className="text-xs text-destructive">
              El destino debe ser un número de almacén distinto del origen.
            </p>
          )}

          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-2">
              <Label htmlFor="traspaso-unidades">Unidades</Label>
              <Input
                id="traspaso-unidades"
                type="number"
                step="any"
                min={0}
                placeholder="Ej: 4"
                value={datos.unidades}
                onChange={(e) => setDatos((d) => ({ ...d, unidades: e.target.value }))}
              />
              {datos.unidades.trim() !== '' && !unidadesValida && (
                <p className="text-xs text-destructive">Las unidades deben ser mayores que cero.</p>
              )}
            </div>
            <div className="grid gap-2">
              <Label htmlFor="traspaso-codigo">Motivo (código BDP)</Label>
              <Input
                id="traspaso-codigo"
                type="number"
                min={0}
                value={datos.codigo}
                onChange={(e) => setDatos((d) => ({ ...d, codigo: e.target.value }))}
              />
              <p className="text-xs text-muted-foreground">
                Es el «CodTransfer» que el restaurante tiene definido en BDP.
              </p>
            </div>
          </div>
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancelar</Button>
          </DialogClose>
          <Button onClick={guardar} disabled={pending || !valido}>
            {pending ? 'Encolando...' : 'Traspasar'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
