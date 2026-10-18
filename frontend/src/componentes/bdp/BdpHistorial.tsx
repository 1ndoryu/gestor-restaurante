/* [BDP-HIST-02] Página de historial BDP.
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * Modo demo incluido para visualizar datos de prueba. */

import { useMemo, useState } from 'react';
import { Search, Eye, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { useBdpAudit, useBdpSnapshots, type BdpAuditEntry, type BdpSnapshot } from '@/api/bdp-backup';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import { mockAuditEntries, mockSnapshots } from './bdp-mocks';
import { AuditDetail } from './bdp-historial/AuditDetail';
import { SnapshotDetail } from './bdp-historial/SnapshotDetail';
import { direccionLabel, formatDate, operacionLabel, origenBadge, resultadoBadge } from './bdp-historial/formato';

function BdpHistorial() {
  /* [159A-3] El toggle demo vive en Configuración → BDP; aquí solo se lee. */
  const { demoMode } = useBdpDemoMode();
  const [filtro, setFiltro] = useState('');
  const [filtroOrigen, setFiltroOrigen] = useState<'todos' | 'local' | 'bdp'>('todos');
  const [entrySeleccionado, setEntrySeleccionado] = useState<BdpAuditEntry | null>(null);
  const [snapshotSeleccionado, setSnapshotSeleccionado] = useState<BdpSnapshot | null>(null);
  const [dialogAbierto, setDialogAbierto] = useState(false);
  const { data: auditData, isLoading: loadingAudit, error: auditError, refetch: refetchAudit } = useBdpAudit(100, !demoMode);
  const { data: snapshotsData, isLoading: loadingSnapshots, error: snapshotsError, refetch: refetchSnapshots } = useBdpSnapshots(50, !demoMode);
  /* Mensaje de error honesto (mismo patrón que Compras/Stock): solo se habla
   * de sesión ante un 401 real; si el backend explica el motivo, se muestra
   * tal cual; en otro caso genérico + Reintentar. */
  const historialAxiosResponse = (
    (auditError ?? snapshotsError) as { response?: { status?: number; data?: { message?: string } } } | null | undefined
  )?.response;
  const historialErrorStatus = historialAxiosResponse?.status;
  const historialErrorMessage = historialAxiosResponse?.data?.message;

  const auditEntries = useMemo(() => {
    if (demoMode) return mockAuditEntries;
    return auditData ?? [];
  }, [demoMode, auditData]);

  const snapshots = useMemo(() => {
    if (demoMode) return mockSnapshots;
    return snapshotsData ?? [];
  }, [demoMode, snapshotsData]);

  function handleDialogOpenChange(open: boolean) {
    setDialogAbierto(open);
    if (!open) {
      setEntrySeleccionado(null);
      setSnapshotSeleccionado(null);
    }
  }

  const auditFiltrado = useMemo(() => {
    const q = filtro.trim().toLowerCase();
    return auditEntries.filter(
      (e) =>
        (filtroOrigen === 'todos' || e.origen_operacion === filtroOrigen) &&
        (operacionLabel(e.operacion).toLowerCase().includes(q) ||
          e.resultado.toLowerCase().includes(q) ||
          e.error_mensaje?.toLowerCase().includes(q)),
    );
  }, [auditEntries, filtro, filtroOrigen]);

  const isLoading = !demoMode && (loadingAudit || loadingSnapshots);
  const hasError = !demoMode && (auditError || snapshotsError);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {auditFiltrado.length} registros de auditoría · {snapshots.length} snapshots
        </p>
      </div>

      <div className="flex flex-wrap gap-3 items-center">
        <div className="relative w-full sm:w-96">
          <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
          <Input
            type="search"
            placeholder="Filtrar operación, resultado, error u origen..."
            value={filtro}
            onChange={(e) => setFiltro(e.target.value)}
            className="pl-9 max-w-xs"
          />
        </div>
        <div className="flex gap-1">
          {(['todos', 'local', 'bdp'] as const).map((origen) => (
            <Button
              key={origen}
              type="button"
              variant={filtroOrigen === origen ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFiltroOrigen(origen)}
            >
              {origen === 'todos' ? 'Todos' : origen === 'local' ? 'Local' : 'BDP'}
            </Button>
          ))}
        </div>
      </div>

      {hasError && (
        <div className="flex flex-col items-start gap-3 rounded-md border border-destructive/50 p-4">
          <p className="text-sm text-destructive">
            {historialErrorStatus === 401
              ? 'Tu sesión ha caducado. Vuelve a iniciar sesión y reintenta.'
              : historialErrorMessage
                ? `Error al cargar el historial: ${historialErrorMessage}`
                : 'Error al cargar el historial. Vuelve a intentarlo; si persiste, revisa la conexión con el servidor.'}
          </p>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              refetchAudit();
              refetchSnapshots();
            }}
          >
            <RefreshCw className="size-3.5 mr-1.5" />
            Reintentar
          </Button>
        </div>
      )}

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : (
        <Tabs defaultValue="auditoria">
          <TabsList>
            <TabsTrigger value="auditoria">Auditoría</TabsTrigger>
            <TabsTrigger value="snapshots">Snapshots</TabsTrigger>
          </TabsList>
          <TabsContent value="auditoria" className="space-y-4">
            <div className="rounded-md border overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Fecha</TableHead>
                    <TableHead>Operación</TableHead>
                    <TableHead>Dirección</TableHead>
                    <TableHead>Origen</TableHead>
                    <TableHead>Resultado</TableHead>
                    <TableHead className="w-10 text-center">Acciones</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {auditFiltrado.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={6} className="text-center text-[13px] text-muted-foreground">
                        Sin registros de auditoría.
                      </TableCell>
                    </TableRow>
                  ) : (
                    auditFiltrado.map((entry) => (
                      <TableRow key={entry.id}>
                        <TableCell className="text-xs">{formatDate(entry.created_at)}</TableCell>
                        <TableCell>
                          <Badge variant="outline">{operacionLabel(entry.operacion)}</Badge>
                        </TableCell>
                        <TableCell className="text-xs">{direccionLabel(entry.direccion)}</TableCell>
                        <TableCell>{origenBadge(entry.origen_operacion)}</TableCell>
                        <TableCell>{resultadoBadge(entry.resultado)}</TableCell>
                        <TableCell>
                          <div className="flex justify-center">
                          <Button
                            variant="outline"
                            size="icon"
                            className="bg-muted/40 hover:bg-muted"
                            onClick={() => {
                              setEntrySeleccionado(entry);
                              setSnapshotSeleccionado(null);
                              setDialogAbierto(true);
                            }}
                          >
                            <Eye className="size-4" />
                          </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </div>
          </TabsContent>
          <TabsContent value="snapshots" className="space-y-4">
            <p className="text-xs text-muted-foreground">
              Los snapshots son capturas de respaldo que la Aplicación Web guarda antes de cada operación
              de escritura BDP (preparar comanda, pago, factura o cliente). Sirven para poder revisar o
              restaurar el estado previo si algo falla.
            </p>
            <div className="rounded-md border overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Tipo</TableHead>
                    <TableHead>Fecha</TableHead>
                    <TableHead>Notas</TableHead>
                    <TableHead className="w-10 text-center">Acciones</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {snapshots.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={4} className="text-center text-[13px] text-muted-foreground">
                        No hay snapshots todavía. Se crean automáticamente antes de cada operación de escritura BDP.
                      </TableCell>
                    </TableRow>
                  ) : (
                    snapshots.map((snapshot) => (
                      <TableRow key={snapshot.id}>
                        <TableCell className="text-xs">{snapshot.tipo}</TableCell>
                        <TableCell className="text-xs">{formatDate(snapshot.created_at)}</TableCell>
                        <TableCell className="text-xs text-muted-foreground max-w-xs truncate">
                          {snapshot.notas ?? '—'}
                        </TableCell>
                        <TableCell>
                          <div className="flex justify-center">
                          <Button
                            variant="outline"
                            size="icon"
                            className="bg-muted/40 hover:bg-muted"
                            onClick={() => {
                              setSnapshotSeleccionado(snapshot);
                              setEntrySeleccionado(null);
                              setDialogAbierto(true);
                            }}
                          >
                            <Eye className="size-4" />
                          </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </div>
          </TabsContent>
        </Tabs>
      )}

      <Dialog open={dialogAbierto} onOpenChange={handleDialogOpenChange}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>
              {entrySeleccionado ? 'Detalle de operación BDP' : 'Detalle del snapshot'}
            </DialogTitle>
            <DialogDescription>
              {entrySeleccionado
                ? 'Información completa del registro de auditoría.'
                : 'Información almacenada en el snapshot.'}
            </DialogDescription>
          </DialogHeader>
          {entrySeleccionado && <AuditDetail entry={entrySeleccionado} />}
          {snapshotSeleccionado && <SnapshotDetail snapshot={snapshotSeleccionado} />}
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default BdpHistorial;
