/* [147A-F6] Utilidades extraídas de useFormularioVenta para cumplir límite de 120 líneas.
 * Contiene: camposIniciales, calcularIva, DetalleTurno, CamposVenta. */

import { Turno, CanalVenta, MetodoPago, type Venta } from '../api/generated';

export interface DetalleTurno {
  importeBase: string;
  metodoPago: MetodoPago;
}

export interface CamposVenta {
  fecha: string;
  comensales: string;
  descripcion: string;
  ivaPorcentaje: string;
  turnos: Turno[];
  canal: CanalVenta;
  detalles: Record<Turno, DetalleTurno>;
}

export const detallePorDefecto = (): DetalleTurno => ({ importeBase: '', metodoPago: MetodoPago.efectivo });

export function calcularIva(importeBase: string, ivaPorcentaje: string): string {
  const base = parseFloat(importeBase);
  const pct = parseFloat(ivaPorcentaje);
  if (isNaN(base) || isNaN(pct)) return '0.00';
  return (base * pct / 100).toFixed(2);
}

export function camposIniciales(ventaInicial?: Venta): CamposVenta {
  if (ventaInicial) {
    const turno = (ventaInicial.turno as Turno) || Turno.mediodia;
    const metodo = (ventaInicial.metodo_pago as MetodoPago) || MetodoPago.efectivo;
    const det = { importeBase: ventaInicial.importe_base, metodoPago: metodo };
    return {
      fecha: ventaInicial.fecha,
      comensales: ventaInicial.comensales?.toString() || '',
      descripcion: ventaInicial.descripcion || '',
      ivaPorcentaje: ventaInicial.iva_porcentaje,
      turnos: [turno],
      canal: (ventaInicial.canal as CanalVenta) || CanalVenta.comedor,
      detalles: {
        [Turno.manana]: turno === Turno.manana ? det : detallePorDefecto(),
        [Turno.mediodia]: turno === Turno.mediodia ? det : detallePorDefecto(),
        [Turno.noche]: turno === Turno.noche ? det : detallePorDefecto(),
      },
    };
  }
  return {
    fecha: new Date().toISOString().split('T')[0],
    comensales: '',
    descripcion: '',
    ivaPorcentaje: '10',
    turnos: [Turno.mediodia],
    canal: CanalVenta.comedor,
    detalles: {
      [Turno.manana]: detallePorDefecto(),
      [Turno.mediodia]: detallePorDefecto(),
      [Turno.noche]: detallePorDefecto(),
    },
  };
}
