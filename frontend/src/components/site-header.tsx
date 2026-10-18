/* [263A-16] Header del sitio — SidebarTrigger + título dinámico por ruta
 * [283A-20] Añadida campana de notificaciones en tiempo real.
 * [237A-3] Indicador rápido de estado BDP en la barra superior.
 * Los menús del indicador BDP viven en bdp-menu-sin-conexion y bdp-menu-conectado. */

import { useMemo, useState } from "react"
import { useLocation } from "react-router-dom"
import { SidebarTrigger } from "@/components/ui/sidebar"
import { NotificationBell } from "@/componentes/NotificationBell"
import { useNotificaciones } from "@/hooks/useNotificaciones"
import {
  getObtenerConfiguracionQueryKey,
  getObtenerModoOperacionQueryKey,
  useObtenerConfiguracion,
  useObtenerModoOperacion,
} from "@/api/generated/configuracion/configuracion"
import { useFlujoModoBdp } from "@/componentes/flujoModoBdp"
import { useFlushBdpPush } from "@/api/bdp"
import { toast } from "sonner"
import axios from "@/api/axios-instance"
import { useQueryClient } from "@tanstack/react-query"
import { useConfiguracionSync } from "@/hooks/useConfiguracionSync"
import { useAuthStore } from "@/stores/authStore"
import { BdpMenuDegradado, BdpMenuDesconectado } from "./bdp-menu-sin-conexion"
import { BdpMenuConectado } from "./bdp-menu-conectado"

const titulos: Record<string, string> = {
  "/": "Dashboard",
  "/ventas": "Ventas",
  "/gastos": "Gastos",
  "/reservas": "Reservas",
  "/reservas/calendario": "Calendario",
  "/clientes": "Clientes",
  "/canales": "Canales de Reserva",
  "/reservas/no-shows": "No-Shows",
  "/plano-sala": "Plano de Sala",
  "/configuracion": "Configuración",
  "/marketing/campanas": "Campañas de Marketing",
  "/marketing/campanas/nueva": "Nueva Campaña",
  "/marketing/plantillas": "Plantillas WhatsApp",
  "/marketing/plantillas/nueva": "Nueva Plantilla",
  "/marketing/recordatorios": "Recordatorios",
  "/bdp/stock": "Stock",
  "/bdp/explorador": "Menús y Packs",
  "/bdp/historial": "Historial",
  "/bdp/compras": "Compras",
  "/bdp/catalogo": "Catálogo",
  "/bdp/inventario": "Inventario",
  "/bdp/sincronizacion": "Sincronización",
}

function BdpStatusIndicator() {
  const { data: config } = useObtenerConfiguracion()
  /* [149A-3/F3] El trabajador no puede cambiar el modo BDP (PATCH exige Admin):
   * la opción se muestra deshabilitada con aviso, no se oculta. */
  const esTrabajador = useAuthStore((s) => s.esTrabajador)()
  /* [198A-2] Memoizar serverData (mismo bug [BKP-008c] ya corregido en
   * useConfiguracion.ts): el literal { status, data } creaba una referencia
   * nueva por render, lo que disparaba el setState de useConfiguracionSync y
   * producía un bucle infinito "Maximum update depth exceeded". */
  const serverData = useMemo(
    () =>
      config
        ? { status: config.status, data: config.data as unknown as Record<string, string | number | boolean> }
        : undefined,
    [config],
  )
  const { config: configSync } = useConfiguracionSync(serverData)
  const { volverSoloLectura, isPending: isChangingMode } = useFlujoModoBdp()
  /* [198A-1/F1] Flush manual de la cola de push (botón "Exportar a BDP").
   * Requerido por D1 (botón manual siempre) y D2 (reintento tras suscripción
   * solo manual). El backend exige rol Admin; aquí se muestra el resultado.
   * [159A-2/F2] Vocabulario Exportar: envía la cola local→BDP. */
  const { mutate: flushPush, isPending: isFlushing } = useFlushBdpPush()
  const queryClient = useQueryClient()
  /* [149A-1/P1.2b] En "BDP: lectura" no había forma de volver atrás desde aquí: el menú
   * solo ofrecía activar escritura temporal, sincronizar y entrar a Configuración. "Desactivar
   * BDP" es el simétrico de "Activar BDP" (que existe en el menú de la integración apagada).
   * El estado va aquí arriba, con el resto de hooks: declararlo después del `if (!cfg)`
   * rompía el orden de hooks ("Rendered more hooks than during the previous render"). */
  const [desactivandoBdp, setDesactivandoBdp] = useState(false)
  /* [149A-1/P1.3] El modo efectivo lo calcula el servidor (incluye la histéresis M2: 3 fallos
   * consecutivos hacia BDP degradan a local). La cabecera solo puede saberlo preguntándolo:
   * derivarlo por su cuenta era lo que hacía que mintiera durante una degradación. */
  const { data: modoServidor } = useObtenerModoOperacion()
  const cfg = config?.status === 200 ? (config.data as unknown as Record<string, unknown>) : null
  if (!cfg) return null

  const syncEnabled = Boolean(cfg?.bdp_sync_enabled ?? configSync?.bdp_sync_enabled)
  const syncMode = String(cfg?.bdp_sync_mode ?? configSync?.bdp_sync_mode ?? 'read_only')
  const modoOperacion = String(cfg?.modo_operacion ?? configSync?.modo_operacion ?? 'auto')

  /* [H-S2-01] El backend redacta bdp_login/password/integrator_code del
   * payload; derivar credencialesOk de esos campos era siempre false tras
   * recargar (badge mentía "BDP: off" con la integración activa). El flag
   * bdp_configurado es la verdad del servidor sin exponer secretos. */
  const credencialesOk = Boolean(cfg?.bdp_configurado ?? configSync?.bdp_configurado ?? false)

  /* [128A-1/F1/M1] 'standalone' es el switch maestro: aunque bdp_sync_enabled
   * siga activo por compatibilidad, se trata como inactivo y el badge muestra
   * el modo independiente. */
  const modoIndependiente = modoOperacion === 'standalone'
  /* [128A-1/F1-5] Misma lógica que el backend (modo_efectivo_desde_config):
   * 'bdp' fuerza modo BDP aunque bdp_sync_enabled esté a false; 'auto' es BDP
   * solo si sync activo y credenciales configuradas. */
  const modoEfectivoBdp =
    modoOperacion === 'bdp' ||
    (modoOperacion === 'auto' && syncEnabled && credencialesOk)

  /* [149A-1/P1.3] Degradado = el usuario pidió BDP y el servidor ya opera en local porque el
   * BDP no respondió a los últimos intentos. Sin esto la cabecera prometía "BDP: lectura"
   * mientras el backend rechazaba toda llamada al BDP. */
  const degradado =
    modoEfectivoBdp && modoServidor?.status === 200 && modoServidor.data.modo_efectivo === 'standalone'

  if (degradado) return <BdpMenuDegradado />

  if (modoIndependiente || !modoEfectivoBdp) {
    return <BdpMenuDesconectado modoIndependiente={modoIndependiente} credencialesOk={credencialesOk} />
  }

  const isWrite = syncMode === 'unidirectional'
  const isAuto = syncMode === 'automatic'
  /* Mientras hay un cambio de modo en vuelo el punto pasa a ámbar pulsante:
   * estado intermedio honesto en vez de afirmar un modo que aún no aplicó. */
  const transitorio = isChangingMode || desactivandoBdp

  async function desactivarIntegracion() {
    if (desactivandoBdp) return
    setDesactivandoBdp(true)
    try {
      await axios.patch('/api/configuracion', { bdp_sync_enabled: false })
      await queryClient.invalidateQueries({
        queryKey: getObtenerConfiguracionQueryKey(),
      })
      await queryClient.invalidateQueries({ queryKey: getObtenerModoOperacionQueryKey() })
      toast.success('BDP desactivado', {
        description: 'La aplicación sigue con datos locales; ya no se consulta el BDP.',
      })
    } catch (err: unknown) {
      toast.error('No se pudo desactivar BDP', {
        description: String((err as { message?: string })?.message ?? 'Error desconocido'),
      })
    } finally {
      setDesactivandoBdp(false)
    }
  }

  /* [189A-1] Vuelta a Solo lectura con el flujo compartido (pide
   * confirmación: es el freno consciente que cierra cualquier escritura). */
  function desactivarEscritura() {
    if (isChangingMode) return
    void volverSoloLectura()
  }

  return (
    <BdpMenuConectado
      isWrite={isWrite}
      isAuto={isAuto}
      transitorio={transitorio}
      isChangingMode={isChangingMode}
      desactivandoBdp={desactivandoBdp}
      esTrabajador={esTrabajador}
      isFlushing={isFlushing}
      flushPush={flushPush}
      desactivarEscritura={desactivarEscritura}
      desactivarIntegracion={desactivarIntegracion}
    />
  )
}

export function SiteHeader() {
  const location = useLocation()
  const titulo = titulos[location.pathname] || "Restaurante"

  /* [283A-20] Conectar SSE de notificaciones al montar el header */
  useNotificaciones()

  return (
    <header className="flex h-(--header-height) shrink-0 items-center gap-2 border-b transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-(--header-height)">
      <div className="flex w-full items-center gap-1 px-4 lg:gap-2 lg:px-6">
        <SidebarTrigger className="-ml-1" />
        {/* [149A-1/P1.1] La rayita divisoria del encabezado se dibuja con un div y no con
            <Separator orientation="vertical">: el primitivo aplica self-stretch, y con un
            alto fijo (h-4) eso la pega al borde superior en vez de centrarla en la fila.
            El padre ya centra con items-center, así que aquí basta el alto explícito. */}
        <div aria-hidden className="mx-2 h-4 w-px shrink-0 bg-border" />
        <h1 className="text-base font-medium">{titulo}</h1>
        <div className="ml-auto flex items-center gap-2">
          <BdpStatusIndicator />
          <NotificationBell />
        </div>
      </div>
    </header>
  )
}
