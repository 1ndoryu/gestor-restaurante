/* [263A-25] Estado del formulario de NuevaReglaDialog, extraído a un hook.
 * Motivo: el diálogo acumulaba varios useState y la regla usestate-excesivo (max 3)
 * lo marcaba. Al separar el estado del formulario en un hook custom, el componente
 * queda con un único useState (open) y la lógica de creación queda testeable y
 * reutilizable. [por que] El proto del área exige extraer lógica >3 useState a hook. */

import { useState } from 'react';

export type TipoRecordatorio = 'antes' | 'despues';

export interface DatosCrearRegla {
  data: {
    nombre: string;
    horas_antes?: number;
    horas_despues?: number;
    tipo?: string;
    canal: string;
    mensaje_plantilla?: string;
  };
}

export function useNuevaReglaForm(onCrear: (data: DatosCrearRegla) => Promise<unknown>) {
  const [open, setOpen] = useState(false);
  const [nombre, setNombre] = useState('');
  const [tipo, setTipo] = useState<TipoRecordatorio>('antes');
  const [horas, setHoras] = useState('24');
  const [canal, setCanal] = useState('sms');
  const [mensaje, setMensaje] = useState('');
  const [enviando, setEnviando] = useState(false);

  const reset = () => {
    setNombre('');
    setTipo('antes');
    setHoras('24');
    setCanal('sms');
    setMensaje('');
  };

  const handleCrear = async () => {
    if (!nombre.trim() || enviando) return;
    setEnviando(true);
    const h = parseInt(horas, 10) || 24;
    try {
      await onCrear({
        data: {
          nombre: nombre.trim(),
          tipo,
          ...(tipo === 'antes' ? { horas_antes: h } : { horas_despues: h }),
          canal,
          mensaje_plantilla: mensaje.trim() || undefined,
        },
      });
      setOpen(false);
      reset();
    } finally {
      setEnviando(false);
    }
  };

  return {
    open, setOpen,
    nombre, setNombre,
    tipo, setTipo,
    horas, setHoras,
    canal, setCanal,
    mensaje, setMensaje,
    enviando, handleCrear,
  };
}