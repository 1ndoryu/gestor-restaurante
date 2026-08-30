/* [208A-2/C2] Estado del formulario de alta de artículo, extraído de
 * NuevoArticuloDialog a un hook custom para mantener el componente por debajo
 * del máximo de useState (protocolo usestate-excesivo). Lógica pura de
 * formulario: campos, validación y limpieza; la mutación vive en el componente. */
import { useState } from 'react';

export function useNuevoArticuloForm() {
  const [codigo, setCodigo] = useState('');
  const [codigoBdp, setCodigoBdp] = useState('');
  const [descripcion, setDescripcion] = useState('');
  const [precio, setPrecio] = useState('');
  const [iva, setIva] = useState('');

  const codigoValido = codigo.trim() !== '';
  const descripcionValida = descripcion.trim() !== '';

  function limpiar() {
    setCodigo('');
    setCodigoBdp('');
    setDescripcion('');
    setPrecio('');
    setIva('');
  }

  return {
    codigo,
    codigoBdp,
    descripcion,
    precio,
    iva,
    setCodigo,
    setCodigoBdp,
    setDescripcion,
    setPrecio,
    setIva,
    codigoValido,
    descripcionValida,
    limpiar,
  };
}