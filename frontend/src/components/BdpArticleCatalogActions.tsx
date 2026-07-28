/* [287A-5] Acciones de catálogo en Configuración BDP. Si ExportArticles
 * devuelve cero, muestra la tarifa configurable en el mismo lugar. */

import { useEffect, useState } from 'react';
import { Loader2, RefreshCw } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { BdpRequiredSetting } from '@/components/bdp-required-setting';
import { useSyncCatalog, useSyncPrices } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpReadProfiles } from '@/hooks/useBdpReadProfiles';

export function BdpArticleCatalogActions() {
  const queryClient = useQueryClient();
  const { catalogPriceType, saveProfile, isSaving } = useBdpReadProfiles();
  const [priceType, setPriceType] = useState(String(catalogPriceType));
  const [requiresConfiguration, setRequiresConfiguration] = useState(false);

  useEffect(() => setPriceType(String(catalogPriceType)), [catalogPriceType]);

  const catalogMutation = useSyncCatalog({
    mutation: {
      onSuccess: (response) => {
        if (response.status !== 200) return;
        const result = response.data;
        if (result.total_bdp === 0) {
          setRequiresConfiguration(true);
          toast.warning('BDP devolvió 0 artículos');
          return;
        }
        setRequiresConfiguration(false);
        toast.success(`Sync completado: ${result.creados} nuevos, ${result.actualizados} actualizados`);
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
      },
      onError: () => toast.error('Error al sincronizar catálogo BDP'),
    },
  });
  const pricesMutation = useSyncPrices({
    mutation: {
      onSuccess: (response) => {
        if (response.status === 200) toast.success(`Precios actualizados: ${response.data.actualizados} artículos`);
      },
      onError: () => toast.error('Error al sincronizar precios BDP'),
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
      catalogMutation.mutate();
    } catch {
      toast.error('No se pudo guardar la tarifa de catálogo');
    }
  }

  return (
    <div className="flex w-full flex-col items-end gap-2 sm:w-auto">
      <div className="flex flex-wrap justify-end gap-2">
        <TooltipButton variant="default" size="sm" onClick={() => catalogMutation.mutate()} disabled={catalogMutation.isPending} tooltip="Importa/actualiza artículos desde BDP a Glory. Crea mapeos automáticos por código.">
          {catalogMutation.isPending ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
          Sync catálogo
        </TooltipButton>
        <TooltipButton variant="outline" size="sm" onClick={() => pricesMutation.mutate()} disabled={pricesMutation.isPending} tooltip="Actualiza los precios de los artículos mapeados desde BDP. El stock solo se actualiza con 'Sync catálogo'.">
          {pricesMutation.isPending ? <Loader2 className="size-3.5 animate-spin" /> : <RefreshCw className="size-3.5" />}
          Sync precios
        </TooltipButton>
      </div>
      {requiresConfiguration && (
        <BdpRequiredSetting
          title="BDP no devolvió artículos"
          description="Selecciona la tarifa de precios del catálogo (1 a 5). Solo se consulta BDP."
          label="Tarifa del catálogo BDP"
          value={priceType}
          max={5}
          saving={isSaving || catalogMutation.isPending}
          onChange={setPriceType}
          onSave={saveAndRetry}
        />
      )}
    </div>
  );
}
