/* [BDP-DEMO] Hook reutilizable para activar datos de prueba en páginas BDP.
 * El modo demo es voluntario y se mantiene en estado local de la sesión.
 * No persiste ni afecta a producción. */

import { useEffect, useState } from 'react';

const DEMO_MODE_KEY = 'bdp_demo_mode';

function readStoredDemoMode(): boolean | null {
  if (typeof localStorage === 'undefined') return null;
  try {
    const raw = localStorage.getItem(DEMO_MODE_KEY);
    if (raw === null) return null;
    return raw === 'true';
  } catch {
    return null;
  }
}

export function useBdpDemoMode() {
  /* [BDP-DEMO-INIT] Persistimos la preferencia en localStorage para que
   * todas las páginas BDP compartan el mismo estado. Si no hay preferencia
   * previa, en desarrollo local cargamos la demo automáticamente para que el
   * usuario pueda visualizar las páginas sin conectar con BDP; en producción
   * siempre arranca apagada. */
  const [demoMode, setDemoMode] = useState(() => {
    const stored = readStoredDemoMode();
    if (stored !== null) return stored;
    return import.meta.env.DEV;
  });

  useEffect(() => {
    if (typeof localStorage === 'undefined') return;
    try {
      localStorage.setItem(DEMO_MODE_KEY, String(demoMode));
    } catch {
      // localStorage puede estar bloqueado en modo privado; ignoramos el error.
    }
  }, [demoMode]);

  return { demoMode, setDemoMode };
}

export default useBdpDemoMode;
