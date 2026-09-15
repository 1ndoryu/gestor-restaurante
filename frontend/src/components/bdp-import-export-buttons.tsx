/* [159A-2/F2] Par Importar/Exportar BDP por sección.
 * Unifica el vocabulario de sincronización: "Importar del BDP" = BDP→local
 * (solo lectura BDP); "Exportar al BDP" = local→BDP vía cola + flush filtrado
 * por dominios. Los archivos locales (CSV/JSON) usan Descargar/Guardar/Cargar,
 * nunca Importar/Exportar, para no confundirlos con el BDP.
 * No ejecuta nada por sí solo: el padre aporta `onImportar` (lectura) y los
 * `dominiosExportar` (filtro del flush). */

import { Download, Loader2, Upload } from 'lucide-react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { useFlushBdpPush, type BdpPushFlushResumen } from '@/api/bdp';

/** Resumen del flush con el mismo criterio que Sincronización (D5/D2). */
export function mostrarResumenFlush(r: BdpPushFlushResumen) {
  if (r.sincronizados > 0) {
    toast.success('Exportado al BDP', { description: `${r.sincronizados} operación(es) enviada(s).` });
  } else if (r.pendientes_suscripcion > 0) {
    toast.warning('Pendiente de suscripción BDP', { description: `${r.pendientes_suscripcion} operación(es) requieren la suscripción WebLink.` });
  } else if (r.rechazados > 0) {
    toast.error('Operación(es) rechazadas por BDP', { description: `${r.rechazados} operación(es) con payload inválido o conflicto: corregir el dato local y reintentar manualmente.` });
  } else if (r.errores > 0) {
    toast.error('Errores al exportar', { description: `${r.errores} operación(es) fallaron.` });
  } else if (r.omitidos_standalone > 0) {
    toast.info('Modo independiente', { description: 'La cola no se envía mientras no haya BDP conectado.' });
  } else {
    toast.info('Sin operaciones pendientes');
  }
}

interface BdpImportExportButtonsProps {
  /** Lectura BDP→local propia de la sección. El padre gestiona su pending. */
  onImportar: () => void;
  importando: boolean;
  importarTooltip: string;
  importarDeshabilitado?: boolean;
  /** Dominios del flush (Exportar local→BDP solo de esta sección). */
  dominiosExportar: string[];
  exportarTooltip: string;
  exportarDeshabilitado?: boolean;
  /** queryKeys a invalidar tras un flush con envíos. */
  invalidarTrasExportar?: string[][];
  /** Secciones sin lectura BDP (p. ej. familias: el BDP no las expone) solo
   * muestran Exportar. */
  mostrarImportar?: boolean;
}

export function BdpImportExportButtons({
  onImportar,
  importando,
  importarTooltip,
  importarDeshabilitado = false,
  dominiosExportar,
  exportarTooltip,
  exportarDeshabilitado = false,
  invalidarTrasExportar = [],
  mostrarImportar = true,
}: BdpImportExportButtonsProps) {
  const queryClient = useQueryClient();
  const flushMutation = useFlushBdpPush();

  function exportar() {
    flushMutation.mutate(dominiosExportar, {
      onSuccess: (r) => {
        mostrarResumenFlush(r);
        if (r.sincronizados > 0) {
          for (const queryKey of invalidarTrasExportar) {
            queryClient.invalidateQueries({ queryKey });
          }
          queryClient.invalidateQueries({ queryKey: ['/api/bdp/push/pendientes'] });
        }
      },
      onError: (err: unknown) => {
        const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
        toast.error('No se pudo exportar al BDP', { description: msg });
      },
    });
  }

  return (
    <div className="flex flex-wrap gap-2">
      {mostrarImportar && (
      <TooltipButton
        variant="outline"
        size="sm"
        onClick={onImportar}
        disabled={importando || importarDeshabilitado}
        tooltip={importarTooltip}
      >
        {importando ? <Loader2 className="size-3.5 animate-spin" /> : <Download className="size-3.5" />}
        Importar del BDP
      </TooltipButton>
      )}
      <TooltipButton
        variant="outline"
        size="sm"
        onClick={exportar}
        disabled={flushMutation.isPending || exportarDeshabilitado}
        tooltip={exportarTooltip}
      >
        {flushMutation.isPending ? <Loader2 className="size-3.5 animate-spin" /> : <Upload className="size-3.5" />}
        Exportar al BDP
      </TooltipButton>
    </div>
  );
}
