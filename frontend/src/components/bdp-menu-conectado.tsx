import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { useNavigate } from "react-router-dom"
import { toast } from "sonner"
import type { useFlushBdpPush } from "@/api/bdp"
import { AVISO_SOLO_PROPIETARIO } from "@/components/nav-main"
import { PuntoConexion } from "./bdp-punto-conexion"

interface Props {
  isWrite: boolean
  isAuto: boolean
  transitorio: boolean
  isChangingMode: boolean
  desactivandoBdp: boolean
  esTrabajador: boolean
  isFlushing: boolean
  flushPush: ReturnType<typeof useFlushBdpPush>['mutate']
  desactivarEscritura: () => void
  desactivarIntegracion: () => Promise<void>
}

/* Menú del badge con BDP conectado (modo lectura, automático o escritura). */
export function BdpMenuConectado({
  isWrite, isAuto, transitorio, isChangingMode, desactivandoBdp, esTrabajador,
  isFlushing, flushPush, desactivarEscritura, desactivarIntegracion,
}: Props) {
  const navigate = useNavigate()
  const etiquetaModo = isWrite ? 'Escritura' : isAuto ? 'Automático' : 'Solo lectura'

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button type="button" className="focus:outline-none" disabled={isChangingMode}>
          {isWrite ? (
            <Badge
              variant="default"
              className="h-auto gap-1.5 px-2.5 py-1 text-xs bg-amber-600 cursor-pointer hover:bg-amber-700">
              <PuntoConexion estado={transitorio ? 'intermedio' : 'conectado'} />
              BDP Conectado · Escritura
            </Badge>
          ) : isAuto ? (
            <Badge
              variant="default"
              className="h-auto gap-1.5 px-2.5 py-1 text-xs bg-emerald-600 cursor-pointer hover:bg-emerald-700">
              <PuntoConexion estado={transitorio ? 'intermedio' : 'conectado'} />
              BDP Conectado · Automático
            </Badge>
          ) : (
            <Badge
              variant="secondary"
              className="h-auto gap-1.5 px-2.5 py-1 text-xs cursor-pointer hover:bg-secondary/80">
              <PuntoConexion estado={transitorio ? 'intermedio' : 'conectado'} />
              BDP Conectado · Solo lectura
            </Badge>
          )}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <div className="px-2 py-1.5 text-sm font-medium">
          BDP Conectado · {etiquetaModo}
        </div>
        <p className="px-2 pb-1.5 text-xs text-muted-foreground">
          {isWrite
            ? 'Permiso temporal de escritura Aplicación Web → BDP. Tras operar, se vuelve solo a lectura sin pasos manuales.'
            : isAuto
              ? 'Las escrituras se envían sin confirmación por operación. Vuelve a Solo lectura cuando quieras.'
              : 'Modo seguro: consultas e importaciones de BDP activas, sin escrituras.'}
        </p>
        <DropdownMenuSeparator />
        {isWrite || isAuto ? (
          <DropdownMenuItem onClick={desactivarEscritura} disabled={isChangingMode}>
            {isChangingMode ? 'Cambiando...' : isAuto ? 'Desactivar modo automático' : 'Desactivar escritura'}
          </DropdownMenuItem>
        ) : (
          <>
            {/* [149A-3/F3] Cierra TODO [C2-3]: el rol ya lo expone el auth store.
             * Trabajador ve la opción deshabilitada con aviso (backend 403). */}
            <DropdownMenuItem
              onClick={() => navigate('/configuracion', { state: { bdpArming: true } })}
              disabled={esTrabajador}
              title={esTrabajador ? AVISO_SOLO_PROPIETARIO : undefined}
            >
              {esTrabajador ? 'Activar escritura temporal (solo propietario)' : 'Activar escritura temporal'}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => navigate('/configuracion', { state: { bdpSection: 'bdp' } })}
              disabled={esTrabajador}
              title={esTrabajador ? AVISO_SOLO_PROPIETARIO : undefined}
            >
              {esTrabajador ? 'Activar modo automático (solo propietario)' : 'Activar modo automático'}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={desactivarIntegracion}
              disabled={desactivandoBdp || esTrabajador}
              title={esTrabajador ? AVISO_SOLO_PROPIETARIO : undefined}
            >
              {desactivandoBdp ? 'Desactivando...' : esTrabajador ? 'Desactivar BDP (solo propietario)' : 'Desactivar BDP (quedar solo en local)'}
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
