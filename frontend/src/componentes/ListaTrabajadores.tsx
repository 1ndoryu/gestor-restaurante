/* [134A-2] Página de gestión de trabajadores con permisos por sección.
 * CRUD + panel de permisos con checkboxes por sección.
 * El propietario asigna/quita acceso a cada sección del sistema. */

import { useTrabajadores } from '../hooks/useTrabajadores';
import { Button } from '@/components/ui/button';
import { TooltipButton } from '@/components/ui/tooltip-button';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Trash2, Pencil, Shield } from 'lucide-react';
import { EstadoError } from '@/components/ui/estado-error';
import { FormularioTrabajador, type CamposForm } from './lista-trabajadores/FormularioTrabajador';
import { etiquetaSeccion } from './lista-trabajadores/etiquetas';

// Se mantienen las exportaciones públicas del módulo original
export { ETIQUETAS_SECCION, etiquetaSeccion } from './lista-trabajadores/etiquetas';

export default function ListaTrabajadores() {
  const {
    trabajadores,
    secciones,
    isLoading,
    isError,
    error,
    seccionesError,
    modalCrear,
    setModalCrear,
    trabajadorEditar,
    setTrabajadorEditar,
    crearTrabajador,
    actualizarTrabajador,
    eliminarTrabajador,
    creando,
    actualizando,
    refetch,
  } = useTrabajadores();

  const handleCrear = (campos: CamposForm) => {
    crearTrabajador({
      nombre: campos.nombre,
      email: campos.email,
      password: campos.password,
      cargo: campos.cargo || null,
      permisos: campos.permisos.length > 0 ? campos.permisos : null,
    });
  };

  const handleActualizar = (campos: CamposForm) => {
    if (!trabajadorEditar) return;
    actualizarTrabajador(trabajadorEditar.id, {
      nombre: campos.nombre,
      email: campos.email,
      password: campos.password || null,
      cargo: campos.cargo || null,
      permisos: campos.permisos,
    });
  };

  /* [149A-3/F2] Si la consulta falló no hay conteo ni vacío que mostrar: se
   * declara "sin permiso" y se oculta el resto, sin botones que van a fallar. */
  if (isError) {
    return <EstadoError error={error} queFallo="la gestión de trabajadores" reintentar={refetch} />;
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {trabajadores.length} trabajador{trabajadores.length !== 1 ? 'es' : ''}
        </p>
        <Button onClick={() => setModalCrear(true)}>+ Nuevo Trabajador</Button>
      </div>

      <Dialog open={modalCrear} onOpenChange={setModalCrear}>
        <DialogContent className="sm:max-w-xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Nuevo Trabajador</DialogTitle>
          </DialogHeader>
          <FormularioTrabajador
            secciones={secciones}
            seccionesError={seccionesError}
            onSubmit={handleCrear}
            cargando={creando}
          />
        </DialogContent>
      </Dialog>

      <Dialog open={!!trabajadorEditar} onOpenChange={(open: boolean) => { if (!open) setTrabajadorEditar(null); }}>
        <DialogContent className="sm:max-w-xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Editar Trabajador</DialogTitle>
          </DialogHeader>
          {trabajadorEditar && (
            <FormularioTrabajador
              secciones={secciones}
              seccionesError={seccionesError}
              trabajador={trabajadorEditar}
              onSubmit={handleActualizar}
              cargando={actualizando}
            />
          )}
        </DialogContent>
      </Dialog>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : trabajadores.length > 0 ? (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Nombre</TableHead>
                <TableHead>Email</TableHead>
                <TableHead>Cargo</TableHead>
                <TableHead>Permisos</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead className="w-20 text-center">Acciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {trabajadores.map((t) => (
                <TableRow key={t.id}>
                  <TableCell className="font-medium">{t.nombre}</TableCell>
                  <TableCell>{t.email}</TableCell>
                  <TableCell>{t.cargo || '—'}</TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1">
                      {t.permisos.filter((p) => p.permitido).map((p) => (
<Badge key={p.seccion} variant="secondary" className="text-xs capitalize">
                {etiquetaSeccion(p.seccion)}
                        </Badge>
                      ))}
                      {t.permisos.filter((p) => p.permitido).length === 0 && (
                        <span className="text-xs text-muted-foreground">Sin permisos</span>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant={t.activo ? 'default' : 'destructive'}>
                      {t.activo ? 'Activo' : 'Inactivo'}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex justify-center gap-1">
                      <TooltipButton variant="outline" size="icon" className="bg-muted/40 hover:bg-muted" onClick={() => setTrabajadorEditar(t)} tooltip="Editar trabajador">
                        <Pencil className="size-4" />
                      </TooltipButton>
                      <TooltipButton
                        variant="outline"
                        size="icon"
                        className="bg-muted/40 hover:bg-muted"
                        onClick={() => eliminarTrabajador(t.id)}
                        tooltip="Eliminar trabajador"
                      >
                        <Trash2 className="size-4" />
                      </TooltipButton>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <div className="text-center py-12 text-muted-foreground">
          <Shield className="size-12 mx-auto mb-3 opacity-30" />
          <p>No hay trabajadores registrados</p>
          <p className="text-sm mt-1">Crea trabajadores y asigna permisos para que accedan al sistema</p>
        </div>
      )}
    </div>
  );
}
