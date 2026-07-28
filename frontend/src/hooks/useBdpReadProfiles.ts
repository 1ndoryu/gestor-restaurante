/* [287A-5] Configuración mínima para lecturas BDP. Se mantiene separada del
 * formulario general para que Stock y Compras puedan corregirse en contexto. */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { customInstance } from '@/api/axios-instance';

export interface BdpReadProfiles {
  bdp_catalog_price_type: number;
  bdp_purchase_notes_profile_id: number | null;
}

type ProfileField = keyof BdpReadProfiles;

async function fetchBdpReadProfiles(signal?: AbortSignal): Promise<BdpReadProfiles> {
  const response = await customInstance<{ data: BdpReadProfiles }>('/api/configuracion', {
    method: 'GET',
    signal,
  });
  return response.data;
}

async function updateBdpReadProfile(field: ProfileField, value: number): Promise<BdpReadProfiles> {
  const response = await customInstance<{ data: BdpReadProfiles }>('/api/configuracion', {
    method: 'PATCH',
    body: JSON.stringify({ [field]: value }),
  });
  return response.data;
}

export function useBdpReadProfiles() {
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: ['bdp-read-profiles'],
    queryFn: ({ signal }) => fetchBdpReadProfiles(signal),
  });
  const mutation = useMutation({
    mutationFn: ({ field, value }: { field: ProfileField; value: number }) =>
      updateBdpReadProfile(field, value),
    onSuccess: (data) => {
      queryClient.setQueryData(['bdp-read-profiles'], data);
      queryClient.invalidateQueries({ queryKey: ['/api/configuracion'] });
    },
  });

  return {
    catalogPriceType: query.data?.bdp_catalog_price_type ?? 1,
    purchaseProfileId: query.data?.bdp_purchase_notes_profile_id ?? null,
    isLoading: query.isLoading,
    saveProfile: mutation.mutateAsync,
    isSaving: mutation.isPending,
  };
}
