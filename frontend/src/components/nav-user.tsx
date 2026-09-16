/* [263A-16] Usuario en el footer del sidebar.
 * Conectado a authStore para cerrar sesión. ThemeToggle integrado.
 * Se elimina avatar/account/billing/notifications — no aplican al restaurante. */

import { useNavigate } from "react-router-dom"
import { useAuthStore } from "@/stores/authStore"
import { ThemeToggle } from "@/components/theme-toggle"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar"
import { EllipsisVerticalIcon, LogOutIcon, UserIcon } from "lucide-react"

export function NavUser() {
  const { isMobile } = useSidebar()
  const cerrarSesion = useAuthStore((s) => s.cerrarSesion)
  /* [169A-3/N1] Nombre real de la sesión (trabajador) en vez del literal fijo.
   * El dueño no tiene nombre en BD: mantiene "Restaurante". */
  const nombreUsuario = useAuthStore((s) => s.nombreUsuario)
  const esTrabajador = useAuthStore((s) => s.esTrabajador)
  const navigate = useNavigate()

  const salir = () => {
    cerrarSesion()
    navigate("/login")
  }

  const nombreMostrado = nombreUsuario ?? (esTrabajador() ? "Trabajador" : "Restaurante")

  return (
    <SidebarMenu>
      <SidebarMenuItem className="flex items-center gap-1">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size="lg"
              className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
            >
              <div className="flex size-8 items-center justify-center rounded-lg bg-sidebar-accent">
                <UserIcon className="size-4" />
              </div>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-medium">{nombreMostrado}</span>
                <span className="truncate text-xs text-muted-foreground">
                  Panel de gestión
                </span>
              </div>
              <EllipsisVerticalIcon className="ml-auto size-4" />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className="w-(--radix-dropdown-menu-trigger-width) min-w-56 rounded-lg"
            side={isMobile ? "bottom" : "right"}
            align="end"
            sideOffset={4}
          >
            <DropdownMenuLabel className="p-0 font-normal">
              <div className="flex items-center gap-2 px-1 py-1.5 text-left text-sm">
                <div className="flex size-8 items-center justify-center rounded-lg bg-muted">
                  <UserIcon className="size-4" />
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-medium">{nombreMostrado}</span>
                  <span className="truncate text-xs text-muted-foreground">
                    Panel de gestión
                  </span>
                </div>
              </div>
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={salir}>
              <LogOutIcon />
              Cerrar sesión
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <ThemeToggle />
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
