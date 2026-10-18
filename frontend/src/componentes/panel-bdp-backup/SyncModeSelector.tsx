import {Loader2} from 'lucide-react';
import {Select, SelectContent, SelectItem, SelectTrigger, SelectValue} from '@/components/ui/select';
import type {SyncMode} from '@/api/bdp-backup';
import {useFlujoModoBdp} from '../flujoModoBdp';
import {SYNC_MODES} from './constantes';

interface SyncModeSelectorProps {
    currentMode: string;
    bdpBaseUrl: string;
    /* [039A-1/H-P1-03] En modo independiente el permiso de operación BDP
     * no aplica: el selector se deshabilita con motivo visible. */
    deshabilitado?: boolean;
}

export function SyncModeSelector({currentMode, bdpBaseUrl, deshabilitado = false}: SyncModeSelectorProps) {
    const { cambiarModo, isPending } = useFlujoModoBdp();
    const effective = currentMode || 'read_only';
    const selectedMode = SYNC_MODES.find(mode => mode.value === effective) ?? SYNC_MODES[0];

    /* [189A-1] Flujo compartido con las tarjetas de ConfigBdp
     * (`useFlujoModoBdp`): confirmar → destino → alcance/objetivo → mutación,
     * con diálogos propios (los nativos no funcionan en el empaquetado). */
    async function handleChange(value: string) {
        await cambiarModo(value as SyncMode, bdpBaseUrl);
    }

    return (
        <div className="flex flex-col gap-2">
            <div className="flex items-center gap-3">
            <Select value={effective} onValueChange={handleChange} disabled={deshabilitado || isPending}>
                <SelectTrigger className="w-full sm:w-[320px]">
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
            {isPending && <Loader2 className="h-4 w-4 animate-spin" />}
            </div>
            <p className="max-w-xl text-xs text-muted-foreground">{selectedMode.desc}</p>
        </div>
    );
}
