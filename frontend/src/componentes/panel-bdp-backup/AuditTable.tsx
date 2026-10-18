import {Badge} from '@/components/ui/badge';
import {Table, TableBody, TableCell, TableHead, TableHeader, TableRow} from '@/components/ui/table';
import type {BdpAuditEntry} from '@/api/bdp-backup';
import {direccionLabel, formatDate, operacionLabel, resultadoBadge} from './formato';

export function AuditTable({entries}: {entries: BdpAuditEntry[]}) {
    if (entries.length === 0) {
        return <p className="text-sm text-muted-foreground py-4 text-center">Sin registros de auditoría todavía.</p>;
    }

    return (
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead>Fecha</TableHead>
                    <TableHead>Operación</TableHead>
                    <TableHead>Dirección</TableHead>
                    <TableHead>Resultado</TableHead>
                    <TableHead>Registro</TableHead>
                    <TableHead>Motivo</TableHead>
                    <TableHead>Error</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {entries.map(e => (
                    <TableRow key={e.id}>
                        <TableCell className="text-[13px]">{formatDate(e.created_at)}</TableCell>
                        <TableCell>
                            <Badge variant="outline">{operacionLabel(e.operacion)}</Badge>
                        </TableCell>
                        <TableCell>
                            <Badge variant="outline">{direccionLabel(e.direccion)}</Badge>
                        </TableCell>
                        <TableCell>{resultadoBadge(e.resultado)}</TableCell>
                        <TableCell className="text-xs">
                            {e.target_entity_type ? `${e.target_entity_type}: ${e.target_entity_id?.slice(0, 8) ?? '—'}…` : '—'}
                        </TableCell>
                        <TableCell className="max-w-[220px] truncate text-xs">{e.authorization_reason ?? '—'}</TableCell>
                        <TableCell className="text-xs text-destructive max-w-[200px] truncate">{e.error_mensaje ?? '—'}</TableCell>
                    </TableRow>
                ))}
            </TableBody>
        </Table>
    );
}
