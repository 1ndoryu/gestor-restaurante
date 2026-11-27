/* [21AB-12] Confirmación escrita para adoptar el estado que el TPV tiene de una
 * comanda. Guillermo lo pidió así: nada se sobrescribe sin confirmación, y la
 * confirmación es explícita (se escribe comanda y todo), no un «Aceptar».
 *
 * Vive aparte de la tarjeta para que cada archivo quede bajo el límite de líneas. */

import { useState } from 'react';
import { Loader2, TriangleAlert } from 'lucide-react';
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

/** Alias de la comanda tal como los usa el receptor de avisos. El contrato de
 * los avisos es plano y los nombres los fija el TPV, así que se leen igual. */
const ALIAS_COMANDA = [
  'IdentificadorComanda',
  'IdComanda',
  'NumeroComanda',
  'Comanda',
  'OrderId',
];

function normalizar(clave: string): string {
  return clave
    .trim()
    .toLowerCase()
    .normalize('NFD')
    /* Marcas combinantes fuera: «IdentificadorComanda» y «identificador_comanda»
     * tienen que caer en la misma clave, igual que hace el backend. */
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[-_.:\s/]/g, '');
}

/** La comanda del payload del aviso, si viene. Sin ella no se puede confirmar. */
export function comandaDelAviso(payload: unknown): string | null {
  if (!payload || typeof payload !== 'object') return null;
  const buscadas = new Set(ALIAS_COMANDA.map(normalizar));
  for (const [clave, valor] of Object.entries(payload as Record<string, unknown>)) {
    if (!buscadas.has(normalizar(clave))) continue;
    const texto = typeof valor === 'number' ? String(valor) : valor;
    if (typeof texto === 'string' && texto.trim()) return texto.trim();
  }
  return null;
}

interface AceptarTpvDialogoProps {
  abierto: boolean;
  /** Texto exacto que hay que escribir. `null` deshabilita la confirmación: sin
   * comanda no hay nada que adoptar y el diálogo no debe dejar seguir. */
  esperada: string | null;
  enviando: boolean;
  onCerrar: () => void;
  onAceptar: (confirmacion: string) => void;
}

export function AceptarTpvDialogo({
  abierto,
  esperada,
  enviando,
  onCerrar,
  onAceptar,
}: AceptarTpvDialogoProps) {
  const [texto, setTexto] = useState('');
  const esperado = esperada ?? '';
  const vale = esperado !== '' && texto.trim() === esperado;

  return (
    <Dialog
      open={abierto}
      onOpenChange={(nuevo: boolean) => {
        if (!nuevo) {
          setTexto('');
          onCerrar();
        }
      }}
    >
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Aceptar el estado del TPV</DialogTitle>
          <DialogDescription>
            Las líneas de la venta se sustituirán por las que el TPV tiene ahora en la comanda y la
            venta dejará de marcar conflicto. No se escribe nada en el TPV.
          </DialogDescription>
        </DialogHeader>

        <div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
          <TriangleAlert className="mt-0.5 size-4 shrink-0" />
          <span>
            Lo que la web tenía se pierde: si el cambio del TPV no es el correcto, corríjalo antes
            allí.
          </span>
        </div>

        <div className="flex flex-col gap-2">
          {/* Tuteo, igual que el resto de los diálogos de confirmación escrita. */}
          <Label htmlFor="confirmacion-aceptar">Escribe {esperado || 'ACEPTAR …'}</Label>
          <Input
            id="confirmacion-aceptar"
            value={texto}
            onChange={(evento) => setTexto(evento.target.value)}
            placeholder={esperado}
            autoComplete="off"
          />
        </div>

        <DialogFooter>
          <Button type="button" variant="secondary" onClick={onCerrar} disabled={enviando}>
            Cancelar
          </Button>
          <Button
            type="button"
            onClick={() => {
              onAceptar(texto.trim());
              setTexto('');
            }}
            disabled={!vale || enviando}
          >
            {enviando ? <Loader2 className="size-4 animate-spin" /> : null}
            Aceptar estado del TPV
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
