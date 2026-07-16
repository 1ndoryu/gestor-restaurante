/* [065A-2] Configuracion de BDP/WebLink REST API.
 * Mantiene credenciales fuera de respuestas publicas y ofrece diagnostico
 * Health + Login + GetVersion para la sesion remota con el PC del restaurante.
 * [147A-F5.6] Secciones de mapeo: tender, order_type, customer_code, poll_interval.
 * [167A-1] Simplificado: mapeos colapsados por defecto, defaults desde env. */

import { useState } from 'react';
import { Activity, CheckCircle2, ChevronDown, ChevronRight, ClipboardCheck, Loader2, XCircle } from 'lucide-react';
import axios from '@/api/axios-instance';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { toast } from 'sonner';
import type { EstadoConfiguracion } from '../hooks/useConfiguracion';
import type { BdpDiagnosticoResponse } from '../api/generated/gestionRestauranteAPI.schemas';
import ConfigBdpMapeos from '@/components/config-bdp-mapeos';

interface BdpSyncDryRunCheck {
  nombre: string;
  endpoint: string;
  ok: boolean;
  mensaje: string;
  cantidad?: number | null;
  muestra?: string | null;
}

interface BdpSyncDryRunResponse {
  configurado: boolean;
  sync_habilitado: boolean;
  escritura_real: boolean;
  listo_para_sincronizar: boolean;
  mensaje: string;
  checks: BdpSyncDryRunCheck[];
}

interface BdpUiState {
  diagnostico: BdpDiagnosticoResponse | null;
  dryRun: BdpSyncDryRunResponse | null;
  diagnosticando: boolean;
  probandoSync: boolean;
}

interface ConfigBdpProps {
  config: EstadoConfiguracion;
  cambiarCampo: <K extends keyof EstadoConfiguracion>(campo: K, valor: EstadoConfiguracion[K]) => void;
  guardar?: () => void;
  guardando?: boolean;
  mensaje?: string;
}

function ConfigBdp({ config, cambiarCampo, guardar, guardando, mensaje }: ConfigBdpProps) {
  const [estadoBdp, setEstadoBdp] = useState<BdpUiState>({
    diagnostico: null,
    dryRun: null,
    diagnosticando: false,
    probandoSync: false,
  });
  const [mostrarMapeos, setMostrarMapeos] = useState(false);
  const { diagnostico, dryRun, diagnosticando, probandoSync } = estadoBdp;

  async function diagnosticar() {
    setEstadoBdp((actual) => ({ ...actual, diagnosticando: true }));
    try {
      const resp = await axios.get<BdpDiagnosticoResponse>('/api/configuracion/bdp/diagnostico');
      setEstadoBdp((actual) => ({ ...actual, diagnostico: resp.data }));
      if (resp.data.health_ok && resp.data.login_ok) {
        toast.success('BDP conectado', { description: resp.data.mensaje });
      } else {
        toast.warning('BDP pendiente', { description: resp.data.mensaje });
      }
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message ?? 'No se pudo diagnosticar BDP';
      toast.error('Error BDP', { description: msg });
    } finally {
      setEstadoBdp((actual) => ({ ...actual, diagnosticando: false }));
    }
  }

  async function probarSincronizacion() {
    setEstadoBdp((actual) => ({ ...actual, probandoSync: true }));
    try {
      const resp = await axios.get<BdpSyncDryRunResponse>('/api/configuracion/bdp/sync-dry-run');
      setEstadoBdp((actual) => ({ ...actual, dryRun: resp.data }));
      if (resp.data.listo_para_sincronizar) {
        toast.success('Sincronización validada', { description: resp.data.mensaje });
      } else {
        toast.warning('Sincronización pendiente', { description: resp.data.mensaje });
      }
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message ?? 'No se pudo probar la sincronización BDP';
      toast.error('Error BDP', { description: msg });
    } finally {
      setEstadoBdp((actual) => ({ ...actual, probandoSync: false }));
    }
  }

  return (
    <>
    <Card>
      <CardHeader>
        <CardTitle>Conexión BDP</CardTitle>
        <CardDescription>Configura la conexión al TPV/BDP del restaurante. Los valores por defecto se cargan desde el servidor.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <Label htmlFor="bdp-base-url">URL pública BDP</Label>
          <Input
            id="bdp-base-url"
            type="url"
            value={config.bdp_base_url}
            onChange={(e) => cambiarCampo('bdp_base_url', e.target.value)}
            placeholder="https://ip-o-dominio:8080"
          />
        </div>
        <div className="grid gap-4 md:grid-cols-3">
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-login">Login</Label>
            <Input
              id="bdp-login"
              value={config.bdp_login}
              onChange={(e) => cambiarCampo('bdp_login', e.target.value)}
              autoComplete="off"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-password">Password</Label>
            <Input
              id="bdp-password"
              type="password"
              value={config.bdp_password}
              onChange={(e) => cambiarCampo('bdp_password', e.target.value)}
              autoComplete="new-password"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-integrator-code">Código integrador</Label>
            <Input
              id="bdp-integrator-code"
              type="password"
              value={config.bdp_integrator_code}
              onChange={(e) => cambiarCampo('bdp_integrator_code', e.target.value)}
              autoComplete="off"
            />
          </div>
        </div>
        <div className="grid gap-4 md:grid-cols-3">
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-pos-id">Terminal POS</Label>
            <Input
              id="bdp-pos-id"
              type="number"
              min={1}
              value={config.bdp_pos_id}
              onChange={(e) => cambiarCampo('bdp_pos_id', Number(e.target.value))}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-employee-id">Empleado</Label>
            <Input
              id="bdp-employee-id"
              type="number"
              min={1}
              value={config.bdp_employee_id}
              onChange={(e) => cambiarCampo('bdp_employee_id', Number(e.target.value))}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="bdp-items-profile-id">Perfil artículos</Label>
            <Input
              id="bdp-items-profile-id"
              type="number"
              min={1}
              value={config.bdp_items_profile_id}
              onChange={(e) => cambiarCampo('bdp_items_profile_id', Number(e.target.value))}
            />
          </div>
        </div>
        <div className="flex items-center justify-between">
          <Label htmlFor="bdp-sync-enabled">Sincronización BDP activa</Label>
          <Switch
            id="bdp-sync-enabled"
            checked={config.bdp_sync_enabled}
            onCheckedChange={(checked: boolean) => cambiarCampo('bdp_sync_enabled', checked)}
          />
        </div>

        {/* [167A-1] Mapeos colapsados por defecto — defaults desde env del servidor */}
        <div className="border-t pt-4">
          <button
            type="button"
            className="flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => setMostrarMapeos(!mostrarMapeos)}
          >
            {mostrarMapeos ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
            Configuración avanzada (mapeos)
          </button>
          {!mostrarMapeos && (
            <p className="mt-1 text-xs text-muted-foreground">
              Los mapeos de formas de pago, canales y artículos se cargan automáticamente desde la configuración del servidor.
              Solo modifica si necesitas personalizar la sincronización.
            </p>
          )}
          {mostrarMapeos && (
            <div className="mt-4">
              <ConfigBdpMapeos config={config} cambiarCampo={cambiarCampo} />
            </div>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Button type="button" variant="outline" onClick={diagnosticar} disabled={diagnosticando}>
            {diagnosticando ? <Loader2 className="size-4 animate-spin" /> : <Activity className="size-4" />}
            Probar conexión
          </Button>
          <Button type="button" variant="secondary" onClick={probarSincronizacion} disabled={probandoSync}>
            {probandoSync ? <Loader2 className="size-4 animate-spin" /> : <ClipboardCheck className="size-4" />}
            Probar sincronización segura
          </Button>
          {diagnostico && (
            <span className={diagnostico.health_ok && diagnostico.login_ok ? 'text-sm text-green-600' : 'text-sm text-destructive'}>
              {diagnostico.mensaje}
            </span>
          )}
        </div>
        {diagnostico?.version && (
          <div className="grid gap-2 rounded-md border p-3 text-sm md:grid-cols-2">
            <span>Versión: {diagnostico.version}.{diagnostico.sub_version ?? 0}</span>
            <span>Aplicación: {diagnostico.application_description || diagnostico.application}</span>
          </div>
        )}
        {dryRun && (
          <div className="flex flex-col gap-3 rounded-md border p-3 text-sm">
            <div className="flex items-start gap-2">
              {dryRun.listo_para_sincronizar ? (
                <CheckCircle2 className="mt-0.5 size-4 text-green-600" />
              ) : (
                <XCircle className="mt-0.5 size-4 text-destructive" />
              )}
              <div className="flex flex-col gap-1">
                <span className={dryRun.listo_para_sincronizar ? 'font-medium text-green-700' : 'font-medium text-destructive'}>
                  {dryRun.mensaje}
                </span>
                <span className="text-xs text-muted-foreground">
                  Escritura real: {dryRun.escritura_real ? 'sí' : 'no'} · Sync activo: {dryRun.sync_habilitado ? 'sí' : 'no'}
                </span>
              </div>
            </div>
            <div className="grid gap-2 md:grid-cols-2">
              {dryRun.checks.map((check) => (
                <div key={`${check.nombre}-${check.endpoint}`} className="rounded-md border p-2">
                  <div className="flex items-center gap-2">
                    {check.ok ? <CheckCircle2 className="size-4 text-green-600" /> : <XCircle className="size-4 text-destructive" />}
                    <span className="font-medium">{check.nombre}</span>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">{check.mensaje}</p>
                  {(check.cantidad !== null && check.cantidad !== undefined) || check.muestra ? (
                    <p className="mt-1 text-xs text-muted-foreground">
                      {check.cantidad !== null && check.cantidad !== undefined ? `${check.cantidad} registros` : check.muestra}
                    </p>
                  ) : null}
                </div>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>

    {/* [167A-1] Botón guardar propio de la pestaña BDP */}
    {guardar && (
      <div className="flex items-center gap-4 mt-4">
        <Button onClick={guardar} disabled={guardando}>
          {guardando ? 'Guardando...' : 'Guardar conexión BDP'}
        </Button>
        {mensaje && (
          <span className={`text-sm ${mensaje.includes('Error') ? 'text-destructive' : 'text-green-600'}`}>
            {mensaje}
          </span>
        )}
      </div>
    )}
    </>
  );
}

export default ConfigBdp;