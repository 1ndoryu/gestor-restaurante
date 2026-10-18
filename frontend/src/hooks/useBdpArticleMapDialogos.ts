import { useState } from 'react';
import type { ArticuloMapEdicion } from '@/components/articulo-map-editar-dialog';

/* Estado de los modales de la tabla de mapeos (alta y edición). */
export function useBdpArticleMapDialogos() {
  const [nuevoOpen, setNuevoOpen] = useState(false);
  const [editando, setEditando] = useState<ArticuloMapEdicion | null>(null);
  return { nuevoOpen, setNuevoOpen, editando, setEditando };
}
