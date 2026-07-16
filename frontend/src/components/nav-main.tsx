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

export function NavMain({
  items,
}: {
  items: {
    title: string
    url: string
    icon?: React.ReactNode
  }[]
}) {
  const location = useLocation()
  const [modalVenta, setModalVenta] = useState(false)
  const [modalGasto, setModalGasto] = useState(false)

  return (
    <SidebarGroup>
      <SidebarGroupContent className="flex flex-col gap-2">
        <SidebarMenu>
          <SidebarMenuItem className="flex items-center gap-2">
            <SidebarMenuButton
              tooltip="Nueva Venta"
              className="min-w-8 bg-primary text-primary-foreground duration-200 ease-linear hover:bg-primary/90 hover:text-primary-foreground active:bg-primary/90 active:text-primary-foreground"
              onClick={() => setModalVenta(true)}
            >
              <CirclePlusIcon />
              <span>Venta</span>
            </SidebarMenuButton>
            <SidebarMenuButton
              tooltip="Nuevo Gasto"
              className="min-w-8 bg-secondary text-secondary-foreground duration-200 ease-linear hover:bg-secondary/80 group-data-[collapsible=icon]:opacity-0"
              onClick={() => setModalGasto(true)}
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

            return (
              <SidebarMenuItem key={item.title}>
                <SidebarMenuButton asChild tooltip={item.title} isActive={activo}>
                  <Link to={item.url}>
                    {item.icon}
                    <span>{item.title}</span>
                  </Link>
                </SidebarMenuButton>
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
