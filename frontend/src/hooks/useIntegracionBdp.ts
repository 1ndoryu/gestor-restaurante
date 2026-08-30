/* [263A-16] Estado y handlers de la integración BDP de clientes
 * (vincular cliente con código explícito + importación desde BDP), extraído de
 * ListaClientes a un hook custom (protocolo usestate-excesivo). El componente
 * pasa `refrescar` (cerrarModalYRefrescar) para recargar tras una escritura. */
import { useState } from 'react';
import { toast } from 'sonner';
import { customInstance } from '@/api/axios-instance';
import type { Cliente } from '@/api/generated';

export interface PreviewImportar {
  imported: number;
  updated: number;
  unchanged: number;
  conflicts: number;
  errors: number;
  total: number;
}

export function useIntegracionBdp(refrescar: () => void) {
  const [clienteBdp, setClienteBdp] = useState<Cliente | null>(null);
  const [codigoBdp, setCodigoBdp] = useState('');
  const [confirmacionBdp, setConfirmacionBdp] = useState('');
  const [sincronizandoBdp, setSincronizandoBdp] = useState(false);
  const [importarBdpAbierto, setImportarBdpAbierto] = useState(false);
  const [importandoBdp, setImportandoBdp] = useState(false);
  const [confirmacionImportar, setConfirmacionImportar] = useState('');
  const [previewImportar, setPreviewImportar] = useState<PreviewImportar | null>(null);

  const sincronizarClienteBdp = async () => {
    if (!clienteBdp || confirmacionBdp !== `CREAR CLIENTE ${clienteBdp.nombre} ${clienteBdp.apellidos} ${codigoBdp}`) return;
    setSincronizandoBdp(true);
    try {
      await customInstance(`/api/clientes/${clienteBdp.id}/bdp-sync`, {
        method: 'POST',
        body: JSON.stringify({
          bdp_customer_code: Number(codigoBdp),
          confirmacion: confirmacionBdp,
        }),
      });
      toast.success('Cliente BDP vinculado o creado con código explícito');
      setClienteBdp(null);
      setCodigoBdp('');
      setConfirmacionBdp('');
      refrescar();
    } catch (error) {
      const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
      toast.error('Sincronización BDP bloqueada', { description: message ?? 'Revisa armado, código e identidad.' });
    } finally {
      setSincronizandoBdp(false);
    }
  };

  const importarClientesBdp = async (aplicar: boolean) => {
    setImportandoBdp(true);
    try {
      const response = await customInstance('/api/bdp/customers/import', {
        method: 'POST',
        body: JSON.stringify({ aplicar, confirmacion: aplicar ? confirmacionImportar : null }),
      }) as { data: PreviewImportar | null };
      if (response.data) setPreviewImportar(response.data);
      if (aplicar) {
        toast.success('Importación local aplicada; no se escribió nada en BDP');
        refrescar();
      } else {
        toast.success('Previsualización completada sin cambios locales');
      }
    } catch (error) {
      const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
      toast.error('Importación BDP bloqueada', { description: message ?? 'No se aplicaron cambios.' });
    } finally {
      setImportandoBdp(false);
    }
  };

  return {
    clienteBdp,
    setClienteBdp,
    codigoBdp,
    setCodigoBdp,
    confirmacionBdp,
    setConfirmacionBdp,
    sincronizandoBdp,
    importarBdpAbierto,
    setImportarBdpAbierto,
    importandoBdp,
    confirmacionImportar,
    setConfirmacionImportar,
    previewImportar,
    setPreviewImportar,
    sincronizarClienteBdp,
    importarClientesBdp,
  };
}