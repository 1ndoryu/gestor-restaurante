/* Modal de exportación masiva de clientes a BDP (Glory → BDP).
 * Muestra lo que se va a subir (pendientes + código por fila), pide la frase
 * exacta de confirmación y arranca en MODO SIMULACIÓN: en ese modo el botón
 * solo valida y previsualiza el payload — no hace ningún POST a bdp-sync. */
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import type { Cliente } from '@/api/generated';
import type { ResultadoExportar } from '@/hooks/useExportarClientesBdp';

interface Props {
  abierto: boolean;
  onAbiertoChange: (abierto: boolean) => void;
  pendientes: Cliente[] | null;
  cargando: boolean;
  codigos: Record<string, string>;
  onCodigoChange: (id: string, valor: string) => void;
  confirmacion: string;
  onConfirmacionChange: (valor: string) => void;
  fraseEsperada: string;
  simulacion: boolean;
  onSimulacionChange: (valor: boolean) => void;
  ejecutando: boolean;
  resultado: ResultadoExportar | null;
  onEjecutar: () => void;
}

function DialogoExportarClientesBdp({
  abierto,
  onAbiertoChange,
  pendientes,
  cargando,
  codigos,
  onCodigoChange,
  confirmacion,
  onConfirmacionChange,
  fraseEsperada,
  simulacion,
  onSimulacionChange,
  ejecutando,
  resultado,
  onEjecutar,
}: Props) {
  const lista = pendientes ?? [];
  const confirmacionOk = confirmacion.trim() === fraseEsperada && fraseEsperada !== '';

  return (
    <Dialog open={abierto} onOpenChange={onAbiertoChange}>
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            Exportar clientes a BDP
            {simulacion ? (
              <Badge variant="secondary">Modo simulación</Badge>
            ) : (
              <Badge variant="destructive">Modo real: escribe en BDP</Badge>
            )}
          </DialogTitle>
          <DialogDescription>
            Sube los clientes locales sin vincular al BDP, uno por uno con su código explícito.
            {simulacion
              ? ' En simulación no se envía nada: solo se valida y se muestra lo que se enviaría.'
              : ' En modo real cada fila hace un POST a BDP con Overwrite=false.'}
          </DialogDescription>
        </DialogHeader>

        <div className="flex items-center justify-between rounded-md border px-3 py-2">
          <Label htmlFor="exportar-simulacion" className="text-sm">
            Simulación (no escribe en BDP)
          </Label>
          <Switch id="exportar-simulacion" checked={simulacion} onCheckedChange={onSimulacionChange} />
        </div>

        {cargando ? (
          <p className="text-sm text-muted-foreground">Cargando pendientes…</p>
        ) : lista.length === 0 ? (
          <p className="text-sm text-muted-foreground">No hay clientes pendientes de subir a BDP.</p>
        ) : (
          <>
            <p className="text-sm text-muted-foreground">
              {lista.length} {lista.length === 1 ? 'pendiente' : 'pendientes'}: asigna un código BDP a cada uno.
            </p>
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Cliente</TableHead>
                    <TableHead>Contacto</TableHead>
                    <TableHead className="w-36">Código BDP</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {lista.map((c) => (
                    <TableRow key={c.id}>
                      <TableCell>{c.nombre} {c.apellidos}</TableCell>
                      <TableCell className="text-muted-foreground">
                        {[c.telefono, c.email].filter(Boolean).join(' · ') || '—'}
                      </TableCell>
                      <TableCell>
                        <Input
                          type="number"
                          min={1}
                          placeholder="Código…"
                          value={codigos[c.id] ?? ''}
                          onChange={(e) => onCodigoChange(c.id, e.target.value)}
                          aria-label={`Código BDP para ${c.nombre} ${c.apellidos}`}
                        />
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>

            <div className="flex flex-col gap-2">
              <Label htmlFor="confirmar-exportar-bdp">Escribe {fraseEsperada}</Label>
              <Input
                id="confirmar-exportar-bdp"
                value={confirmacion}
                onChange={(e) => onConfirmacionChange(e.target.value)}
              />
            </div>
          </>
        )}

        {resultado && (
          <div className="flex flex-col gap-2">
            <p className="text-sm font-medium">
              {resultado.simulado ? 'Simulación' : 'Resultado'}: {resultado.filas.filter((f) => f.estado !== 'error').length}/{resultado.filas.length} correctos
            </p>
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Cliente</TableHead>
                    <TableHead>Código</TableHead>
                    <TableHead>Estado</TableHead>
                    <TableHead>Detalle</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {resultado.filas.map((f) => (
                    <TableRow key={f.id}>
                      <TableCell>{f.nombre}</TableCell>
                      <TableCell>{f.codigo}</TableCell>
                      <TableCell>
                        {f.estado === 'ok' ? (
                          <Badge variant="outline">Subido</Badge>
                        ) : f.estado === 'simulado' ? (
                          <Badge variant="secondary">Simulado</Badge>
                        ) : (
                          <Badge variant="destructive">Error</Badge>
                        )}
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground" title={`${f.url} · ${f.confirmacion}`}>
                        {f.detalle}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onAbiertoChange(false)}>Cerrar</Button>
          {lista.length > 0 && (
            <Button disabled={ejecutando || !confirmacionOk} onClick={onEjecutar}>
              {ejecutando ? 'Procesando…' : simulacion ? 'Simular envío (sin escribir)' : 'Subir a BDP de verdad'}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default DialogoExportarClientesBdp;
