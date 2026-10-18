import {Badge} from '@/components/ui/badge';

export function formatDate(iso: string): string {
    return new Date(iso).toLocaleString('es-ES', {
        day: '2-digit',
        month: 'short',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
    });
}

export function tipoSnapshotBadge(tipo: string) {
    const variants: Record<string, 'default' | 'secondary' | 'outline'> = {
        completo: 'default',
        parcial: 'secondary',
        glory: 'outline',
        pre_write: 'outline'
    };
    return <Badge variant={variants[tipo] ?? 'outline'}>{tipo}</Badge>;
}

export function datosResumen(datos: Record<string, unknown>): string {
    const parts: string[] = [];
    for (const [key, val] of Object.entries(datos)) {
        if (Array.isArray(val)) parts.push(`${val.length} ${key}`);
    }
    return parts.length > 0 ? parts.join(', ') : '—';
}

export function resultadoBadge(resultado: string) {
    if (resultado === 'exito')
        return (
            <Badge variant="default" className="bg-green-600">
                Completada
            </Badge>
        );
    if (resultado === 'error') return <Badge variant="destructive">Falló</Badge>;
    if (resultado === 'ambiguo') return <Badge variant="destructive">Requiere revisión</Badge>;
    if (resultado === 'pendiente') return <Badge variant="secondary">En proceso</Badge>;
    if (resultado === 'parcial') return <Badge variant="outline">Parcial</Badge>;
    return <Badge variant="outline">{resultado}</Badge>;
}

export function operacionLabel(operacion: string): string {
    const labels: Record<string, string> = {
        create_order: 'Crear comanda',
        create_customer: 'Crear cliente',
        add_payment: 'Registrar pago',
        invoice: 'Facturar',
        config_bootstrap: 'Preparar configuración inicial'
    };
    return labels[operacion] ?? operacion;
}

export function direccionLabel(direccion: string): string {
    if (direccion === 'glory_to_bdp') return 'Aplicación Web → BDP';
    if (direccion === 'bdp_to_glory') return 'BDP → Aplicación Web';
    if (direccion === 'internal') return 'Configuración de la Aplicación Web';
    return direccion;
}
