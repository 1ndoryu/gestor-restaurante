/* [BKP-005] Panel de Backup BDP — snapshots, restauración, auditoría y modo sync.
 * Accesible desde /configuracion como sección expandible o como página independiente. */

import {useState} from 'react';
import {Database, Download, Loader2, RefreshCw, Shield, Trash2, Upload} from 'lucide-react';
import {Button} from '@/components/ui/button';
import {Card, CardContent, CardDescription, CardHeader, CardTitle} from '@/components/ui/card';
import {Badge} from '@/components/ui/badge';
import {Tabs, TabsContent, TabsList, TabsTrigger} from '@/components/ui/tabs';
import {Select, SelectContent, SelectItem, SelectTrigger, SelectValue} from '@/components/ui/select';
import {Table, TableBody, TableCell, TableHead, TableHeader, TableRow} from '@/components/ui/table';
import {Textarea} from '@/components/ui/textarea';
import {toast} from 'sonner';
import {useBdpSnapshots, useBdpAudit, useCreateSnapshotCompleto, useCreateSnapshotParcial, useCreateSnapshotGlory, useDeleteSnapshot, useRestoreSnapshot, useSetSyncMode, type BdpSnapshot, type BdpAuditEntry, type SyncMode} from '@/api/bdp-backup';
import type {EstadoConfiguracion} from '@/hooks/useConfiguracion';

/* ========== Constantes ========== */

const SYNC_MODES: {value: SyncMode; label: string; desc: string}[] = [
    {value: 'read_only', label: 'Solo lectura', desc: 'Glory lee datos de BDP pero no envía nada.'},
    {value: 'unidirectional', label: 'Unidireccional', desc: 'Glory → BDP (ventas, clientes).'},
    {value: 'bidirectional', label: 'Bidireccional', desc: 'BDP ↔ Glory (sincronización completa).'}
];

const SNAPSHOT_TIPOS_BDP = ['articulos', 'clientes', 'departamentos', 'salones', 'empleados'];
const SNAPSHOT_TIPOS_GLORY = ['ventas', 'clientes', 'mapeos'];

/* ========== Helpers ========== */

function formatDate(iso: string): string {
    return new Date(iso).toLocaleString('es-ES', {
        day: '2-digit',
        month: 'short',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
    });
}

function tipoSnapshotBadge(tipo: string) {
    const variants: Record<string, 'default' | 'secondary' | 'outline'> = {
        completo: 'default',
        parcial: 'secondary',
        glory: 'outline',
        pre_write: 'outline'
    };
    return <Badge variant={variants[tipo] ?? 'outline'}>{tipo}</Badge>;
}

function datosResumen(datos: Record<string, unknown>): string {
    const parts: string[] = [];
    for (const [key, val] of Object.entries(datos)) {
        if (Array.isArray(val)) parts.push(`${val.length} ${key}`);
    }
    return parts.length > 0 ? parts.join(', ') : '—';
}

function resultadoBadge(resultado: string) {
    if (resultado === 'ok')
        return (
            <Badge variant="default" className="bg-green-600">
                OK
            </Badge>
        );
    if (resultado === 'error') return <Badge variant="destructive">Error</Badge>;
    if (resultado === 'skipped') return <Badge variant="outline">Omitido</Badge>;
    return <Badge variant="secondary">{resultado}</Badge>;
}

/* ========== Sub-componentes ========== */

interface SyncModeSelectorProps {
    currentMode: string;
}

function SyncModeSelector({currentMode}: SyncModeSelectorProps) {
    const setMode = useSetSyncMode();
    const effective = currentMode || 'read_only';

    function handleChange(value: string) {
        setMode.mutate(value as SyncMode, {
            onSuccess: () => toast.success('Modo BDP actualizado', {description: `Modo: ${value}`}),
            onError: (err: unknown) => {
                const msg = (err as {response?: {data?: {message?: string}}})?.response?.data?.message ?? 'Error al cambiar modo';
                toast.error('Error', {description: msg});
            }
        });
    }

    return (
        <div className="flex items-center gap-3">
            <Select value={effective} onValueChange={handleChange} disabled={setMode.isPending}>
                <SelectTrigger className="w-[200px]">
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    {SYNC_MODES.map(m => (
                        <SelectItem key={m.value} value={m.value}>
                            {m.label}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
            {setMode.isPending && <Loader2 className="h-4 w-4 animate-spin" />}
        </div>
    );
}

function SnapshotActions() {
    const crearCompleto = useCreateSnapshotCompleto();
    const crearParcial = useCreateSnapshotParcial();
    const crearGlory = useCreateSnapshotGlory();
    const [notas, setNotas] = useState('');
    const [tiposSeleccionados, setTiposSeleccionados] = useState<string[]>([]);
    const [tiposGlory, setTiposGlory] = useState<string[]>([]);

    function toggleTipo(lista: string[], setLista: (v: string[]) => void, tipo: string) {
        setLista(lista.includes(tipo) ? lista.filter(t => t !== tipo) : [...lista, tipo]);
    }

    const anyLoading = crearCompleto.isPending || crearParcial.isPending || crearGlory.isPending;

    return (
        <div className="space-y-4">
            <div>
                <label className="text-sm font-medium">Notas (opcional)</label>
                <Textarea placeholder="Ej: antes de migración de artículos..." value={notas} onChange={e => setNotas(e.target.value)} className="mt-1" rows={2} />
            </div>

            <div className="flex flex-wrap gap-2">
                <Button
                    onClick={() =>
                        crearCompleto.mutate(notas || undefined, {
                            onSuccess: () => {
                                toast.success('Snapshot completo creado');
                                setNotas('');
                            },
                            onError: (e: unknown) => toast.error('Error', {description: String(e)})
                        })
                    }
                    disabled={anyLoading}>
                    {crearCompleto.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Database className="h-4 w-4 mr-2" />}
                    Snapshot completo BDP
                </Button>
            </div>

            <div className="border rounded-md p-3 space-y-2">
                <p className="text-sm font-medium">Snapshot parcial BDP</p>
                <div className="flex flex-wrap gap-1.5">
                    {SNAPSHOT_TIPOS_BDP.map(t => (
                        <Badge key={t} variant={tiposSeleccionados.includes(t) ? 'default' : 'outline'} className="cursor-pointer select-none" onClick={() => toggleTipo(tiposSeleccionados, setTiposSeleccionados, t)}>
                            {t}
                        </Badge>
                    ))}
                </div>
                <Button
                    size="sm"
                    variant="secondary"
                    disabled={tiposSeleccionados.length === 0 || anyLoading}
                    onClick={() =>
                        crearParcial.mutate(
                            {tipos: tiposSeleccionados, notas: notas || undefined},
                            {
                                onSuccess: () => {
                                    toast.success('Snapshot parcial creado');
                                    setNotas('');
                                    setTiposSeleccionados([]);
                                },
                                onError: (e: unknown) => toast.error('Error', {description: String(e)})
                            }
                        )
                    }>
                    {crearParcial.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-1" /> : <Download className="h-4 w-4 mr-1" />}
                    Crear parcial
                </Button>
            </div>

            <div className="border rounded-md p-3 space-y-2">
                <p className="text-sm font-medium">Snapshot Glory (local, 0 llamadas BDP)</p>
                <div className="flex flex-wrap gap-1.5">
                    {SNAPSHOT_TIPOS_GLORY.map(t => (
                        <Badge key={t} variant={tiposGlory.includes(t) ? 'default' : 'outline'} className="cursor-pointer select-none" onClick={() => toggleTipo(tiposGlory, setTiposGlory, t)}>
                            {t}
                        </Badge>
                    ))}
                </div>
                <Button
                    size="sm"
                    variant="secondary"
                    disabled={tiposGlory.length === 0 || anyLoading}
                    onClick={() =>
                        crearGlory.mutate(
                            {tipos: tiposGlory, notas: notas || undefined},
                            {
                                onSuccess: () => {
                                    toast.success('Snapshot Glory creado');
                                    setNotas('');
                                    setTiposGlory([]);
                                },
                                onError: (e: unknown) => toast.error('Error', {description: String(e)})
                            }
                        )
                    }>
                    {crearGlory.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-1" /> : <Download className="h-4 w-4 mr-1" />}
                    Crear Glory
                </Button>
            </div>
        </div>
    );
}

function SnapshotTable({snapshots}: {snapshots: BdpSnapshot[]}) {
    const eliminar = useDeleteSnapshot();
    const restaurar = useRestoreSnapshot();
    const [confirmRestore, setConfirmRestore] = useState<string | null>(null);

    if (snapshots.length === 0) {
        return <p className="text-sm text-muted-foreground py-4 text-center">No hay snapshots todavía. Crea uno para empezar.</p>;
    }

    return (
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead>Tipo</TableHead>
                    <TableHead>Fecha</TableHead>
                    <TableHead>Datos</TableHead>
                    <TableHead>Notas</TableHead>
                    <TableHead className="text-right">Acciones</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {snapshots.map(s => (
                    <TableRow key={s.id}>
                        <TableCell>{tipoSnapshotBadge(s.tipo)}</TableCell>
                        <TableCell className="text-sm">{formatDate(s.created_at)}</TableCell>
                        <TableCell className="text-xs text-muted-foreground">{datosResumen(s.datos)}</TableCell>
                        <TableCell className="text-xs max-w-[200px] truncate">{s.notas ?? '—'}</TableCell>
                        <TableCell className="text-right">
                            <div className="flex justify-end gap-1">
                                {confirmRestore === s.id ? (
                                    <>
                                        <Button
                                            size="sm"
                                            variant="destructive"
                                            onClick={() => {
                                                restaurar.mutate(s.id, {
                                                    onSuccess: r => {
                                                        toast.success('Restauración completada', {
                                                            description: `${r.registros_restaurados} registros restaurados. ${r.detalles}`
                                                        });
                                                        setConfirmRestore(null);
                                                    },
                                                    onError: (e: unknown) => toast.error('Error', {description: String(e)})
                                                });
                                            }}
                                            disabled={restaurar.isPending}>
                                            {restaurar.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : 'Confirmar'}
                                        </Button>
                                        <Button size="sm" variant="outline" onClick={() => setConfirmRestore(null)}>
                                            Cancelar
                                        </Button>
                                    </>
                                ) : (
                                    <>
                                        <Button
                                            size="sm"
                                            variant="outline"
                                            onClick={() => {
                                                toast.info('Restaurar sobre datos actuales', {
                                                    description: 'Esto sobrescribirá los datos locales de Glory con los del snapshot.'
                                                });
                                                setConfirmRestore(s.id);
                                            }}
                                            title="Restaurar Glory desde este snapshot">
                                            <Upload className="h-3 w-3" />
                                        </Button>
                                        <Button
                                            size="sm"
                                            variant="ghost"
                                            onClick={() => {
                                                if (window.confirm('¿Eliminar este snapshot permanentemente?')) {
                                                    eliminar.mutate(s.id, {
                                                        onSuccess: () => toast.success('Snapshot eliminado'),
                                                        onError: (e: unknown) => toast.error('Error', {description: String(e)})
                                                    });
                                                }
                                            }}
                                            title="Eliminar snapshot">
                                            <Trash2 className="h-3 w-3 text-destructive" />
                                        </Button>
                                    </>
                                )}
                            </div>
                        </TableCell>
                    </TableRow>
                ))}
            </TableBody>
        </Table>
    );
}

function AuditTable({entries}: {entries: BdpAuditEntry[]}) {
    if (entries.length === 0) {
        return <p className="text-sm text-muted-foreground py-4 text-center">Sin registros de auditoría todavía.</p>;
    }

    return (
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead>Fecha</TableHead>
                    <TableHead>Operación</TableHead>
                    <TableHead>Dirección</TableHead>
                    <TableHead>Resultado</TableHead>
                    <TableHead>Snapshot pre</TableHead>
                    <TableHead>Error</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {entries.map(e => (
                    <TableRow key={e.id}>
                        <TableCell className="text-sm">{formatDate(e.created_at)}</TableCell>
                        <TableCell>
                            <Badge variant="outline">{e.operacion}</Badge>
                        </TableCell>
                        <TableCell>
                            <Badge variant="outline">{e.direccion}</Badge>
                        </TableCell>
                        <TableCell>{resultadoBadge(e.resultado)}</TableCell>
                        <TableCell className="text-xs font-mono">{e.snapshot_pre_id ? e.snapshot_pre_id.slice(0, 8) + '…' : '—'}</TableCell>
                        <TableCell className="text-xs text-destructive max-w-[200px] truncate">{e.error_mensaje ?? '—'}</TableCell>
                    </TableRow>
                ))}
            </TableBody>
        </Table>
    );
}

/* ========== Panel principal ========== */

interface PanelBdpBackupProps {
    config: EstadoConfiguracion;
}

export default function PanelBdpBackup({config}: PanelBdpBackupProps) {
    const {data: snapshots, isLoading: loadingSnapshots} = useBdpSnapshots();
    const {data: audit, isLoading: loadingAudit} = useBdpAudit();

    return (
        <Card>
            <CardHeader>
                <div className="flex items-center justify-between">
                    <div>
                        <CardTitle className="flex items-center gap-2">
                            <Shield className="h-5 w-5" />
                            Backup & Seguridad BDP
                        </CardTitle>
                        <CardDescription>Snapshots, restauración y auditoría de la sincronización con BDP.</CardDescription>
                    </div>
                    <div className="flex items-center gap-2">
                        <span className="text-sm text-muted-foreground">Modo:</span>
                        <SyncModeSelector currentMode={config.bdp_sync_mode} />
                    </div>
                </div>
            </CardHeader>
            <CardContent>
                <Tabs defaultValue="snapshots">
                    <TabsList>
                        <TabsTrigger value="snapshots">
                            <Database className="h-4 w-4 mr-1" />
                            Snapshots
                        </TabsTrigger>
                        <TabsTrigger value="crear">
                            <Download className="h-4 w-4 mr-1" />
                            Crear
                        </TabsTrigger>
                        <TabsTrigger value="auditoria">
                            <RefreshCw className="h-4 w-4 mr-1" />
                            Auditoría
                        </TabsTrigger>
                    </TabsList>

                    <TabsContent value="snapshots" className="mt-4">
                        {loadingSnapshots ? (
                            <div className="flex items-center justify-center py-8">
                                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                            </div>
                        ) : (
                            <SnapshotTable snapshots={snapshots ?? []} />
                        )}
                    </TabsContent>

                    <TabsContent value="crear" className="mt-4">
                        <SnapshotActions />
                    </TabsContent>

                    <TabsContent value="auditoria" className="mt-4">
                        {loadingAudit ? (
                            <div className="flex items-center justify-center py-8">
                                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                            </div>
                        ) : (
                            <AuditTable entries={audit ?? []} />
                        )}
                    </TabsContent>
                </Tabs>
            </CardContent>
        </Card>
    );
}
