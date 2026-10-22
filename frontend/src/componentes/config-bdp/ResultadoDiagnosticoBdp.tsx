/* [065A-2] Resultado del diagnóstico y del simulador de envío de la conexión
 * BDP. Vive aparte de `ConexionBdpCard` porque son dos bloques de lectura, no
 * campos: lo que se edita y lo que se comprueba no deben crecer en el mismo
 * archivo (y el de arriba ya pasaba del límite de líneas). */

import { CheckCircle2, XCircle } from 'lucide-react';
import type { BdpDiagnosticoResponse } from '@/api/generated/gestionRestauranteAPI.schemas';
import type { BdpSyncDryRunResponse } from './tipos';

interface Props {
  diagnostico: BdpDiagnosticoResponse | null;
  dryRun: BdpSyncDryRunResponse | null;
}

export default function ResultadoDiagnosticoBdp({ diagnostico, dryRun }: Props) {
  return (
    <>
      {diagnostico?.version && (
        <div className="grid gap-2 rounded-md border p-3 text-sm md:grid-cols-2">
          <span>Versión: {diagnostico.version}.{diagnostico.sub_version ?? 0}</span>
          <span>Aplicación: {diagnostico.application_description || diagnostico.application}</span>
          {/* [L10/D01] Cuarto valor de la ficha, junto a salud, acceso y versión. */}
          <span className={diagnostico.licencia_ok ? 'text-green-600' : 'text-destructive'}>
            Licencia 84: {diagnostico.licencia
              ? `v${diagnostico.licencia.version}.${diagnostico.licencia.subversion} (${diagnostico.licencia.application_description})`
              : 'sin respuesta'}
          </span>
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
    </>
  );
}
