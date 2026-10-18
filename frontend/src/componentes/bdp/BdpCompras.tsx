/* [247A-11] Página de albaranes de compra BDP — Fase 1 (solo lectura).
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * Modo demo incluido para visualizar datos de prueba. */

import { useMemo, useState } from 'react';
import { Search, Plus, RefreshCw } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { toast } from 'sonner';
import { confirmarConDialogo } from '../dialogoConfirmacion';
import { useQueryClient } from '@tanstack/react-query';
import { useObtenerConfiguracion } from '@/api/generated/configuracion/configuracion';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import {
  useBdpPurchaseNotes,
  useDraftBdpPurchaseNote,
  useReconcileBdpPurchaseNote,
  useCrearBdpPurchaseNote,
  useActualizarBdpPurchaseNote,
  useEliminarBdpPurchaseNote,
} from '@/api/bdp';
import type {
  ActualizarBdpPurchaseNoteRequest,
  BdpPurchaseNote,
  BdpPurchaseNoteReconcileRequest,
  CrearBdpPurchaseNoteRequest,
} from '@/api/bdp';
import { mockPurchaseNotes } from './bdp-mocks';
import { useFiltrosAlbaranes } from '@/hooks/useFiltrosAlbaranes';
import { BdpComprasReconcileModal } from './BdpComprasReconcileModal';
import { BdpComprasLocalModal } from './BdpComprasLocalModal';
import { BdpPurchaseSyncControls } from './BdpPurchaseSyncControls';
import { BdpPurchaseFeatureNotice } from './BdpPurchaseFeatureNotice';
import { BdpComprasTabla } from './bdp-compras/BdpComprasTabla';

/* Skeletons estáticos: filas sin identidad propia; claves de un array estable
 * (no el índice del map) para reconciliación consistente. */
const SKELETON_IDS = [0, 1, 2, 3, 4];

function BdpCompras() {
  const queryClient = useQueryClient();
  /* [159A-3] El toggle demo vive en Configuración → BDP; aquí solo se lee. */
  const { demoMode } = useBdpDemoMode();
  const { data: configResponse, isLoading: isLoadingConfig } = useObtenerConfiguracion();
  const {
    proveedor,
    setProveedor,
    fechaDesde,
    setFechaDesde,
    fechaHasta,
    setFechaHasta,
    filtros: filters,
  } = useFiltrosAlbaranes();
  const [reconcileNote, setReconcileNote] = useState<BdpPurchaseNote | null>(null);
  const [localModalOpen, setLocalModalOpen] = useState(false);
  const [localModalNote, setLocalModalNote] = useState<BdpPurchaseNote | null>(null);

  /* [128A-1/F5][M12] Los flags BDP solo gatean en modo efectivo `bdp`.
   * En `standalone` el CRUD local siempre está disponible sin consultar flags. */
  const configData = configResponse?.status === 200 ? configResponse.data : undefined;
  const modoEfectivoBdp = !!configData && (
    configData.modo_operacion === 'bdp'
    || (configData.modo_operacion === 'auto'
      && configData.bdp_sync_enabled
      && (configData.bdp_base_url ?? '').trim() !== '')
  );
  /* El flag `ff_bdp_purchase_notes_read` solo gatea la IMPORTACIÓN desde BDP
   * (sync). El CRUD local (crear/editar/eliminar/listar albaranes `L-`)
   * funciona sin BDP y sin flag — igual que el backend, que solo exige el
   * flag en `sync`. */
  const purchaseFeatureEnabled = !isLoadingConfig
    && (!modoEfectivoBdp || !!configData?.ff_bdp_purchase_notes_read);
  /* [287A-7] El backend protege Compras con feature flag en modo bdp. No generar
   * 422 previsibles mientras la función está apagada; la pantalla explica cómo activarla. */
  const shouldLoadPurchases = !demoMode && !isLoadingConfig;
  const { data, isLoading, error, refetch } = useBdpPurchaseNotes(filters, shouldLoadPurchases);
  /* Mensaje de error honesto: el fallo de hoy era el flag apagado (422 con
   * "La lectura de albaranes de compra BDP no está activada"), no la sesión.
   * Solo se habla de sesión ante un 401 real; si el backend explica el motivo,
   * se muestra tal cual. */
  const purchaseAxiosResponse = (error as { response?: { status?: number; data?: { message?: string } } } | null)?.response;
  const purchaseErrorStatus = purchaseAxiosResponse?.status;
  const purchaseErrorMessage = purchaseAxiosResponse?.data?.message;
  const draftMutation = useDraftBdpPurchaseNote(queryClient);
  const reconcileMutation = useReconcileBdpPurchaseNote(queryClient);
  const crearMutation = useCrearBdpPurchaseNote(queryClient);
  const actualizarMutation = useActualizarBdpPurchaseNote(queryClient);
  const eliminarMutation = useEliminarBdpPurchaseNote(queryClient);

  const filteredDemoNotes = useMemo(() => {
    return mockPurchaseNotes.filter((n) => {
      const q = proveedor.trim().toLowerCase();
      const matchesProveedor =
        !q ||
        (n.nombre_proveedor?.toLowerCase().includes(q) ?? false) ||
        (n.codigo_proveedor?.toLowerCase().includes(q) ?? false);
      const matchesFecha =
        (!fechaDesde || (n.fecha && n.fecha >= fechaDesde)) &&
        (!fechaHasta || (n.fecha && n.fecha <= fechaHasta));
      return matchesProveedor && matchesFecha;
    });
  }, [proveedor, fechaDesde, fechaHasta]);

  const notes = demoMode ? filteredDemoNotes : (data ?? []);

  function handleDraft(note: BdpPurchaseNote) {
    if (demoMode) {
      toast.info('En modo demo no se guardan cambios reales');
      return;
    }
    draftMutation.mutate(note.id, {
      onSuccess: () => {
        toast.success('Albarán marcado como borrador');
      },
      onError: () => {
        toast.error('No se pudo marcar como borrador');
      },
    });
  }

  function handleReconcile(note: BdpPurchaseNote) {
    if (demoMode) {
      toast.info('En modo demo no se guardan cambios reales');
      return;
    }
    setReconcileNote(note);
  }

  function submitReconcile(req: BdpPurchaseNoteReconcileRequest) {
    if (!reconcileNote) return;
    reconcileMutation.mutate(
      { id: reconcileNote.id, req },
      {
        onSuccess: (res) => {
          toast.success(`Albarán conciliado con gasto ${res.gasto_id}`);
          setReconcileNote(null);
        },
        onError: () => {
          toast.error('No se pudo conciliar el albarán');
        },
      },
    );
  }

  function openNuevoAlbaran() {
    if (demoMode) {
      toast.info('En modo demo no se guardan cambios reales');
      return;
    }
    setLocalModalNote(null);
    setLocalModalOpen(true);
  }

  function openEditarAlbaran(note: BdpPurchaseNote) {
    if (demoMode) {
      toast.info('En modo demo no se guardan cambios reales');
      return;
    }
    setLocalModalNote(note);
    setLocalModalOpen(true);
  }

  function submitLocalAlbaran(
    req: CrearBdpPurchaseNoteRequest | ActualizarBdpPurchaseNoteRequest,
  ) {
    if (localModalNote) {
      actualizarMutation.mutate(
        { id: localModalNote.id, req: req as ActualizarBdpPurchaseNoteRequest },
        {
          onSuccess: () => {
            toast.success('Albarán local actualizado');
            setLocalModalOpen(false);
          },
          onError: () => {
            toast.error('No se pudo actualizar el albarán local');
          },
        },
      );
      return;
    }
    crearMutation.mutate(req as CrearBdpPurchaseNoteRequest, {
      onSuccess: () => {
        toast.success('Albarán local creado');
        setLocalModalOpen(false);
      },
      onError: () => {
        toast.error('No se pudo crear el albarán local');
      },
    });
  }

  /* [267A-8] Confirmación con diálogo propio: `window.confirm` no funciona
   * en el navegador empaquetado. */
  async function handleEliminarLocal(note: BdpPurchaseNote) {
    if (demoMode) {
      toast.info('En modo demo no se guardan cambios reales');
      return;
    }
    const confirmado = await confirmarConDialogo({
      titulo: `¿Eliminar el albarán local ${note.serie}-${note.numero}?`,
      textoConfirmar: 'Eliminar',
    });
    if (!confirmado) return;
    eliminarMutation.mutate(note.id, {
      onSuccess: () => {
        toast.success('Albarán local eliminado');
      },
      onError: (err) => {
        /* [208A-2] Muestra el motivo real del backend (p. ej. albarán
         * conciliado no eliminable) en vez de un mensaje genérico. */
        const msg =
          (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
        toast.error(msg ?? 'No se pudo eliminar el albarán local');
      },
    });
  }

  return (
    <div className="flex flex-col gap-4">
      <BdpPurchaseSyncControls
        count={notes.length}
        demoMode={demoMode}
        /* [128A-1/F5] Sync con BDP solo tiene sentido en modo efectivo bdp. */
        featureEnabled={modoEfectivoBdp && purchaseFeatureEnabled}
        fechaDesde={fechaDesde}
        fechaHasta={fechaHasta}
      />

      {!demoMode && !isLoadingConfig && !purchaseFeatureEnabled && <BdpPurchaseFeatureNotice />}

      <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        <div className="flex flex-wrap gap-3 items-center">
          <div className="relative w-full sm:w-64">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
            <Input
              type="search"
              placeholder="Proveedor..."
              value={proveedor}
              onChange={(e) => setProveedor(e.target.value)}
              className="pl-9 max-w-xs"
            />
          </div>
          <div className="flex items-center gap-1.5">
            <span className="text-sm text-muted-foreground shrink-0">Desde:</span>
            <Input
              type="date"
              value={fechaDesde}
              onChange={(e) => setFechaDesde(e.target.value)}
              className="max-w-40"
              aria-label="Fecha desde"
            />
          </div>
          <div className="flex items-center gap-1.5">
            <span className="text-sm text-muted-foreground shrink-0">Hasta:</span>
            <Input
              type="date"
              value={fechaHasta}
              onChange={(e) => setFechaHasta(e.target.value)}
              className="max-w-40"
              aria-label="Fecha hasta"
            />
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="default" onClick={openNuevoAlbaran} disabled={demoMode}>
            <Plus className="mr-1 size-4" />
            Nuevo albarán
          </Button>
        </div>
      </div>

      {isLoading && !demoMode ? (
        <div className="space-y-2">
          {SKELETON_IDS.map((id) => (
            <div key={id} className="h-10 w-full animate-pulse rounded bg-muted" />
          ))}
        </div>
      ) : error && !demoMode ? (
        <div className="flex flex-col items-start gap-3 rounded-md border border-destructive/50 p-4">
          <p className="text-sm text-destructive">
            {purchaseErrorStatus === 401
              ? 'Tu sesión ha caducado. Vuelve a iniciar sesión y reintenta.'
              : purchaseErrorMessage
                ? `Error al cargar los albaranes: ${purchaseErrorMessage}`
                : 'Error al cargar los albaranes. Vuelve a intentarlo.'}
          </p>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="size-3.5 mr-1.5" />
            Reintentar
          </Button>
        </div>
      ) : notes.length === 0 ? (
        <div className="flex flex-col items-start gap-3 rounded-md border border-dashed p-4">
          <p className="text-sm text-muted-foreground">
            {purchaseFeatureEnabled
              ? "No hay albaranes todavía. Puedes crear uno local (serie L-) o, si la integración BDP está activa, sincronizarlos desde el terminal."
              : "No hay albaranes todavía. Puedes crear uno local (serie L-) sin necesidad del BDP; la importación desde el terminal requiere activar la lectura de albaranes en Configuración → BDP."}
          </p>
          {/* [159A-3] Sin botón "Cargar demo": el modo demo se activa desde
            * Configuración → pestaña BDP. */}
          <div className="flex flex-wrap gap-2">
            <Button variant="default" size="sm" onClick={openNuevoAlbaran} disabled={demoMode}>
              <Plus className="mr-1 size-3.5" />
              Nuevo albarán
            </Button>
          </div>
        </div>
      ) : (
        <BdpComprasTabla
          notas={notes}
          actualizando={actualizarMutation.isPending || eliminarMutation.isPending || draftMutation.isPending || reconcileMutation.isPending}
          onEditar={openEditarAlbaran}
          onEliminar={handleEliminarLocal}
          onBorrador={handleDraft}
          onConciliar={handleReconcile}
        />
      )}

      <BdpComprasReconcileModal
        open={!!reconcileNote}
        note={reconcileNote}
        onClose={() => setReconcileNote(null)}
        onSubmit={submitReconcile}
      />

      <BdpComprasLocalModal
        open={localModalOpen}
        note={localModalNote}
        isSubmitting={crearMutation.isPending || actualizarMutation.isPending}
        onClose={() => setLocalModalOpen(false)}
        onSubmit={submitLocalAlbaran}
      />
    </div>
  );
}

export default BdpCompras;
