/* [263A-16] Header del sitio — SidebarTrigger + título dinámico por ruta
 * [283A-20] Añadida campana de notificaciones en tiempo real.
 * [237A-3] Indicador rápido de estado BDP en la barra superior. */

import { useMemo } from "react"
import { useLocation, useNavigate } from "react-router-dom"
import { Separator } from "@/components/ui/separator"
import { SidebarTrigger } from "@/components/ui/sidebar"
import { NotificationBell } from "@/componentes/NotificationBell"
import { useNotificaciones } from "@/hooks/useNotificaciones"
import { useObtenerConfiguracion } from "@/api/generated/configuracion/configuracion"
import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useSetSyncMode } from "@/api/bdp-backup"
import { useFlushBdpPush } from "@/api/bdp"
import { toast } from "sonner"
import axios from "@/api/axios-instance"
import { useQueryClient } from "@tanstack/react-query"
import { useConfiguracionSync } from "@/hooks/useConfiguracionSync"

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
  const { mutate: setSyncMode, isPending: isChangingMode } = useSetSyncMode()
  /* [198A-1/F1] Flush manual de la cola de push (botón "Sincronizar a BDP").
   * Requerido por D1 (botón manual siempre) y D2 (reintento tras suscripción
   * solo manual). El backend exige rol Admin; aquí se muestra el resultado. */
  const { mutate: flushPush, isPending: isFlushing } = useFlushBdpPush()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const cfg = config?.status === 200 ? (config.data as unknown as Record<string, unknown>) : null
  if (!cfg) return null

  const syncEnabled = Boolean(cfg?.bdp_sync_enabled ?? configSync?.bdp_sync_enabled)
  const syncMode = String(cfg?.bdp_sync_mode ?? configSync?.bdp_sync_mode ?? 'read_only')
  const modoOperacion = String(cfg?.modo_operacion ?? configSync?.modo_operacion ?? 'auto')

  const credencialesOk =
    Boolean(cfg?.bdp_base_url ?? configSync?.bdp_base_url) &&
    Boolean(cfg?.bdp_login ?? configSync?.bdp_login) &&
    Boolean(cfg?.bdp_password ?? configSync?.bdp_password) &&
    Boolean(cfg?.bdp_integrator_code ?? configSync?.bdp_integrator_code)

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

  if (modoIndependiente || !modoEfectivoBdp) {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button type="button" className="focus:outline-none">
            <Badge variant="outline" className="text-xs gap-1 cursor-pointer hover:bg-muted">
              {modoIndependiente ? 'Modo independiente' : 'BDP: off'}
            </Badge>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-64">
          <div className="px-2 py-1.5 text-sm font-medium">
            {modoIndependiente
              ? 'Modo independiente (sin BDP)'
              : 'Integración BDP desactivada'}
          </div>
          <p className="px-2 pb-1.5 text-xs text-muted-foreground">
            {modoIndependiente
              ? 'Todas las operaciones del restaurante funcionan con datos locales; el BDP no se usa.'
              : 'Los datos y columnas de BDP no se muestran en la Aplicación Web hasta que se active la integración.'}
          </p>
          <DropdownMenuSeparator />
          {modoIndependiente ? (
            <DropdownMenuItem onClick={async () => {
              try {
                await axios.patch('/api/configuracion/modo', { modo: 'auto' })
                await queryClient.invalidateQueries({ queryKey: ['configuracion'] })
                toast.success('Modo automático activado', {
                  description: 'El sistema usará BDP si está configurado y disponible.',
                })
              } catch {
                toast.error('No se pudo cambiar el modo')
              }
            }}>
              Volver a modo automático
            </DropdownMenuItem>
          ) : credencialesOk ? (
            <DropdownMenuItem onClick={async () => {
              try {
                await axios.patch('/api/configuracion', { bdp_sync_enabled: true })
                await queryClient.invalidateQueries({ queryKey: ['configuracion'] })
                toast.success('BDP activado', { description: 'La integración BDP está ahora en modo lectura.' })
              } catch {
                toast.error('No se pudo activar BDP')
              }
            }}>
              Activar BDP
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem disabled>
              Sin credenciales — configura BDP primero
            </DropdownMenuItem>
          )}
          <DropdownMenuItem onClick={() => navigate('/configuracion', { state: { bdpSection: 'bdp' } })}>
            {modoIndependiente
              ? 'Configuración'
              : credencialesOk
                ? 'Configuración BDP'
                : 'Configurar credenciales BDP'}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    )
  }

  const isWrite = syncMode === 'unidirectional'
  const bdpBaseUrl = String(cfg?.bdp_base_url ?? configSync?.bdp_base_url ?? '')

  function desactivarEscritura() {
    if (isChangingMode) return
    const baseUrl = bdpBaseUrl
    setSyncMode(
      {
        modo: 'read_only',
        confirmarDestino: baseUrl,
        alcances: [],
        duracionMinutos: 0,
        maxOperaciones: 0,
        motivo: '',
        targetEntityType: '',
        targetEntityId: '',
      },
      {
        onSuccess: () => toast.success('BDP vuelve a modo solo lectura'),
        onError: (err: unknown) =>
          toast.error('No se pudo cambiar el modo BDP', {
            description: String((err as { message?: string })?.message ?? 'Error desconocido'),
          }),
      }
    )
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button type="button" className="focus:outline-none" disabled={isChangingMode}>
          {isWrite ? (
            <Badge variant="default" className="text-xs gap-1 bg-amber-600 cursor-pointer hover:bg-amber-700">
              BDP: escritura
            </Badge>
          ) : (
            <Badge variant="secondary" className="text-xs gap-1 cursor-pointer hover:bg-secondary/80">
              BDP: lectura
            </Badge>
          )}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <div className="px-2 py-1.5 text-sm font-medium">
          Estado BDP: {isWrite ? 'Escritura temporal' : 'Solo lectura'}
        </div>
        <p className="px-2 pb-1.5 text-xs text-muted-foreground">
          {isWrite
            ? 'Permiso temporal de escritura Aplicación Web → BDP. Tras operar, se vuelve solo a lectura sin pasos manuales.'
            : 'Modo seguro: consultas e importaciones de BDP activas, sin escrituras.'}
        </p>
        <DropdownMenuSeparator />
        {isWrite ? (
          <DropdownMenuItem onClick={desactivarEscritura} disabled={isChangingMode}>
            {isChangingMode ? 'Cambiando...' : 'Desactivar escritura'}
          </DropdownMenuItem>
        ) : (
          /* [C2-3] TODO: restringir a admin/owner cuando el auth store exponga rol. */
          <DropdownMenuItem onClick={() => navigate('/configuracion', { state: { bdpArming: true } })}>
            Activar escritura temporal
          </DropdownMenuItem>
        )}
        <DropdownMenuItem
          disabled={isFlushing}
          onClick={() =>
            flushPush(undefined, {
              onSuccess: (r) => {
                if (r.sincronizados > 0) {
                  toast.success('Sincronizado con BDP', {
                    description: `${r.sincronizados} operación(es) enviada(s).`,
                  })
                } else if (r.pendientes_suscripcion > 0) {
                  toast.warning('Pendiente de suscripción BDP', {
                    description: `${r.pendientes_suscripcion} operación(es) no se enviaron: suscripción no activada.`,
                  })
                } else if (r.errores > 0) {
                  toast.error('Errores al sincronizar', {
                    description: `${r.errores} operación(es) fallaron.`,
                  })
                } else {
                  toast.info('Sin operaciones pendientes', {
                    description: 'No hay cambios locales pendientes de enviar a BDP.',
                  })
                }
              },
              onError: (err: unknown) =>
                toast.error('No se pudo sincronizar con BDP', {
                  description: String((err as { message?: string })?.message ?? 'Error desconocido'),
                }),
            })
          }
        >
          {isFlushing ? 'Sincronizando...' : 'Sincronizar a BDP'}
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => navigate('/configuracion/bdp-backup')}>
          Ver historial BDP
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => navigate('/configuracion', { state: { bdpSection: 'bdp' } })}>
          Configuración BDP
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
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
