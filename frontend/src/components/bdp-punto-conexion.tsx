/* Punto semáforo de conexión BDP: verde = conectado, ámbar pulsante =
 * intermedio (cambiando de modo o sin respuesta), rojo = desconectado. */
export function PuntoConexion({ estado }: { estado: 'conectado' | 'intermedio' | 'desconectado' }) {
  const color =
    estado === 'conectado'
      ? 'bg-emerald-500'
      : estado === 'intermedio'
        ? 'bg-amber-500 animate-pulse'
        : 'bg-red-500'
  return <span aria-hidden className={`h-2 w-2 shrink-0 rounded-full ${color}`} />
}
