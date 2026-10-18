/* Esqueleto de carga de la tabla de stock: cinco filas de placeholder. */
import { Skeleton } from '@/components/ui/skeleton';

/* Skeletons estáticos: filas sin identidad propia; claves de un array estable
 * (no el índice del map) para reconciliación consistente. */
const SKELETON_IDS = [0, 1, 2, 3, 4];

export function TableSkeleton() {
  return (
    <div className="space-y-2">
      {SKELETON_IDS.map((id) => (
        <Skeleton key={id} className="h-10 w-full" />
      ))}
    </div>
  );
}
