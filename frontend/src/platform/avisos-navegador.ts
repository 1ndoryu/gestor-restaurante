/* [D33] Aviso al salir con cambios sin guardar. Vive en el límite de plataforma
 * (`platform/`) porque `beforeunload` es del navegador: los componentes piden
 * el aviso y no tocan `window` directamente. */

/** Registra el aviso del navegador mientras `debeAvisar()` diga que sí.
 * Devuelve la función para retirarlo. */
export function avisarAlSalir(debeAvisar: () => boolean): () => void {
  const aviso = (evento: BeforeUnloadEvent) => {
    if (!debeAvisar()) return;
    evento.preventDefault();
    evento.returnValue = '';
  };
  window.addEventListener('beforeunload', aviso);
  return () => window.removeEventListener('beforeunload', aviso);
}
