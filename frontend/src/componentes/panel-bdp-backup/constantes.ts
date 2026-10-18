/* Constantes del panel de backup BDP: modos de sincronización y tipos de snapshot. */

import type {SyncMode} from '@/api/bdp-backup';

export const SYNC_MODES: {value: SyncMode; label: string; desc: string}[] = [
    {value: 'read_only', label: 'Solo lectura (BDP → Aplicación Web)', desc: 'Permite consultas e importaciones; la Aplicación Web no crea ni modifica datos en BDP.'},
    {value: 'unidirectional', label: 'Autorizar una operación (Aplicación Web → BDP)', desc: 'Permiso excepcional para un cliente o venta exactos. Se cierra después de una operación.'},
    {value: 'automatic', label: 'Automático (Aplicación Web → BDP sin pedir)', desc: 'Las escrituras se envían sin confirmación por operación, siempre auditadas. Reversible en un clic.'}
];

export const SNAPSHOT_TIPOS_BDP = ['articulos', 'clientes', 'departamentos', 'salones', 'empleados'];
export const SNAPSHOT_TIPOS_GLORY = ['ventas', 'clientes', 'mapeos'];
