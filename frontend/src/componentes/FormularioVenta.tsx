/* [263A-16] Formulario de venta — reescrito con shadcn Input + Button + Label.
 * Multi-turno: cada turno seleccionado genera una venta independiente.
 * [283A-22] Soporte edición — acepta venta opcional para pre-rellenar y usar PUT.
 * [147A-F6] Integración multi-item: LineasVentaEditor para artículos individuales. */

import { Turno, CanalVenta, Venta } from '../api/generated';
import useFormularioVenta, { calcularIva } from '../hooks/useFormularioVenta';
import LineasVentaEditor from '../components/lineas-venta-editor';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

const ETIQUETAS_TURNO: Record<Turno, string> = {
    [Turno.manana]: 'Mañana',
    [Turno.mediodia]: 'Mediodía',
    [Turno.noche]: 'Noche',
};

interface Props {
    onExito?: () => void;
    /* [283A-22] venta opcional para modo edición */
    venta?: Venta;
}

function FormularioVenta({ onExito, venta }: Props) {
    const { campos, cambiarCampo, toggleTurno, cambiarDetalle, error, manejarEnvio, cargando, esEdicion, lineasHook } = useFormularioVenta(onExito, venta);

    return (
        <div>
            {error && (
                <div className="mb-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>
            )}

            <form className="flex flex-col gap-4" onSubmit={manejarEnvio}>
                <div className="grid grid-cols-2 gap-4">
                    <div className="flex flex-col gap-2">
                        <Label htmlFor="fecha">Fecha</Label>
                        <Input id="fecha" type="date" value={campos.fecha} onChange={e => cambiarCampo('fecha', e.target.value)} />
                    </div>
                    <div className="flex flex-col gap-2">
                        <Label htmlFor="comensales">Comensales</Label>
                        <Input id="comensales" type="number" min="1" value={campos.comensales} onChange={e => cambiarCampo('comensales', e.target.value)} placeholder="Opcional" />
                    </div>
                </div>

                <div className="flex flex-col gap-2">
                    <Label htmlFor="descripcion">Descripción</Label>
                    <Input id="descripcion" type="text" value={campos.descripcion} onChange={e => cambiarCampo('descripcion', e.target.value)} placeholder="Opcional" />
                </div>

                <div className="flex flex-col gap-2">
                    <Label>Turno(s)</Label>
                    <div className="flex gap-2">
                        {(Object.values(Turno) as Turno[]).map(t => (
                            <Button
                                key={t}
                                type="button"
                                variant={campos.turnos.includes(t) ? 'default' : 'outline'}
                                size="sm"
                                onClick={() => toggleTurno(t)}
                                aria-pressed={campos.turnos.includes(t)}
                            >
                                {ETIQUETAS_TURNO[t]}
                            </Button>
                        ))}
                    </div>
                </div>

                <div className="flex flex-col gap-2">
                    <Label htmlFor="canal">Canal</Label>
                    <Select value={campos.canal} onValueChange={v => cambiarCampo('canal', v as CanalVenta)}>
                        <SelectTrigger id="canal"><SelectValue /></SelectTrigger>
                        <SelectContent>
                            <SelectItem value={CanalVenta.comedor}>Comedor</SelectItem>
                            <SelectItem value={CanalVenta.barra}>Barra</SelectItem>
                            <SelectItem value={CanalVenta.terraza}>Terraza</SelectItem>
                            <SelectItem value={CanalVenta.delivery}>Delivery</SelectItem>
                            <SelectItem value={CanalVenta.just_eat}>Just Eat</SelectItem>
                            <SelectItem value={CanalVenta.eventos}>Eventos</SelectItem>
                        </SelectContent>
                    </Select>
                </div>

                {campos.turnos.map(t => {
                    const d = campos.detalles[t];
                    const iva = calcularIva(d.importeBase, campos.ivaPorcentaje);
                    const total = d.importeBase ? (parseFloat(d.importeBase) + parseFloat(iva)).toFixed(2) : '';
                    return (
                        <div key={t} className="rounded-md border p-3 flex flex-col gap-3">
                            <span className="text-sm font-medium">{ETIQUETAS_TURNO[t]}</span>
                            <div className="grid grid-cols-2 gap-4">
                                <div className="flex flex-col gap-2">
                                    <Label htmlFor={`importe-${t}`}>Importe base (€)</Label>
                                    <Input id={`importe-${t}`} type="number" step="0.01" min="0" value={d.importeBase} onChange={e => cambiarDetalle(t, 'importeBase', e.target.value)} placeholder="0.00" />
                                </div>
                                <div className="flex flex-col gap-2">
                                    <Label>IVA + Total</Label>
                                    <div className="flex h-9 items-center text-sm text-muted-foreground">{d.importeBase ? `+${iva}€ IVA = ${total}€` : '—'}</div>
                                </div>
                            </div>
                            <div className="flex flex-col gap-2">
                                <Label htmlFor={`pago-${t}`}>Método de pago</Label>
                                <Select value={d.metodoPago} onValueChange={v => cambiarDetalle(t, 'metodoPago', v)}>
                                    <SelectTrigger id={`pago-${t}`}><SelectValue /></SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="efectivo">Efectivo</SelectItem>
                                        <SelectItem value="tarjeta">Tarjeta</SelectItem>
                                        <SelectItem value="transferencia">Transferencia</SelectItem>
                                        <SelectItem value="otros">Otros</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                        </div>
                    );
                })}

                <details className="text-sm">
                    <summary className="cursor-pointer text-muted-foreground hover:text-foreground">Opciones avanzadas</summary>
                    <div className="mt-3 flex items-center gap-4">
                        <div className="flex flex-col gap-2">
                            <Label htmlFor="ivaPorcentaje">IVA %</Label>
                            <Input id="ivaPorcentaje" type="number" step="0.01" value={campos.ivaPorcentaje} onChange={e => cambiarCampo('ivaPorcentaje', e.target.value)} className="max-w-24" />
                        </div>
                    </div>
                </details>

                {/* [147A-F6] Editor de líneas multi-item para BDP */}
                {lineasHook.lineas.length > 0 ? (
                    <LineasVentaEditor
                        lineas={lineasHook.lineas}
                        onAgregar={lineasHook.agregarLinea}
                        onEliminar={lineasHook.eliminarLinea}
                        onActualizar={lineasHook.actualizarLinea}
                        totalBase={lineasHook.totalBase}
                        totalIva={lineasHook.totalIva}
                        totalConDescuento={lineasHook.totalConDescuento}
                    />
                ) : (
                    <div className="flex flex-col gap-2">
                        <div className="flex items-center justify-between">
                            <span className="text-sm text-muted-foreground">
                                ¿Añadir artículos individuales para BDP?
                            </span>
                            <Button
                                type="button"
                                variant="outline"
                                size="sm"
                                onClick={lineasHook.agregarLinea}
                                className="h-7 gap-1 text-xs"
                            >
                                + Añadir líneas
                            </Button>
                        </div>
                        <p className="text-xs text-muted-foreground">
                            Si añade líneas, el total se calculará a partir de los artículos.
                            Si no, se usa el importe manual como hasta ahora.
                        </p>
                    </div>
                )}

                <Button type="submit" className="w-full" disabled={cargando}>
                    {cargando
                        ? (esEdicion ? 'Guardando...' : 'Registrando...')
                        : esEdicion
                            ? 'Guardar cambios'
                            : `Registrar ${campos.turnos.length > 1 ? `${campos.turnos.length} ventas` : 'venta'}`
                    }
                </Button>
            </form>
        </div>
    );
}

export default FormularioVenta;

