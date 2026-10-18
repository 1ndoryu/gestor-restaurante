/* Tipos compartidos de la configuración BDP: resultado del dry-run y setter de campos. */

import type { EstadoConfiguracion } from '@/hooks/useConfiguracion';

interface BdpSyncDryRunCheck {
  nombre: string;
  endpoint: string;
  ok: boolean;
  mensaje: string;
  cantidad?: number | null;
  muestra?: string | null;
}

export interface BdpSyncDryRunResponse {
  configurado: boolean;
  sync_habilitado: boolean;
  escritura_real: boolean;
  listo_para_sincronizar: boolean;
  mensaje: string;
  checks: BdpSyncDryRunCheck[];
}

/** Setter tipado por clave de la configuración BDP (lo comparten todas las tarjetas). */
export type CambiarCampo = <K extends keyof EstadoConfiguracion>(campo: K, valor: EstadoConfiguracion[K]) => void;
