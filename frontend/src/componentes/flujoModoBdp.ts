/* [189A-1] Flujo compartido de cambio de modo BDP (`read_only` /
 * `unidirectional` / `automatic`) con diálogos propios (los nativos no
 * funcionan en el navegador empaquetado) y mensajes de error reales del
 * backend. Lo usan las tarjetas de `ConfigBdp`, el selector de
 * `PanelBdpBackup` y el menú del badge en `site-header`, para no triplicar
 * la secuencia confirmar → alcance/objetivo → mutación. [189A-7] Sin
 * diálogo de teclear el destino: el front ya conoce la URL configurada y
 * la envía directamente como `confirmarDestino` tras la confirmación. */

import { toast } from 'sonner';
import { useSetSyncMode, type SyncMode } from '@/api/bdp-backup';
import {
  confirmarConDialogo,
  pedirTextoConDialogo,
  elegirOpcionConDialogo,
} from './dialogoConfirmacion';

/* El backend devuelve el motivo en response.data.message (axios); err.message
 * solo trae "Request failed with status code ...". Los rechazos del extractor
 * axum llegan como texto plano: usarlos tal cual. */
export function mensajeErrorBackend(err: unknown): string {
  const datos = (err as { response?: { data?: unknown } })?.response?.data;
  if (typeof datos === 'string' && datos.length > 0) return datos.slice(0, 300);
  const mensaje = (datos as { message?: string } | null)?.message;
  if (typeof mensaje === 'string' && mensaje.length > 0) return mensaje;
  return String((err as { message?: string })?.message ?? 'Error desconocido');
}

const uuidRegex =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export function useFlujoModoBdp() {
  const setMode = useSetSyncMode();

  function mutate(
    input: Parameters<typeof setMode.mutate>[0],
    ok: string,
    ko: string,
  ) {
    setMode.mutate(input, {
      onSuccess: () => toast.success(ok),
      onError: (err: unknown) =>
        toast.error(ko, { description: mensajeErrorBackend(err) }),
    });
  }

  function baseLimpia(bdpBaseUrl: string): string {
    return bdpBaseUrl.trim().replace(/\/$/, '');
  }

  function exigirBaseUrl(bdpBaseUrl: string): string | null {
    const base = baseLimpia(bdpBaseUrl);
    if (!base) {
      toast.error('Sin URL BDP', {
        description: 'Configura y guarda primero la URL BDP de destino.',
      });
      return null;
    }
    return base;
  }

  /* Vuelta a Solo lectura: pide confirmación (es el freno consciente que
   * cierra cualquier modo de escritura). */
  async function volverSoloLectura(): Promise<void> {
    const confirmed = await confirmarConDialogo({
      titulo: '¿Volver a Solo lectura?',
      descripcion:
        'Se desactivan las escrituras hacia BDP/TPV. Las consultas e importaciones siguen activas.',
      textoConfirmar: 'Sí, volver',
    });
    if (!confirmed) return;
    mutate(
      {
        modo: 'read_only',
        confirmarDestino: '',
        alcances: [],
        duracionMinutos: 0,
        maxOperaciones: 0,
        motivo: '',
        targetEntityType: '',
        targetEntityId: '',
      },
      'BDP vuelve a modo solo lectura',
      'No se pudo cambiar el modo BDP',
    );
  }

  async function activarAutomatico(bdpBaseUrl: string): Promise<void> {
    const base = exigirBaseUrl(bdpBaseUrl);
    if (!base) return;
    const confirmed = await confirmarConDialogo({
      titulo: '¿Activar el modo automático?',
      descripcion:
        'El modo automático envía las escrituras a BDP/TPV sin pedir confirmación por operación (siempre auditadas). ' +
        'Confirma solo si hay autorización explícita.',
      textoConfirmar: 'Sí, continuar',
    });
    if (!confirmed) return;
    mutate(
      {
        modo: 'automatic',
        confirmarDestino: base,
        alcances: [],
        duracionMinutos: 0,
        maxOperaciones: 0,
        motivo: '',
        targetEntityType: '',
        targetEntityId: '',
      },
      'Modo automático activado',
      'No se pudo activar el modo automático',
    );
  }

  /* Autorización manual: confirmación + destino + una sola operación con
   * objetivo, motivo y duración (arming temporal que vuelve solo a lectura). */
  async function activarManual(bdpBaseUrl: string): Promise<void> {
    const base = exigirBaseUrl(bdpBaseUrl);
    if (!base) return;
    const confirmed = await confirmarConDialogo({
      titulo: '¿Habilitar escrituras en BDP/TPV?',
      descripcion:
        'Este modo habilita escrituras reales e irreversibles en BDP/TPV. ' +
        'Confirma únicamente si existe autorización explícita y se completó el checklist pre-write.',
      textoConfirmar: 'Sí, continuar',
    });
    if (!confirmed) return;
    const operacion = await elegirOpcionConDialogo({
      titulo: 'Elige una sola operación',
      opciones: [
        { valor: 'create_order', etiqueta: 'Crear comanda' },
        { valor: 'create_customer', etiqueta: 'Crear cliente' },
        { valor: 'add_payment', etiqueta: 'Registrar pago' },
        { valor: 'invoice', etiqueta: 'Facturar' },
      ],
    });
    if (!operacion) return;
    const targetEntityType: 'venta' | 'cliente' =
      operacion === 'create_customer' ? 'cliente' : 'venta';
    const entityId = await pedirTextoConDialogo({
      titulo: `Identificador del ${targetEntityType}`,
      descripcion: `Pega el identificador interno exacto (UUID) del ${targetEntityType} sobre el que se operará.`,
      placeholder: 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx',
      validar: (valor) =>
        uuidRegex.test(valor.trim()) ? null : 'Debe ser un UUID válido.',
    });
    if (entityId === null) return;
    const motivoEscrito = await pedirTextoConDialogo({
      titulo: 'Motivo del armado',
      descripcion: 'Describe brevemente quién autorizó la operación y para qué se realizará.',
      placeholder: 'Autorizado por … para …',
      validar: (valor) =>
        valor.trim().length >= 5 ? null : 'Mínimo 5 caracteres.',
    });
    if (motivoEscrito === null) return;
    const duracion = await pedirTextoConDialogo({
      titulo: 'Duración del armado',
      descripcion: 'Duración del armado en minutos (1-15).',
      placeholder: '5',
      validar: (valor) => {
        const n = Number(valor.trim());
        return Number.isInteger(n) && n >= 1 && n <= 15
          ? null
          : 'Introduce un entero entre 1 y 15.';
      },
    });
    if (duracion === null) return;
    mutate(
      {
        modo: 'unidirectional',
        confirmarDestino: base,
        alcances: [operacion],
        duracionMinutos: Number(duracion.trim()),
        maxOperaciones: 1,
        motivo: motivoEscrito.trim(),
        targetEntityType,
        targetEntityId: entityId.trim(),
      },
      'Autorización manual activada',
      'No se pudo activar la autorización manual',
    );
  }

  /* Atajo para el selector "Permiso de operación": read_only vuelve sin más
   * diálogos previos que la confirmación propia del freno consciente. */
  async function cambiarModo(modo: SyncMode, bdpBaseUrl: string): Promise<void> {
    if (modo === 'read_only') {
      await volverSoloLectura();
    } else if (modo === 'automatic') {
      await activarAutomatico(bdpBaseUrl);
    } else {
      await activarManual(bdpBaseUrl);
    }
  }

  return {
    activarAutomatico,
    activarManual,
    volverSoloLectura,
    cambiarModo,
    isPending: setMode.isPending,
  };
}
