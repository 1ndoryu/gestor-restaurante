/* [BDP-HIST-01] Página de historial BDP.
 * Muestra el audit log y los snapshots en una página dedicada.
 * Las acciones son seguras: solo ver detalles o crear snapshots locales. */

import { useState } from 'react';
import { Database, Search, ArrowLeft, Eye, Download } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
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
  if (resultado === 'exito')
    return (
      <Badge variant="default" className="bg-green-600">
        Completada
      </Badge>
    );
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
  };
  return labels[operacion] ?? operacion;
}

function direccionLabel(direccion: string): string {
  if (direccion === 'glory_to_bdp') return 'Glory → BDP';
  if (direccion === 'bdp_to_glory') return 'BDP → Glory';
  if (direccion === 'internal') return 'Configuración de Glory';
  return direccion;
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
  const navigate = useNavigate();
  const [filtro, setFiltro] = useState('');
  const [entrySeleccionado, setEntrySeleccionado] = useState<BdpAuditEntry | null>(null);
  const [snapshotSeleccionado, setSnapshotSeleccionado] = useState<BdpSnapshot | null>(null);
  const [dialogAbierto, setDialogAbierto] = useState(false);
  const { data: auditData, isLoading: loadingAudit, error: auditError } = useBdpAudit(100);
  const { data: snapshotsData, isLoading: loadingSnapshots, error: snapshotsError } = useBdpSnapshots(50);

  const auditEntries = auditData ?? [];
  const snapshots = snapshotsData ?? [];

  function handleDialogOpenChange(open: boolean) {
    setDialogAbierto(open);
    if (!open) {
      setEntrySeleccionado(null);
      setSnapshotSeleccionado(null);
    }
  }

  const auditFiltrado = auditEntries.filter(
    (e) =>
      operacionLabel(e.operacion).toLowerCase().includes(filtro.toLowerCase()) ||
      e.resultado.toLowerCase().includes(filtro.toLowerCase()) ||
      e.error_mensaje?.toLowerCase().includes(filtro.toLowerCase()),
  );

  return (
    <div className="space-y-4 p-4 md:p-6">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="icon" onClick={() => navigate('/configuracion')}>
          <ArrowLeft className="size-4" />
        </Button>
        <h1 className="text-xl font-semibold">Historial BDP</h1>
      </div>

      <Tabs defaultValue="auditoria">
        <TabsList>
          <TabsTrigger value="auditoria">Auditoría</TabsTrigger>
          <TabsTrigger value="snapshots">Snapshots</TabsTrigger>
        </TabsList>
        <TabsContent value="auditoria" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Database className="size-4" />
                Log de operaciones BDP
              </CardTitle>
              <CardDescription>Registro de cada escritura, sincronización y estado entre Glory y BDP.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="relative w-full sm:w-96">
                <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
                <Input
                  placeholder="Filtrar operación, resultado, error..."
                  value={filtro}
                  onChange={(e) => setFiltro(e.target.value)}
                  className="pl-9"
                />
              </div>
              {auditError ? (
                <p className="text-sm text-destructive">
                  Error al cargar la auditoría. Revisa que la sesión esté activa y vuelve a intentarlo.
                </p>
              ) : loadingAudit ? (
                <p className="text-sm text-muted-foreground">Cargando auditoría...</p>
              ) : auditFiltrado.length === 0 ? (
                <p className="text-sm text-muted-foreground">Sin registros de auditoría.</p>
              ) : (
                <div className="rounded-md border">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Fecha</TableHead>
                        <TableHead>Operación</TableHead>
                        <TableHead>Dirección</TableHead>
                        <TableHead>Resultado</TableHead>
                        <TableHead className="w-10"></TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {auditFiltrado.map((entry) => (
                        <TableRow key={entry.id}>
                          <TableCell className="text-xs">{formatDate(entry.created_at)}</TableCell>
                          <TableCell>
                            <Badge variant="outline">{operacionLabel(entry.operacion)}</Badge>
                          </TableCell>
                          <TableCell className="text-xs">{direccionLabel(entry.direccion)}</TableCell>
                          <TableCell>{resultadoBadge(entry.resultado)}</TableCell>
                          <TableCell>
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={() => {
                                setEntrySeleccionado(entry);
                                setSnapshotSeleccionado(null);
                                setDialogAbierto(true);
                              }}
                            >
                              <Eye className="size-4" />
                            </Button>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
        <TabsContent value="snapshots" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Download className="size-4" />
                Copias de seguridad BDP
              </CardTitle>
              <CardDescription>Snapshots locales de configuración y datos de BDP.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {snapshotsError ? (
                <p className="text-sm text-destructive">
                  Error al cargar los snapshots. Revisa que la sesión esté activa y vuelve a intentarlo.
                </p>
              ) : loadingSnapshots ? (
                <p className="text-sm text-muted-foreground">Cargando snapshots...</p>
              ) : snapshots.length === 0 ? (
                <p className="text-sm text-muted-foreground">No hay snapshots todavía.</p>
              ) : (
                <div className="rounded-md border">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Tipo</TableHead>
                        <TableHead>Fecha</TableHead>
                        <TableHead>Notas</TableHead>
                        <TableHead className="w-10"></TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {snapshots.map((snapshot) => (
                        <TableRow key={snapshot.id}>
                          <TableCell className="text-xs">{snapshot.tipo}</TableCell>
                          <TableCell className="text-xs">{formatDate(snapshot.created_at)}</TableCell>
                          <TableCell className="text-xs text-muted-foreground max-w-xs truncate">
                            {snapshot.notas ?? '—'}
                          </TableCell>
                          <TableCell>
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={() => {
                                setSnapshotSeleccionado(snapshot);
                                setEntrySeleccionado(null);
                                setDialogAbierto(true);
                              }}
                            >
                              <Eye className="size-4" />
                            </Button>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

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
