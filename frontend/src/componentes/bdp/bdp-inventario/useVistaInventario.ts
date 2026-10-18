import { useState } from 'react';

/* Estado de vista del inventario: pestaña activa, confirmación de guardado, búsqueda y
 * paginación. Se agrupa aquí para que el componente se centre en el recuento. */
export function useVistaInventario() {
  const [confirmar, setConfirmar] = useState(false);
  const [tab, setTab] = useState('inventario');
  const [pagina, setPagina] = useState(1);
  const [busqueda, setBusqueda] = useState('');
  const [paginaConteos, setPaginaConteos] = useState(1);
  return { confirmar, setConfirmar, tab, setTab, pagina, setPagina, busqueda, setBusqueda, paginaConteos, setPaginaConteos };
}
