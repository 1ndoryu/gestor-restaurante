/* Estado y handlers de la exportación masiva de clientes a BDP (Glory → BDP).
 * Reutiliza el endpoint individual POST /api/clientes/:id/bdp-sync en secuencia.
 * SEGURIDAD: `simulacion` arranca en true y en ese modo NUNCA se llama al
 * endpoint — solo se valida localmente y se muestra el payload que se enviaría.
 * El modo real exige desactivar la simulación + frase exacta + códigos por fila. */
import { useState } from 'react';
import { toast } from 'sonner';
import { customInstance } from '@/api/axios-instance';
import type { Cliente, ClientesPaginados } from '@/api/generated';
import {
  fraseConfirmacionExportar,
  construirPayloadExportar,
  parseCodigoBdp,
  type FilaExportar,
} from '@/utils/exportarClientesBdp';

export interface ResultadoFila {
  id: string;
  nombre: string;
  codigo: number;
  url: string;
  confirmacion: string;
  estado: 'simulado' | 'ok' | 'error';
  detalle: string;
}

export interface ResultadoExportar {
  simulado: boolean;
  filas: ResultadoFila[];
}

const filaDe = (c: Cliente): FilaExportar => ({
  id: c.id,
  nombre: c.nombre,
  apellidos: c.apellidos,
  telefono: c.telefono ? `${c.prefijo_telefono} ${c.telefono}` : '',
  email: c.email,
});

export function useExportarClientesBdp(refrescar: () => void) {
  const [abierto, setAbierto] = useState(false);
  const [pendientes, setPendientes] = useState<Cliente[] | null>(null);
  const [cargando, setCargando] = useState(false);
  const [codigos, setCodigos] = useState<Record<string, string>>({});
  const [confirmacion, setConfirmacion] = useState('');
  const [simulacion, setSimulacion] = useState(true);
  const [ejecutando, setEjecutando] = useState(false);
  const [resultado, setResultado] = useState<ResultadoExportar | null>(null);

  /* Solo lectura (GET): carga todos los locales sin vínculo BDP. */
  const cargarPendientes = async () => {
    setCargando(true);
    setResultado(null);
    setConfirmacion('');
    try {
      const response = (await customInstance('/api/clientes?page=1&per_page=1000', {
        method: 'GET',
      })) as unknown as { data: ClientesPaginados };
      const sinVincular = (response.data?.items ?? []).filter((c) => !c.bdp_synced);
      setPendientes(sinVincular);
      setCodigos({});
      if (sinVincular.length === 0) toast.info('No hay clientes pendientes de subir a BDP');
    } catch (error) {
      const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
      toast.error('No se pudo cargar la lista de pendientes', { description: message ?? 'Revisa la conexión.' });
      setPendientes([]);
    } finally {
      setCargando(false);
    }
  };

  const abrir = () => {
    setAbierto(true);
    void cargarPendientes();
  };

  const setCodigo = (id: string, valor: string) =>
    setCodigos((prev) => ({ ...prev, [id]: valor }));

  const fraseEsperada = (lista: Cliente[]) => fraseConfirmacionExportar(lista.length);

  const ejecutar = async () => {
    const lista = pendientes ?? [];
    if (lista.length === 0) return;
    if (confirmacion.trim() !== fraseEsperada(lista)) return;
    const codigosNum = new Map<string, number>();
    for (const c of lista) {
      const codigo = parseCodigoBdp(codigos[c.id] ?? '');
      if (codigo === null) {
        toast.error(`Código BDP inválido para ${c.nombre} ${c.apellidos}`, {
          description: 'Cada pendiente necesita un código entero mayor que cero.',
        });
        return;
      }
      codigosNum.set(c.id, codigo);
    }

    setEjecutando(true);
    try {
      if (simulacion) {
        /* Camino simulado: cero llamadas POST, solo payload previsto. */
        const filas: ResultadoFila[] = lista.map((c) => {
          const codigo = codigosNum.get(c.id) as number;
          const payload = construirPayloadExportar(filaDe(c), codigo);
          return {
            id: c.id,
            nombre: `${c.nombre} ${c.apellidos}`.trim(),
            codigo,
            url: payload.url,
            confirmacion: payload.body.confirmacion,
            estado: 'simulado' as const,
            detalle: 'Simulado: no se envió nada a BDP.',
          };
        });
        setResultado({ simulado: true, filas });
        toast.success(`Simulación de ${filas.length} subidas completada sin escribir en BDP`);
        return;
      }
      /* Camino real: un POST por cliente, en secuencia, con resumen final. */
      const filas: ResultadoFila[] = [];
      for (const c of lista) {
        const codigo = codigosNum.get(c.id) as number;
        const payload = construirPayloadExportar(filaDe(c), codigo);
        try {
          await customInstance(payload.url, {
            method: 'POST',
            body: JSON.stringify(payload.body),
          });
          filas.push({
            id: c.id,
            nombre: `${c.nombre} ${c.apellidos}`.trim(),
            codigo,
            url: payload.url,
            confirmacion: payload.body.confirmacion,
            estado: 'ok',
            detalle: 'Subido a BDP.',
          });
        } catch (error) {
          const message = (error as { response?: { data?: { message?: string } } }).response?.data?.message;
          filas.push({
            id: c.id,
            nombre: `${c.nombre} ${c.apellidos}`.trim(),
            codigo,
            url: payload.url,
            confirmacion: payload.body.confirmacion,
            estado: 'error',
            detalle: message ?? 'Error al subir.',
          });
        }
      }
      setResultado({ simulado: false, filas });
      const ok = filas.filter((f) => f.estado === 'ok').length;
      toast.success(`Exportación real: ${ok}/${filas.length} clientes subidos a BDP`);
      refrescar();
    } finally {
      setEjecutando(false);
    }
  };

  return {
    exportarAbierto: abierto,
    setExportarAbierto: setAbierto,
    abrirExportar: abrir,
    pendientesExportar: pendientes,
    cargandoPendientes: cargando,
    codigosExportar: codigos,
    setCodigoExportar: setCodigo,
    confirmacionExportar: confirmacion,
    setConfirmacionExportar: setConfirmacion,
    fraseExportarEsperada: pendientes ? fraseEsperada(pendientes) : '',
    simulacionExportar: simulacion,
    setSimulacionExportar: setSimulacion,
    ejecutandoExportar: ejecutando,
    resultadoExportar: resultado,
    ejecutarExportar: ejecutar,
  };
}
