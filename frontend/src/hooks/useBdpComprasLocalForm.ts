/* [128A-1] Estado del formulario de albaranes de compra locales BDP, extraído
 * de BdpComprasLocalModal a un hook custom para mantener el componente bajo
 * el máximo de useState (protocolo usestate-excesivo). Comportamiento idéntico;
 * el sincronizado inicial se hace al abrir el modal por primera vez. */
import { useState } from 'react';
import type {
  ActualizarBdpPurchaseNoteRequest,
  BdpPurchaseNote,
  BdpPurchaseNoteLineaLocal,
  CrearBdpPurchaseNoteRequest,
} from '@/api/bdp';

export interface LineaForm {
  key: number;
  descripcion: string;
  cantidad: string;
  precio_unitario: string;
  iva_pct: string;
}

function lineasDesdeDatos(datos: Record<string, unknown>): LineaForm[] {
  const raw = Array.isArray(datos.lineas) ? datos.lineas : [];
  return raw
    .map((item, index) => {
      const linea = item as Partial<BdpPurchaseNoteLineaLocal>;
      return {
        key: index,
        descripcion: linea.descripcion ?? '',
        cantidad: linea.cantidad != null ? String(linea.cantidad) : '',
        precio_unitario: linea.precio_unitario != null ? String(linea.precio_unitario) : '',
        iva_pct: linea.iva_pct != null ? String(linea.iva_pct) : '',
      };
    })
    .filter((l) => l.descripcion || l.cantidad || l.precio_unitario || l.iva_pct);
}

export function useBdpComprasLocalForm(open: boolean, isEdit: boolean, note: BdpPurchaseNote | null) {
  const [serie, setSerie] = useState('');
  const [numero, setNumero] = useState('');
  const [fecha, setFecha] = useState('');
  const [nombreProveedor, setNombreProveedor] = useState('');
  const [codigoProveedor, setCodigoProveedor] = useState('');
  const [total, setTotal] = useState('');
  const [lineas, setLineas] = useState<LineaForm[]>([]);
  const [nextLineaKey, setNextLineaKey] = useState(1);
  const [error, setError] = useState('');

  /* Sincroniza el formulario cada vez que se abre el modal. */
  const [lastOpenedFor, setLastOpenedFor] = useState<string | null>(null);
  const openedKey = isEdit ? note!.id : 'nuevo';
  if (open && lastOpenedFor !== openedKey) {
    setLastOpenedFor(openedKey);
    setError('');
    if (note) {
      setSerie(note.serie);
      setNumero(note.numero);
      setFecha(note.fecha ?? '');
      setNombreProveedor(note.nombre_proveedor ?? '');
      setCodigoProveedor(note.codigo_proveedor ?? '');
      setTotal(note.total ?? '');
      const iniciales = lineasDesdeDatos(note.datos_bdp);
      setLineas(iniciales);
      setNextLineaKey(iniciales.length + 1);
    } else {
      setSerie('L');
      setNumero('');
      setFecha(new Date().toISOString().slice(0, 10));
      setNombreProveedor('');
      setCodigoProveedor('');
      setTotal('');
      setLineas([]);
      setNextLineaKey(1);
    }
  }

  function addLinea() {
    setLineas((prev) => [
      ...prev,
      { key: nextLineaKey, descripcion: '', cantidad: '', precio_unitario: '', iva_pct: '21' },
    ]);
    setNextLineaKey((k) => k + 1);
  }

  function updateLinea(key: number, campo: keyof LineaForm, valor: string) {
    setLineas((prev) => prev.map((l) => (l.key === key ? { ...l, [campo]: valor } : l)));
  }

  function removeLinea(key: number) {
    setLineas((prev) => prev.filter((l) => l.key !== key));
  }

  /* Normaliza decimales con coma (formato español) a punto antes de enviar;
   * el backend espera `Decimal` con punto (serde). [208A-2/F5] */
  function normalizarDecimal(valor: string): string {
    const limpio = valor.trim().replace(/\s/g, '');
    return limpio.replace(',', '.');
  }

  function handleSubmit(e: React.FormEvent, onSubmit: (req: CrearBdpPurchaseNoteRequest | ActualizarBdpPurchaseNoteRequest) => void) {
    e.preventDefault();
    const lineasValidas: BdpPurchaseNoteLineaLocal[] = lineas
      .filter((l) => l.descripcion.trim() && l.cantidad !== '' && l.precio_unitario !== '')
      .map((l) => ({
        descripcion: l.descripcion.trim(),
        cantidad: normalizarDecimal(l.cantidad),
        precio_unitario: normalizarDecimal(l.precio_unitario),
        iva_pct: l.iva_pct === '' ? '21' : normalizarDecimal(l.iva_pct),
      }));
    const tieneProveedor = nombreProveedor.trim() !== '' || codigoProveedor.trim() !== '';
    const tieneImporte = total.trim() !== '' || lineasValidas.length > 0;
    if (!tieneProveedor) {
      setError('Indica el proveedor (nombre o código)');
      return;
    }
    if (!tieneImporte) {
      setError('Indica un total o al menos una línea completa');
      return;
    }
    const base = {
      numero: numero.trim() || undefined,
      fecha: fecha || undefined,
      codigo_proveedor: codigoProveedor.trim() || undefined,
      nombre_proveedor: nombreProveedor.trim() || undefined,
      total: total.trim() ? normalizarDecimal(total) : undefined,
      lineas: lineasValidas.length > 0 ? lineasValidas : undefined,
    };
    if (isEdit) {
      onSubmit(base as ActualizarBdpPurchaseNoteRequest);
    } else {
      onSubmit({ ...base, serie: serie.trim() || undefined } as CrearBdpPurchaseNoteRequest);
    }
  }

  return {
    serie,
    setSerie,
    numero,
    setNumero,
    fecha,
    setFecha,
    nombreProveedor,
    setNombreProveedor,
    codigoProveedor,
    setCodigoProveedor,
    total,
    setTotal,
    lineas,
    addLinea,
    updateLinea,
    removeLinea,
    error,
    handleSubmit,
  };
}