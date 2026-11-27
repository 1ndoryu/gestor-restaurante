/* [L9] Buzón de avisos del TPV: lo que el TPV manda a la web y cómo se procesó.
 *
 * Sin esta tarjeta, un aviso que llegaba mal o que fallaba al procesarse se
 * quedaba invisible: el restaurante no podía saber que su comanda no había
 * cruzado. Aquí se ve el estado, el motivo del fallo y se puede reintentar. */

import { useState } from 'react';
import { Inbox, Loader2, Play, RefreshCw, RotateCcw, ShieldCheck, TriangleAlert } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ESTADOS_AVISO, useBuzonAvisos, type Aviso } from './useBuzonAvisos';
import { AceptarTpvDialogo, comandaDelAviso } from './AceptarTpvDialogo';
import { AyudaTarjeta } from '../configuracion/AyudaTarjeta';

/** Los estados en que un aviso todavía pide algo de una persona. */
const ESTADOS_ATENCION = new Set(['requiere_revision', 'rechazado']);

const ETIQUETA_ESTADO: Record<string, string> = {
  recibido: 'Sin procesar',
  procesando: 'En curso',
  procesado: 'Procesado',
  requiere_revision: 'Requiere revisión',
  rechazado: 'Rechazado',
};

function fecha(iso: string): string {
  const valor = new Date(iso);
  return Number.isNaN(valor.getTime()) ? iso : valor.toLocaleString();
}

/** Etiqueta del aviso: la acción tal como la nombra el TPV. */
const ETIQUETA_ACCION: Record<string, string> = {
  crear: 'Comanda creada',
  modificar: 'Comanda modificada',
  'cambiar-estado': 'Cambio de estado',
  borrar: 'Comanda borrada',
  finalizar: 'Comanda finalizada',
  traspasar: 'Traspaso de mesa',
  'traspasar-linea': 'Traspaso de línea',
};

function Fila({ aviso, reintentando, aceptando, onReintentar, onAceptar }: {
  aviso: Aviso;
  reintentando: boolean;
  aceptando: boolean;
  onReintentar: () => void;
  onAceptar: () => void;
}) {
  const atencion = ESTADOS_ATENCION.has(aviso.estado);
  /* [21AB-12] Solo tiene sentido conciliar si el aviso está en revisión y el
   * payload dice de qué comanda habla: sin comanda no hay nada que reclamar al TPV. */
  const comanda = comandaDelAviso(aviso.payload);
  return (
    <div className="flex flex-col gap-2 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm font-medium">
          {ETIQUETA_ACCION[aviso.accion] ?? aviso.accion}
        </span>
        <Badge variant={atencion ? 'destructive' : 'secondary'}>
          {ETIQUETA_ESTADO[aviso.estado] ?? aviso.estado}
        </Badge>
        {aviso.intentos > 0 && (
          <span className="text-xs text-muted-foreground">
            {aviso.intentos === 1 ? '1 intento' : `${aviso.intentos} intentos`}
          </span>
        )}
        <span className="ml-auto text-xs tabular-nums text-muted-foreground">
          {fecha(aviso.recibido_en)}
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        <span className="font-mono">{aviso.clave_idempotencia}</span>
        {aviso.origen && <span>· desde {aviso.origen}</span>}
        {aviso.procesado_en && <span>· procesado {fecha(aviso.procesado_en)}</span>}
      </div>

      {aviso.ultimo_error && (
        <p className="flex items-start gap-2 rounded-md bg-destructive/10 p-2 text-sm text-destructive">
          <TriangleAlert className="mt-0.5 size-4 shrink-0" />
          <span>{aviso.ultimo_error}</span>
        </p>
      )}

      <div className="flex flex-wrap items-center gap-2">
        {aviso.estado === 'requiere_revision' && (
          <Button type="button" size="sm" variant="secondary" onClick={onReintentar} disabled={reintentando || aceptando}>
            {reintentando ? <Loader2 className="size-4 animate-spin" /> : <RotateCcw className="size-4" />}
            Reabrir
          </Button>
        )}
        {aviso.estado === 'requiere_revision' && comanda && (
          <Button type="button" size="sm" onClick={onAceptar} disabled={reintentando || aceptando}>
            {aceptando ? <Loader2 className="size-4 animate-spin" /> : <ShieldCheck className="size-4" />}
            Aceptar el estado del TPV
          </Button>
        )}
        {/* El payload puede traer datos de cliente: se guarda tal cual llegó y se
            muestra plegado, solo si alguien va a diagnosticar el aviso. */}
        <details className="w-full text-xs">
          <summary className="cursor-pointer text-muted-foreground">Ver lo que mandó el TPV</summary>
          <pre className="mt-2 max-h-48 overflow-auto rounded-md bg-muted p-2">
            {JSON.stringify(aviso.payload, null, 2)}
          </pre>
        </details>
      </div>
    </div>
  );
}

export default function BuzonAvisosCard() {
  const {
    avisos,
    filtro,
    setFiltro,
    cargando,
    procesando,
    reintentando,
    aceptando,
    resumen,
    cargar,
    procesar,
    reintentar,
    aceptarTpv,
  } = useBuzonAvisos();

  /* [21AB-12] Qué aviso se va a conciliar y con qué comanda. La confirmación la
   * pide el diálogo: nada se adopta con un clic solo. */
  const [conciliando, setConciliando] = useState<{ id: string; comanda: string } | null>(null);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Inbox className="size-4" />
          Avisos del TPV
        </CardTitle>
        <CardDescription>
          Lo que el TPV manda a la web (comandas creadas, modificadas, traspasos, cobros) queda
          aquí hasta cruzarse con BDP. Procesar un lote solo lee de BDP: no cambia nada en el TPV.
        </CardDescription>
        <CardAction>
          <AyudaTarjeta id="bdp-avisos" titulo="Avisos del TPV" />
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center gap-2">
          <Select value={filtro} onValueChange={setFiltro}>
            <SelectTrigger className="w-56">
              <SelectValue placeholder="Todos" />
            </SelectTrigger>
            <SelectContent>
              {ESTADOS_AVISO.map((estado) => (
                <SelectItem key={estado.valor || 'todos'} value={estado.valor}>
                  {estado.etiqueta}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button type="button" variant="secondary" onClick={() => void cargar(filtro)} disabled={cargando}>
            <RefreshCw className={`size-4 ${cargando ? 'animate-spin' : ''}`} />
            Actualizar
          </Button>
          <Button type="button" onClick={() => void procesar()} disabled={procesando}>
            {procesando ? <Loader2 className="size-4 animate-spin" /> : <Play className="size-4" />}
            Procesar lote
          </Button>
          {resumen && (
            <span className="text-xs text-muted-foreground">
              {resumen.procesados} procesados · {resumen.reintentos} para reintentar ·{' '}
              {resumen.revision} para revisar
            </span>
          )}
        </div>

        {cargando && avisos.length === 0 ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Leyendo el buzón...
          </div>
        ) : avisos.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No hay avisos con ese filtro. Si el TPV está configurado para avisar, aparecerán aquí
            en cuanto mande el primero.
          </p>
        ) : (
          <div className="flex flex-col divide-y rounded-md border">
            {avisos.map((aviso) => {
              const comanda = comandaDelAviso(aviso.payload);
              return (
                <Fila
                  key={aviso.id}
                  aviso={aviso}
                  reintentando={reintentando === aviso.id}
                  aceptando={aceptando === aviso.id}
                  onReintentar={() => void reintentar(aviso.id)}
                  onAceptar={() => {
                    if (comanda) setConciliando({ id: aviso.id, comanda });
                  }}
                />
              );
            })}
          </div>
        )}
      </CardContent>

      <AceptarTpvDialogo
        abierto={conciliando !== null}
        esperada={conciliando ? `ACEPTAR ${conciliando.comanda}` : null}
        enviando={aceptando !== null}
        onCerrar={() => setConciliando(null)}
        onAceptar={(confirmacion: string) => {
          if (!conciliando) return;
          void aceptarTpv(conciliando.id, confirmacion)
            .then((bien: boolean) => {
              if (bien) setConciliando(null);
            })
            /* `aceptarTpv` ya avisa del fallo y deja el diálogo abierto para
             * corregir la confirmación; aquí solo se evita la promesa suelta. */
            .catch(() => undefined);
        }}
      />
    </Card>
  );
}
