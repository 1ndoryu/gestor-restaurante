/* [263A-16] Header del sitio — SidebarTrigger + título dinámico por ruta
 * [283A-20] Añadida campana de notificaciones en tiempo real.
 * [237A-3] Indicador rápido de estado BDP en la barra superior. */

import { useLocation } from "react-router-dom"
import { Separator } from "@/components/ui/separator"
import { SidebarTrigger } from "@/components/ui/sidebar"
import { NotificationBell } from "@/componentes/NotificationBell"
import { useNotificaciones } from "@/hooks/useNotificaciones"
import { useObtenerConfiguracion } from "@/api/generated/configuracion/configuracion"
import { Badge } from "@/components/ui/badge"

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
}

function BdpStatusIndicator() {
  const { data: config } = useObtenerConfiguracion()
  const cfg = config?.status === 200 ? (config.data as unknown as Record<string, unknown>) : null
  if (!cfg) return null

  const syncEnabled = Boolean(cfg.bdp_sync_enabled)
  const syncMode = String(cfg.bdp_sync_mode ?? 'read_only')

  if (!syncEnabled) {
    return (
      <Badge variant="outline" className="text-xs gap-1">
        BDP: off
      </Badge>
    )
  }

  if (syncMode === 'unidirectional') {
    return (
      <Badge variant="default" className="text-xs gap-1 bg-amber-600">
        BDP: escritura
      </Badge>
    )
  }

  return (
    <Badge variant="secondary" className="text-xs gap-1">
      BDP: lectura
    </Badge>
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
        <Separator
          orientation="vertical"
          className="mx-2 data-[orientation=vertical]:h-4"
        />
        <h1 className="text-base font-medium">{titulo}</h1>
        <div className="ml-auto flex items-center gap-2">
          <BdpStatusIndicator />
          <NotificationBell />
        </div>
      </div>
    </header>
  )
}
