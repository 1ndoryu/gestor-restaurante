/* [287A-5] Ajuste guiado reutilizable para parámetros de lectura BDP que la
 * aplicación no puede descubrir de forma fiable en una instalación real. */

import { AlertCircle, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

interface BdpRequiredSettingProps {
  title: string;
  description: string;
  label: string;
  value: string;
  min?: number;
  max?: number;
  saving: boolean;
  onChange: (value: string) => void;
  onSave: () => void;
  /** [208A-2] Si se pasa, muestra un botón para ocultar el aviso de nuevo. */
  onDismiss?: () => void;
}

export function BdpRequiredSetting({
  title,
  description,
  label,
  value,
  min = 1,
  max,
  saving,
  onChange,
  onSave,
  onDismiss,
}: BdpRequiredSettingProps) {
  return (
    <div className="flex w-full flex-col gap-3 rounded-md border p-3">
      <div className="flex items-start gap-2">
        <AlertCircle className="mt-0.5 size-4 shrink-0 text-amber-600" />
        <div className="flex-1">
          <p className="text-sm font-medium">{title}</p>
          <p className="text-xs text-muted-foreground">{description}</p>
        </div>
        {onDismiss && (
          <Button variant="ghost" size="icon" className="size-6 shrink-0" onClick={onDismiss} aria-label="Ocultar este aviso">
            <X className="size-3.5" />
          </Button>
        )}
      </div>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
        <div className="flex flex-1 flex-col gap-1">
          <Label htmlFor={`bdp-required-${label}`} className="text-xs">{label}</Label>
          <Input
            id={`bdp-required-${label}`}
            type="number"
            min={min}
            max={max}
            value={value}
            onChange={(event) => onChange(event.target.value)}
          />
        </div>
        <Button onClick={onSave} disabled={saving || !value.trim()}>
          {saving ? 'Guardando...' : 'Guardar y reintentar'}
        </Button>
      </div>
    </div>
  );
}
