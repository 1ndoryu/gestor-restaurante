/* [BDP-DEMO-TOGGLE] Botón compartido para activar/desactivar modo demo en páginas BDP.
 * Estado visible: «Modo demo activado» + salida explícita; tooltip para evitar
 * confusión con datos reales (duda 3 de Guillermo). */

import { FlaskConical } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';

interface BdpDemoToggleProps {
  demoMode: boolean;
  onToggle: (next: boolean) => void;
}

export function BdpDemoToggle({ demoMode, onToggle }: BdpDemoToggleProps) {
  if (demoMode) {
    return (
      <div className="flex items-center gap-2 rounded-md border border-amber-300/70 bg-amber-50 px-3 py-1.5 text-xs dark:border-amber-800 dark:bg-amber-950/30">
        <FlaskConical className="size-4 text-amber-700 dark:text-amber-400" />
        <span className="font-medium text-amber-900 dark:text-amber-200">Modo demo activado</span>
        <TooltipButton
          variant="outline"
          size="sm"
          className="h-7"
          aria-pressed={false}
          tooltip="Los datos son simulados y se pueden reactivar cuando quieras. No afectan a la Aplicación Web ni a BDP."
          onClick={() => onToggle(false)}
        >
          Salir del modo demo
        </TooltipButton>
      </div>
    );
  }
  return (
    <Button
      variant="outline"
      onClick={() => onToggle(!demoMode)}
      aria-pressed={demoMode}
    >
      <FlaskConical className="size-4 mr-1.5" />
      Cargar modo demo
    </Button>
  );
}

export default BdpDemoToggle;
