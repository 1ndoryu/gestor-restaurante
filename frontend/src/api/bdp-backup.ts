/* [BKP-005] Hooks manuales para endpoints de backup BDP.
 * Estos endpoints no están generados por Orval (no están en el spec OpenAPI aún).
 * Cuando se regenere el spec, estos hooks pueden reemplazarse por los generados. */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { customInstance } from '@/api/axios-instance';

/* ========== Tipos ========== */

export interface BdpSnapshot {
  id: string;
  user_id: string;
  tipo_snapshot: string;
  fecha_snapshot: string;
  datos: Record<string, unknown>;
  notas: string | null;
  tamano_bytes: number;
  cantidad_articulos: number | null;
  cantidad_clientes: number | null;
  cantidad_departamentos: number | null;
  cantidad_salones: number | null;
  cantidad_empleados: number | null;
  expira_en: string | null;
}

export interface BdpAuditEntry {
  id: string;
  user_id: string;
  operacion: string;
  bdp_order_id: number | null;
  resultado: string;
  snapshot_pre_id: string | null;
  datos_enviados: Record<string, unknown> | null;
  error_message: string | null;
  created_at: string;
}

export interface RestoreResult {
  exitoso: boolean;
  mensaje: string;
  entidades_restauradas: number;
  errores: string[];
}

export type SyncMode = 'read_only' | 'unidirectional' | 'bidirectional';

/* ========== Fetchers ========== */

async function fetchSnapshots(limit = 50): Promise<BdpSnapshot[]> {
  return customInstance<BdpSnapshot[]>(`/api/bdp/backup/snapshots?limit=${limit}`, {
    method: 'GET',
  });
}

async function fetchAudit(limit = 100): Promise<BdpAuditEntry[]> {
  return customInstance<BdpAuditEntry[]>(`/api/bdp/audit?limit=${limit}`, {
    method: 'GET',
  });
}

async function createSnapshotCompleto(notas?: string): Promise<BdpSnapshot> {
  return customInstance<BdpSnapshot>('/api/bdp/backup/completo', {
    method: 'POST',
    body: JSON.stringify(notas ?? null),
  });
}

async function createSnapshotParcial(
  tipos: string[],
  notas?: string
): Promise<BdpSnapshot> {
  return customInstance<BdpSnapshot>('/api/bdp/backup/parcial', {
    method: 'POST',
    body: JSON.stringify({ tipos, notas }),
  });
}

async function createSnapshotGlory(
  tipos: string[],
  notas?: string
): Promise<BdpSnapshot> {
  return customInstance<BdpSnapshot>('/api/bdp/backup/glory', {
    method: 'POST',
    body: JSON.stringify({ tipos, notas }),
  });
}

async function deleteSnapshot(id: string): Promise<void> {
  return customInstance<void>(`/api/bdp/backup/snapshots/${id}`, {
    method: 'DELETE',
  });
}

async function restoreSnapshot(id: string): Promise<RestoreResult> {
  return customInstance<RestoreResult>(`/api/bdp/backup/restaurar/${id}`, {
    method: 'POST',
  });
}

async function setSyncMode(modo: SyncMode): Promise<unknown> {
  return customInstance('/api/configuracion/bdp/sync-mode', {
    method: 'PUT',
    body: JSON.stringify({ modo }),
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
    mutationFn: (id: string) => restoreSnapshot(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bdp-snapshots'] });
      queryClient.invalidateQueries({ queryKey: ['bdp-audit'] });
    },
  });
}

export function useSetSyncMode() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (modo: SyncMode) => setSyncMode(modo),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['configuracion'] });
    },
  });
}
