import { useState } from 'react';
import type { BdpAuditEntry, BdpSnapshot } from '@/api/bdp-backup';

/* Selección del detalle del historial (auditoría o snapshot) y estado del diálogo.
 * Sale de BdpHistorial para que el componente no acumule useState (usestate-excesivo). */
export function useSeleccionHistorial() {
  const [entrySeleccionado, setEntrySeleccionado] = useState<BdpAuditEntry | null>(null);
  const [snapshotSeleccionado, setSnapshotSeleccionado] = useState<BdpSnapshot | null>(null);
  const [dialogAbierto, setDialogAbierto] = useState(false);

  function verEntry(entry: BdpAuditEntry) {
    setEntrySeleccionado(entry);
    setSnapshotSeleccionado(null);
    setDialogAbierto(true);
  }

  function verSnapshot(snapshot: BdpSnapshot) {
    setSnapshotSeleccionado(snapshot);
    setEntrySeleccionado(null);
    setDialogAbierto(true);
  }

  function handleDialogOpenChange(open: boolean) {
    setDialogAbierto(open);
    if (!open) {
      setEntrySeleccionado(null);
      setSnapshotSeleccionado(null);
    }
  }

  return {
    entrySeleccionado,
    snapshotSeleccionado,
    dialogAbierto,
    verEntry,
    verSnapshot,
    handleDialogOpenChange,
  };
}
