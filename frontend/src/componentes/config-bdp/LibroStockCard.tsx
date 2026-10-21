/* [D26g] Libro de stock: las operaciones que BDP dejó ambiguas y cómo cerrarlas.
 *
 * Cuando la respuesta de un ajuste se pierde, el envío se decide releyendo el
 * stock del BDP: si el valor es S+V, ya está; si es S, se reintenta una vez; si
 * es otro, no se puede atribuir (una venta de caja también mueve stock) y se
 * para. Esa parada bloquea el artículo hasta que alguien mire el TPV y diga qué
 * pasó. Sin esta tarjeta ese bloqueo no se podía levantar desde la web. */

import { useState } from 'react';
import { BookOpenCheck, Loader2, RefreshCw, TriangleAlert } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { useLibroStock, type DecisionStock, type RevisionStock } from './useLibroStock';

const ETIQUETA_OPERACION: Record<string, string> = {
  regularizar: 'Ajuste de stock',
  traspasar: 'Traspaso',
  inventario: 'Inventario',
};

/** Número como lo enseña el BDP: sin decimales de más y con coma decimal. */
function unidades(valor: number | null): string {
  if (valor === null || valor === undefined) return '—';
  return valor.toLocaleString('es-ES', { maximumFractionDigits: 3 });
}

function fecha(iso: string): string {
  const valor = new Date(iso);
  return Number.isNaN(valor.getTime()) ? iso : valor.toLocaleString();
}

/** Cada fila se resuelve con una decisión y un motivo; el motivo va dentro de
 * la propia fila porque es lo que hace auditable el dictamen. */
function Fila({
  fila,
  resolviendo,
  onResolver,
}: {
  fila: RevisionStock;
  resolviendo: boolean;
  onResolver: (decision: DecisionStock, motivo: string) => void;
}) {
  const [motivo, setMotivo] = useState('');
  const valido = motivo.trim().length >= 5;

  return (
    <div className="flex flex-col gap-3 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm font-medium">
          {ETIQUETA_OPERACION[fila.operacion] ?? fila.operacion}
        </span>
        <Badge variant="destructive">Requiere revisión</Badge>
        <span className="font-mono text-xs text-muted-foreground">
          artículo {fila.articulo_codigo} · almacén {fila.almacen}
        </span>
        <span className="ml-auto text-xs tabular-nums text-muted-foreground">
          {fecha(fila.creado_en)}
        </span>
      </div>

      <div className="grid grid-cols-3 gap-2 rounded-md bg-muted/50 p-2 text-xs">
        <div>
          <p className="text-muted-foreground">Antes (S)</p>
          <p className="tabular-nums">{unidades(fila.stock_previo)}</p>
        </div>
        <div>
          <p className="text-muted-foreground">Esperado (S+V)</p>
          <p className="tabular-nums">{unidades(fila.stock_esperado)}</p>
        </div>
        <div>
          <p className="text-muted-foreground">Visto ahora</p>
          <p className="tabular-nums">{unidades(fila.stock_posterior)}</p>
        </div>
      </div>

      {fila.nota && (
        <p className="flex items-start gap-2 rounded-md bg-destructive/10 p-2 text-sm text-destructive">
          <TriangleAlert className="mt-0.5 size-4 shrink-0" />
          <span>{fila.nota}</span>
        </p>
      )}

      <p className="text-xs text-muted-foreground">
        Mira el artículo en el TPV y decide: si el stock ya incluye el movimiento, ciérralo como
        aplicado y no se reenvía; si no lo incluye, vuelve a la cola.
      </p>

      <div className="flex flex-wrap items-center gap-2">
        <Input
          value={motivo}
          onChange={(evento) => setMotivo(evento.target.value)}
          placeholder="Qué has visto en el TPV (mínimo 5 caracteres)"
          className="max-w-sm"
        />
        <Button
          type="button"
          size="sm"
          onClick={() => onResolver('aplicado', motivo)}
          disabled={!valido || resolviendo}
        >
          {resolviendo ? <Loader2 className="size-4 animate-spin" /> : null}
          Ya está aplicado
        </Button>
        <Button
          type="button"
          size="sm"
          variant="secondary"
          onClick={() => onResolver('no_aplicado', motivo)}
          disabled={!valido || resolviendo}
        >
          No se aplicó
        </Button>
      </div>
    </div>
  );
}

export default function LibroStockCard() {
  const { filas, cargando, resolviendo, cargar, resolver } = useLibroStock();

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <BookOpenCheck className="size-4" />
          Libro de stock
        </CardTitle>
        <CardDescription>
          Operaciones de stock cuyo resultado no se pudo confirmar con BDP: la respuesta se perdió
          y el stock no cuadra ni con el valor previo ni con el esperado. Mientras no se resuelvan,
          ese artículo y almacén no admiten otro movimiento.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="secondary" onClick={() => void cargar()} disabled={cargando}>
            <RefreshCw className={`size-4 ${cargando ? 'animate-spin' : ''}`} />
            Actualizar
          </Button>
          <span className="text-xs text-muted-foreground">
            {filas.length === 0 ? 'Sin operaciones pendientes' : `${filas.length} para revisar`}
          </span>
        </div>

        {cargando && filas.length === 0 ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Leyendo el libro...
          </div>
        ) : filas.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No hay operaciones de stock ambiguas. Aquí aparecerá cualquiera cuyo envío a BDP no se
            pueda confirmar ni descartar.
          </p>
        ) : (
          <div className="flex flex-col divide-y rounded-md border">
            {filas.map((fila) => (
              <Fila
                key={fila.id_local}
                fila={fila}
                resolviendo={resolviendo === fila.id_local}
                onResolver={(decision, motivo) => void resolver(fila.id_local, decision, motivo)}
              />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
