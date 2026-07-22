/* [BKP-005] Hooks manuales para endpoints de backup BDP.
 * Estos endpoints no están generados por Orval (no están en el spec OpenAPI aún).
 * Cuando se regenere el spec, estos hooks pueden reemplazarse por los generados. */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { customInstance } from '@/api/axios-instance';

/* ========== Tipos ========== */

export interface BdpSnapshot {
  id: string;
  user_id: string;
  tipo: string;
  direccion: string;
  trigger_tipo: string;
  datos: Record<string, unknown>;
  metadata: Record<string, unknown> | null;
  target_base_url: string | null;
  connection_fingerprint: string | null;
  created_at: string;
  expires_at: string | null;
  notas: string | null;
}

export interface BdpAuditEntry {
  id: string;
  user_id: string;
  operacion: string;
  direccion: string;
  snapshot_pre_id: string | null;
  datos_enviados: Record<string, unknown> | null;
  resultado: string;
  datos_respuesta: Record<string, unknown> | null;
  error_mensaje: string | null;
  target_base_url: string | null;
  target_entity_type: string | null;
  target_entity_id: string | null;
  authorization_reason: string | null;
  created_at: string;
  updated_at: string;
}

export interface RestoreResult {
  snapshot_id: string;
  tipo: string;
  registros_restaurados: number;
  errores: number;
  detalles: string;
}

export type SyncMode = 'read_only' | 'unidirectional';

/* ========== Fetchers ========== */

async function fetchSnapshots(limit = 50): Promise<BdpSnapshot[]> {
  const res = await customInstance<{ data: BdpSnapshot[] }>(`/api/bdp/backup/snapshots?limit=${limit}`, {
    method: 'GET',
  });
  return res.data;
}

async function fetchAudit(limit = 100): Promise<BdpAuditEntry[]> {
  const res = await customInstance<{ data: BdpAuditEntry[] }>(`/api/bdp/audit?limit=${limit}`, {
    method: 'GET',
  });
  return res.data;
}

async function createSnapshotCompleto(notas?: string): Promise<BdpSnapshot> {
  const res = await customInstance<{ data: BdpSnapshot }>('/api/bdp/backup/completo', {
    method: 'POST',
    body: JSON.stringify(notas ?? null),
  });
  return res.data;
}

async function createSnapshotParcial(
  tipos: string[],
  notas?: string
): Promise<BdpSnapshot> {
  const res = await customInstance<{ data: BdpSnapshot }>('/api/bdp/backup/parcial', {
    method: 'POST',
    body: JSON.stringify({ tipos, notas }),
  });
  return res.data;
}

async function createSnapshotGlory(
  tipos: string[],
  notas?: string
): Promise<BdpSnapshot> {
  const res = await customInstance<{ data: BdpSnapshot }>('/api/bdp/backup/glory', {
    method: 'POST',
    body: JSON.stringify({ tipos, notas }),
  });
  return res.data;
}

async function deleteSnapshot(id: string): Promise<void> {
  await customInstance(`/api/bdp/backup/snapshots/${id}`, {
    method: 'DELETE',
  });
}

async function restoreSnapshot(id: string, confirmacion: string): Promise<RestoreResult> {
  const res = await customInstance<{ data: RestoreResult }>(`/api/bdp/backup/restaurar/${id}`, {
    method: 'POST',
    body: JSON.stringify({ confirmacion }),
  });
  return res.data;
}

interface SetSyncModeInput {
  modo: SyncMode;
  confirmarDestino: string;
  alcances: string[];
  duracionMinutos: number;
  maxOperaciones: number;
  motivo: string;
  targetEntityType: 'venta' | 'cliente' | '';
  targetEntityId: string;
}

async function setSyncMode(input: SetSyncModeInput): Promise<unknown> {
  const {modo, confirmarDestino, alcances, duracionMinutos, maxOperaciones, motivo, targetEntityType, targetEntityId} = input;
  return customInstance('/api/configuracion/bdp/sync-mode', {
    method: 'PUT',
    body: JSON.stringify({
      modo,
      confirmar_escritura: modo !== 'read_only',
      confirmar_destino: modo === 'read_only' ? '' : confirmarDestino,
      alcances: modo === 'read_only' ? [] : alcances,
      duracion_minutos: modo === 'read_only' ? 0 : duracionMinutos,
      max_operaciones: modo === 'read_only' ? 0 : maxOperaciones,
      motivo: modo === 'read_only' ? '' : motivo,
      target_entity_type: modo === 'read_only' ? null : targetEntityType,
      target_entity_id: modo === 'read_only' ? null : targetEntityId,
    }),
  });
}

/* ========== React Query hooks ========== */

export function useBdpSnapshots(limit = 50) {
  return useQuery({
    queryKey: ['bdp-snapshots', limit],
    queryFn: () => fetchSnapshots(limit),
  });
}

export function useBdpAudit(limit = 100) {
  return useQuery({
    queryKey: ['bdp-audit', limit],
    queryFn: () => fetchAudit(limit),
  });
}

export function useCreateSnapshotCompleto() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (notas?: string) => createSnapshotCompleto(notas),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bdp-snapshots'] });
    },
  });
}

export function useCreateSnapshotParcial() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ tipos, notas }: { tipos: string[]; notas?: string }) =>
      createSnapshotParcial(tipos, notas),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bdp-snapshots'] });
    },
  });
}

export function useCreateSnapshotGlory() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ tipos, notas }: { tipos: string[]; notas?: string }) =>
      createSnapshotGlory(tipos, notas),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bdp-snapshots'] });
    },
  });
}

export function useDeleteSnapshot() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteSnapshot(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bdp-snapshots'] });
    },
  });
}

export function useRestoreSnapshot() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, confirmacion }: { id: string; confirmacion: string }) => restoreSnapshot(id, confirmacion),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bdp-snapshots'] });
      queryClient.invalidateQueries({ queryKey: ['bdp-audit'] });
    },
  });
}

export function useSetSyncMode() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: SetSyncModeInput) => setSyncMode(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['configuracion'] });
    },
  });
}
