/* [287A-5] Acciones de Stock y recuperación guiada cuando ExportArticles
 * responde sin artículos. Cambiar TypePrice sigue siendo una lectura BDP. */

import { useEffect, useState } from 'react';
import { Download, Loader2, RefreshCw } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { BdpRequiredSetting } from '@/components/bdp-required-setting';
import { useSyncCatalog } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpReadProfiles } from '@/hooks/useBdpReadProfiles';
import { BdpDemoToggle } from './BdpDemoToggle';

interface BdpStockActionsProps {
  summary: string;
  demoMode: boolean;
  /** [208A-2/C2] Modo efectivo BDP: si es false (standalone/auto sin BDP), las
   * acciones que consultan BDP se deshabilitan (H7). */
  bdpMode: boolean;
  exportDisabled: boolean;
  onToggleDemo: (enabled: boolean) => void;
  onExport: () => void;
}

export function BdpStockActions({
  summary,
  demoMode,
  bdpMode,
  exportDisabled,
  onToggleDemo,
  onExport,
}: BdpStockActionsProps) {
  const queryClient = useQueryClient();
  const { catalogPriceType, saveProfile, isSaving } = useBdpReadProfiles();
  const [priceType, setPriceType] = useState(String(catalogPriceType));
  const [requiresConfiguration, setRequiresConfiguration] = useState(false);

  useEffect(() => setPriceType(String(catalogPriceType)), [catalogPriceType]);

  const syncMutation = useSyncCatalog({
    mutation: {
      onSuccess: (response) => {
        if (response.status !== 200) return;
        const result = response.data;
        if (result.total_bdp === 0) {
          setRequiresConfiguration(true);
          toast.warning('BDP devolvió 0 artículos', {
            description: 'Selecciona la tarifa de catálogo y vuelve a intentarlo.',
          });
          return;
        }
        setRequiresConfiguration(false);
        toast.success(`Sync completado: ${result.creados} nuevos, ${result.actualizados} actualizados`);
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al sincronizar catálogo BDP'),
    },
  });

  async function saveAndRetry() {
    const value = Number(priceType);
    if (!Number.isInteger(value) || value < 1 || value > 5) {
      toast.error('La tarifa de catálogo debe estar entre 1 y 5');
      return;
    }
    try {
      await saveProfile({ field: 'bdp_catalog_price_type', value });
      syncMutation.mutate();
    } catch {
      toast.error('No se pudo guardar la tarifa de catálogo');
    }
  }

  return (
    <div className="flex w-full flex-col gap-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-muted-foreground">{summary}</p>
        <div className="flex flex-wrap items-center gap-2">
          <BdpDemoToggle demoMode={demoMode} onToggle={onToggleDemo} />
          <Button variant="outline" onClick={onExport} disabled={exportDisabled} title="Exportar a CSV con BOM para Excel">
            <Download className="mr-1.5 size-4" />
            CSV
          </Button>
          <TooltipButton
            variant="outline"
            onClick={() => syncMutation.mutate()}
            disabled={syncMutation.isPending || demoMode || !bdpMode}
            tooltip={bdpMode ? 'Importa/actualiza artículos y stock desde BDP a la Aplicación Web. No modifica BDP.' : 'Requiere BDP conectado (modo BDP). En modo independiente el stock se gestiona localmente.'}
          >
            {syncMutation.isPending ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
            Sync catálogo
          </TooltipButton>
        </div>
      </div>
      {requiresConfiguration && (
        <BdpRequiredSetting
          title="BDP no devolvió artículos"
          description="Prueba la tarifa de precios que usa el catálogo del restaurante (1 a 5). Esto solo consulta BDP y guarda la selección en la Aplicación Web."
          label="Tarifa del catálogo BDP"
          value={priceType}
          max={5}
          saving={isSaving || syncMutation.isPending}
          onChange={setPriceType}
          onSave={saveAndRetry}
        />
      )}
    </div>
  );
}
