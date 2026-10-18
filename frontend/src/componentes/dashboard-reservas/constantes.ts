/* CSS vars del tema — se adaptan automáticamente a dark/light mode */

import type { ChartConfig } from '@/components/ui/chart';

export const PIE_COLORS = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)', 'var(--chart-5)'];
export const CONFIG_SIMPLE: ChartConfig = {total: {label: 'Reservas', color: 'var(--chart-1)'}};
export const CONFIG_TURNO: ChartConfig = {
    total: {label: 'Reservas', color: 'var(--chart-1)'},
    personas: {label: 'Personas', color: 'var(--chart-2)'}
};
