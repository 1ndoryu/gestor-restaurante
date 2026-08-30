/* [128A-1/F5] Modal de alta/edición de albaranes de compra locales.
 * Solo aplica a `origen='local'`; los importados de BDP no se editan. */

import { useState } from 'react';
import { Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type {
  ActualizarBdpPurchaseNoteRequest,
  BdpPurchaseNote,
  BdpPurchaseNoteLineaLocal,
  CrearBdpPurchaseNoteRequest,
} from '@/api/bdp';

interface LineaForm {
  key: number;
  descripcion: string;
  cantidad: string;
  precio_unitario: string;
  iva_pct: string;
}

interface BdpComprasLocalModalProps {
  open: boolean;
  /** null = crear; con valor = editar ese albarán. */
  note: BdpPurchaseNote | null;
  isSubmitting: boolean;
  onClose: () => void;
  onSubmit: (req: CrearBdpPurchaseNoteRequest | ActualizarBdpPurchaseNoteRequest) => void;
}

function lineasDesdeDatos(datos: Record<string, unknown>): LineaForm[] {
  const raw = Array.isArray(datos.lineas) ? datos.lineas : [];
  return raw
    .map((item, index) => {
      const linea = item as Partial<BdpPurchaseNoteLineaLocal>;
      return {
        key: index,
        descripcion: linea.descripcion ?? '',
        cantidad: linea.cantidad != null ? String(linea.cantidad) : '',
        precio_unitario: linea.precio_unitario != null ? String(linea.precio_unitario) : '',
        iva_pct: linea.iva_pct != null ? String(linea.iva_pct) : '',
      };
    })
    .filter((l) => l.descripcion || l.cantidad || l.precio_unitario || l.iva_pct);
}

export function BdpComprasLocalModal({
  open,
  note,
  isSubmitting,
  onClose,
  onSubmit,
}: BdpComprasLocalModalProps) {
  const isEdit = note !== null;
  const [serie, setSerie] = useState('');
  const [numero, setNumero] = useState('');
  const [fecha, setFecha] = useState('');
  const [nombreProveedor, setNombreProveedor] = useState('');
  const [codigoProveedor, setCodigoProveedor] = useState('');
  const [total, setTotal] = useState('');
  const [lineas, setLineas] = useState<LineaForm[]>([]);
  const [nextLineaKey, setNextLineaKey] = useState(1);
  const [error, setError] = useState('');

  /* Sincroniza el formulario cada vez que se abre el modal. */
  const [lastOpenedFor, setLastOpenedFor] = useState<string | null>(null);
  const openedKey = isEdit ? note.id : 'nuevo';
  if (open && lastOpenedFor !== openedKey) {
    setLastOpenedFor(openedKey);
    setError('');
    if (note) {
      setSerie(note.serie);
      setNumero(note.numero);
      setFecha(note.fecha ?? '');
      setNombreProveedor(note.nombre_proveedor ?? '');
      setCodigoProveedor(note.codigo_proveedor ?? '');
      setTotal(note.total ?? '');
      const iniciales = lineasDesdeDatos(note.datos_bdp);
      setLineas(iniciales);
      setNextLineaKey(iniciales.length + 1);
    } else {
      setSerie('L');
      setNumero('');
      setFecha(new Date().toISOString().slice(0, 10));
      setNombreProveedor('');
      setCodigoProveedor('');
      setTotal('');
      setLineas([]);
      setNextLineaKey(1);
    }
  }

  function addLinea() {
    setLineas((prev) => [
      ...prev,
      { key: nextLineaKey, descripcion: '', cantidad: '', precio_unitario: '', iva_pct: '21' },
    ]);
    setNextLineaKey((k) => k + 1);
  }

  function updateLinea(key: number, campo: keyof LineaForm, valor: string) {
    setLineas((prev) => prev.map((l) => (l.key === key ? { ...l, [campo]: valor } : l)));
  }

  function removeLinea(key: number) {
    setLineas((prev) => prev.filter((l) => l.key !== key));
  }

  /* Normaliza decimales con coma (formato español) a punto antes de enviar;
   * el backend espera `Decimal` con punto (serde). [208A-2/F5] */
  function normalizarDecimal(valor: string): string {
    const limpio = valor.trim().replace(/\s/g, '');
    return limpio.replace(',', '.');
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const lineasValidas: BdpPurchaseNoteLineaLocal[] = lineas
      .filter((l) => l.descripcion.trim() && l.cantidad !== '' && l.precio_unitario !== '')
      .map((l) => ({
        descripcion: l.descripcion.trim(),
        cantidad: normalizarDecimal(l.cantidad),
        precio_unitario: normalizarDecimal(l.precio_unitario),
        iva_pct: l.iva_pct === '' ? '21' : normalizarDecimal(l.iva_pct),
      }));
    const tieneProveedor = nombreProveedor.trim() !== '' || codigoProveedor.trim() !== '';
    const tieneImporte = total.trim() !== '' || lineasValidas.length > 0;
    if (!tieneProveedor) {
      setError('Indica el proveedor (nombre o código)');
      return;
    }
    if (!tieneImporte) {
      setError('Indica un total o al menos una línea completa');
      return;
    }
    const base = {
      numero: numero.trim() || undefined,
      fecha: fecha || undefined,
      codigo_proveedor: codigoProveedor.trim() || undefined,
      nombre_proveedor: nombreProveedor.trim() || undefined,
      total: total.trim() ? normalizarDecimal(total) : undefined,
      lineas: lineasValidas.length > 0 ? lineasValidas : undefined,
    };
    if (isEdit) {
      onSubmit(base as ActualizarBdpPurchaseNoteRequest);
    } else {
      onSubmit({ ...base, serie: serie.trim() || undefined } as CrearBdpPurchaseNoteRequest);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{isEdit ? 'Editar albarán local' : 'Nuevo albarán local'}</DialogTitle>
          <DialogDescription>
            {isEdit
              ? `Serie ${note.serie} — Número ${note.numero}. Los albaranes locales se guardan sin conexión a BDP.`
              : 'Crea un albarán de compra en modo independiente. No se envía nada a BDP.'}
          </DialogDescription>
        </DialogHeader>

        <form id="albaran-local-form" onSubmit={handleSubmit} className="space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="albaran-serie">Serie</Label>
              <Input
                id="albaran-serie"
                value={serie}
                onChange={(e) => setSerie(e.target.value)}
                placeholder="L"
                disabled={isEdit}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="albaran-numero">Número (opcional)</Label>
              <Input
                id="albaran-numero"
                value={numero}
                onChange={(e) => setNumero(e.target.value)}
                placeholder="Siguiente secuencial"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="albaran-fecha">Fecha</Label>
            <Input
              id="albaran-fecha"
              type="date"
              value={fecha}
              onChange={(e) => setFecha(e.target.value)}
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="albaran-nombre">Proveedor (nombre)</Label>
              <Input
                id="albaran-nombre"
                value={nombreProveedor}
                onChange={(e) => setNombreProveedor(e.target.value)}
                placeholder="Nombre del proveedor"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="albaran-codigo">Proveedor (código)</Label>
              <Input
                id="albaran-codigo"
                value={codigoProveedor}
                onChange={(e) => setCodigoProveedor(e.target.value)}
                placeholder="Código opcional"
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="albaran-total">Total (si no indicas líneas)</Label>
            <Input
              id="albaran-total"
              inputMode="decimal"
              value={total}
              onChange={(e) => setTotal(e.target.value)}
              placeholder="0.00"
            />
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>Líneas (IVA por línea)</Label>
              <Button type="button" variant="outline" size="sm" onClick={addLinea}>
                <Plus className="mr-1 size-3.5" />
                Añadir línea
              </Button>
            </div>
            {lineas.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                Sin líneas. El total se toma del campo anterior.
              </p>
            ) : (
              <div className="space-y-2">
                {lineas.map((linea) => (
                  <div key={linea.key} className="grid grid-cols-[1fr_auto_auto_auto_auto] gap-2 items-center">
                    <Input
                      value={linea.descripcion}
                      onChange={(e) => updateLinea(linea.key, 'descripcion', e.target.value)}
                      placeholder="Descripción"
                      aria-label="Descripción de la línea"
                    />
                    <Input
                      className="w-20"
                      inputMode="decimal"
                      value={linea.cantidad}
                      onChange={(e) => updateLinea(linea.key, 'cantidad', e.target.value)}
                      placeholder="Cant."
                      aria-label="Cantidad"
                    />
                    <Input
                      className="w-24"
                      inputMode="decimal"
                      value={linea.precio_unitario}
                      onChange={(e) => updateLinea(linea.key, 'precio_unitario', e.target.value)}
                      placeholder="Precio"
                      aria-label="Precio unitario"
                    />
                    <Input
                      className="w-20"
                      inputMode="decimal"
                      value={linea.iva_pct}
                      onChange={(e) => updateLinea(linea.key, 'iva_pct', e.target.value)}
                      placeholder="IVA %"
                      aria-label="Porcentaje de IVA"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => removeLinea(linea.key)}
                      aria-label="Eliminar línea"
                    >
                      <Trash2 className="size-4 text-destructive" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>

          {error && <p className="text-sm text-destructive">{error}</p>}
        </form>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="submit" form="albaran-local-form" disabled={isSubmitting}>
            {isEdit ? 'Guardar cambios' : 'Crear albarán'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
