import {useState} from 'react';
import {Loader2, Trash2, Upload} from 'lucide-react';
import {Button} from '@/components/ui/button';
import {TooltipButton} from '@/components/ui/tooltip-button';
import {Table, TableBody, TableCell, TableHead, TableHeader, TableRow} from '@/components/ui/table';
import {toast} from 'sonner';
import {confirmarConDialogo} from '../dialogoConfirmacion';
import {useDeleteSnapshot, useRestoreSnapshot} from '@/api/bdp-backup';
import type {BdpSnapshot} from '@/api/bdp-backup';
import {datosResumen, formatDate, tipoSnapshotBadge} from './formato';

export function SnapshotTable({snapshots}: {snapshots: BdpSnapshot[]}) {
    const eliminar = useDeleteSnapshot();
    const restaurar = useRestoreSnapshot();
    const [confirmRestore, setConfirmRestore] = useState<string | null>(null);
    const [restoreInput, setRestoreInput] = useState('');

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
                    <TableHead className="text-center">Acciones</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {snapshots.map(s => (
                    <TableRow key={s.id}>
                        <TableCell>{tipoSnapshotBadge(s.tipo)}</TableCell>
                        <TableCell className="text-[13px]">{formatDate(s.created_at)}</TableCell>
                        <TableCell className="text-xs text-muted-foreground">{datosResumen(s.datos)}</TableCell>
                        <TableCell className="text-xs max-w-[200px] truncate">{s.notas ?? '—'}</TableCell>
                        <TableCell className="text-right">
                            <div className="flex justify-center gap-1">
                                {confirmRestore === s.id ? (
                                    <div className="flex flex-col gap-1 items-end">
                                        <p className="text-xs text-destructive">Escribe exactamente: <code className="break-all">RESTAURAR {s.id}</code></p>
                                        <input
                                            className="text-xs border rounded px-1 py-0.5 w-72 font-mono"
                                            placeholder={`RESTAURAR ${s.id}`}
                                            value={restoreInput}
                                            onChange={e => setRestoreInput(e.target.value)}
                                        />
                                        <div className="flex gap-1">
                                            <Button
                                                size="sm"
                                                variant="destructive"
                                                onClick={() => {
                                                    restaurar.mutate({ id: s.id, confirmacion: restoreInput }, {
                                                        onSuccess: r => {
                                                            toast.success('Restauración completada', {
                                                                description: `${r.registros_restaurados} registros restaurados. ${r.detalles}`
                                                            });
                                                            setConfirmRestore(null);
                                                            setRestoreInput('');
                                                        },
                                                        onError: (e: unknown) => toast.error('Error', {description: String(e)})
                                                    });
                                                }}
                                                disabled={restaurar.isPending || restoreInput.trim() !== `RESTAURAR ${s.id}`}>
                                                {restaurar.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : 'Confirmar'}
                                            </Button>
                                            <Button size="sm" variant="outline" onClick={() => { setConfirmRestore(null); setRestoreInput(''); }}>
                                                Cancelar
                                            </Button>
                                        </div>
                                    </div>
                                ) : (
                                    <>
                                        <TooltipButton
                                            size="sm"
                                            variant="outline"
                                            className="bg-muted/40 hover:bg-muted"
                                            onClick={() => {
                                                toast.info('Restaurar sobre datos actuales', {
                                                    description: 'Esto sobrescribirá los datos locales de la Aplicación Web con los del snapshot.'
                                                });
                                                setConfirmRestore(s.id);
                                            }}
                                            tooltip="Restaurar la Aplicación Web desde este snapshot">
                                            <Upload className="h-3 w-3" />
                                        </TooltipButton>
                                        <TooltipButton
                                            size="sm"
                                            variant="outline"
                                            className="bg-muted/40 hover:bg-muted"
                                            onClick={async () => {
                                                /* [267A-8] Confirmación con diálogo propio. */
                                                const confirmado = await confirmarConDialogo({
                                                    titulo: '¿Eliminar este snapshot permanentemente?',
                                                    textoConfirmar: 'Eliminar',
                                                });
                                                if (confirmado) {
                                                    eliminar.mutate(s.id, {
                                                        onSuccess: () => toast.success('Snapshot eliminado'),
                                                        onError: (e: unknown) => toast.error('Error', {description: String(e)})
                                                    });
                                                }
                                            }}
                                            tooltip="Eliminar snapshot">
                                            <Trash2 className="h-3 w-3 text-destructive" />
                                        </TooltipButton>
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
