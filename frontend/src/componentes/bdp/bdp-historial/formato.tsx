/* Formato del historial BDP: fechas, etiquetas de operación y dirección, y badges de
 * resultado y origen. Funciones de presentación compartidas por la tabla y los detalles. */
import { Badge } from '@/components/ui/badge';

export function formatDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleString('es-ES', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function resultadoBadge(resultado: string) {
  if (resultado === 'exito') return <Badge className="bg-green-600 hover:bg-green-600">Completada</Badge>;
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
    config_bootstrap: 'Preparar configuración',
    pago_parcial_local: 'Pago parcial local',
    factura_local: 'Factura local',
    anular_venta: 'Anular venta',
    menu_local_crear: 'Crear menú local',
    menu_local_actualizar: 'Actualizar menú local',
    menu_local_eliminar: 'Eliminar menú local',
    stock_ajuste: 'Ajuste de stock',
    inventory: 'Conteo de inventario',
    regularize_stock: 'Regularizar stock',
    albaran_local_crear: 'Crear albarán local',
    albaran_local_borrador: 'Pasar albarán a borrador',
    albaran_local_conciliar: 'Conciliar albarán',
    gasto_local_crear: 'Crear gasto',
  };
  return labels[operacion] ?? operacion;
}

export function direccionLabel(direccion: string): string {
  if (direccion === 'glory_to_bdp') return 'Aplicación Web → BDP';
  if (direccion === 'bdp_to_glory') return 'BDP → Aplicación Web';
  if (direccion === 'internal') return 'Configuración de la Aplicación Web';
  return direccion;
}

/* [128A-1/F6] Origen de la operación: 'local' (anulaciones, ajustes de stock,
 * pagos y facturas locales) o 'bdp' (default — implican al BDP). */
export function origenBadge(origen: string) {
  if (origen === 'local') {
    return <Badge className="bg-sky-600 hover:bg-sky-600">Local</Badge>;
  }
  return <Badge variant="secondary">BDP</Badge>;
}
