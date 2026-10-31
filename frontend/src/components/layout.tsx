/* [263A-16] Layout principal — guard de auth + SidebarProvider + AppSidebar + SiteHeader.
 * Reemplaza el viejo Layout con BarraLateral por el nuevo con shadcn sidebar. */

import { Navigate, Outlet } from "react-router-dom"
import { useAuthStore } from "@/stores/authStore"
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar"
import { AppSidebar } from "@/components/app-sidebar"
import { SiteHeader } from "@/components/site-header"
import { BannerEntorno } from "@/components/banner-entorno"

export default function Layout() {
  const autenticado = useAuthStore((s) => s.estaAutenticado)()

  if (!autenticado) {
    return <Navigate to="/login" replace />
  }

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <BannerEntorno />
        <SiteHeader />
        <div className="flex flex-1 flex-col gap-4 p-4">
          <Outlet />
        </div>
      </SidebarInset>
    </SidebarProvider>
  )
}
