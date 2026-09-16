/* Lógica pura de la exportación masiva de clientes a BDP (Glory → BDP).
 * Sin imports de React ni de red: testeable con node sin tocar BDP.
 * El endpoint real es POST /api/clientes/:id/bdp-sync, que exige por cliente
 * un código BDP explícito > 0 y la confirmación exacta
 * "CREAR CLIENTE {nombre} {apellidos} {codigo}". */

export interface FilaExportar {
  id: string;
  nombre: string;
  apellidos: string;
  telefono: string;
  email: string;
}

/** Frase global que desbloquea la exportación masiva de N pendientes. */
export function fraseConfirmacionExportar(total: number): string {
  return `EXPORTAR ${total} CLIENTES A BDP`;
}

/** Confirmación exacta que exige el backend para un cliente concreto. */
export function fraseConfirmacionCliente(fila: FilaExportar, codigo: number): string {
  return `CREAR CLIENTE ${fila.nombre} ${fila.apellidos} ${codigo}`;
}

/** Payload exacto que se enviaría al endpoint de subida individual. */
export function construirPayloadExportar(fila: FilaExportar, codigo: number): {
  url: string;
  body: { bdp_customer_code: number; confirmacion: string };
} {
  return {
    url: `/api/clientes/${fila.id}/bdp-sync`,
    body: {
      bdp_customer_code: codigo,
      confirmacion: fraseConfirmacionCliente(fila, codigo),
    },
  };
}

/** Valida un código BDP tecleado: entero mayor que cero. */
export function parseCodigoBdp(valor: string): number | null {
  if (!/^\d+$/.test(valor.trim())) return null;
  const codigo = Number(valor.trim());
  return codigo > 0 ? codigo : null;
}
