import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useNavigate } from "react-router-dom"
import { useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import axios from "@/api/axios-instance"
import {
  getObtenerConfiguracionQueryKey,
  getObtenerModoOperacionQueryKey,
} from "@/api/generated/configuracion/configuracion"
import { PuntoConexion } from "./bdp-punto-conexion"

/* Menú del badge cuando BDP no responde y el servidor ya opera en local. */
export function BdpMenuDegradado() {
  const navigate = useNavigate()
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button type="button" className="focus:outline-none">
          <Badge
            variant="outline"
            className="h-auto gap-1.5 border-amber-600 px-2.5 py-1 text-xs text-amber-600 cursor-pointer hover:bg-muted">
            <PuntoConexion estado="intermedio" />
            BDP Sin respuesta
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

interface PropsDesconectado {
  modoIndependiente: boolean
  credencialesOk: boolean
}

/* Menú del badge con la integración apagada (o modo independiente). */
export function BdpMenuDesconectado({ modoIndependiente, credencialesOk }: PropsDesconectado) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button type="button" className="focus:outline-none">
          <Badge
            variant="outline"
            className="h-auto gap-1.5 px-2.5 py-1 text-xs cursor-pointer hover:bg-muted">
            <PuntoConexion estado="desconectado" />
            {modoIndependiente ? 'BDP Desconectado · Solo local' : 'BDP Desconectado'}
          </Badge>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <div className="px-2 py-1.5 text-sm font-medium">
          {modoIndependiente
            ? 'BDP Desconectado · Solo local'
            : 'BDP Desconectado'}
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
