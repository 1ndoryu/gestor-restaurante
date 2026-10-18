/* Importación de salones y mesas desde BDP: estado del diálogo, previsualización y aplicación.
 * BDP solo se lee; al aplicar se crean únicamente zonas y mesas que falten en la Aplicación Web. */
import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import { useSyncTables } from '@/api/generated/bdp-mapeos/bdp-mapeos';

/* Resultado de previsualizar o aplicar la importación de mesas */
export interface PreviewSyncMesasBdp {
  salones_bdp: number;
  zonas_creadas: number;
  mesas_creadas: number;
  applied: boolean;
}

export function useSincronizarMesasBdp() {
  const queryClient = useQueryClient();
  const [sincronizandoMesas, setSincronizandoMesas] = useState(false);
  const [dialogoSyncBdp, setDialogoSyncBdp] = useState(false);
  const [confirmacionSyncBdp, setConfirmacionSyncBdp] = useState('');
  const [previewSyncBdp, setPreviewSyncBdp] = useState<PreviewSyncMesasBdp | null>(null);

  const syncTablesMutation = useSyncTables({
    mutation: {
      onSuccess: (resp) => {
        const d = (resp as unknown as { data: { salones_bdp: number; zonas_creadas: number; mesas_creadas: number; applied: boolean } }).data;
        setPreviewSyncBdp(d);
        if (d.applied) {
          toast.success(`Importación local aplicada: ${d.mesas_creadas} mesas en ${d.zonas_creadas} zonas`);
          queryClient.invalidateQueries({ queryKey: ['/api/plano-sala'] });
          setDialogoSyncBdp(false);
        } else {
          toast.success('Previsualización completada sin cambios');
        }
      },
      onError: () => toast.error('Error al sincronizar mesas desde BDP'),
      onSettled: () => setSincronizandoMesas(false),
    },
  });

  /* Previsualiza sin escribir nada; aplicar exige la confirmación escrita del diálogo. */
  const previsualizar = () => {
    setSincronizandoMesas(true);
    syncTablesMutation.mutate({ data: { aplicar: false } });
  };
  const aplicar = () => {
    setSincronizandoMesas(true);
    syncTablesMutation.mutate({ data: { aplicar: true, confirmacion: confirmacionSyncBdp } });
  };

  return {
    sincronizandoMesas, dialogoSyncBdp, setDialogoSyncBdp, confirmacionSyncBdp, setConfirmacionSyncBdp,
    previewSyncBdp, setPreviewSyncBdp, previsualizar, aplicar,
  };
}
