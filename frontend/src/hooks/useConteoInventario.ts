/* [198A-1/D6=A] Estado del conteo de inventario, extraído de BdpInventario a
 * un hook custom para mantener el componente bajo el máximo de useState
 * (protocolo usestate-excesivo). Agrupa el estado del formulario de conteo:
 * contadas por artículo, observaciones, id de contexto en retomar y la clave
 * de idempotencia por sesión (D4: en un reintento tras fallo ambiguo se
 * reenvía la misma clave para que el backend no aplique dos veces la
 * diferencia). La mutación/envío vive en el componente. */
import { useState } from 'react';
import { obtenerConteoInventario } from '@/api/bdp';

export function useConteoInventario() {
  const [contadas, setContadas] = useState<Record<string, string>>({});
  const [observaciones, setObservaciones] = useState('');
  const [retomando, setRetomando] = useState<string | null>(null);
  /* [208A-2/C3] Clave de idempotencia por sesión de conteo: se genera al
   * montar y se regenera tras guardar con éxito; en un reintento tras un
   * fallo ambiguo se reenvía la misma clave. */
  const [conteoKey, setConteoKey] = useState(() => crypto.randomUUID());
  /* [L7] Almacén del conteo: «0» es el agregado local «General» y es el que
   * trae todo lo que existía antes de que el conteo tuviera almacén propio. */
  const [almacen, setAlmacen] = useState('0');

  function setContada(articuloGloryCodigo: string, valor: string) {
    setContadas((p) => ({ ...p, [articuloGloryCodigo]: valor }));
  }

  async function retomar(conteoId: string) {
    setRetomando(conteoId);
    try {
      const detalle = await obtenerConteoInventario(conteoId);
      const mapa: Record<string, string> = {};
      for (const l of detalle.lineas) mapa[l.articulo_glory_codigo] = String(l.contado);
      setContadas(mapa);
      /* [L7] Retomar un conteo de otro almacén cambia la selección: si no, el
       * recuento se aplicaría al almacén que estuviera puesto y no al suyo. */
      setAlmacen(detalle.conteo.warehouse_id || '0');
      return true;
    } catch {
      return false;
    } finally {
      setRetomando(null);
    }
  }

  function limpiar() {
    setObservaciones('');
    setContadas({});
    setConteoKey(crypto.randomUUID());
  }

  return {
    contadas,
    observaciones,
    setObservaciones,
    retomando,
    conteoKey,
    almacen,
    setAlmacen,
    setContada,
    retomarConteo: retomar,
    limpiar,
  };
}