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

function formatDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleString('es-ES', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function resultadoBadge(resultado: string) {
  if (resultado === 'exito') return <Badge className="bg-green-600 hover:bg-green-600">Completada</Badge>;
  if (resultado === 'error') return <Badge variant="destructive">Falló</Badge>;
  if (resultado === 'ambiguo') return <Badge variant="destructive">Requiere revisión</Badge>;
  if (resultado === 'pendiente') return <Badge variant="secondary">En proceso</Badge>;
  if (resultado === 'parcial') return <Badge variant="outline">Parcial</Badge>;
  return <Badge variant="outline">{resultado}</Badge>;
}

function operacionLabel(operacion: string): string {
  const labels: Record<string, string> = {
    create_order: 'Crear comanda',
    create_customer: 'Crear cliente',
    add_payment: 'Registrar pago',
    invoice: 'Facturar',
    config_bootstrap: 'Preparar configuración',
    pago_parcial_local: 'Pago parcial local',
    factura_local: 'Factura local',
    anular_venta: 'Anular venta',
    menu_local_crear: 'Crear menú local',
    menu_local_actualizar: 'Actualizar menú local',
    menu_local_eliminar: 'Eliminar menú local',
    stock_ajuste: 'Ajuste de stock',
    inventory: 'Conteo de inventario',
    regularize_stock: 'Regularizar stock',
  };
  return labels[operacion] ?? operacion;
}

function direccionLabel(direccion: string): string {
  if (direccion === 'glory_to_bdp') return 'Aplicación Web → BDP';
  if (direccion === 'bdp_to_glory') return 'BDP → Aplicación Web';
  if (direccion === 'internal') return 'Configuración de la Aplicación Web';
  return direccion;
}

/* [128A-1/F6] Origen de la operación: 'local' (anulaciones, ajustes de stock,
 * pagos y facturas locales) o 'bdp' (default — implican al BDP). */
function origenBadge(origen: string) {
  if (origen === 'local') {
    return <Badge className="bg-sky-600 hover:bg-sky-600">Local</Badge>;
  }
  return <Badge variant="secondary">BDP</Badge>;
}

function AuditDetail({ entry }: { entry: BdpAuditEntry }) {
  return (
    <div className="space-y-3 text-sm">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <p className="text-xs text-muted-foreground">Operación</p>
          <p className="font-medium">{operacionLabel(entry.operacion)}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Resultado</p>
          {resultadoBadge(entry.resultado)}
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Dirección</p>
          <p>{direccionLabel(entry.direccion)}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Origen</p>
          {origenBadge(entry.origen_operacion)}
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Fecha</p>
          <p>{formatDate(entry.created_at)}</p>
        </div>
      </div>
      {entry.target_entity_type && (
        <div>
          <p className="text-xs text-muted-foreground">Entidad</p>
          <p>
            {entry.target_entity_type}: {entry.target_entity_id}
          </p>
        </div>
      )}
      {entry.authorization_reason && (
        <div>
          <p className="text-xs text-muted-foreground">Motivo de autorización</p>
          <p>{entry.authorization_reason}</p>
        </div>
      )}
      {entry.error_mensaje && (
        <div>
          <p className="text-xs text-muted-foreground">Error</p>
          <p className="text-destructive">{entry.error_mensaje}</p>
        </div>
      )}
      {entry.datos_enviados && (
        <div>
          <p className="text-xs text-muted-foreground">Datos enviados</p>
          <pre className="mt-1 rounded bg-muted p-2 text-xs overflow-auto max-h-48">
            {JSON.stringify(entry.datos_enviados, null, 2)}
          </pre>
        </div>
      )}
      {entry.datos_respuesta && (
        <div>
          <p className="text-xs text-muted-foreground">Respuesta</p>
          <pre className="mt-1 rounded bg-muted p-2 text-xs overflow-auto max-h-48">
            {JSON.stringify(entry.datos_respuesta, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}

function SnapshotDetail({ snapshot }: { snapshot: BdpSnapshot }) {
  return (
    <div className="space-y-3 text-sm">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <p className="text-xs text-muted-foreground">Tipo</p>
          <p className="font-medium">{snapshot.tipo}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Fecha</p>
          <p>{formatDate(snapshot.created_at)}</p>
        </div>
      </div>
      {snapshot.notas && (
        <div>
          <p className="text-xs text-muted-foreground">Notas</p>
          <p>{snapshot.notas}</p>
        </div>
      )}
      <div>
        <p className="text-xs text-muted-foreground">Datos</p>
        <pre className="mt-1 rounded bg-muted p-2 text-xs overflow-auto max-h-48">
          {JSON.stringify(snapshot.datos, null, 2)}
        </pre>
      </div>
    </div>
  );
}

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
