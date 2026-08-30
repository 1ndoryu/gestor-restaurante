/* 253A-10: Hook para FormularioGasto — reduce 10 useState a 2 (regla usestate-excesivo)
   253A-14: acepta onExito para uso en modales
   253A-21: metodo_pago es opcional — puede enviarse null si el usuario no selecciona
   283A-22: soporte edición — acepta gastoInicial para pre-rellenar y usa PUT
   044A-10: autocomplete de proveedores — debounce 300ms, ≥2 chars */

import { useState, useEffect, useRef, FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useCrearGasto, useActualizarGasto, useListarCategorias, useListarProveedores, MetodoPago, TipoDocumento, Gasto } from '../api/generated';

interface CamposGasto {
  fecha: string;
  proveedor: string;
  categoriaId: string;
  tipoDocumento: TipoDocumento;
  metodoPago: MetodoPago | '';
  numeroDocumento: string;
  recurrente: boolean;
  importeBase: string;
  importeIva: string;
}

function useFormularioGasto(onExito?: () => void, gastoInicial?: Gasto) {
  const navigate = useNavigate();
  const [error, setError] = useState('');
  const esEdicion = !!gastoInicial;

  const [campos, setCampos] = useState<CamposGasto>(() => {
    if (gastoInicial) {
      return {
        fecha: gastoInicial.fecha,
        proveedor: gastoInicial.proveedor || '',
        categoriaId: gastoInicial.categoria_id || '',
        tipoDocumento: (gastoInicial.tipo_documento as TipoDocumento) || TipoDocumento.factura,
        metodoPago: (gastoInicial.metodo_pago as MetodoPago) || '',
        numeroDocumento: gastoInicial.numero_documento || '',
        recurrente: gastoInicial.recurrente,
        importeBase: gastoInicial.importe_base,
        importeIva: gastoInicial.importe_iva,
      };
    }
    return {
      fecha: new Date().toISOString().split('T')[0],
      proveedor: '',
      categoriaId: '',
      tipoDocumento: TipoDocumento.factura,
      metodoPago: MetodoPago.efectivo,
      numeroDocumento: '',
      recurrente: false,
      importeBase: '',
      importeIva: '',
    };
  });

  const { data: categoriasResp } = useListarCategorias();
  const categorias = categoriasResp?.status === 200 ? categoriasResp.data : [];

  /* [044A-10] Autocomplete de proveedores — mismo patrón que clientes en reservas.
   * Debounce 300ms, se activa al escribir ≥2 chars en el campo proveedor. */
  const [busquedaProveedor, setBusquedaProveedor] = useState('');
  const [autocompletarAbierto, setAutocompletarAbierto] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  /* [FIX-autocomplete] Al abrir el modal en edición el campo llega pre-rellenado
   * (≥2 chars) y el debounce abría la lista sin interacción del usuario.
   * Solo se abre si el usuario escribió o enfocó el campo. */
  const interaccionProveedorRef = useRef(false);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (campos.proveedor.length < 2) {
      setBusquedaProveedor('');
      setAutocompletarAbierto(false);
      return;
    }
    debounceRef.current = setTimeout(() => {
      setBusquedaProveedor(campos.proveedor);
      /* Buscar siempre (para tener sugerencias listas), abrir solo con interacción. */
      if (interaccionProveedorRef.current) setAutocompletarAbierto(true);
    }, 300);
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current); };
  }, [campos.proveedor]);

  const marcarInteraccionProveedor = () => { interaccionProveedorRef.current = true; };

  const { data: proveedoresData } = useListarProveedores(
    { busqueda: busquedaProveedor },
    { query: { enabled: busquedaProveedor.length >= 2 } },
  );
  const sugerenciasProveedores: string[] = proveedoresData?.status === 200
    ? (proveedoresData.data as string[])
    : [];

  const seleccionarProveedor = (nombre: string) => {
    setCampos(prev => ({ ...prev, proveedor: nombre }));
    setAutocompletarAbierto(false);
    setBusquedaProveedor('');
  };

  const cambiarCampo = <K extends keyof CamposGasto>(campo: K, valor: CamposGasto[K]) => {
    setCampos(prev => ({ ...prev, [campo]: valor }));
  };

  const crearMutation = useCrearGasto({
    mutation: {
      onSuccess: (res) => {
        if (res.status === 201) {
          if (onExito) onExito();
          else navigate('/gastos');
        }
      },
      onError: () => setError('Error al crear el gasto'),
    },
  });

  const actualizarMutation = useActualizarGasto({
    mutation: {
      onSuccess: (res) => {
        if (res.status === 200) {
          if (onExito) onExito();
          else navigate('/gastos');
        }
      },
      onError: () => setError('Error al actualizar el gasto'),
    },
  });

  const manejarEnvio = (e: FormEvent) => {
    e.preventDefault();
    setError('');
    if (!campos.fecha || !campos.importeBase || !campos.importeIva) {
      setError('Completa los campos obligatorios');
      return;
    }

    if (esEdicion) {
      actualizarMutation.mutate({
        id: gastoInicial!.id,
        data: {
          fecha: campos.fecha,
          proveedor: campos.proveedor || null,
          categoria_id: campos.categoriaId || null,
          tipo_documento: campos.tipoDocumento,
          metodo_pago: campos.metodoPago || null,
          numero_documento: campos.numeroDocumento || null,
          recurrente: campos.recurrente || null,
          importe_base: campos.importeBase,
          importe_iva: campos.importeIva,
        },
      });
    } else {
      crearMutation.mutate({
        data: {
          fecha: campos.fecha,
          proveedor: campos.proveedor || null,
          categoria_id: campos.categoriaId || null,
          tipo_documento: campos.tipoDocumento,
          metodo_pago: campos.metodoPago || null,
          numero_documento: campos.numeroDocumento || null,
          recurrente: campos.recurrente || null,
          importe_base: campos.importeBase,
          importe_iva: campos.importeIva,
        },
      });
    }
  };

  return {
    campos,
    cambiarCampo,
    error,
    manejarEnvio,
    cargando: crearMutation.isPending || actualizarMutation.isPending,
    categorias,
    esEdicion,
    sugerenciasProveedores,
    autocompletarAbierto,
    setAutocompletarAbierto,
    seleccionarProveedor,
    marcarInteraccionProveedor,
  };
}

export default useFormularioGasto;
