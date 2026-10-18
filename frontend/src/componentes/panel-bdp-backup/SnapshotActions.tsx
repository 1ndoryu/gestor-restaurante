import {useState} from 'react';
import {Database, Download, Loader2} from 'lucide-react';
import {Button} from '@/components/ui/button';
import {Badge} from '@/components/ui/badge';
import {Textarea} from '@/components/ui/textarea';
import {toast} from 'sonner';
import {useCreateSnapshotCompleto, useCreateSnapshotParcial, useCreateSnapshotGlory} from '@/api/bdp-backup';
import {SNAPSHOT_TIPOS_BDP, SNAPSHOT_TIPOS_GLORY} from './constantes';

export function SnapshotActions({bdpActivo}: {bdpActivo: boolean}) {
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
            {/* [039A-1/H-P1-03] Los snapshots "completo"/"parcial" descargan de
             * BDP (red real): en modo independiente se deshabilitan con motivo,
             * igual que el resto de controles BDP (P11.2); el snapshot local
             * (0 llamadas BDP) y los listados siguen disponibles. */}
            {!bdpActivo && (
                <p className="text-xs text-muted-foreground rounded-md border bg-muted/40 p-2">
                    Modo independiente: los snapshots de BDP requieren el modo BDP conectado
                    (lectura real). El snapshot de la Aplicación Web es local y sigue disponible.
                </p>
            )}
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
                    disabled={!bdpActivo || anyLoading}>
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
                    variant="secondary"
                    disabled={!bdpActivo || tiposSeleccionados.length === 0 || anyLoading}
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
                <p className="text-sm font-medium">Snapshot de la Aplicación Web (local, 0 llamadas BDP)</p>
                <div className="flex flex-wrap gap-1.5">
                    {SNAPSHOT_TIPOS_GLORY.map(t => (
                        <Badge key={t} variant={tiposGlory.includes(t) ? 'default' : 'outline'} className="cursor-pointer select-none" onClick={() => toggleTipo(tiposGlory, setTiposGlory, t)}>
                            {t}
                        </Badge>
                    ))}
                </div>
                <Button
                    variant="secondary"
                    disabled={tiposGlory.length === 0 || anyLoading}
                    onClick={() =>
                        crearGlory.mutate(
                            {tipos: tiposGlory, notas: notas || undefined},
                            {
                                onSuccess: () => {
                                    toast.success('Snapshot de la Aplicación Web creado');
                                    setNotas('');
                                    setTiposGlory([]);
                                },
                                onError: (e: unknown) => toast.error('Error', {description: String(e)})
                            }
                        )
                    }>
                    {crearGlory.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-1" /> : <Download className="h-4 w-4 mr-1" />}
                    Crear snapshot local
                </Button>
            </div>
        </div>
    );
}
