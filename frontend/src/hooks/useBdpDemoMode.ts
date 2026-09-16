/* [BDP-DEMO] Hook reutilizable para activar datos de prueba en páginas BDP.
 * El modo demo es voluntario y se mantiene en estado local de la sesión.
 * No persiste ni afecta a producción.
 * [159A-3] El modo demo arranca APAGADO por defecto (antes arrancaba
 * encendido en dev y el badge de la toolbar rompía el layout). La única
 * forma de activarlo es el switch de Configuración → pestaña BDP. */

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
   * todas las páginas BDP compartan el mismo estado. Sin preferencia previa
   * arranca APAGADO [159A-3]: el modo demo solo se activa desde el switch
   * de Configuración → pestaña BDP. */
  const [demoMode, setDemoMode] = useState(() => {
    const stored = readStoredDemoMode();
    if (stored !== null) return stored;
    return false;
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
