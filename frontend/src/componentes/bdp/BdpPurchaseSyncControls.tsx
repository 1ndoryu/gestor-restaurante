/* [287A-5] Configura y persiste la plantilla ExportPurchaseNotes en contexto.
 * La operación remota es de lectura; los documentos se importan solo a Glory. */

import { useEffect, useState } from 'react';
import axios from 'axios';
import { Loader2, RefreshCw } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { BdpRequiredSetting } from '@/components/bdp-required-setting';
import { useSyncBdpPurchaseNotes } from '@/api/bdp';
import { useBdpReadProfiles } from '@/hooks/useBdpReadProfiles';
import { BdpDemoToggle } from './BdpDemoToggle';

interface BdpPurchaseSyncControlsProps {
  count: number;
  demoMode: boolean;
  fechaDesde: string;
  fechaHasta: string;
  onToggleDemo: (enabled: boolean) => void;
}

function getErrorMessage(error: unknown): string {
  if (!axios.isAxiosError(error)) return 'No se pudo consultar Compras en BDP';
  return String(error.response?.data?.message ?? 'No se pudo consultar Compras en BDP');
}

export function BdpPurchaseSyncControls({
  count,
  demoMode,
  fechaDesde,
  fechaHasta,
  onToggleDemo,
}: BdpPurchaseSyncControlsProps) {
  const queryClient = useQueryClient();
  const { purchaseProfileId, saveProfile, isSaving } = useBdpReadProfiles();
  const [profileCode, setProfileCode] = useState(purchaseProfileId ? String(purchaseProfileId) : '');
  const [profileProblem, setProfileProblem] = useState('');

  useEffect(() => {
    if (purchaseProfileId) setProfileCode(String(purchaseProfileId));
  }, [purchaseProfileId]);

  const syncMutation = useSyncBdpPurchaseNotes(queryClient);

  async function syncWithSavedProfile() {
    const code = Number(profileCode);
    if (!Number.isInteger(code) || code <= 0) {
      setProfileProblem('Indica el código numérico de la plantilla de Compras configurada en BDP.');
      return;
    }
    if (!fechaDesde || !fechaHasta) {
      toast.error('Indica las fechas desde y hasta para consultar los albaranes');
      return;
    }
    try {
      await saveProfile({ field: 'bdp_purchase_notes_profile_id', value: code });
      syncMutation.mutate(
        { export_profile_code: code, fecha_desde: fechaDesde, fecha_hasta: fechaHasta },
        {
          onSuccess: (result) => {
            setProfileProblem('');
            toast.success(`Sync completado: ${result.procesados} albaranes procesados de ${result.total_bdp}`);
          },
          onError: (error) => {
            const message = getErrorMessage(error);
            if (/plantilla|perfil|exportpurchasenotes/i.test(message)) setProfileProblem(message);
            toast.error('BDP no pudo consultar los albaranes', { description: message });
          },
        },
      );
    } catch {
      toast.error('No se pudo guardar la plantilla de Compras');
    }
  }

  const requiresConfiguration = !purchaseProfileId || !!profileProblem;

  return (
    <div className="flex w-full flex-col gap-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-muted-foreground">{count} albaranes</p>
        <div className="flex flex-wrap items-center gap-2">
          <BdpDemoToggle demoMode={demoMode} onToggle={onToggleDemo} />
          <TooltipButton
            variant="outline"
            onClick={syncWithSavedProfile}
            disabled={syncMutation.isPending || isSaving || demoMode}
            tooltip="Consulta albaranes en BDP y los importa en Glory. No modifica BDP."
          >
            {syncMutation.isPending || isSaving ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
            Sync albaranes
          </TooltipButton>
        </div>
      </div>
      {requiresConfiguration && (
        <BdpRequiredSetting
          title="Configura la plantilla de Compras"
          description={profileProblem || 'BDP necesita el código de la plantilla ExportPurchaseNotes creada para albaranes. Se guarda en Glory y esta consulta no modifica BDP.'}
          label="Código de plantilla ExportPurchaseNotes"
          value={profileCode}
          saving={syncMutation.isPending || isSaving}
          onChange={setProfileCode}
          onSave={syncWithSavedProfile}
        />
      )}
    </div>
  );
}
