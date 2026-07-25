/* [263A-16] Sidebar principal del restaurante.
 * Navegación adaptada de dashboard-01 con react-router-dom Links.
 * Collapsible "icon" para colapsar a solo iconos (tarea 22).
 * [283A-36] Botón "Reportar error" en sidebar con modal. */

import * as React from "react"
import { useState } from "react"
import { Link } from "react-router-dom"

import { NavMain } from "@/components/nav-main"
import { NavSecondary } from "@/components/nav-secondary"
import { NavUser } from "@/components/nav-user"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarGroup,
  SidebarGroupContent,
} from "@/components/ui/sidebar"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { toast } from "sonner"
import axios from "@/api/axios-instance"
import {
  DollarSign,
  BarChart3,
  ClipboardList,
  Calendar,
  Users,
  Radio,
  UserX,
  Map,
  Settings,
  UtensilsCrossed,
  Megaphone,
  LayoutDashboard,
  MessageSquare,
  Bell,
  Bug,
  ExternalLink,
  Shield,
  Star,
  Clock,
  Package,
  Search,
  Database,
} from "lucide-react"
import { useObtenerConfiguracion } from "@/api/generated/configuracion/configuracion"

const navPrincipal = [
  { title: "Dashboard", url: "/", icon: <LayoutDashboard /> },
  { title: "Ventas", url: "/ventas", icon: <DollarSign /> },
  { title: "Gastos", url: "/gastos", icon: <BarChart3 /> },
  { title: "Reservas", url: "/reservas", icon: <ClipboardList /> },
  { title: "Calendario", url: "/reservas/calendario", icon: <Calendar /> },
  { title: "Clientes", url: "/clientes", icon: <Users /> },
  { title: "Canales", url: "/canales", icon: <Radio /> },
  { title: "No-Shows", url: "/reservas/no-shows", icon: <UserX /> },
  { title: "Plano de Sala", url: "/plano-sala", icon: <Map /> },
  { title: "Campañas", url: "/marketing/campanas", icon: <Megaphone /> },
  { title: "Plantillas WA", url: "/marketing/plantillas", icon: <MessageSquare /> },
  { title: "Recordatorios", url: "/marketing/recordatorios", icon: <Bell /> },
  { title: "Trabajadores", url: "/trabajadores", icon: <Shield /> },
  { title: "Reseñas", url: "/resenas", icon: <Star /> },
  { title: "Inactividad", url: "/inactividad", icon: <Clock /> },
]

const navBdp = [
  { title: "BDP Stock", url: "/bdp/stock", icon: <Package /> },
  { title: "BDP Explorador", url: "/bdp/explorador", icon: <Search /> },
  { title: "BDP Historial", url: "/bdp/historial", icon: <Database /> },
]

const navSecundario = [
  { title: "Configuración", url: "/configuracion", icon: <Settings /> },
]

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const [modalError, setModalError] = useState(false)
  const [mensaje, setMensaje] = useState("")
  const [enviando, setEnviando] = useState(false)
  const { data: configData } = useObtenerConfiguracion()
  const urlHaddock = configData?.status === 200
    ? (configData.data as { url_haddock?: string }).url_haddock
    : undefined

  /* [303A-2] Migrado de raw fetch a axios para usar interceptors JWT/401 */
  const enviarReporte = async () => {
    if (!mensaje.trim()) return
    setEnviando(true)
    try {
      await axios.post("/api/reportar-error", {
        mensaje: mensaje.trim(),
        stack: null,
        url: window.location.href,
        navegador: navigator.userAgent,
      })
      toast.success("Reporte enviado correctamente")
      setMensaje("")
      setModalError(false)
    } catch {
      toast.error("No se pudo enviar el reporte")
    } finally {
      setEnviando(false)
    }
  }

  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              className="data-[slot=sidebar-menu-button]:p-1.5!"
            >
              <Link to="/">
                <UtensilsCrossed className="size-5!" />
                <span className="text-base font-semibold">Restaurante</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={navPrincipal} />
        <NavSecondary items={navBdp} label="Integración BDP" className="mt-auto" />
        <NavSecondary items={navSecundario} />
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {/* [034A-3] Botón Haddock: solo visible si url_haddock está configurada */}
              {urlHaddock && (
                <SidebarMenuItem>
                  <SidebarMenuButton tooltip="Ver en Haddock" asChild>
                    <a href={urlHaddock} target="_blank" rel="noopener noreferrer">
                      <ExternalLink />
                      <span>Ver en Haddock</span>
                    </a>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              )}
              <SidebarMenuItem>
                <SidebarMenuButton tooltip="Reportar error" onClick={() => setModalError(true)}>
                  <Bug />
                  <span>Reportar error</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <NavUser />
      </SidebarFooter>

      <Dialog open={modalError} onOpenChange={setModalError}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reportar un error</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="error-msg">Describe el problema</Label>
              <Textarea
                id="error-msg"
                placeholder="¿Qué ocurrió? ¿Qué esperabas que pasara?"
                value={mensaje}
                onChange={(e) => setMensaje(e.target.value)}
                rows={5}
              />
            </div>
            <Button onClick={enviarReporte} disabled={enviando || !mensaje.trim()}>
              {enviando ? "Enviando…" : "Enviar reporte"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </Sidebar>
  )
}
