/* [263A-16] Header del sitio — SidebarTrigger + título dinámico por ruta
 * [283A-20] Añadida campana de notificaciones en tiempo real.
 * [237A-3] Indicador rápido de estado BDP en la barra superior. */

import { useMemo, useState } from "react"
import { useLocation, useNavigate } from "react-router-dom"
import { SidebarTrigger } from "@/components/ui/sidebar"
import { NotificationBell } from "@/componentes/NotificationBell"
import { useNotificaciones } from "@/hooks/useNotificaciones"
import {
  getObtenerConfiguracionQueryKey,
  getObtenerModoOperacionQueryKey,
  useObtenerConfiguracion,
  useObtenerModoOperacion,
} from "@/api/generated/configuracion/configuracion"
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
  /* [198A-1/F1] Flush manual de la cola de push (botón "Exportar a BDP").
   * Requerido por D1 (botón manual siempre) y D2 (reintento tras suscripción
   * solo manual). El backend exige rol Admin; aquí se muestra el resultado.
   * [159A-2/F2] Vocabulario Exportar: envía la cola local→BDP. */
  const { mutate: flushPush, isPending: isFlushing } = useFlushBdpPush()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
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

  if (degradado) {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button type="button" className="focus:outline-none">
            <Badge
              variant="outline"
              className="h-auto gap-1 border-amber-600 px-2.5 py-1 text-xs text-amber-600 cursor-pointer hover:bg-muted">
              BDP: sin respuesta
            </Badge>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-72">
          <div className="px-2 py-1.5 text-sm font-medium">BDP sin respuesta — se sigue en local</div>
          <p className="px-2 pb-1.5 text-xs text-muted-foreground">
            Tras varios intentos fallidos, la aplicación dejó de llamar al BDP y siguió trabajando con
            datos locales. No se pierde nada: los cambios pendientes quedan en la cola de
            sincronización. El BDP se retoma solo cuando vuelva a responder.
          </p>
          <DropdownMenuSeparator />
          <DropdownMenuItem onClick={() => navigate('/bdp/sincronizacion')}>
            Ver cola de sincronización
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => navigate('/bdp/historial')}>
            Ver historial BDP
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => navigate('/configuracion', { state: { bdpSection: 'bdp' } })}>
            Configuración BDP
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    )
  }

  if (modoIndependiente || !modoEfectivoBdp) {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button type="button" className="focus:outline-none">
            <Badge
              variant="outline"
              className="h-auto gap-1 px-2.5 py-1 text-xs cursor-pointer hover:bg-muted">
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
                await queryClient.invalidateQueries({
                  queryKey: getObtenerConfiguracionQueryKey(),
                })
                await queryClient.invalidateQueries({ queryKey: getObtenerModoOperacionQueryKey() })
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
                await queryClient.invalidateQueries({
                  queryKey: getObtenerConfiguracionQueryKey(),
                })
                await queryClient.invalidateQueries({ queryKey: getObtenerModoOperacionQueryKey() })
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
            <Badge
              variant="default"
              className="h-auto gap-1 px-2.5 py-1 text-xs bg-amber-600 cursor-pointer hover:bg-amber-700">
              BDP: escritura
            </Badge>
          ) : (
            <Badge
              variant="secondary"
              className="h-auto gap-1 px-2.5 py-1 text-xs cursor-pointer hover:bg-secondary/80">
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
          <>
            {/* [C2-3] TODO: restringir a admin/owner cuando el auth store exponga rol. */}
            <DropdownMenuItem onClick={() => navigate('/configuracion', { state: { bdpArming: true } })}>
              Activar escritura temporal
            </DropdownMenuItem>
            <DropdownMenuItem onClick={desactivarIntegracion} disabled={desactivandoBdp}>
              {desactivandoBdp ? 'Desactivando...' : 'Desactivar BDP (quedar solo en local)'}
            </DropdownMenuItem>
          </>
        )}
        <DropdownMenuItem
          disabled={isFlushing}
          onClick={() =>
            flushPush(undefined, {
              onSuccess: (r) => {
                if (r.sincronizados > 0) {
                  toast.success('Exportado al BDP', {
                    description: `${r.sincronizados} operación(es) enviada(s).`,
                  })
                } else if (r.pendientes_suscripcion > 0) {
                  toast.warning('Pendiente de suscripción BDP', {
                    description: `${r.pendientes_suscripcion} operación(es) no se enviaron: suscripción no activada.`,
                  })
                } else if (r.rechazados > 0) {
                  toast.error('Operación(es) rechazadas por BDP', {
                    description: `${r.rechazados} operación(es) con payload inválido o conflicto: corregir el dato local y reintentar manualmente.`,
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
                toast.error('No se pudo exportar a BDP', {
                  description: String((err as { message?: string })?.message ?? 'Error desconocido'),
                }),
            })
          }
        >
          {isFlushing ? 'Exportando...' : 'Exportar a BDP'}
        </DropdownMenuItem>
        {/* [149A-1/P1.2] Apuntaba a '/configuracion/bdp-backup', ruta inexistente → el click no
            hacía nada. El historial real (auditoría + snapshots) vive en '/bdp/historial'. */}
        <DropdownMenuItem onClick={() => navigate('/bdp/historial')}>
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
