/* [BDP-DEMO] Hook reutilizable para activar datos de prueba en páginas BDP.
 * El modo demo es voluntario y se mantiene en estado local de la sesión.
 * No persiste ni afecta a producción. */

import { useState } from 'react';

export function useBdpDemoMode() {
  const [demoMode, setDemoMode] = useState(false);
  return { demoMode, setDemoMode };
}

export default useBdpDemoMode;
