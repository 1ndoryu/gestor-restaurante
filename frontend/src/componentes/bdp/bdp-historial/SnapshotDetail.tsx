/* Detalle de un snapshot BDP: tipo, fecha, notas y datos guardados. */
import type { BdpSnapshot } from '@/api/bdp-backup';
import { formatDate } from './formato';

export function SnapshotDetail({ snapshot }: { snapshot: BdpSnapshot }) {
  return (
    <div className="space-y-3 text-sm">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <p className="text-xs text-muted-foreground">Tipo</p>
          <p className="font-medium">{snapshot.tipo}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Fecha</p>
          <p>{formatDate(snapshot.created_at)}</p>
        </div>
      </div>
      {snapshot.notas && (
        <div>
          <p className="text-xs text-muted-foreground">Notas</p>
          <p>{snapshot.notas}</p>
        </div>
      )}
      <div>
        <p className="text-xs text-muted-foreground">Datos</p>
        <pre className="mt-1 rounded bg-muted p-2 text-xs overflow-auto max-h-48">
          {JSON.stringify(snapshot.datos, null, 2)}
        </pre>
      </div>
    </div>
  );
}
