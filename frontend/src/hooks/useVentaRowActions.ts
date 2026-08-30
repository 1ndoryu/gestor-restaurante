/* [064A-10] Estado y lógica de las acciones por fila de venta (pago BDP, pago/factura
 * local, propina, anulación, consulta de estado), extraído de venta-row-actions a un
 * hook custom (protocolo usestate-excesivo). El componente solo declara el estado y
 * renderiza los diálogos; la API pública del componente no cambia. */
import { useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import instance from '@/api/axios-instance';
import { toast } from 'sonner';
import type { VentaConCliente } from '../api/generated';
import type { VentaConClienteBdp } from '../api/bdp';
import { fetchBdpStatus } from '../api/bdp';

export interface BdpPaymentHistoryItem {
  id: string;
  amount: string;
  tender_id: number;
  resultado: 'exito' | 'ambiguo' | 'error' | string;
  created_at: string;
}

export interface BdpPaymentsResponse {
  venta_id: string;
  total: string;
  pagado: string;
  pendiente: string;
  pagos: BdpPaymentHistoryItem[];
}

interface UseVentaRowActionsArgs {
  venta: VentaConCliente;
  bdpSyncEnabled?: boolean;
  onAnular?: (ventaId: string, motivo: string) => void;
  anulacionModalidad?: string;
}

export function useVentaRowActions({ venta: v, bdpSyncEnabled = false, onAnular, anulacionModalidad = 'credito_completo' }: UseVentaRowActionsArgs) {
  const bdp = v as unknown as VentaConClienteBdp;
  const totalVenta = Number(v.importe_base) + Number(v.importe_iva);
  const total = totalVenta.toFixed(2);
  const queryClient = useQueryClient();
  const [accion, setAccion] = useState<'pago' | 'factura' | 'pagoLocal' | 'facturaLocal' | 'propina' | null>(null);
  const [tenderId, setTenderId] = useState('');
  const [importe, setImporte] = useState(total);
  const [confirmacion, setConfirmacion] = useState('');
  const [enviando, setEnviando] = useState(false);
  const [consultandoEstado, setConsultandoEstado] = useState(false);
  const [pagos, setPagos] = useState<BdpPaymentsResponse | null>(null);
  const [cargandoPagos, setCargandoPagos] = useState(false);
  /* [128A-1/F4] Anulación local */
  const [anularAbierto, setAnularAbierto] = useState(false);
  const [motivo, setMotivo] = useState('');
  /* [198A-1/D8] Propina por venta (sumar/sustituir, D8). */
  const [propinaImporte, setPropinaImporte] = useState('');
  const [propinaSumar, setPropinaSumar] = useState(true);

  const hayAmbiguo = useMemo(() => pagos?.pagos.some((p) => p.resultado === 'ambiguo') ?? false, [pagos]);
  const pendiente = useMemo(() => Number(pagos?.pendiente ?? totalVenta), [pagos, totalVenta]);
  const pagado = useMemo(() => Number(pagos?.pagado ?? 0), [pagos]);

  useEffect(() => {
    if (accion === 'pago' || accion === 'pagoLocal' || accion === 'facturaLocal') {
      setCargandoPagos(true);
      instance.get<BdpPaymentsResponse>(`/api/ventas/${v.id}/bdp-payments`)
        .then((r) => setPagos(r.data))
        .catch(() => toast.error('No se pudo cargar el historial de pagos BDP'))
        .finally(() => setCargandoPagos(false));
    }
  }, [accion, v.id]);

  useEffect(() => {
    if (accion === 'pago' || accion === 'pagoLocal') {
      setImporte(pendiente.toFixed(2));
    }
  }, [accion, pendiente]);

  const cerrar = () => {
    setAccion(null);
    setConfirmacion('');
    setTenderId('');
    setImporte(total);
    setPagos(null);
    setPropinaImporte('');
    setPropinaSumar(true);
  };

  const ejecutarBdp = async () => {
    if (accion !== 'pago' && accion !== 'factura') return;
    const importeCanonico = Number(importe).toFixed(2);
    setEnviando(true);
    try {
      if (accion === 'pago') {
        const idempotencyKey = crypto.randomUUID();
        await instance.post(`/api/ventas/${v.id}/bdp-payment`, {
          amount: Number(importeCanonico),
          tender_id: Number(tenderId),
          confirmacion,
          idempotency_key: idempotencyKey,
          auto_arm: true,
        });
        toast.success('Pago registrado en BDP');
      } else {
        await instance.post(`/api/ventas/${v.id}/bdp-invoice`, {
          confirmacion,
          auto_arm: true,
        });
        toast.success('Factura confirmada por BDP');
      }
      cerrar();
      queryClient.invalidateQueries({ queryKey: ['listarVentas'] });
    } catch (error) {
      const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
      toast.error('Operación BDP bloqueada', { description: message ?? 'Revisa preflight, armado y auditoría antes de reintentar.' });
    } finally {
      setEnviando(false);
    }
  };

  /* [128A-1/F6] Pago parcial local y factura local mínima. */
  /* [198A-1/D8] Propina por venta: guarda local y, con BDP, encola AddOrderTip. */
  const ejecutarPropina = async () => {
    if (accion !== 'propina') return;
    const importeCanonico = Number(propinaImporte).toFixed(2);
    setEnviando(true);
    try {
      await instance.post(`/api/ventas/${v.id}/propina`, {
        amount: Number(importeCanonico),
        add_tip: propinaSumar,
      });
      toast.success(`Propina de ${importeCanonico} € registrada`);
      cerrar();
      queryClient.invalidateQueries({ queryKey: ['listarVentas'] });
    } catch (error) {
      const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
      toast.error('No se pudo guardar la propina', { description: message ?? 'Revisa el estado de la venta.' });
    } finally {
      setEnviando(false);
    }
  };

  /* [128A-1/F6] Pago parcial local y factura local mínima. */
  const ejecutarLocal = async () => {
    if (accion !== 'pagoLocal' && accion !== 'facturaLocal') return;
    const importeCanonico = Number(importe).toFixed(2);
    setEnviando(true);
    try {
      if (accion === 'pagoLocal') {
        await instance.post(`/api/ventas/${v.id}/pagos-locales`, {
          amount: Number(importeCanonico),
          tender_id: Number(tenderId),
          confirmacion,
          idempotency_key: crypto.randomUUID(),
        });
        toast.success(`Pago local registrado (${importeCanonico} €)`);
      } else {
        await instance.post(`/api/ventas/${v.id}/factura-local`, {
          confirmacion,
          idempotency_key: crypto.randomUUID(),
        });
        toast.success('Venta facturada localmente');
      }
      cerrar();
      queryClient.invalidateQueries({ queryKey: ['listarVentas'] });
    } catch (error) {
      const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
      toast.error('Operación local bloqueada', { description: message ?? 'Revisa el estado de la venta y reintenta.' });
    } finally {
      setEnviando(false);
    }
  };

  const puedePagar = !v.anulada && bdpSyncEnabled && bdp.bdp_synced && bdp.bdp_order_id && !bdp.bdp_invoiced && bdp.bdp_order_status !== 'cancelled' && bdp.bdp_order_status !== 'invoiced';
  const esFacturada = Boolean(bdp.facturada_local || bdp.bdp_invoiced || bdp.bdp_order_status === 'invoiced');
  const puedePagoLocal = !v.anulada && !esFacturada && !puedePagar;
  const puedeFacturaLocal = !v.anulada && !esFacturada && !puedePagar;
  const motivoObligatorio = anulacionModalidad === 'credito_completo';
  const anulacionPendienteBdp = v.anulada && bdp.bdp_synced && bdp.bdp_order_status !== 'cancelled' && bdp.bdp_order_status !== 'invoiced';

  const consultarEstado = async () => {
    setConsultandoEstado(true);
    try {
      const status = await fetchBdpStatus(v.id);
      toast.info(`Estado BDP: ${status.bdp_order_status ?? 'desconocido'}`, {
        description: `Orden: ${status.bdp_order_id ?? '—'} · Sync: ${status.bdp_synced ? 'sí' : 'no'}${status.bdp_sync_error ? ` · Error: ${status.bdp_sync_error}` : ''}`,
      });
      queryClient.invalidateQueries({ queryKey: ['listarVentas'] });
    } catch {
      toast.error('No se pudo consultar el estado BDP');
    } finally {
      setConsultandoEstado(false);
    }
  };

  const cerrarAnulacion = () => {
    setAnularAbierto(false);
    setMotivo('');
  };

  const ejecutarAnulacion = () => {
    if (!onAnular) return;
    onAnular(v.id, motivo.trim());
    cerrarAnulacion();
  };

  return {
    bdp,
    totalVenta,
    total,
    accion,
    setAccion,
    tenderId,
    setTenderId,
    importe,
    setImporte,
    confirmacion,
    setConfirmacion,
    enviando,
    consultandoEstado,
    consultarEstado,
    pagos,
    cargandoPagos,
    anularAbierto,
    setAnularAbierto,
    motivo,
    setMotivo,
    propinaImporte,
    setPropinaImporte,
    propinaSumar,
    setPropinaSumar,
    hayAmbiguo,
    pendiente,
    pagado,
    cerrar,
    ejecutarBdp,
    ejecutarPropina,
    ejecutarLocal,
    puedePagar,
    esFacturada,
    puedePagoLocal,
    puedeFacturaLocal,
    motivoObligatorio,
    anulacionPendienteBdp,
    cerrarAnulacion,
    ejecutarAnulacion,
  };
}