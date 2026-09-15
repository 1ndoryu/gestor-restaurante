/* [287A-5] Acciones de catálogo en Configuración BDP. Si ExportArticles
 * devuelve cero, muestra la tarifa configurable en el mismo lugar.
 * [159A-2/F2] Par Importar/Exportar: un solo "Importar del BDP" (catálogo +
 * precios encadenados, un resumen) y "Exportar al BDP" (flush dominio
 * articulo). */

import { useEffect, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { BdpImportExportButtons } from '@/components/bdp-import-export-buttons';
import { BdpRequiredSetting } from '@/components/bdp-required-setting';
import { useSyncCatalog, useSyncPrices } from '@/api/generated/bdp-mapeos/bdp-mapeos';
import { useBdpReadProfiles } from '@/hooks/useBdpReadProfiles';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';

export function BdpArticleCatalogActions() {
  const queryClient = useQueryClient();
  /* [208A-2/C2] H7: en modo independiente no se ofrecen acciones que consultan BDP. */
  const { data: configResponse } = useObtenerConfiguracion();
  const configData = configResponse?.status === 200 ? configResponse.data : undefined;
  const modoEfectivoBdp = !!configData && (
    configData.modo_operacion === 'bdp'
    || (configData.modo_operacion === 'auto'
      && configData.bdp_sync_enabled
      && (configData.bdp_base_url ?? '').trim() !== '')
  );
  const { catalogPriceType, saveProfile, isSaving } = useBdpReadProfiles();
  const [priceType, setPriceType] = useState(String(catalogPriceType));
  const [requiresConfiguration, setRequiresConfiguration] = useState(false);

  useEffect(() => setPriceType(String(catalogPriceType)), [catalogPriceType]);

  const pricesMutation = useSyncPrices({
    mutation: {
      onSuccess: (respPrecios) => {
        if (respPrecios.status === 200) toast.success(`Precios actualizados: ${respPrecios.data.actualizados} artículos`);
      },
      onError: () => toast.error('No se pudieron importar los precios del BDP'),
    },
  });
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
        const omitidos = result.omitidos_ediciones_locales;
        const desactivados = result.desactivados_localmente;
        const extras = [omitidos > 0 && `${omitidos} omitidos (edición local)`, desactivados > 0 && `${desactivados} desactivados localmente`]
          .filter(Boolean)
          .join(', ');
        toast.success(`Importados del BDP: ${result.creados} nuevos, ${result.actualizados} actualizados${extras ? ` — ${extras}` : ''}`);
        queryClient.invalidateQueries({ queryKey: ['/api/bdp/article-maps'] });
        pricesMutation.mutate();
      },
      onError: () => toast.error('No se pudo importar el catálogo del BDP'),
    },
  });
  const importando = catalogMutation.isPending || pricesMutation.isPending;

  /* [159A-2/F2] Importar = catálogo y luego precios, con un solo botón.
   * Solo lectura BDP; las ediciones locales de mapeos se omiten (backend). */
  function importar() {
    catalogMutation.mutate();
  }

  async function saveAndRetry() {
    const value = Number(priceType);
    if (!Number.isInteger(value) || value < 1 || value > 5) {
      toast.error('La tarifa de catálogo debe estar entre 1 y 5');
      return;
    }
    try {
      await saveProfile({ field: 'bdp_catalog_price_type', value });
      importar();
    } catch {
      toast.error('No se pudo guardar la tarifa de catálogo');
    }
  }

  return (
    <div className="flex w-full flex-col items-end gap-2 sm:w-auto">
      <BdpImportExportButtons
        onImportar={importar}
        importando={importando}
        importarTooltip={modoEfectivoBdp ? 'Importa artículos, stock y precios desde BDP a la Aplicación Web. Crea mapeos automáticos por código. Solo lectura BDP.' : 'Requiere BDP conectado (modo BDP). En modo independiente el catálogo se gestiona localmente.'}
        importarDeshabilitado={!modoEfectivoBdp}
        dominiosExportar={['articulo']}
        exportarTooltip={modoEfectivoBdp ? 'Envía al BDP los cambios locales de artículos pendientes en la cola.' : 'Requiere BDP conectado (modo BDP).'}
        exportarDeshabilitado={!modoEfectivoBdp}
        invalidarTrasExportar={[['/api/bdp/article-maps']]}
      />
      {requiresConfiguration && (
        <BdpRequiredSetting
          title="BDP no devolvió artículos"
          description="Selecciona la tarifa de precios del catálogo (1 a 5). Solo se consulta BDP."
          label="Tarifa del catálogo BDP"
          value={priceType}
          max={5}
          saving={isSaving || importando}
          onChange={setPriceType}
          onSave={saveAndRetry}
        />
      )}
    </div>
  );
}
