/* [BDP-EXPLORER-02] Explorador de menús, packs y fastfoods BDP.
 * Estructura coherente con ListaVentas/ListaGastos/ListaReservas.
 * En modo real permite consultar por código numérico a BDP.
 * Modo demo muestra una tabla con datos de ejemplo. */

import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { confirmarConDialogo } from '../dialogoConfirmacion';
import { useQueryClient } from '@tanstack/react-query';
import {
  useGetMenuDefinition,
  useGetFastfoodDefinition,
  useGetPackDefinition,
} from '@/api/generated/bdp-mapeos/bdp-mapeos';
import {
  useBdpMenusLocales,
  useCrearBdpMenuLocal,
  useActualizarBdpMenuLocal,
  useEliminarBdpMenuLocal,
} from '@/api/bdp';
import type {
  ActualizarBdpMenuLocalRequest,
  BdpMenuLocalConLineas,
  CrearBdpMenuLocalRequest,
} from '@/api/bdp';
import { useBdpDemoMode } from '@/hooks/useBdpDemoMode';
import { mockExplorerItems, type BdpExplorerItem } from './bdp-mocks';
import { BdpMenuLocalModal } from './BdpMenuLocalModal';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { DefinitionDetail } from './bdp-explorador/DefinitionDetail';
import { MenusLocales } from './bdp-explorador/MenusLocales';
import { BarraConsulta } from './bdp-explorador/BarraConsulta';
import { ResultadoConsulta } from './bdp-explorador/ResultadoConsulta';
import { TablaDefinicionesDemo } from './bdp-explorador/TablaDefinicionesDemo';
import { extraerMensajeError, type BdpDefinition } from './bdp-explorador/constantes';
import { useConsultaBdp } from './bdp-explorador/useConsultaBdp';
import { useModalMenuLocal } from './bdp-explorador/useModalMenuLocal';

function BdpExplorador() {
  const queryClient = useQueryClient();
  /* [159A-3] El toggle demo vive en Configuración → BDP; aquí solo se lee. */
  const { demoMode } = useBdpDemoMode();
  const { tipo, setTipo, busqueda, setBusqueda, identificador, setIdentificador } = useConsultaBdp();
  const [buscado, setBuscado] = useState(false);
  const [seleccionado, setSeleccionado] = useState<BdpExplorerItem | null>(null);
  const { localModalOpen, localModalMenu, abrirNuevo, abrirEdicion, cerrarModal } = useModalMenuLocal();

  /* [128A-1/F7] Menús/packs locales: CRUD local, siempre disponible sin BDP. */
  const { data: menusLocales, isLoading: isLoadingMenus } = useBdpMenusLocales();
  const crearMutation = useCrearBdpMenuLocal(queryClient);
  const actualizarMutation = useActualizarBdpMenuLocal(queryClient);
  const eliminarMutation = useEliminarBdpMenuLocal(queryClient);

  const idNum = Number(identificador);
  const idValido = Number.isInteger(idNum) && idNum > 0;

  const menuQuery = useGetMenuDefinition(idNum, {
    query: { enabled: buscado && tipo === 'menu' && !demoMode },
  });
  const fastfoodQuery = useGetFastfoodDefinition(idNum, {
    query: { enabled: buscado && tipo === 'fastfood' && !demoMode },
  });
  const packQuery = useGetPackDefinition(idNum, {
    query: { enabled: buscado && tipo === 'pack' && !demoMode },
  });

  const isLoading = menuQuery.isLoading || fastfoodQuery.isLoading || packQuery.isLoading;
  const error = menuQuery.error || fastfoodQuery.error || packQuery.error;

  const items = useMemo(() => {
    if (!demoMode) return [];
    return mockExplorerItems.filter((item) => {
      const matchesTipo = tipo === 'all' || item.type === tipo;
      const q = busqueda.trim().toLowerCase();
      const matchesText =
        !q ||
        item.name.toLowerCase().includes(q) ||
        item.code.toLowerCase().includes(q) ||
        item.description.toLowerCase().includes(q);
      return matchesTipo && matchesText;
    });
  }, [demoMode, tipo, busqueda]);

  /* [287A-4] Cada tipo consulta exclusivamente su endpoint. Esto evita que
   * Fastfood y Pack disparen también GetMenu y muestren una respuesta ajena. */
  const resultadoReal = buscado
    ? tipo === 'menu'
      ? (menuQuery.data as { data?: BdpDefinition })?.data
      : tipo === 'fastfood'
        ? (fastfoodQuery.data as { data?: BdpDefinition })?.data
        : tipo === 'pack'
          ? (packQuery.data as { data?: BdpDefinition })?.data
          : null
    : null;

  function buscar() {
    if (!idValido) {
      toast.warning('Introduce un código numérico válido');
      return;
    }
    if (tipo === 'all') {
      toast.warning('Selecciona un tipo (Menú, Fastfood o Pack)');
      return;
    }
    setBuscado(true);
  }

  function handleMenuSubmit(req: CrearBdpMenuLocalRequest | ActualizarBdpMenuLocalRequest) {
    if (localModalMenu) {
      actualizarMutation.mutate(
        { id: localModalMenu.id, req },
        {
          onSuccess: () => {
            toast.success('Menú/pack actualizado');
            cerrarModal();
          },
          onError: () => toast.error('No se pudo actualizar el menú/pack'),
        },
      );
    } else {
      crearMutation.mutate(req as CrearBdpMenuLocalRequest, {
        onSuccess: () => {
          toast.success('Menú/pack creado');
          cerrarModal();
        },
        onError: () => toast.error('No se pudo crear el menú/pack'),
      });
    }
  }

  /* [267A-8] Confirmación con diálogo propio: `window.confirm` no funciona
   * en el navegador empaquetado. */
  async function handleEliminar(menu: BdpMenuLocalConLineas) {
    if (demoMode) {
      toast.info('En modo demo no se borran datos reales');
      return;
    }
    const confirmado = await confirmarConDialogo({
      titulo: `¿Eliminar «${menu.nombre}»?`,
      textoConfirmar: 'Eliminar',
    });
    if (!confirmado) return;
    eliminarMutation.mutate(menu.id, {
      onSuccess: () => toast.success('Menú/pack eliminado'),
      onError: () => toast.error('No se pudo eliminar el menú/pack'),
    });
  }

  return (
    <div className="flex flex-col gap-4">
      <Tabs defaultValue="menus">
        <TabsList>
          <TabsTrigger value="menus">Menús y packs</TabsTrigger>
          <TabsTrigger value="consultar">Consultar BDP</TabsTrigger>
        </TabsList>

        <TabsContent value="menus" className="flex flex-col gap-4">
          <MenusLocales
            menusLocales={menusLocales}
            cargando={isLoadingMenus}
            onNuevo={abrirNuevo}
            onEditar={abrirEdicion}
            onEliminar={handleEliminar}
          />
        </TabsContent>

        <TabsContent value="consultar" className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {demoMode ? `${items.length} definiciones` : 'Consulta una definición de BDP por código'}
        </p>
      </div>

      <BarraConsulta
        demoMode={demoMode}
        busqueda={busqueda}
        setBusqueda={setBusqueda}
        tipo={tipo}
        setTipo={setTipo}
        identificador={identificador}
        setIdentificador={setIdentificador}
        setBuscado={setBuscado}
        isLoading={isLoading}
        buscar={buscar}
      />

      {!demoMode && !buscado && (
        <p className="text-sm text-muted-foreground">
          Selecciona un tipo e introduce un código numérico para consultar BDP.
        </p>
      )}

      {error && !demoMode && <p className="text-sm text-destructive">{extraerMensajeError(error)}</p>}

      {!demoMode && buscado && !isLoading && !resultadoReal && (
        <p className="text-sm text-muted-foreground">
          No se encontró un {tipo} con código {identificador}.
        </p>
      )}

      {!demoMode && resultadoReal && (
        <ResultadoConsulta resultado={resultadoReal} tipo={tipo} />
      )}

      {demoMode && (
        <TablaDefinicionesDemo items={items} onVer={(item) => setSeleccionado(item)} />
      )}
        </TabsContent>
      </Tabs>

      <Dialog open={!!seleccionado} onOpenChange={(open) => !open && setSeleccionado(null)}>
        <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Detalle de definición BDP</DialogTitle>
            <DialogDescription>{seleccionado?.name}</DialogDescription>
          </DialogHeader>
          {seleccionado && <DefinitionDetail data={seleccionado} />}
        </DialogContent>
      </Dialog>

      <BdpMenuLocalModal
        open={localModalOpen}
        menu={localModalMenu}
        isSubmitting={crearMutation.isPending || actualizarMutation.isPending}
        onClose={cerrarModal}
        onSubmit={handleMenuSubmit}
      />
    </div>
  );
}

export default BdpExplorador;
