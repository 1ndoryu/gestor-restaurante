/* [128A-1/F7] Estado del formulario de menús/packs locales BDP, extraído de
 * BdpMenuLocalModal a un hook custom (protocolo usestate-excesivo).
 * Comportamiento idéntico; el sincronizado inicial ocurre al abrir el modal. */
import { useState } from 'react';
import type {
  ActualizarBdpMenuLocalRequest,
  BdpMenuLocalConLineas,
  BdpMenuLocalLineaRequest,
  BdpMenuLocalTipo,
  CrearBdpMenuLocalRequest,
} from '@/api/bdp';

export interface MenuLineaForm {
  key: number;
  articulo_codigo: string;
  descripcion: string;
  cantidad: string;
  precio_unitario: string;
}

function lineasDesdeMenu(menu: BdpMenuLocalConLineas): MenuLineaForm[] {
  return menu.lineas.map((linea, index) => ({
    key: index,
    articulo_codigo: linea.articulo_codigo ?? '',
    descripcion: linea.descripcion,
    cantidad: String(linea.cantidad),
    precio_unitario: String(linea.precio_unitario),
  }));
}

export function useBdpMenuLocalForm(open: boolean, isEdit: boolean, menu: BdpMenuLocalConLineas | null) {
  const [tipo, setTipo] = useState<BdpMenuLocalTipo>('menu');
  const [nombre, setNombre] = useState('');
  const [descripcion, setDescripcion] = useState('');
  const [precio, setPrecio] = useState('');
  const [activo, setActivo] = useState(true);
  const [lineas, setLineas] = useState<MenuLineaForm[]>([]);
  const [nextLineaKey, setNextLineaKey] = useState(1);
  const [error, setError] = useState('');

  /* Sincroniza el formulario cada vez que se abre el modal. */
  const [lastOpenedFor, setLastOpenedFor] = useState<string | null>(null);
  const openedKey = isEdit ? menu!.id : 'nuevo';
  if (open && lastOpenedFor !== openedKey) {
    setLastOpenedFor(openedKey);
    setError('');
    if (menu) {
      setTipo(menu.tipo);
      setNombre(menu.nombre);
      setDescripcion(menu.descripcion ?? '');
      setPrecio(menu.precio);
      setActivo(menu.activo);
      const iniciales = lineasDesdeMenu(menu);
      setLineas(iniciales);
      setNextLineaKey(iniciales.length + 1);
    } else {
      setTipo('menu');
      setNombre('');
      setDescripcion('');
      setPrecio('');
      setActivo(true);
      setLineas([]);
      setNextLineaKey(1);
    }
  }

  function addLinea() {
    setLineas((prev) => [
      ...prev,
      { key: nextLineaKey, articulo_codigo: '', descripcion: '', cantidad: '1', precio_unitario: '' },
    ]);
    setNextLineaKey((k) => k + 1);
  }

  function updateLinea(key: number, campo: keyof MenuLineaForm, valor: string) {
    setLineas((prev) => prev.map((l) => (l.key === key ? { ...l, [campo]: valor } : l)));
  }

  function seleccionarArticulo(
    key: number,
    articuloCodigo: string,
    catalog?: { articulo_glory_codigo: string; articulo_bdp_nombre?: string | null }[],
  ) {
    const articulo = catalog?.find((a) => a.articulo_glory_codigo === articuloCodigo);
    setLineas((prev) =>
      prev.map((l) =>
        l.key === key
          ? {
              ...l,
              articulo_codigo: articuloCodigo,
              descripcion: articulo?.articulo_bdp_nombre ?? l.descripcion,
            }
          : l,
      ),
    );
  }

  function removeLinea(key: number) {
    setLineas((prev) => prev.filter((l) => l.key !== key));
  }

  /* Normaliza decimales con coma (formato español) a punto antes de enviar;
   * el backend espera `Decimal` con punto (serde). [208A-2/F7] */
  function normalizarDecimal(valor: string): string {
    const limpio = valor.trim().replace(/\s/g, '');
    return limpio.replace(',', '.');
  }

  function handleSubmit(
    e: React.FormEvent,
    onSubmit: (req: CrearBdpMenuLocalRequest | ActualizarBdpMenuLocalRequest) => void,
  ) {
    e.preventDefault();
    const nombreTrim = nombre.trim();
    if (!nombreTrim) {
      setError('Indica el nombre del menú/pack');
      return;
    }
    const lineasValidas: BdpMenuLocalLineaRequest[] = lineas
      .filter((l) => l.descripcion.trim() !== '')
      .map((l) => ({
        articulo_codigo: l.articulo_codigo.trim() || undefined,
        descripcion: l.descripcion.trim(),
        cantidad: l.cantidad !== '' ? normalizarDecimal(l.cantidad) : undefined,
        precio_unitario: l.precio_unitario !== '' ? normalizarDecimal(l.precio_unitario) : undefined,
      }));
    if (lineasValidas.length === 0) {
      setError('Añade al menos una línea con descripción');
      return;
    }

    const base = {
      tipo,
      nombre: nombreTrim,
      descripcion: descripcion.trim() || undefined,
      precio: precio.trim() ? normalizarDecimal(precio) : undefined,
      activo,
      lineas: lineasValidas,
    };
    if (isEdit) {
      onSubmit(base as ActualizarBdpMenuLocalRequest);
    } else {
      onSubmit(base as CrearBdpMenuLocalRequest);
    }
  }

  return {
    tipo,
    setTipo,
    nombre,
    setNombre,
    descripcion,
    setDescripcion,
    precio,
    setPrecio,
    activo,
    setActivo,
    lineas,
    addLinea,
    updateLinea,
    seleccionarArticulo,
    removeLinea,
    error,
    handleSubmit,
  };
}