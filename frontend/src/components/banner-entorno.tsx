/* [31AA-1] Franja fija de entorno de pruebas (ámbar, distinta del rojo de errores).
 * Sin etiqueta (producción) no pinta nada. Sin botón de cerrar a propósito. */

import { FlaskConical } from "lucide-react"
import { useEntorno } from "@/hooks/useEntorno"

export function BannerEntorno() {
  const etiqueta = useEntorno()
  if (!etiqueta) return null

  return (
    <div
      role="status"
      className="flex w-full flex-wrap items-center justify-center gap-x-3 gap-y-0.5 border-b border-amber-500 bg-amber-100 px-4 py-1.5 text-center text-sm text-amber-950 dark:bg-amber-950 dark:text-amber-100"
    >
      <FlaskConical className="size-4 shrink-0" aria-hidden="true" />
      <span className="font-semibold">{etiqueta}</span>
      <span>Los datos de aquí son de prueba; no afectan al restaurante.</span>
    </div>
  )
}
