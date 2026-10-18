/* [267A-4] Funcionalidades BDP: interruptores de funciones avanzadas (feature flags),
 * modalidad de anulación, permisos operativos y permisos de albaranes. Los cambios
 * se aplican al instante, sin redeploy. */

import { Info, ToggleLeft } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { EstadoConfiguracion } from '@/hooks/useConfiguracion';
import type { CambiarCampo } from './tipos';

interface FuncionesBdpCardProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
}

export default function FuncionesBdpCard({ config, cambiarCampo }: FuncionesBdpCardProps) {
  return (
    <Card className="mt-4">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <ToggleLeft className="size-4" />
          Funcionalidades BDP
        </CardTitle>
        <CardDescription>
          Activa o desactiva funcionalidades avanzadas de la integración BDP. Los cambios se aplican inmediatamente sin necesidad de redeploy. Todos están desactivados por defecto por seguridad.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="ff-auto-arm">Auto-arming</Label>
            <p className="text-xs text-muted-foreground">Permite que las operaciones de escritura (comandas, pagos, facturas) activen automáticamente un permiso temporal sin ir a Configuración. Tras la operación vuelve a solo lectura.</p>
          </div>
          <Switch
            id="ff-auto-arm"
            checked={config.ff_bdp_auto_arm}
            onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_auto_arm', checked)}
          />
        </div>
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="ff-partial-payments">Pagos parciales</Label>
            <p className="text-xs text-muted-foreground">Permite pagar una comanda BDP en varios pagos parciales en vez de un único pago completo. Cada pago se registra en un ledger local con clave de idempotencia.</p>
          </div>
          <Switch
            id="ff-partial-payments"
            checked={config.ff_bdp_partial_payments}
            onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_partial_payments', checked)}
          />
        </div>
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <div className="flex items-center gap-2">
              <Label htmlFor="ff-cancel-order">Cancelar comandas</Label>
              <Badge variant="outline" className="text-xs bg-emerald-50 text-emerald-700 border-emerald-300 dark:bg-emerald-950/30 dark:text-emerald-400 dark:border-emerald-700">Verificado en BDP 2026-09-18</Badge>
              <TooltipButton
                variant="ghost"
                size="icon"
                className="size-5"
                tooltip="CancelOrder verificado funcionando contra el BDP real el 2026-09-18 (comanda 6338 cancelada vía API, auditoría «exito»). La anulación local encola venta/cancelar y el push la envía."
              >
                <Info className="size-3.5 text-muted-foreground" />
              </TooltipButton>
            </div>
            <p className="text-xs text-muted-foreground">Permite cancelar comandas directamente en BDP (verificado 2026-09-18: CancelOrder con éxito). La anulación local la encola y el push la envía.</p>
          </div>
          <Switch
            id="ff-cancel-order"
            checked={config.ff_bdp_cancel_order}
            onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_cancel_order', checked)}
          />
        </div>
        {/* [128A-1/F4] Modalidad de anulación local de ventas */}
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="anulacion-modalidad">Modalidad de anulación</Label>
            <p className="text-xs text-muted-foreground">
              «Crédito completo» exige motivo y revierte la venta en el resumen diario;
              «Solo estado» marca la venta como anulada sin exigir motivo.
            </p>
          </div>
          <Select
            value={config.anulacion_modalidad}
            onValueChange={(v) => cambiarCampo('anulacion_modalidad', v)}
          >
            <SelectTrigger id="anulacion-modalidad" className="w-52">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="credito_completo">Crédito completo</SelectItem>
              <SelectItem value="estado_solo">Solo estado</SelectItem>
            </SelectContent>
          </Select>
        </div>
        {/* [128A-1/F8] Permisos operativos por acción (D8/M17) */}
        <div className="border-t pt-3">
          <p className="text-xs font-medium text-muted-foreground mb-3">Permisos operativos</p>
          <p className="text-xs text-muted-foreground mb-3">
            Quién puede ejecutar cada acción sensible. El backend aplica el permiso en cada
            operación; la UI solo refleja la configuración.
          </p>
          <div className="grid gap-3 md:grid-cols-2">
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="permisos-catalogo-edicion">Catálogo</Label>
                <p className="text-xs text-muted-foreground">Crear/editar/eliminar artículos y mapeos.</p>
              </div>
              <Select
                value={config.permisos_catalogo_edicion}
                onValueChange={(v) => cambiarCampo('permisos_catalogo_edicion', v)}
              >
                <SelectTrigger id="permisos-catalogo-edicion" className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">Solo admin</SelectItem>
                  <SelectItem value="admin_trabajador">Admin y trabajadores</SelectItem>
                  <SelectItem value="todos">Todos</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="permisos-stock-ajuste">Ajuste de stock</Label>
                <p className="text-xs text-muted-foreground">Ajustes manuales de stock local.</p>
              </div>
              <Select
                value={config.permisos_stock_ajuste}
                onValueChange={(v) => cambiarCampo('permisos_stock_ajuste', v)}
              >
                <SelectTrigger id="permisos-stock-ajuste" className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">Solo admin</SelectItem>
                  <SelectItem value="admin_trabajador">Admin y trabajadores</SelectItem>
                  <SelectItem value="todos">Todos</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="permisos-albaranes-gestion">Albaranes de compra</Label>
                <p className="text-xs text-muted-foreground">Crear, editar, borrar, borrador y conciliar.</p>
              </div>
              <Select
                value={config.permisos_albaranes_gestion}
                onValueChange={(v) => cambiarCampo('permisos_albaranes_gestion', v)}
              >
                <SelectTrigger id="permisos-albaranes-gestion" className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">Solo admin</SelectItem>
                  <SelectItem value="admin_trabajador">Admin y trabajadores</SelectItem>
                  <SelectItem value="todos">Todos</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="permisos-anulacion-ventas">Anulación de ventas</Label>
                <p className="text-xs text-muted-foreground">Anular ventas locales.</p>
              </div>
              <Select
                value={config.permisos_anulacion_ventas}
                onValueChange={(v) => cambiarCampo('permisos_anulacion_ventas', v)}
              >
                <SelectTrigger id="permisos-anulacion-ventas" className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">Solo admin</SelectItem>
                  <SelectItem value="admin_trabajador">Admin y trabajadores</SelectItem>
                  <SelectItem value="todos">Todos</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="permisos-pagos-locales">Pagos parciales locales</Label>
                <p className="text-xs text-muted-foreground">Registrar pagos parciales en el ledger local.</p>
              </div>
              <Select
                value={config.permisos_pagos_locales}
                onValueChange={(v) => cambiarCampo('permisos_pagos_locales', v)}
              >
                <SelectTrigger id="permisos-pagos-locales" className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">Solo admin</SelectItem>
                  <SelectItem value="admin_trabajador">Admin y trabajadores</SelectItem>
                  <SelectItem value="todos">Todos</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="permisos-facturacion-local">Facturación local</Label>
                <p className="text-xs text-muted-foreground">Emitir facturas locales de ventas.</p>
              </div>
              <Select
                value={config.permisos_facturacion_local}
                onValueChange={(v) => cambiarCampo('permisos_facturacion_local', v)}
              >
                <SelectTrigger id="permisos-facturacion-local" className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">Solo admin</SelectItem>
                  <SelectItem value="admin_trabajador">Admin y trabajadores</SelectItem>
                  <SelectItem value="todos">Todos</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>
        <div className="border-t pt-3">
          <p className="text-xs font-medium text-muted-foreground mb-3">Compras (albaranes de proveedores BDP)</p>
          <div className="grid gap-3 md:grid-cols-3">
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="ff-pn-read">Lectura</Label>
                <p className="text-xs text-muted-foreground">Sincronizar albaranes de compra desde BDP.</p>
              </div>
              <Switch
                id="ff-pn-read"
                checked={config.ff_bdp_purchase_notes_read}
                onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_purchase_notes_read', checked)}
              />
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="ff-pn-draft">Borradores</Label>
                <p className="text-xs text-muted-foreground">Crear borradores de compra locales (sin escribir en BDP).</p>
              </div>
              <Switch
                id="ff-pn-draft"
                checked={config.ff_bdp_purchase_notes_draft}
                onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_purchase_notes_draft', checked)}
              />
            </div>
            <div className="flex items-center justify-between gap-4 rounded-md border p-3">
              <div>
                <Label htmlFor="ff-pn-receive">Conciliación</Label>
                <p className="text-xs text-muted-foreground">Recepcionar y conciliar compras con gastos existentes o nuevos.</p>
              </div>
              <Switch
                id="ff-pn-receive"
                checked={config.ff_bdp_purchase_notes_receive}
                onCheckedChange={(checked: boolean) => cambiarCampo('ff_bdp_purchase_notes_receive', checked)}
              />
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
