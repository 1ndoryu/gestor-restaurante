/* [BKP-005] Panel de Backup BDP — snapshots, restauración, auditoría y modo sync.
 * Accesible desde /configuracion como sección expandible o como página independiente. */

import {Database, Download, Loader2, RefreshCw, Shield} from 'lucide-react';
import {Card, CardContent, CardDescription, CardHeader, CardTitle} from '@/components/ui/card';
import {Tabs, TabsContent, TabsList, TabsTrigger} from '@/components/ui/tabs';
import {useBdpSnapshots, useBdpAudit} from '@/api/bdp-backup';
import type {EstadoConfiguracion} from '@/hooks/useConfiguracion';
import {SyncModeSelector} from './panel-bdp-backup/SyncModeSelector';
import {SnapshotActions} from './panel-bdp-backup/SnapshotActions';
import {SnapshotTable} from './panel-bdp-backup/SnapshotTable';
import {AuditTable} from './panel-bdp-backup/AuditTable';

/* Panel principal */

interface PanelBdpBackupProps {
    config: EstadoConfiguracion;
}

export default function PanelBdpBackup({config}: PanelBdpBackupProps) {
    const {data: snapshots, isLoading: loadingSnapshots} = useBdpSnapshots();
    const {data: audit, isLoading: loadingAudit} = useBdpAudit();
    /* [039A-1/H-P1-03] Mismo criterio que PlanoSala (modo efectivo BDP =
     * solo configuración, sin red): controla los snapshots/permiso BDP. */
    const modoEfectivoBdp =
        config.modo_operacion === 'bdp' ||
        (config.modo_operacion === 'auto' &&
            config.bdp_sync_enabled &&
            Boolean(
                config.bdp_base_url &&
                    config.bdp_login &&
                    config.bdp_password &&
                    config.bdp_integrator_code
            ));

    return (
        <Card>
            <CardHeader>
                <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
                    <div>
                        <CardTitle className="flex items-center gap-2">
                            <Shield className="h-5 w-5" />
                            Seguridad, respaldos e historial BDP
                        </CardTitle>
                        <CardDescription>Consulta evidencias, respaldos locales y el resultado de cada escritura real autorizada.</CardDescription>
                    </div>
                    <div className="flex flex-col gap-1">
                        <span className="text-sm font-medium">Permiso de operación</span>
                        <SyncModeSelector
                            currentMode={config.bdp_sync_mode}
                            bdpBaseUrl={config.bdp_base_url}
                            deshabilitado={!modoEfectivoBdp}
                        />
                    </div>
                </div>
            </CardHeader>
            <CardContent>
                <Tabs defaultValue="snapshots">
                    <TabsList>
                        <TabsTrigger value="snapshots">
                            <Database className="h-4 w-4 mr-1" />
                            Snapshots
                        </TabsTrigger>
                        <TabsTrigger value="crear">
                            <Download className="h-4 w-4 mr-1" />
                            Crear
                        </TabsTrigger>
                        <TabsTrigger value="auditoria">
                            <RefreshCw className="h-4 w-4 mr-1" />
                            Auditoría
                        </TabsTrigger>
                    </TabsList>

                    <TabsContent value="snapshots" className="mt-4">
                        {loadingSnapshots ? (
                            <div className="flex items-center justify-center py-8">
                                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                            </div>
                        ) : (
                            <SnapshotTable snapshots={snapshots ?? []} />
                        )}
                    </TabsContent>

                    <TabsContent value="crear" className="mt-4">
                        <SnapshotActions bdpActivo={modoEfectivoBdp} />
                    </TabsContent>

                    <TabsContent value="auditoria" className="mt-4">
                        {loadingAudit ? (
                            <div className="flex items-center justify-center py-8">
                                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                            </div>
                        ) : (
                            <AuditTable entries={audit ?? []} />
                        )}
                    </TabsContent>
                </Tabs>
            </CardContent>
        </Card>
    );
}
