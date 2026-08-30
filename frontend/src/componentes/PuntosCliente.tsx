/* [198A-1/D9] Sección de puntos de fidelización en la ficha de cliente.
 * Ledger local (sumar/restar con motivo) 100% operativo sin BDP; el push
 * AddPoints lo encola el backend solo si el cliente tiene bdp_customer_code. */

import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { toast } from 'sonner';
import { usePuntosCliente, useSumarPuntosCliente } from '../api/bdp';

function PuntosCliente({ clienteId }: { clienteId: string }) {
  const queryClient = useQueryClient();
  const { data, isLoading } = usePuntosCliente(clienteId);
  const sumarMutation = useSumarPuntosCliente(queryClient);
  const [puntos, setPuntos] = useState('');
  const [motivo, setMotivo] = useState('');
  const [sumar, setSumar] = useState(true);

  const enviar = () => {
    const puntosNum = Number(puntos);
    if (!Number.isFinite(puntosNum) || puntosNum === 0 || !motivo.trim()) return;
    const importe = sumar ? Math.abs(puntosNum) : -Math.abs(puntosNum);
    sumarMutation.mutate(
      { clienteId, req: { points_added: String(importe), reason: motivo.trim() } },
      {
        onSuccess: () => {
          toast.success('Movimiento de puntos registrado');
          setPuntos('');
          setMotivo('');
        },
        onError: () => toast.error('No se pudo registrar el movimiento de puntos'),
      },
    );
  };

  return (
    <div className="rounded-md border p-3 flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-muted-foreground">Puntos de fidelización</span>
        <span className="font-semibold tabular-nums">{isLoading ? '…' : data?.saldo ?? '0'}</span>
      </div>
      {data && data.historial.length > 0 && (
        <div className="max-h-24 overflow-auto text-xs text-muted-foreground">
          {data.historial.slice(0, 5).map((h) => (
            <div key={h.id} className="flex justify-between gap-2 border-b border-muted/40 py-1 last:border-0">
              <span className="truncate">{h.reason}</span>
              <span className={Number(h.points_added) >= 0 ? 'text-emerald-700' : 'text-destructive'}>
                {Number(h.points_added) >= 0 ? '+' : ''}{h.points_added}
              </span>
            </div>
          ))}
        </div>
      )}
      <div className="grid grid-cols-[1fr_auto] gap-2 items-end">
        <div className="flex flex-col gap-1">
          <Label htmlFor={`puntos-cant-${clienteId}`}>Puntos</Label>
          <Input id={`puntos-cant-${clienteId}`} type="number" step="any" value={puntos} onChange={(e) => setPuntos(e.target.value)} placeholder="Ej: 50" />
        </div>
        <div className="flex items-center gap-1">
          <Button size="sm" variant={sumar ? 'default' : 'outline'} onClick={() => setSumar(true)}>Sumar</Button>
          <Button size="sm" variant={!sumar ? 'destructive' : 'outline'} onClick={() => setSumar(false)}>Restar</Button>
        </div>
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor={`puntos-motivo-${clienteId}`}>Motivo</Label>
        <Input id={`puntos-motivo-${clienteId}`} value={motivo} onChange={(e) => setMotivo(e.target.value)} placeholder="Ej: promoción, reclamación..." maxLength={255} />
      </div>
      <Button size="sm" onClick={enviar} disabled={sumarMutation.isPending || !puntos || Number(puntos) === 0 || !motivo.trim()}>
        {sumarMutation.isPending ? 'Guardando…' : 'Registrar movimiento'}
      </Button>
    </div>
  );
}

export default PuntosCliente;
