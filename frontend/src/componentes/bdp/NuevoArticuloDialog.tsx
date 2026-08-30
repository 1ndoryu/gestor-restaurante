/* [208A-2/C2] Diálogo de alta de artículo desde Stock (decisión D2).
 * Reutiliza el mismo flujo de alta que el CRUD del Catálogo
 * (POST /api/bdp/article-maps): crea el artículo local (origen 'local') y,
 * si el modo efectivo es BDP, el backend encola el alta; en standalone no se
 * encola ni envía nada (invariante 128A-1/198A-1). El código BDP es opcional:
 * sin él el artículo es 100% local. */

import { Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogClose } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useCrearArticleMap } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useNuevoArticuloForm } from '@/hooks/useNuevoArticuloForm';

interface NuevoArticuloDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreado?: () => void;
}

function NuevoArticuloDialog({ open, onOpenChange, onCreado }: NuevoArticuloDialogProps) {
  const queryClient = useQueryClient();
  const {
    codigo,
    codigoBdp,
    descripcion,
    precio,
    iva,
    setCodigo,
    setCodigoBdp,
    setDescripcion,
    setPrecio,
    setIva,
    codigoValido,
    descripcionValida,
    limpiar,
  } = useNuevoArticuloForm();

  const crearMutation = useCrearArticleMap({
    mutation: {
      onSuccess: () => {
        toast.success('Artículo creado');
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
        limpiar();
        onOpenChange(false);
        onCreado?.();
      },
      onError: () => toast.error('Error al crear artículo'),
    },
  });

  function crear() {
    if (!codigoValido || !descripcionValida) return;
    crearMutation.mutate({
      data: {
        articulo_glory_codigo: codigo.trim(),
        articulo_bdp_codigo: codigoBdp.trim() || undefined,
        descripcion: descripcion.trim() || undefined,
        precio_tarifa1: precio ? String(precio) : undefined,
        iva_pct: iva ? String(iva) : undefined,
      },
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Nuevo artículo</DialogTitle>
          <DialogDescription>
            Crea un artículo local (funciona sin BDP). Si la integración BDP está activa, el alta se
            encola para enviarse al terminal.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-2">
          <div className="grid gap-2">
            <Label htmlFor="nuevo-articulo-codigo">Código *</Label>
            <Input
              id="nuevo-articulo-codigo"
              className="font-mono text-xs"
              value={codigo}
              onChange={(e) => setCodigo(e.target.value)}
              placeholder="SKU interno"
              maxLength={50}
            />
            {!codigoValido && (
              <p className="text-xs text-muted-foreground">Código obligatorio (identifica el artículo localmente).</p>
            )}
          </div>
          <div className="grid gap-2">
            <Label htmlFor="nuevo-articulo-descripcion">Nombre / descripción *</Label>
            <Input
              id="nuevo-articulo-descripcion"
              value={descripcion}
              onChange={(e) => setDescripcion(e.target.value)}
              placeholder="Ej: Café con leche"
              maxLength={255}
            />
            {!descripcionValida && (
              <p className="text-xs text-muted-foreground">Nombre obligatorio.</p>
            )}
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-2">
              <Label htmlFor="nuevo-articulo-precio">Precio (€)</Label>
              <Input
                id="nuevo-articulo-precio"
                type="number"
                min="0"
                step="0.01"
                value={precio}
                onChange={(e) => setPrecio(e.target.value)}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="nuevo-articulo-iva">IVA (%)</Label>
              <Input
                id="nuevo-articulo-iva"
                type="number"
                min="0"
                step="0.01"
                value={iva}
                onChange={(e) => setIva(e.target.value)}
              />
            </div>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="nuevo-articulo-bdp">Código BDP (opcional)</Label>
            <Input
              id="nuevo-articulo-bdp"
              className="font-mono text-xs"
              value={codigoBdp}
              onChange={(e) => setCodigoBdp(e.target.value)}
              placeholder="Vacío = artículo solo local"
            />
            <p className="text-xs text-muted-foreground">
              Si el artículo ya existe en BDP, indica su código para mapearlo; si no, quedará local.
            </p>
          </div>
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancelar</Button>
          </DialogClose>
          <Button
            onClick={crear}
            disabled={!codigoValido || !descripcionValida || crearMutation.isPending}
          >
            <Plus className="size-3.5 mr-1" />
            Crear artículo
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default NuevoArticuloDialog;
