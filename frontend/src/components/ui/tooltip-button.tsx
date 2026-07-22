/* [223A-1] TooltipButton — wrapper de Button + Tooltip de shadcn.
 * Uso: <TooltipButton tooltip="Texto explicativo" onClick={...}>Label</TooltipButton>
 * Todas las props de Button se pasan directamente (variant, size, disabled, etc).
 * Si tooltip está vacío, renderiza el Button sin Tooltip (no hay overhead).
 *
 * Para icon buttons (size="icon"): el tooltip es esencial.
 * Para botones con texto claro: se puede omitir el tooltip. */

import * as React from 'react';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { Button, buttonVariants } from '@/components/ui/button';
import { type VariantProps } from 'class-variance-authority';

interface TooltipButtonProps
  extends React.ComponentProps<'button'>,
    VariantProps<typeof buttonVariants> {
  /** Texto del tooltip. Si se omite o es vacío, no se envuelve en Tooltip. */
  tooltip?: string;
  /** Lado del tooltip. Por defecto "top". */
  tooltipSide?: 'top' | 'bottom' | 'left' | 'right';
  asChild?: boolean;
  children?: React.ReactNode;
}

function TooltipButton({ tooltip, tooltipSide = 'top', children, ...buttonProps }: TooltipButtonProps) {
  if (!tooltip) {
    return <Button {...buttonProps}>{children}</Button>;
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button {...buttonProps}>{children}</Button>
      </TooltipTrigger>
      <TooltipContent side={tooltipSide}>
        <p>{tooltip}</p>
      </TooltipContent>
    </Tooltip>
  );
}

TooltipButton.displayName = 'TooltipButton';

export { TooltipButton };
export type { TooltipButtonProps };
