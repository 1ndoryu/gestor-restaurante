/* [D28-auto-R] Documentos de venta leídos del BDP.
 *
 * La lectura existía desde D28 sin ninguna pantalla y con el disparo solo en
 * variables de entorno: en la práctica nadie podía ver los documentos ni
 * encender la lectura. Aquí se ven los últimos guardados, se fuerza una lectura
 * del perfil configurado y se decide cada cuánto se lee sola.
 *
 * Los campos del documento no están documentados por el distribuidor: la tarjeta
 * enseña serie, número y total cuando el JSON los trae y, si no, el JSON en crudo. */

import { useState } from 'react';
import { ChevronDown, ChevronRight, FileText, Loader2, RefreshCw } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { EstadoConfiguracion } from '@/hooks/useConfiguracion';
import { useDocumentosVenta, type DocumentoVenta } from './useDocumentosVenta';

/** Cadencia en segundos → texto para quien no piensa en segundos. */
function cadenciaTexto(segundos: number): string {
  if (segundos === 0) return 'No se lee sola';
  if (segundos % 3600 === 0) return `Cada ${segundos / 3600} h`;
  if (segundos % 60 === 0) return `Cada ${segundos / 60} min`;
  return `Cada ${segundos} s`;
}

function fecha(iso: string | null): string {
  if (!iso) return '—';
  const valor = new Date(iso.length === 10 ? `${iso}T00:00:00` : iso);
  return Number.isNaN(valor.getTime()) ? iso : valor.toLocaleDateString('es-ES');
}

function importe(total: DocumentoVenta['total']): string {
  if (total === null || total === undefined) return '—';
  const numero = Number(total);
  return Number.isNaN(numero)
    ? String(total)
    : numero.toLocaleString('es-ES', { style: 'currency', currency: 'EUR' });
}

/** Una fila con su JSON desplegable: es la única forma de ver un campo que el
 *  parser no conozca, y la lectura no escribe nada que se pueda romper. */
function Fila({ documento }: { documento: DocumentoVenta }) {
  const [abierta, setAbierta] = useState(false);
  const Icono = abierta ? ChevronDown : ChevronRight;
  return (
    <div className="border-b last:border-b-0">
      <button
        type="button"
        onClick={() => setAbierta((previa) => !previa)}
        className="flex w-full flex-wrap items-center gap-2 px-3 py-2 text-left hover:bg-muted/50"
        aria-expanded={abierta}
      >
        <Icono className="size-4 shrink-0 text-muted-foreground" />
        <span className="text-sm font-medium">{fecha(documento.fecha)}</span>
        {/* BDP manda serie y número por separado, y el número como decimal. */}
        <span className="font-mono text-xs">
          {[documento.serie, documento.numero].filter(Boolean).join(' ') ||
            documento.bdp_documento_id}
        </span>
        <Badge variant="outline">perfil {documento.perfil}</Badge>
        <span className="ml-auto text-sm tabular-nums">{importe(documento.total)}</span>
      </button>
      {abierta && (
        <pre className="mx-3 mb-3 max-h-64 overflow-auto rounded bg-muted p-2 text-xs">
          {JSON.stringify(documento.datos_bdp, null, 2)}
        </pre>
      )}
    </div>
  );
}

/** Cadencias ofrecidas: de «apagada» a una vez por hora, que es el rango útil. */
const CADENCIAS = [0, 300, 900, 1800, 3600];

interface Props {
  config: EstadoConfiguracion;
  cambiarCampo: <K extends keyof EstadoConfiguracion>(campo: K, valor: EstadoConfiguracion[K]) => void;
}

export function DocumentosVentaCard({ config, cambiarCampo }: Props) {
  const { documentos, cargando, leyendo, leerAhora } = useDocumentosVenta();
  const cadencia = Number(config.bdp_documentos_auto_segundos) || 0;
  const perfil = config.bdp_documentos_perfil ?? null;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <FileText className="size-4" /> Documentos de venta del BDP
        </CardTitle>
        <CardDescription>
          Las facturas y tickets que el TPV emite. Se leen por perfil de exportación y no
          se escribe nada en el BDP.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {/* Cadencia y perfil se editan aquí pero se guardan con el botón de «A qué
            se conecta»: son configuración del restaurante y comparten formulario. */}
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <Label htmlFor="documentos-cadencia" className="text-xs text-muted-foreground">
              Lectura automática
            </Label>
            <Select
              value={String(cadencia)}
              onValueChange={(valor) => cambiarCampo('bdp_documentos_auto_segundos', Number(valor))}
            >
              <SelectTrigger id="documentos-cadencia" className="w-44">
                <SelectValue>{cadenciaTexto(cadencia)}</SelectValue>
              </SelectTrigger>
              <SelectContent>
                {[...new Set([...CADENCIAS, cadencia])].sort((a, b) => a - b).map((segundos) => (
                  <SelectItem key={segundos} value={String(segundos)}>
                    {cadenciaTexto(segundos)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1">
            <Label htmlFor="documentos-perfil" className="text-xs text-muted-foreground">
              Perfil de exportación
            </Label>
            <Input
              id="documentos-perfil"
              type="number"
              min={1}
              className="w-32"
              placeholder="Sin configurar"
              value={perfil ?? ''}
              onChange={(evento) =>
                cambiarCampo(
                  'bdp_documentos_perfil',
                  evento.target.value ? Number(evento.target.value) : null,
                )
              }
            />
          </div>
          <Button
            type="button"
            variant="outline"
            className="ml-auto"
            disabled={leyendo || cargando}
            onClick={() => void leerAhora(perfil)}
          >
            {leyendo ? (
              <Loader2 className="mr-2 size-4 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 size-4" />
            )}
            Leer ahora
          </Button>
        </div>

        {!perfil && (
          <p className="text-sm text-muted-foreground">
            Indica el perfil de exportación del BDP para poder leer.
          </p>
        )}

        {cargando ? (
          <p className="text-sm text-muted-foreground">Cargando documentos…</p>
        ) : documentos.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            Todavía no hay documentos guardados. «Leer ahora» trae los de la ventana pendiente.
          </p>
        ) : (
          <div className="rounded-md border">
            {documentos.map((documento) => (
              <Fila key={`${documento.perfil}-${documento.bdp_documento_id}`} documento={documento} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
