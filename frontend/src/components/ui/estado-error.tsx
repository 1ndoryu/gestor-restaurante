/* [149A-3/F2] Estado de error honesto para consultas fallidas.
 * Antes: una consulta en error dejaba la pantalla en "Cargando..." para siempre
 * o pintaba un vacío falso ("0 trabajadores", "No hay operaciones en la cola"),
 * porque ninguna página leía `isError`. Aquí se declara una sola vez cómo se ve
 * un fallo y qué se le dice al usuario, para no repetirlo en cada pantalla. */

import { ShieldAlert, TriangleAlert } from 'lucide-react';
import { Button } from '@/components/ui/button';

type ErrorConRespuesta = {
  response?: { status?: number; data?: { message?: string; error?: string } };
  message?: string;
};

/** Código HTTP del error si lo trae (axios), o undefined. */
export function estadoDelError(error: unknown): number | undefined {
  const e = error as ErrorConRespuesta | null | undefined;
  return e?.response?.status;
}

/** Un 401/403 no debe reintentarse ni disfrazarse: el permiso no cambia solo. */
export function esErrorDePermiso(error: unknown): boolean {
  const status = estadoDelError(error);
  return status === 401 || status === 403;
}

function mensajeDelError(error: unknown): string {
  const e = error as ErrorConRespuesta | null | undefined;
  return e?.response?.data?.message || e?.response?.data?.error || e?.message || '';
}

export function EstadoError({
  error,
  queFallo = 'esta sección',
  reintentar,
}: {
  error: unknown;
  /** Nombre de lo que se intentaba cargar, para el texto ("la cola de sincronización"). */
  queFallo?: string;
  reintentar?: () => void;
}) {
  const status = estadoDelError(error);
  const sinPermiso = esErrorDePermiso(error);
  const detalle = mensajeDelError(error);

  return (
    <div className="flex flex-col items-start gap-2 rounded-md border border-dashed p-4">
      <div className="flex items-center gap-2">
        {sinPermiso ? (
          <ShieldAlert className="size-4 text-destructive" />
        ) : (
          <TriangleAlert className="size-4 text-destructive" />
        )}
        <p className="text-sm font-medium">
          {sinPermiso
            ? `No tienes permiso para ver ${queFallo}`
            : `No se pudo cargar ${queFallo}`}
        </p>
      </div>
      <p className="text-xs text-muted-foreground">
        {sinPermiso
          ? 'Tu usuario no tiene acceso a esta sección. Pide al propietario que te lo habilite.'
          : detalle || 'El servidor no respondió correctamente. Puedes intentarlo de nuevo.'}
        {status ? ` (código ${status})` : ''}
      </p>
      {/* Reintentar solo tiene sentido cuando el fallo puede ser transitorio. */}
      {!sinPermiso && reintentar && (
        <Button variant="outline" size="sm" onClick={reintentar}>
          Reintentar
        </Button>
      )}
    </div>
  );
}
