/* [263A-16] Navegación principal del sidebar.
 * Usa react-router-dom Link para SPA navigation. Incluye botones de acción rápida
 * "Nueva Venta" y "Nuevo Gasto" como pide el roadmap (sección 3).
 * [283A-10] Los botones de acción abren modales en vez de navegar. */

import { useState } from "react"
import { Link, useLocation } from "react-router-dom"
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { CirclePlusIcon, ReceiptIcon } from "lucide-react"
import FormularioVenta from "@/componentes/FormularioVenta"
import FormularioGasto from "@/componentes/FormularioGasto"
import { useAuthStore } from "@/stores/authStore"

/* [149A-3/F3] Entradas con soloAdmin quedan deshabilitadas con aviso para
 * sesiones de trabajador (se muestran, no se ocultan). La autorización real
 * la aplica el backend (403); esto es UI honesta, no control de acceso.
 * [169A-3] Añadido filtro por sección concedida (`seccion`): el trabajador
 * solo tiene habilitadas sus secciones firmadas en el JWT (`secs`). Las
 * `soloAdmin` (dueño por rol en backend) quedan deshabilitadas aunque la
 * sección aparezca concedida. */
export const AVISO_SECCION =
  "No tienes permiso para esta sección (tu sesión es de trabajador; pídele al propietario que te la habilite)"

/* Compat: el aviso anterior de solo-propietario ahora es este unificado. */
export const AVISO_SOLO_PROPIETARIO = AVISO_SECCION

export function NavMain({
  items,
}: {
  items: {
    title: string
    url: string
    icon?: React.ReactNode
    soloAdmin?: boolean
    seccion?: string
  }[]
}) {
  const location = useLocation()
  const esTrabajador = useAuthStore((s) => s.esTrabajador)()
  const tieneSeccion = useAuthStore((s) => s.tieneSeccion)
  const [modalVenta, setModalVenta] = useState(false)
  const [modalGasto, setModalGasto] = useState(false)
  /* [169A-3] Acciones rápidas también respetan la sección (abren modales de
   * escritura: sin "ventas"/"gastos" no tienen sentido para el trabajador). */
  const puedeVender = tieneSeccion("ventas")
  const puedeGastar = tieneSeccion("gastos")

  return (
    <SidebarGroup>
      {/* [208A-2] Botones del menú más compactos: size sm (h-7, text-xs) e
       * iconos 3.5 para que quepa todo sin scroll excesivo. */}
      <SidebarGroupContent className="flex flex-col gap-1.5">
        <SidebarMenu>
          <SidebarMenuItem className="flex items-center gap-1.5">
            <SidebarMenuButton
              tooltip={puedeVender ? "Nueva Venta" : AVISO_SECCION}
              size="sm"
              className="min-w-8 bg-primary text-primary-foreground duration-200 ease-linear hover:bg-primary/90 hover:text-primary-foreground active:bg-primary/90 active:text-primary-foreground disabled:opacity-50 disabled:cursor-not-allowed"
              onClick={() => setModalVenta(true)}
              disabled={!puedeVender}
              aria-disabled={!puedeVender}
            >
              <CirclePlusIcon />
              <span>Venta</span>
            </SidebarMenuButton>
            <SidebarMenuButton
              tooltip={puedeGastar ? "Nuevo Gasto" : AVISO_SECCION}
              size="sm"
              className="min-w-8 bg-secondary text-secondary-foreground duration-200 ease-linear hover:bg-secondary/80 group-data-[collapsible=icon]:opacity-0 disabled:opacity-50 disabled:cursor-not-allowed"
              onClick={() => setModalGasto(true)}
              disabled={!puedeGastar}
              aria-disabled={!puedeGastar}
            >
              <ReceiptIcon />
              <span>Gasto</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <SidebarMenu>
          {items.map((item) => {
            const activo = item.url === "/"
              ? location.pathname === "/"
              : location.pathname.startsWith(item.url)
            const deshabilitado = esTrabajador
              && (Boolean(item.soloAdmin)
                || (typeof item.seccion === "string" && !tieneSeccion(item.seccion)))

            return (
              <SidebarMenuItem key={item.title}>
                {deshabilitado ? (
                  <SidebarMenuButton
                    size="sm"
                    tooltip={AVISO_SECCION}
                    aria-disabled="true"
                    disabled
                    title={AVISO_SECCION}
                    className="[&_svg]:size-3.5 opacity-50 cursor-not-allowed"
                  >
                    {item.icon}
                    <span>{item.title}</span>
                  </SidebarMenuButton>
                ) : (
                  <SidebarMenuButton
                    asChild
                    size="sm"
                    tooltip={item.title}
                    isActive={activo}
                    className="[&_svg]:size-3.5"
                  >
                    <Link to={item.url}>
                      {item.icon}
                      <span>{item.title}</span>
                    </Link>
                  </SidebarMenuButton>
                )}
              </SidebarMenuItem>
            )
          })}
        </SidebarMenu>
      </SidebarGroupContent>

      <Dialog open={modalVenta} onOpenChange={setModalVenta}>
        <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Nueva Venta</DialogTitle>
          </DialogHeader>
          <FormularioVenta onExito={() => setModalVenta(false)} />
        </DialogContent>
      </Dialog>

      <Dialog open={modalGasto} onOpenChange={setModalGasto}>
        {/* [283A-19] Modal gasto ampliado a sm:max-w-4xl (tareas 9+16+17).
         * DialogContent base usa sm:max-w-sm, hay que sobreescribir con el mismo
         * breakpoint sm: para que twMerge resuelva correctamente. */}
        <DialogContent className="sm:max-w-4xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Nuevo Gasto</DialogTitle>
          </DialogHeader>
          <FormularioGasto onExito={() => setModalGasto(false)} />
        </DialogContent>
      </Dialog>
    </SidebarGroup>
  )
}
