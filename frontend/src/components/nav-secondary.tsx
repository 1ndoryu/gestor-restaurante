/* [263A-16] Navegación secundaria (Configuración, etc.) */

import * as React from "react"
import { Link, useLocation } from "react-router-dom"

import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { useAuthStore } from "@/stores/authStore"
import { AVISO_SECCION } from "@/components/nav-main"

export function NavSecondary({
  items,
  label,
  ...props
}: {
  items: {
    title: string
    url: string
    icon: React.ReactNode
    soloAdmin?: boolean
    seccion?: string
  }[]
  label?: string
} & React.ComponentPropsWithoutRef<typeof SidebarGroup>) {
  const location = useLocation()
  const esTrabajador = useAuthStore((s) => s.esTrabajador)()
  const tieneSeccion = useAuthStore((s) => s.tieneSeccion)

  return (
    <SidebarGroup {...props}>
      {label && <SidebarGroupLabel>{label}</SidebarGroupLabel>}
      <SidebarGroupContent>
        <SidebarMenu>
          {items.map((item) => {
            /* [169A-3] Mismo filtro que NavMain: soloAdmin o sección no concedida. */
            const deshabilitado = esTrabajador
              && (Boolean(item.soloAdmin)
                || (typeof item.seccion === "string" && !tieneSeccion(item.seccion)))
            return (
              <SidebarMenuItem key={item.title}>
                {/* [208A-2] Mismo tamaño compacto que la navegación principal. */}
                {/* [149A-3/F3] Entradas soloAdmin deshabilitadas con aviso para trabajador. */}
                {deshabilitado ? (
                  <SidebarMenuButton
                    size="sm"
                    tooltip={AVISO_SECCION}
                    aria-disabled="true"
                    disabled
                    title={AVISO_SECCION}
                    className="opacity-50 cursor-not-allowed"
                  >
                    {item.icon}
                    <span>{item.title}</span>
                  </SidebarMenuButton>
                ) : (
                  <SidebarMenuButton asChild size="sm" isActive={location.pathname === item.url}>
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
    </SidebarGroup>
  )
}
