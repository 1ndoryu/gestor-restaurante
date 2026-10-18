import { toast } from 'sonner';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import { useLlamarCamarero } from '@/api/bdp';

export function useModoBdpPlano() {
  /* [198A-1/D10] CallWaiter: push directo a BDP. El botón solo se muestra en
   * modo BDP efectivo (mismo criterio que BdpCatalogo y resto de secciones:
   * auto + sync + base_url; las credenciales no se exigen aquí porque el
   * backend nunca las expone en GET /api/configuracion por diseño). */
  const { data: configData } = useObtenerConfiguracion();
  const llamarCamareroMutation = useLlamarCamarero();
  const cfg = configData?.status === 200 ? (configData.data as unknown as Record<string, unknown>) : null;
  const modoOperacion = String(cfg?.modo_operacion ?? 'auto');
  const bdpSyncEnabled = Boolean(cfg?.bdp_sync_enabled ?? false);
  const modoEfectivoBdp = modoOperacion === 'bdp' || (modoOperacion === 'auto' && bdpSyncEnabled && String(cfg?.bdp_base_url ?? '').trim() !== '');
  const handleLlamarCamarero = (mesaId: string) => {
    llamarCamareroMutation.mutate(mesaId, {
      onSuccess: () => toast.success('Aviso enviado al TPV'),
      onError: (err) => {
        const message = (err as { response?: { data?: { message?: string } } }).response?.data?.message;
        toast.error('No se pudo llamar al camarero', { description: message ?? 'Revisa la conexión BDP.' });
      },
    });
  };

  return { modoEfectivoBdp, handleLlamarCamarero, llamarCamareroPending: llamarCamareroMutation.isPending };
}
