/* Tipos de diálogo para reemplazar prompt/confirm nativos del plano de sala.
 * Se re-exportan desde usePlanoSala.ts para no romper a los importadores. */
export interface DialogoEntrada {
  titulo: string;
  label: string;
  valorInicial: string;
  tipo?: 'text' | 'number';
  onConfirmar: (valor: string) => void;
}
export interface DialogoConfirmar {
  titulo: string;
  mensaje: string;
  onConfirmar: () => void;
}
export interface DialogoCombinacion {
  onConfirmar: (nombre: string, maxPersonas: number, mesaIds: string[]) => void;
}
