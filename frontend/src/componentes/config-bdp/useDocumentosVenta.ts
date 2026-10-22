/* [D28-auto-R] Estado de los documentos de venta leídos del BDP.
 *
 * El servicio y la tabla existían desde D28 sin ninguna ruta ni pantalla: la
 * lectura automática solo se podía encender con variables de entorno. Aquí vive
 * el estado de la tarjeta: la lista guardada y la lectura bajo demanda. */

import { useCallback, useEffect, useState } from 'react';
import axios from '@/api/axios-instance';
import { toast } from 'sonner';

/** Una fila de `GET /api/bdp/documentos`. */
export interface DocumentoVenta {
  bdp_documento_id: string;
  perfil: number;
  fecha: string | null;
  serie: string | null;
  numero: string | null;
  total: number | string | null;
  recibido_at: string;
  /** JSON original de BDP: los campos no están documentados, se enseña en crudo. */
  datos_bdp: Record<string, unknown>;
}

/** Lo que devuelve `POST /api/bdp/documentos/sincronizar`. */
export interface ResultadoSincronizacion {
  ok: boolean;
  perfil: number;
  resultado: { procesados: number; total_bdp: number };
}

interface EstadoDocumentos {
  documentos: DocumentoVenta[];
  cargando: boolean;
  leyendo: boolean;
}

export function useDocumentosVenta() {
  const [estado, setEstado] = useState<EstadoDocumentos>({
    documentos: [],
    cargando: true,
    leyendo: false,
  });

  const cargar = useCallback(async () => {
    setEstado((previo) => ({ ...previo, cargando: true }));
    try {
      const resp = await axios.get<DocumentoVenta[]>('/api/bdp/documentos');
      setEstado((previo) => ({ ...previo, documentos: resp.data, cargando: false }));
    } catch {
      // Sin BDP configurado la lista no existe: la tarjeta lo dice y no rompe la página.
      setEstado((previo) => ({ ...previo, cargando: false }));
    }
  }, []);

  useEffect(() => {
    void cargar();
  }, [cargar]);

  /** Lee ahora la ventana pendiente. Sin perfil, el backend usa el configurado. */
  const leerAhora = useCallback(
    async (perfil?: number | null) => {
      setEstado((previo) => ({ ...previo, leyendo: true }));
      try {
        const resp = await axios.post<ResultadoSincronizacion>(
          '/api/bdp/documentos/sincronizar',
          perfil ? { perfil } : {},
        );
        const { procesados } = resp.data.resultado;
        toast.success(
          procesados === 0
            ? `Perfil ${resp.data.perfil}: no había documentos nuevos`
            : `Perfil ${resp.data.perfil}: ${procesados} documento(s) guardados`,
        );
        await cargar();
      } catch (error) {
        toast.error(mensajeDeError(error));
      } finally {
        setEstado((previo) => ({ ...previo, leyendo: false }));
      }
    },
    [cargar],
  );

  return { ...estado, cargar, leerAhora };
}

/** El backend contesta `{ error }` o `{ message }`: se prefiere lo que diga. */
function mensajeDeError(error: unknown): string {
  const respuesta = (error as { response?: { data?: { message?: string; error?: string } } })
    ?.response?.data;
  return respuesta?.message ?? respuesta?.error ?? 'No se pudo leer los documentos del BDP';
}
