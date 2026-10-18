import {useState} from 'react';
import {Button} from '@/components/ui/button';
import {Dialog, DialogContent, DialogHeader, DialogTitle} from '@/components/ui/dialog';
import FormularioVenta from '../FormularioVenta';
import FormularioGasto from '../FormularioGasto';
import FormularioReserva from '../FormularioReserva';

/* [283A-7] Acciones rápidas extraídas al header del dashboard.
 * Muestran botones + abren modales de creación. */
export function AccionesRapidas() {
    const [modalVenta, setModalVenta] = useState(false);
    const [modalGasto, setModalGasto] = useState(false);
    const [modalReserva, setModalReserva] = useState(false);

    return (
        <>
            <div className="flex flex-wrap gap-2">
                <Button onClick={() => setModalVenta(true)} className="bg-green-600 hover:bg-green-700 text-white">
                    + Venta
                </Button>
                <Button onClick={() => setModalGasto(true)} variant="default">
                    + Gasto
                </Button>
                <Button onClick={() => setModalReserva(true)} variant="secondary">
                    + Reserva
                </Button>
            </div>
            <Dialog open={modalVenta} onOpenChange={setModalVenta}>
                <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
                    <DialogHeader><DialogTitle>Nueva Venta</DialogTitle></DialogHeader>
                    <FormularioVenta onExito={() => setModalVenta(false)} />
                </DialogContent>
            </Dialog>
            <Dialog open={modalGasto} onOpenChange={setModalGasto}>
                {/* [283A-30] sm:max-w-lg = 512px (tarea 18) */}
                <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
                    <DialogHeader><DialogTitle>Nuevo Gasto</DialogTitle></DialogHeader>
                    <FormularioGasto onExito={() => setModalGasto(false)} />
                </DialogContent>
            </Dialog>
            <Dialog open={modalReserva} onOpenChange={setModalReserva}>
                <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
                    <DialogHeader><DialogTitle>Nueva Reserva</DialogTitle></DialogHeader>
                    <FormularioReserva onExito={() => setModalReserva(false)} />
                </DialogContent>
            </Dialog>
        </>
    );
}
