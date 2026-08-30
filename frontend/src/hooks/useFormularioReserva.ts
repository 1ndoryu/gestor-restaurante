/* 253A-10: Hook para FormularioReserva
   263A-6: Añade num_mesa y apellidos_cliente
   283A-24: Mesa dropdown con mesa_id desde plano de sala
   024A-5: Soporte edición — acepta reserva existente, usa useActualizarReserva
   [034A-7] Autocomplete de clientes existentes al escribir nombre */

import { useState, useMemo, useEffect, useRef, FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useCrearReserva, useActualizarReserva, useObtenerPlano, useListarClientes, EstadoReserva, Reserva, Cliente } from '../api/generated';

interface CamposReserva {
  fecha: string;
  hora: string;
  nombreCliente: string;
  apellidosCliente: string;
  numPersonas: string;
  mesaId: string;
  telefono: string;
  notas: string;
  estado: EstadoReserva;
}

/* [024A-5] Convierte una Reserva existente en campos iniciales del formulario */
function camposDesdeReserva(r: Reserva): CamposReserva {
  return {
    fecha: r.fecha,
    hora: r.hora,
    nombreCliente: r.nombre_cliente,
    apellidosCliente: r.apellidos_cliente || '',
    numPersonas: String(r.num_personas),
    mesaId: r.mesa_id ?? '',
    telefono: r.telefono || '',
    notas: r.notas || '',
    estado: r.estado as EstadoReserva,
  };
}

const CAMPOS_VACIOS: CamposReserva = {
  fecha: new Date().toISOString().split('T')[0],
  hora: '20:00',
  nombreCliente: '',
  apellidosCliente: '',
  numPersonas: '2',
  mesaId: '',
  telefono: '',
  notas: '',
  estado: EstadoReserva.pendiente,
};

function useFormularioReserva(onExito?: () => void, reservaExistente?: Reserva) {
  const navigate = useNavigate();
  const [error, setError] = useState('');
  const esEdicion = !!reservaExistente;

  const [campos, setCampos] = useState<CamposReserva>(
    reservaExistente ? camposDesdeReserva(reservaExistente) : CAMPOS_VACIOS
  );

  const { data: planoData } = useObtenerPlano();
  const plano = planoData?.status === 200 ? planoData.data : null;

  /* [283A-24] Lista plana de todas las mesas de todas las zonas, con nombre de zona */
  const mesasDisponibles = useMemo(() => {
    if (!plano) return [];
    return plano.zonas.flatMap(z =>
      z.mesas
        .filter(m => m.activa)
        .map(m => ({ id: m.id, numero: m.numero, zona: z.nombre }))
    ).sort((a, b) => a.numero - b.numero);
  }, [plano]);

  const cambiarCampo = <K extends keyof CamposReserva>(campo: K, valor: CamposReserva[K]) => {
    setCampos(prev => ({ ...prev, [campo]: valor }));
  };

  /* [034A-7] Búsqueda de clientes con debounce de 300ms.
   * Se activa al escribir en el campo nombre (≥2 chars).
   * Al seleccionar un cliente, rellena nombre, apellidos y teléfono. */
  const [busquedaCliente, setBusquedaCliente] = useState('');
  const [autocompletarAbierto, setAutocompletarAbierto] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  /* [FIX-autocomplete] Al abrir el modal en edición el nombre llega pre-rellenado
   * (≥2 chars) y el debounce abría la lista sin interacción del usuario.
   * Solo se abre si el usuario escribió o enfocó el campo. */
  const interaccionClienteRef = useRef(false);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (campos.nombreCliente.length < 2) {
      setBusquedaCliente('');
      setAutocompletarAbierto(false);
      return;
    }
    debounceRef.current = setTimeout(() => {
      setBusquedaCliente(campos.nombreCliente);
      /* Buscar siempre (para tener sugerencias listas), abrir solo con interacción. */
      if (interaccionClienteRef.current) setAutocompletarAbierto(true);
    }, 300);
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current); };
  }, [campos.nombreCliente]);

  const marcarInteraccionCliente = () => { interaccionClienteRef.current = true; };

  const { data: clientesData } = useListarClientes(
    { busqueda: busquedaCliente, per_page: 8 },
    { query: { enabled: busquedaCliente.length >= 2 } },
  );
  const sugerenciasClientes = clientesData?.status === 200 ? clientesData.data.items : [];

  const seleccionarCliente = (cliente: Cliente) => {
    setCampos(prev => ({
      ...prev,
      nombreCliente: cliente.nombre,
      apellidosCliente: cliente.apellidos,
      telefono: cliente.telefono,
    }));
    setAutocompletarAbierto(false);
    setBusquedaCliente('');
  };

  const crearMutation = useCrearReserva({
    mutation: {
      onSuccess: (res) => {
        if (res.status === 201) {
          if (onExito) onExito();
          else navigate('/reservas');
        }
      },
      /* [303A-5] Extraer mensaje real del backend (ej: "No hay mesas disponibles",
       * "Capacidad insuficiente", "Mesa ya ocupada"). Axios guarda la respuesta
       * en error.response.data.message. Sin esto, el 409 solo aparecía en consola. */
      onError: (err: unknown) => {
        const axiosErr = err as { response?: { data?: { message?: string } } };
        const msg = axiosErr?.response?.data?.message;
        setError(msg || 'Error al crear la reserva');
      },
    },
  });

  /* [024A-5] Mutation de actualización para modo edición */
  const editarMutation = useActualizarReserva({
    mutation: {
      onSuccess: (res) => {
        if (res.status === 200) {
          if (onExito) onExito();
          else navigate('/reservas');
        }
      },
      onError: (err: unknown) => {
        const axiosErr = err as { response?: { data?: { message?: string } } };
        const msg = axiosErr?.response?.data?.message;
        setError(msg || 'Error al actualizar la reserva');
      },
    },
  });

  const manejarEnvio = (e: FormEvent) => {
    e.preventDefault();
    setError('');
    if (!campos.fecha || !campos.hora || !campos.nombreCliente || !campos.numPersonas) {
      setError('Completa los campos obligatorios');
      return;
    }
    /* [283A-24] Resolver mesa_id y num_mesa desde el dropdown */
    const mesaSel = mesasDisponibles.find(m => m.id === campos.mesaId);

    if (esEdicion && reservaExistente) {
      /* [024A-5] Modo edición: enviar PUT con los campos actualizados */
      editarMutation.mutate({
        id: reservaExistente.id,
        data: {
          fecha: campos.fecha,
          hora: campos.hora,
          nombre_cliente: campos.nombreCliente,
          num_personas: parseInt(campos.numPersonas, 10),
          telefono: campos.telefono || null,
          notas: campos.notas || null,
          estado: campos.estado,
          num_mesa: mesaSel ? mesaSel.numero : null,
          apellidos_cliente: campos.apellidosCliente || null,
          mesa_id: mesaSel ? mesaSel.id : null,
        },
      });
    } else {
      crearMutation.mutate({
        data: {
          fecha: campos.fecha,
          hora: campos.hora,
          nombre_cliente: campos.nombreCliente,
          num_personas: parseInt(campos.numPersonas, 10),
          telefono: campos.telefono || null,
          notas: campos.notas || null,
          estado: campos.estado,
          num_mesa: mesaSel ? mesaSel.numero : null,
          apellidos_cliente: campos.apellidosCliente || null,
          mesa_id: mesaSel ? mesaSel.id : null,
        },
      });
    }
  };

  return {
    campos,
    cambiarCampo,
    error,
    manejarEnvio,
    cargando: esEdicion ? editarMutation.isPending : crearMutation.isPending,
    mesasDisponibles,
    esEdicion,
    sugerenciasClientes,
    autocompletarAbierto,
    setAutocompletarAbierto,
    seleccionarCliente,
    marcarInteraccionCliente,
  };
}

export default useFormularioReserva;
