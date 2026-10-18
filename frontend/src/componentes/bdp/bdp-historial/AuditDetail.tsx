/* Detalle de una operación BDP de auditoría: operación, resultado, datos y respuesta. */
import type { BdpAuditEntry } from '@/api/bdp-backup';
import { direccionLabel, formatDate, operacionLabel, origenBadge, resultadoBadge } from './formato';

export function AuditDetail({ entry }: { entry: BdpAuditEntry }) {
  return (
    <div className="space-y-3 text-sm">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <p className="text-xs text-muted-foreground">Operación</p>
          <p className="font-medium">{operacionLabel(entry.operacion)}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Resultado</p>
          {resultadoBadge(entry.resultado)}
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Dirección</p>
          <p>{direccionLabel(entry.direccion)}</p>
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Origen</p>
          {origenBadge(entry.origen_operacion)}
        </div>
        <div>
          <p className="text-xs text-muted-foreground">Fecha</p>
          <p>{formatDate(entry.created_at)}</p>
        </div>
      </div>
      {entry.target_entity_type && (
        <div>
          <p className="text-xs text-muted-foreground">Entidad</p>
          <p>
            {entry.target_entity_type}: {entry.target_entity_id}
          </p>
        </div>
      )}
      {entry.authorization_reason && (
        <div>
          <p className="text-xs text-muted-foreground">Motivo de autorización</p>
          <p>{entry.authorization_reason}</p>
        </div>
      )}
      {entry.error_mensaje && (
        <div>
          <p className="text-xs text-muted-foreground">Error</p>
          <p className="text-destructive">{entry.error_mensaje}</p>
        </div>
      )}
      {entry.datos_enviados && (
        <div>
          <p className="text-xs text-muted-foreground">Datos enviados</p>
          <pre className="mt-1 rounded bg-muted p-2 text-xs overflow-auto max-h-48">
            {JSON.stringify(entry.datos_enviados, null, 2)}
          </pre>
        </div>
      )}
      {entry.datos_respuesta && (
        <div>
          <p className="text-xs text-muted-foreground">Respuesta</p>
          <pre className="mt-1 rounded bg-muted p-2 text-xs overflow-auto max-h-48">
            {JSON.stringify(entry.datos_respuesta, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}
