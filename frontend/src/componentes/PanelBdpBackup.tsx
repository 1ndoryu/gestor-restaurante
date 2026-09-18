/* [BKP-005] Panel de Backup BDP — snapshots, restauración, auditoría y modo sync.
 * Accesible desde /configuracion como sección expandible o como página independiente. */

import {useState} from 'react';
import {Database, Download, Loader2, RefreshCw, Shield, Trash2, Upload} from 'lucide-react';
import {Button} from '@/components/ui/button';
import {TooltipButton} from '@/components/ui/tooltip-button';
import {Card, CardContent, CardDescription, CardHeader, CardTitle} from '@/components/ui/card';
import {Badge} from '@/components/ui/badge';
import {Tabs, TabsContent, TabsList, TabsTrigger} from '@/components/ui/tabs';
import {Select, SelectContent, SelectItem, SelectTrigger, SelectValue} from '@/components/ui/select';
import {Table, TableBody, TableCell, TableHead, TableHeader, TableRow} from '@/components/ui/table';
import {Textarea} from '@/components/ui/textarea';
import {toast} from 'sonner';
import { confirmarConDialogo, elegirOpcionConDialogo, pedirTextoConDialogo } from './dialogoConfirmacion';
import {useBdpSnapshots, useBdpAudit, useCreateSnapshotCompleto, useCreateSnapshotParcial, useCreateSnapshotGlory, useDeleteSnapshot, useRestoreSnapshot, useSetSyncMode, type BdpSnapshot, type BdpAuditEntry, type SyncMode} from '@/api/bdp-backup';
import type {EstadoConfiguracion} from '@/hooks/useConfiguracion';

/* Constantes */

const SYNC_MODES: {value: SyncMode; label: string; desc: string}[] = [
    {value: 'read_only', label: 'Solo lectura (BDP → Aplicación Web)', desc: 'Permite consultas e importaciones; la Aplicación Web no crea ni modifica datos en BDP.'},
    {value: 'unidirectional', label: 'Autorizar una operación (Aplicación Web → BDP)', desc: 'Permiso excepcional para un cliente o venta exactos. Se cierra después de una operación.'},
    {value: 'automatic', label: 'Automático (Aplicación Web → BDP sin pedir)', desc: 'Las escrituras se envían sin confirmación por operación, siempre auditadas. Reversible en un clic.'}
];

const SNAPSHOT_TIPOS_BDP = ['articulos', 'clientes', 'departamentos', 'salones', 'empleados'];
const SNAPSHOT_TIPOS_GLORY = ['ventas', 'clientes', 'mapeos'];

/* Helpers */

function formatDate(iso: string): string {
    return new Date(iso).toLocaleString('es-ES', {
        day: '2-digit',
        month: 'short',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
    });
}

function tipoSnapshotBadge(tipo: string) {
    const variants: Record<string, 'default' | 'secondary' | 'outline'> = {
        completo: 'default',
        parcial: 'secondary',
        glory: 'outline',
        pre_write: 'outline'
    };
    return <Badge variant={variants[tipo] ?? 'outline'}>{tipo}</Badge>;
}

function datosResumen(datos: Record<string, unknown>): string {
    const parts: string[] = [];
    for (const [key, val] of Object.entries(datos)) {
        if (Array.isArray(val)) parts.push(`${val.length} ${key}`);
    }
    return parts.length > 0 ? parts.join(', ') : '—';
}

function resultadoBadge(resultado: string) {
    if (resultado === 'exito')
        return (
            <Badge variant="default" className="bg-green-600">
                Completada
            </Badge>
        );
    if (resultado === 'error') return <Badge variant="destructive">Falló</Badge>;
    if (resultado === 'ambiguo') return <Badge variant="destructive">Requiere revisión</Badge>;
    if (resultado === 'pendiente') return <Badge variant="secondary">En proceso</Badge>;
    if (resultado === 'parcial') return <Badge variant="outline">Parcial</Badge>;
    return <Badge variant="outline">{resultado}</Badge>;
}

function operacionLabel(operacion: string): string {
    const labels: Record<string, string> = {
        create_order: 'Crear comanda',
        create_customer: 'Crear cliente',
        add_payment: 'Registrar pago',
        invoice: 'Facturar',
        config_bootstrap: 'Preparar configuración inicial'
    };
    return labels[operacion] ?? operacion;
}

function direccionLabel(direccion: string): string {
    if (direccion === 'glory_to_bdp') return 'Aplicación Web → BDP';
    if (direccion === 'bdp_to_glory') return 'BDP → Aplicación Web';
    if (direccion === 'internal') return 'Configuración de la Aplicación Web';
    return direccion;
}

/* Sub-componentes */

interface SyncModeSelectorProps {
    currentMode: string;
    bdpBaseUrl: string;
    /* [039A-1/H-P1-03] En modo independiente el permiso de operación BDP
     * no aplica: el selector se deshabilita con motivo visible. */
    deshabilitado?: boolean;
}

function SyncModeSelector({currentMode, bdpBaseUrl, deshabilitado = false}: SyncModeSelectorProps) {
    const setMode = useSetSyncMode();
    const effective = currentMode || 'read_only';
    const selectedMode = SYNC_MODES.find(mode => mode.value === effective) ?? SYNC_MODES[0];

    /* [267A-8] Confirmaciones con diálogo propio: los diálogos nativos
     * (`window.confirm`/`prompt`) no funcionan en el navegador empaquetado. */
    async function handleChange(value: string) {
        const destino = bdpBaseUrl.trim().replace(/\/$/, '');
        const confirmarDestino = async (titulo: string): Promise<boolean> => {
            const typed = await pedirTextoConDialogo({
                titulo,
                descripcion: 'Escribe exactamente la URL BDP de destino para confirmar.',
                placeholder: destino,
                validar: (valor) =>
                    valor.trim().replace(/\/$/, '') === destino && destino
                        ? null
                        : 'La URL escrita no coincide exactamente.',
            });
            return typed !== null;
        };
        let alcances: string[] = [];
        let motivo = '';
        let maxOperaciones = 0;
        let duracionMinutos = 0;
        let targetEntityType: 'venta' | 'cliente' | '' = '';
        let targetEntityId = '';
        if (value === 'automatic') {
            const confirmed = await confirmarConDialogo({
                titulo: '¿Activar el modo automático?',
                descripcion:
                    'El modo automático envía las escrituras a BDP/TPV sin pedir confirmación por operación (siempre auditadas). ' +
                    'Solo la activación y la vuelta a Solo lectura requieren confirmación.',
                textoConfirmar: 'Sí, continuar',
            });
            if (!confirmed) return;
            if (!(await confirmarDestino('Confirma el destino BDP'))) return;
        } else if (value !== 'read_only') {
            const confirmed = await confirmarConDialogo({
                titulo: '¿Habilitar escrituras en BDP/TPV?',
                descripcion:
                    'Este modo habilita escrituras reales e irreversibles en BDP/TPV. ' +
                    'Confirma únicamente si existe autorización explícita y se completó el checklist pre-write.',
                textoConfirmar: 'Sí, continuar',
            });
            if (!confirmed) return;
            if (!(await confirmarDestino('Confirma el destino BDP'))) return;
            const operacion = await elegirOpcionConDialogo({
                titulo: 'Elige una sola operación',
                opciones: [
                    { valor: 'create_order', etiqueta: 'Crear comanda' },
                    { valor: 'create_customer', etiqueta: 'Crear cliente' },
                    { valor: 'add_payment', etiqueta: 'Registrar pago' },
                    { valor: 'invoice', etiqueta: 'Facturar' },
                ],
            });
            if (!operacion) return;
            alcances = [operacion];
            const customerOnly = alcances.every(scope => scope === 'create_customer');
            const saleOnly = alcances.every(scope => ['create_order', 'add_payment', 'invoice'].includes(scope));
            if (!customerOnly && !saleOnly) {
                toast.error('Alcances incompatibles', {description: 'No mezcles clientes con operaciones de venta en un mismo armado.'});
                return;
            }
            targetEntityType = customerOnly ? 'cliente' : 'venta';
            const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
            const entityId = await pedirTextoConDialogo({
                titulo: `Identificador del ${targetEntityType}`,
                descripcion: `Pega el identificador interno exacto (UUID) del ${targetEntityType} que se probará.`,
                placeholder: 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx',
                validar: (valor) => (uuidRegex.test(valor.trim()) ? null : 'Debe ser un UUID válido.'),
            });
            if (entityId === null) return;
            targetEntityId = entityId.trim();
            const motivoEscrito = await pedirTextoConDialogo({
                titulo: 'Motivo del armado',
                descripcion: 'Describe brevemente quién autorizó la prueba y para qué se realizará.',
                placeholder: 'Autorizado por … para …',
                validar: (valor) => (valor.trim().length >= 5 ? null : 'Mínimo 5 caracteres.'),
            });
            if (motivoEscrito === null) return;
            motivo = motivoEscrito.trim();
            maxOperaciones = 1;
            const duracion = await pedirTextoConDialogo({
                titulo: 'Duración del armado',
                descripcion: 'Duración del armado en minutos (1-15).',
                placeholder: '5',
                validar: (valor) => {
                    const n = Number(valor.trim());
                    return Number.isInteger(n) && n >= 1 && n <= 15 ? null : 'Introduce un entero entre 1 y 15.';
                },
            });
            if (duracion === null) return;
            duracionMinutos = Number(duracion.trim());
            if (!alcances.length || !uuidRegex.test(targetEntityId) || motivo.length < 5 || !Number.isInteger(maxOperaciones) || !Number.isInteger(duracionMinutos)) {
                toast.error('Armado incompleto', {description: 'Revisa alcance, UUID objetivo, motivo, duración y máximo de operaciones.'});
                return;
            }
        }
        setMode.mutate({
            modo: value as SyncMode,
            confirmarDestino: value === 'read_only' ? '' : bdpBaseUrl.trim().replace(/\/$/, ''),
            alcances,
            duracionMinutos,
            maxOperaciones,
            motivo,
            targetEntityType,
            targetEntityId,
        }, {
            onSuccess: () => toast.success('Modo BDP actualizado', {description: `Modo: ${value}`}),
            onError: (err: unknown) => {
                const msg = (err as {response?: {data?: {message?: string}}})?.response?.data?.message ?? 'Error al cambiar modo';
                toast.error('Error', {description: msg});
            }
        });
    }

    return (
        <div className="flex flex-col gap-2">
            <div className="flex items-center gap-3">
            <Select value={effective} onValueChange={handleChange} disabled={deshabilitado || setMode.isPending}>
                <SelectTrigger className="w-full sm:w-[320px]">
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    {SYNC_MODES.map(m => (
                        <SelectItem key={m.value} value={m.value}>
                            {m.label}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
            {setMode.isPending && <Loader2 className="h-4 w-4 animate-spin" />}
            </div>
            <p className="max-w-xl text-xs text-muted-foreground">{selectedMode.desc}</p>
        </div>
    );
}

function SnapshotActions({bdpActivo}: {bdpActivo: boolean}) {
    const crearCompleto = useCreateSnapshotCompleto();
    const crearParcial = useCreateSnapshotParcial();
    const crearGlory = useCreateSnapshotGlory();
    const [notas, setNotas] = useState('');
    const [tiposSeleccionados, setTiposSeleccionados] = useState<string[]>([]);
    const [tiposGlory, setTiposGlory] = useState<string[]>([]);

    function toggleTipo(lista: string[], setLista: (v: string[]) => void, tipo: string) {
        setLista(lista.includes(tipo) ? lista.filter(t => t !== tipo) : [...lista, tipo]);
    }

    const anyLoading = crearCompleto.isPending || crearParcial.isPending || crearGlory.isPending;

    return (
        <div className="space-y-4">
            {/* [039A-1/H-P1-03] Los snapshots "completo"/"parcial" descargan de
             * BDP (red real): en modo independiente se deshabilitan con motivo,
             * igual que el resto de controles BDP (P11.2); el snapshot local
             * (0 llamadas BDP) y los listados siguen disponibles. */}
            {!bdpActivo && (
                <p className="text-xs text-muted-foreground rounded-md border bg-muted/40 p-2">
                    Modo independiente: los snapshots de BDP requieren el modo BDP conectado
                    (lectura real). El snapshot de la Aplicación Web es local y sigue disponible.
                </p>
            )}
            <div>
                <label className="text-sm font-medium">Notas (opcional)</label>
                <Textarea placeholder="Ej: antes de migración de artículos..." value={notas} onChange={e => setNotas(e.target.value)} className="mt-1" rows={2} />
            </div>

            <div className="flex flex-wrap gap-2">
                <Button
                    onClick={() =>
                        crearCompleto.mutate(notas || undefined, {
                            onSuccess: () => {
                                toast.success('Snapshot completo creado');
                                setNotas('');
                            },
                            onError: (e: unknown) => toast.error('Error', {description: String(e)})
                        })
                    }
                    disabled={!bdpActivo || anyLoading}>
                    {crearCompleto.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-2" /> : <Database className="h-4 w-4 mr-2" />}
                    Snapshot completo BDP
                </Button>
            </div>

            <div className="border rounded-md p-3 space-y-2">
                <p className="text-sm font-medium">Snapshot parcial BDP</p>
                <div className="flex flex-wrap gap-1.5">
                    {SNAPSHOT_TIPOS_BDP.map(t => (
                        <Badge key={t} variant={tiposSeleccionados.includes(t) ? 'default' : 'outline'} className="cursor-pointer select-none" onClick={() => toggleTipo(tiposSeleccionados, setTiposSeleccionados, t)}>
                            {t}
                        </Badge>
                    ))}
                </div>
                <Button
                    variant="secondary"
                    disabled={!bdpActivo || tiposSeleccionados.length === 0 || anyLoading}
                    onClick={() =>
                        crearParcial.mutate(
                            {tipos: tiposSeleccionados, notas: notas || undefined},
                            {
                                onSuccess: () => {
                                    toast.success('Snapshot parcial creado');
                                    setNotas('');
                                    setTiposSeleccionados([]);
                                },
                                onError: (e: unknown) => toast.error('Error', {description: String(e)})
                            }
                        )
                    }>
                    {crearParcial.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-1" /> : <Download className="h-4 w-4 mr-1" />}
                    Crear parcial
                </Button>
            </div>

            <div className="border rounded-md p-3 space-y-2">
                <p className="text-sm font-medium">Snapshot de la Aplicación Web (local, 0 llamadas BDP)</p>
                <div className="flex flex-wrap gap-1.5">
                    {SNAPSHOT_TIPOS_GLORY.map(t => (
                        <Badge key={t} variant={tiposGlory.includes(t) ? 'default' : 'outline'} className="cursor-pointer select-none" onClick={() => toggleTipo(tiposGlory, setTiposGlory, t)}>
                            {t}
                        </Badge>
                    ))}
                </div>
                <Button
                    variant="secondary"
                    disabled={tiposGlory.length === 0 || anyLoading}
                    onClick={() =>
                        crearGlory.mutate(
                            {tipos: tiposGlory, notas: notas || undefined},
                            {
                                onSuccess: () => {
                                    toast.success('Snapshot de la Aplicación Web creado');
                                    setNotas('');
                                    setTiposGlory([]);
                                },
                                onError: (e: unknown) => toast.error('Error', {description: String(e)})
                            }
                        )
                    }>
                    {crearGlory.isPending ? <Loader2 className="h-4 w-4 animate-spin mr-1" /> : <Download className="h-4 w-4 mr-1" />}
                    Crear snapshot local
                </Button>
            </div>
        </div>
    );
}

function SnapshotTable({snapshots}: {snapshots: BdpSnapshot[]}) {
    const eliminar = useDeleteSnapshot();
    const restaurar = useRestoreSnapshot();
    const [confirmRestore, setConfirmRestore] = useState<string | null>(null);
    const [restoreInput, setRestoreInput] = useState('');

    if (snapshots.length === 0) {
        return <p className="text-sm text-muted-foreground py-4 text-center">No hay snapshots todavía. Crea uno para empezar.</p>;
    }

    return (
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead>Tipo</TableHead>
                    <TableHead>Fecha</TableHead>
                    <TableHead>Datos</TableHead>
                    <TableHead>Notas</TableHead>
                    <TableHead className="text-center">Acciones</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {snapshots.map(s => (
                    <TableRow key={s.id}>
                        <TableCell>{tipoSnapshotBadge(s.tipo)}</TableCell>
                        <TableCell className="text-[13px]">{formatDate(s.created_at)}</TableCell>
                        <TableCell className="text-xs text-muted-foreground">{datosResumen(s.datos)}</TableCell>
                        <TableCell className="text-xs max-w-[200px] truncate">{s.notas ?? '—'}</TableCell>
                        <TableCell className="text-right">
                            <div className="flex justify-center gap-1">
                                {confirmRestore === s.id ? (
                                    <div className="flex flex-col gap-1 items-end">
                                        <p className="text-xs text-destructive">Escribe exactamente: <code className="break-all">RESTAURAR {s.id}</code></p>
                                        <input
                                            className="text-xs border rounded px-1 py-0.5 w-72 font-mono"
                                            placeholder={`RESTAURAR ${s.id}`}
                                            value={restoreInput}
                                            onChange={e => setRestoreInput(e.target.value)}
                                        />
                                        <div className="flex gap-1">
                                            <Button
                                                size="sm"
                                                variant="destructive"
                                                onClick={() => {
                                                    restaurar.mutate({ id: s.id, confirmacion: restoreInput }, {
                                                        onSuccess: r => {
                                                            toast.success('Restauración completada', {
                                                                description: `${r.registros_restaurados} registros restaurados. ${r.detalles}`
                                                            });
                                                            setConfirmRestore(null);
                                                            setRestoreInput('');
                                                        },
                                                        onError: (e: unknown) => toast.error('Error', {description: String(e)})
                                                    });
                                                }}
                                                disabled={restaurar.isPending || restoreInput.trim() !== `RESTAURAR ${s.id}`}>
                                                {restaurar.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : 'Confirmar'}
                                            </Button>
                                            <Button size="sm" variant="outline" onClick={() => { setConfirmRestore(null); setRestoreInput(''); }}>
                                                Cancelar
                                            </Button>
                                        </div>
                                    </div>
                                ) : (
                                    <>
                                        <TooltipButton
                                            size="sm"
                                            variant="outline"
                                            className="bg-muted/40 hover:bg-muted"
                                            onClick={() => {
                                                toast.info('Restaurar sobre datos actuales', {
                                                    description: 'Esto sobrescribirá los datos locales de la Aplicación Web con los del snapshot.'
                                                });
                                                setConfirmRestore(s.id);
                                            }}
                                            tooltip="Restaurar la Aplicación Web desde este snapshot">
                                            <Upload className="h-3 w-3" />
                                        </TooltipButton>
                                        <TooltipButton
                                            size="sm"
                                            variant="outline"
                                            className="bg-muted/40 hover:bg-muted"
                                            onClick={async () => {
                                                /* [267A-8] Confirmación con diálogo propio. */
                                                const confirmado = await confirmarConDialogo({
                                                    titulo: '¿Eliminar este snapshot permanentemente?',
                                                    textoConfirmar: 'Eliminar',
                                                });
                                                if (confirmado) {
                                                    eliminar.mutate(s.id, {
                                                        onSuccess: () => toast.success('Snapshot eliminado'),
                                                        onError: (e: unknown) => toast.error('Error', {description: String(e)})
                                                    });
                                                }
                                            }}
                                            tooltip="Eliminar snapshot">
                                            <Trash2 className="h-3 w-3 text-destructive" />
                                        </TooltipButton>
                                    </>
                                )}
                            </div>
                        </TableCell>
                    </TableRow>
                ))}
            </TableBody>
        </Table>
    );
}

function AuditTable({entries}: {entries: BdpAuditEntry[]}) {
    if (entries.length === 0) {
        return <p className="text-sm text-muted-foreground py-4 text-center">Sin registros de auditoría todavía.</p>;
    }

    return (
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHead>Fecha</TableHead>
                    <TableHead>Operación</TableHead>
                    <TableHead>Dirección</TableHead>
                    <TableHead>Resultado</TableHead>
                    <TableHead>Registro</TableHead>
                    <TableHead>Motivo</TableHead>
                    <TableHead>Error</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {entries.map(e => (
                    <TableRow key={e.id}>
                        <TableCell className="text-[13px]">{formatDate(e.created_at)}</TableCell>
                        <TableCell>
                            <Badge variant="outline">{operacionLabel(e.operacion)}</Badge>
                        </TableCell>
                        <TableCell>
                            <Badge variant="outline">{direccionLabel(e.direccion)}</Badge>
                        </TableCell>
                        <TableCell>{resultadoBadge(e.resultado)}</TableCell>
                        <TableCell className="text-xs">
                            {e.target_entity_type ? `${e.target_entity_type}: ${e.target_entity_id?.slice(0, 8) ?? '—'}…` : '—'}
                        </TableCell>
                        <TableCell className="max-w-[220px] truncate text-xs">{e.authorization_reason ?? '—'}</TableCell>
                        <TableCell className="text-xs text-destructive max-w-[200px] truncate">{e.error_mensaje ?? '—'}</TableCell>
                    </TableRow>
                ))}
            </TableBody>
        </Table>
    );
}

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
