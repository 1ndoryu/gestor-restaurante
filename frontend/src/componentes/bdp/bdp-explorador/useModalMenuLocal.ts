import { useState } from 'react';
import type { BdpMenuLocalConLineas } from '@/api/bdp';

/* Modal de alta y edición de menús/packs locales. Sale de BdpExplorador para
 * que el componente no acumule useState (usestate-excesivo). */
export function useModalMenuLocal() {
  const [localModalOpen, setLocalModalOpen] = useState(false);
  const [localModalMenu, setLocalModalMenu] = useState<BdpMenuLocalConLineas | null>(null);

  function abrirNuevo() {
    setLocalModalMenu(null);
    setLocalModalOpen(true);
  }

  function abrirEdicion(menu: BdpMenuLocalConLineas) {
    setLocalModalMenu(menu);
    setLocalModalOpen(true);
  }

  function cerrarModal() {
    setLocalModalOpen(false);
  }

  return { localModalOpen, localModalMenu, abrirNuevo, abrirEdicion, cerrarModal };
}
