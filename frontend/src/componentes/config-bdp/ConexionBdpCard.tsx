/* [065A-2] Tarjeta de conexión BDP: datos de conexión, integración, actualización
 * de estados, diagnóstico y simulador de envío. El simulador solo se habilita con
 * el BDP local, para no lanzar la escritura real contra el restaurante. */

import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Activity, ArrowRight, BookOpen, ChevronDown, ChevronRight, ClipboardCheck, Loader2, Radio, Settings } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import ConfigBdpMapeos from '@/components/config-bdp-mapeos';
import MaestrosBdpCard from './MaestrosBdpCard';
import ResultadoDiagnosticoBdp from './ResultadoDiagnosticoBdp';
import type { BdpDiagnosticoResponse } from '@/api/generated/gestionRestauranteAPI.schemas';
import type { EstadoConfiguracion } from '@/hooks/useConfiguracion';
import type { BdpSyncDryRunResponse, CambiarCampo } from './tipos';
import { AyudaTarjeta } from '../configuracion/AyudaTarjeta';

interface ConexionBdpCardProps {
  config: EstadoConfiguracion;
  cambiarCampo: CambiarCampo;
  diagnostico: BdpDiagnosticoResponse | null;
  dryRun: BdpSyncDryRunResponse | null;
  diagnosticando: boolean;
  probandoSync: boolean;
  onDiagnosticar: () => void;
  onProbarSincronizacion: () => void;
}

export default function ConexionBdpCard({
  config,
  cambiarCampo,
  diagnostico,
  dryRun,
  diagnosticando,
  probandoSync,
  onDiagnosticar,
  onProbarSincronizacion,
}: ConexionBdpCardProps) {
  const navigate = useNavigate();
  const [mostrarMapeos, setMostrarMapeos] = useState(false);
  const simuladorLocal = /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?\/?$/i.test(
    config.bdp_base_url.trim(),
  );

  return (
    <Card>
      <CardHeader>
        <CardTitle>Conexión BDP</CardTitle>
        <CardDescription>Conexión al TPV/BDP del restaurante. La puesta en marcha debe dejar estos datos preparados; el cliente no debe inventar códigos ni identificadores.</CardDescription>
        <CardAction>
          <AyudaTarjeta id="bdp-conexion" titulo="Conexión BDP" />
        </CardAction>
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
        {/* [D24-p] Los tres números de arriba se comprueban contra las listas
            reales del BDP: un id inventado solo se descubre al enviar. */}
        <MaestrosBdpCard
          terminal={config.bdp_pos_id}
          empleado={config.bdp_employee_id}
          perfil={config.bdp_items_profile_id}
          cadencia={Number(config.bdp_maestros_auto_segundos) || 0}
          onCadencia={(segundos) => cambiarCampo('bdp_maestros_auto_segundos', segundos)}
        />
        <div className="flex items-center justify-between gap-4 rounded-md border p-3">
          <div>
            <Label htmlFor="bdp-sync-enabled">Integración BDP activa</Label>
            <p className="text-xs text-muted-foreground">Interruptor general. Activarlo no concede permiso para crear clientes, comandas, pagos ni facturas.</p>
          </div>
          <Switch
            id="bdp-sync-enabled"
            checked={config.bdp_sync_enabled}
            onCheckedChange={(checked: boolean) => cambiarCampo('bdp_sync_enabled', checked)}
          />
        </div>

        <div className="grid gap-3 md:grid-cols-3">
          <div className="rounded-md border p-3">
            <p className="text-sm font-medium">BDP → Aplicación Web</p>
            <p className="mt-1 text-xs text-muted-foreground">Catálogo, clientes, mesas y estados se consultan o importan sin modificar BDP.</p>
          </div>
          <div className="rounded-md border p-3">
            <p className="text-sm font-medium">Aplicación Web → BDP</p>
            <p className="mt-1 text-xs text-muted-foreground">Solo una operación concreta con permiso temporal. Después vuelve a Solo lectura.</p>
          </div>
          <div className="rounded-md border p-3">
            <p className="text-sm font-medium">Dos vías automáticas</p>
            <p className="mt-1 text-xs text-muted-foreground">
              {config.push_modalidad === 'automatico'
                ? 'Activas: cada cambio sale a BDP y los estados se releen solos.'
                : 'En manual: los cambios esperan a que pulses «Enviar a BDP».'}
            </p>
          </div>
        </div>

        {/* [208A-2/C1] El CRUD de artículos ya no vive en Configuración (D1/D6):
         * se gestiona en la página Catálogo del menú. Aquí solo queda el aviso
         * y un enlace para no romper el acceso a quien lo buscaba aquí. */}
        <div className="border-t pt-4">
          <div className="flex items-center gap-2 mb-3">
            <BookOpen className="size-4 text-muted-foreground" />
            <span className="text-sm font-medium">Catálogo de artículos</span>
          </div>
          <p className="mb-3 text-xs text-muted-foreground">
            Los artículos (alta, edición, precios, IVA, activar/desactivar) se gestionan en la página
            <strong> Catálogo</strong> del menú lateral, junto a departamentos y familias. Desde allí también
            puedes sincronizar el catálogo desde BDP cuando la integración esté activa.
          </p>
          <Button type="button" variant="outline" size="sm" onClick={() => navigate('/bdp/catalogo')}>
            Ir a Catálogo
            <ArrowRight className="ml-1 size-3.5" />
          </Button>
        </div>

        {/* [237A-3] Polling automático — visible en vista principal */}
        <div className="border-t pt-4">
          <div className="flex items-center gap-2 mb-3">
            <Radio className="size-4 text-muted-foreground" />
            <span className="text-sm font-medium">Actualización de estados</span>
          </div>
          {/* Las dos piezas del mismo ajuste van en la misma caja: antes el
              interruptor tenía borde y la frecuencia no, y parecían cosas distintas. */}
          <div className="flex flex-col gap-4 rounded-md border p-3 md:flex-row md:items-center">
            <div className="flex flex-1 items-center justify-between gap-4">
              <div>
                <Label htmlFor="bdp-poll-enabled-main">Actualizar estados automáticamente</Label>
                <p className="text-xs text-muted-foreground">
                  Consulta periódicamente el estado de comandas BDP. Se enciende solo con el envío
                  automático; aquí puedes apagarlo. No crea ni modifica registros.
                </p>
              </div>
              <Switch
                id="bdp-poll-enabled-main"
                checked={config.bdp_poll_enabled}
                onCheckedChange={(checked: boolean) => cambiarCampo('bdp_poll_enabled', checked)}
              />
            </div>
            <div className="flex flex-col gap-2 md:w-56">
              <Label htmlFor="bdp-poll-interval-main">Frecuencia de actualización (segundos)</Label>
              <Input
                id="bdp-poll-interval-main"
                type="number"
                min={10}
                max={600}
                value={config.bdp_poll_interval_secs}
                onChange={(e) => cambiarCampo('bdp_poll_interval_secs', Number(e.target.value))}
              />
              <p className="text-xs text-muted-foreground">Cada cuántos segundos la Aplicación Web consulta el estado de comandas (10-600).</p>
            </div>
          </div>
        </div>

        {/* [237A-3] Configuración técnica — colapsable, contenido restante */}
        <div className="border-t pt-4">
          <button
            type="button"
            className="flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => setMostrarMapeos(!mostrarMapeos)}
          >
            {mostrarMapeos ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
            <Settings className="size-4" />
            Correspondencias Glory ↔ BDP (solo soporte)
          </button>
          {!mostrarMapeos && (
            <p className="mt-1 text-xs text-muted-foreground">
              Formas de pago, canales, artículo por defecto, cliente por defecto y exigir cliente confirmado.
              Ejemplo: «efectivo» → 1, «comedor» → 1; deben coincidir con el BDP real del restaurante.
              No debe modificarse sin verificar antes.
            </p>
          )}
          {mostrarMapeos && (
            <div className="mt-4">
              <p className="mb-4 rounded-md border p-3 text-xs text-muted-foreground">
                Estos valores no son universales: formas de pago y canales pueden ser distintos en cada BDP.
              </p>
              <ConfigBdpMapeos config={config} cambiarCampo={cambiarCampo} />
            </div>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Button type="button" variant="outline" onClick={onDiagnosticar} disabled={diagnosticando}>
            {diagnosticando ? <Loader2 className="size-4 animate-spin" /> : <Activity className="size-4" />}
            Probar conexión
          </Button>
          {/* [159A-2/F2] El dry-run no escribe: valida que el envío a BDP
           * funcionaría. "Probar conexión" es el diagnóstico de al lado. */}
          <Button type="button" variant="secondary" onClick={onProbarSincronizacion} disabled={probandoSync || !simuladorLocal} title="Simula el envío a BDP sin escribir nada">
            {probandoSync ? <Loader2 className="size-4 animate-spin" /> : <ClipboardCheck className="size-4" />}
            Probar envío (simulador)
          </Button>
          {diagnostico && (
            <span className={diagnostico.health_ok && diagnostico.login_ok ? 'text-sm text-green-600' : 'text-sm text-destructive'}>
              {diagnostico.mensaje}
            </span>
          )}
        </div>
        {!simuladorLocal && (
          <p className="text-xs text-muted-foreground">
            La validación de comandas está bloqueada contra el restaurante porque utiliza el mismo endpoint de creación. Solo puede ejecutarse con el simulador local.
          </p>
        )}
        <ResultadoDiagnosticoBdp diagnostico={diagnostico} dryRun={dryRun} />
      </CardContent>
    </Card>
  );
}
