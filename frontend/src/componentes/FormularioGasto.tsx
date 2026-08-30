/* [263A-16] Formulario de gasto — reescrito con shadcn Button + Input + Label + Switch.
 * Menú 3 opciones (manual / digitalizar / por correo).
 * "Por correo" descartado por el cliente — deshabilitado.
 * [283A-8] "Digitalizar" implementado con Groq IA (Llama 4 Scout) — extrae datos de imagen. */

import { useState, useRef } from 'react';
import { Gasto, MetodoPago, TipoDocumento } from '../api/generated';
import useFormularioGasto from '../hooks/useFormularioGasto';
import { useDigitalizacion } from '../hooks/useDigitalizacion';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { ClipboardList, Camera, Mail, Upload, Loader2, CheckCircle } from 'lucide-react';

type ModoGasto = 'menu' | 'manual' | 'digitalizar';

interface Props {
  onExito?: () => void;
  /* [283A-22] gasto opcional para modo edición — pre-rellena el formulario */
  gasto?: Gasto;
}

function FormularioGasto({ onExito, gasto }: Props) {
  /* En modo edición saltar directamente al formulario manual */
  const [modo, setModo] = useState<ModoGasto>(gasto ? 'manual' : 'menu');
  const {
    campos, cambiarCampo, error, manejarEnvio, cargando, categorias, esEdicion,
    sugerenciasProveedores, autocompletarAbierto, setAutocompletarAbierto, seleccionarProveedor,
    marcarInteraccionProveedor,
  } = useFormularioGasto(onExito, gasto);
  const digitalizacion = useDigitalizacion();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [preview, setPreview] = useState<string | null>(null);

  if (modo === 'menu') {
    return (
      <div className="flex flex-col gap-3">
        <Card className="cursor-pointer hover:border-primary transition-colors" onClick={() => setModo('manual')}>
          <CardHeader className="flex flex-row items-center gap-4">
            <ClipboardList className="size-6 text-muted-foreground" />
            <div>
              <CardTitle className="text-base">Gasto manual</CardTitle>
              <CardDescription>Introduce los datos a mano</CardDescription>
            </div>
          </CardHeader>
        </Card>
        <Card className="cursor-pointer hover:border-primary transition-colors" onClick={() => setModo('digitalizar')}>
          <CardHeader className="flex flex-row items-center gap-4">
            <Camera className="size-6 text-muted-foreground" />
            <div>
              <CardTitle className="text-base">Digitalizar archivos</CardTitle>
              <CardDescription>Sube una foto del documento</CardDescription>
            </div>
          </CardHeader>
        </Card>
        <Card className="opacity-50 cursor-not-allowed" title="Funcionalidad no disponible en esta versión">
          <CardHeader className="flex flex-row items-center gap-4">
            <Mail className="size-6 text-muted-foreground" />
            <div>
              <CardTitle className="text-base">Por correo</CardTitle>
              <CardDescription>Próximamente</CardDescription>
            </div>
          </CardHeader>
        </Card>
      </div>
    );
  }

  if (modo === 'digitalizar') {
    /* [283A-8] Subida de imagen + extracción de datos con Groq IA */
    const manejarArchivo = async (e: React.ChangeEvent<HTMLInputElement>) => {
      const archivo = e.target.files?.[0];
      if (!archivo) return;
      setPreview(URL.createObjectURL(archivo));
      const datos = await digitalizacion.digitalizar(archivo);
      if (datos) {
        /* Pre-rellenar campos del formulario manual con los datos extraídos */
        if (datos.fecha) cambiarCampo('fecha', datos.fecha);
        if (datos.proveedor) cambiarCampo('proveedor', datos.proveedor);
        if (datos.numero_documento) cambiarCampo('numeroDocumento', datos.numero_documento);
        if (datos.importe_base) cambiarCampo('importeBase', datos.importe_base);
        if (datos.importe_iva) cambiarCampo('importeIva', datos.importe_iva);
        if (datos.tipo_documento) {
          const tipoMap: Record<string, TipoDocumento> = {
            factura: TipoDocumento.factura,
            albaran: TipoDocumento.albaran,
            ticket: TipoDocumento.ticket,
          };
          const tipo = tipoMap[datos.tipo_documento];
          if (tipo) cambiarCampo('tipoDocumento', tipo);
        }
      }
    };

    /* Si la IA extrajo datos, pasar automáticamente al formulario manual para revisión */
    if (digitalizacion.datos) {
      return (
        <div className="flex flex-col gap-4">
          <div className="flex items-center gap-2 rounded-md bg-green-100 p-3 text-sm text-green-800 dark:bg-green-900/30 dark:text-green-300">
            <CheckCircle className="size-4" />
            <span>Datos extraídos (confianza: {Math.round(digitalizacion.datos.confianza * 100)}%). Revisa y ajusta antes de guardar.</span>
          </div>
          {digitalizacion.datos.notas && (
            <p className="text-xs text-muted-foreground">{digitalizacion.datos.notas}</p>
          )}
          <Button variant="outline" size="sm" onClick={() => { digitalizacion.limpiar(); setPreview(null); setModo('manual'); }}>
            Revisar y completar manualmente
          </Button>
        </div>
      );
    }

    return (
      <div className="flex flex-col items-center gap-6 py-8">
        <Input
          ref={fileInputRef}
          type="file"
          accept="image/jpeg,image/png,image/webp,image/gif"
          className="hidden"
          onChange={manejarArchivo}
        />

        {preview ? (
          <img src={preview} alt="Preview del documento" className="max-h-48 rounded-md border object-contain" />
        ) : (
          <Camera className="size-12 text-muted-foreground" />
        )}

        {digitalizacion.cargando ? (
          <div className="flex flex-col items-center gap-3">
            <Loader2 className="size-8 animate-spin text-primary" />
            <p className="text-sm text-muted-foreground">Analizando documento con IA...</p>
          </div>
        ) : (
          <>
            <p className="text-muted-foreground text-center">Sube una foto de tu factura, albarán o ticket</p>
            <p className="text-sm text-muted-foreground text-center">La IA extraerá automáticamente la fecha, proveedor, importes y más.</p>
            <Button onClick={() => fileInputRef.current?.click()}>
              <Upload className="mr-2 size-4" />
              Seleccionar imagen
            </Button>
          </>
        )}

        {digitalizacion.error && (
          <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{digitalizacion.error}</div>
        )}

        <Button variant="outline" onClick={() => { digitalizacion.limpiar(); setPreview(null); setModo('menu'); }}>Volver</Button>
      </div>
    );
  }

  return (
    <div>
      {error && (
        <div className="mb-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>
      )}

      <form className="flex flex-col gap-4" onSubmit={manejarEnvio}>
        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="fecha">Fecha</Label>
            <Input id="fecha" type="date" value={campos.fecha} onChange={e => cambiarCampo('fecha', e.target.value)} />
          </div>
          {/* [044A-10] Proveedor con autocomplete de proveedores existentes */}
          <div className="flex flex-col gap-2 relative">
            <Label htmlFor="proveedor">Proveedor</Label>
            <Input
              id="proveedor"
              type="text"
              value={campos.proveedor}
              onChange={e => { marcarInteraccionProveedor(); cambiarCampo('proveedor', e.target.value); }}
              onFocus={() => { marcarInteraccionProveedor(); if (campos.proveedor.trim().length >= 2) setAutocompletarAbierto(true); }}
              onBlur={() => { setTimeout(() => setAutocompletarAbierto(false), 200); }}
              placeholder="Opcional"
              autoComplete="off"
            />
            {autocompletarAbierto && sugerenciasProveedores.length > 0 && (
              <div className="absolute top-full left-0 right-0 z-50 mt-1 max-h-48 overflow-y-auto rounded-md border bg-popover shadow-md">
                {sugerenciasProveedores.map(nombre => (
                  <Button
                    key={nombre}
                    variant="ghost"
                    className="w-full justify-start h-auto py-2 px-3 rounded-none text-left"
                    onMouseDown={(e) => { e.preventDefault(); seleccionarProveedor(nombre); }}
                  >
                    <span className="text-sm">{nombre}</span>
                  </Button>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="categoria">Categoría</Label>
            <Select value={campos.categoriaId || '__none__'} onValueChange={v => cambiarCampo('categoriaId', v === '__none__' ? '' : v)}>
              <SelectTrigger id="categoria"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="__none__">Sin categoría</SelectItem>
                {categorias.map(cat => (
                  <SelectItem key={cat.id} value={cat.id}>{cat.nombre}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="tipoDocumento">Tipo documento</Label>
            <Select value={campos.tipoDocumento} onValueChange={v => cambiarCampo('tipoDocumento', v as TipoDocumento)}>
              <SelectTrigger id="tipoDocumento"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value={TipoDocumento.factura}>Factura</SelectItem>
                <SelectItem value={TipoDocumento.albaran}>Albarán</SelectItem>
                <SelectItem value={TipoDocumento.ticket}>Ticket</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="metodoPago">Método de pago <span className="text-muted-foreground">(Opcional)</span></Label>
            <Select value={campos.metodoPago || '__none__'} onValueChange={v => cambiarCampo('metodoPago', v === '__none__' ? '' : v as MetodoPago | '')}>
              <SelectTrigger id="metodoPago"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="__none__">— sin especificar —</SelectItem>
                <SelectItem value={MetodoPago.efectivo}>Efectivo</SelectItem>
                <SelectItem value={MetodoPago.tarjeta}>Tarjeta</SelectItem>
                <SelectItem value={MetodoPago.transferencia}>Transferencia</SelectItem>
                <SelectItem value={MetodoPago.otros}>Otros</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="numeroDocumento">Nº documento <span className="text-muted-foreground">(Opcional)</span></Label>
            <Input id="numeroDocumento" type="text" value={campos.numeroDocumento} onChange={e => cambiarCampo('numeroDocumento', e.target.value)} placeholder="Para programación automática futura" />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="importeBase">Importe base (€)</Label>
            <Input id="importeBase" type="number" step="0.01" min="0" value={campos.importeBase} onChange={e => cambiarCampo('importeBase', e.target.value)} placeholder="0.00" />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="importeIva">Importe IVA (€)</Label>
            <Input id="importeIva" type="number" step="0.01" min="0" value={campos.importeIva} onChange={e => cambiarCampo('importeIva', e.target.value)} placeholder="0.00" />
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Switch id="recurrente" checked={campos.recurrente} onCheckedChange={checked => cambiarCampo('recurrente', checked)} />
          <Label htmlFor="recurrente">Gasto recurrente</Label>
        </div>

        <div className="flex gap-3">
          {!esEdicion && <Button type="button" variant="outline" onClick={() => setModo('menu')}>Volver</Button>}
          <Button type="submit" disabled={cargando}>
            {cargando ? (esEdicion ? 'Guardando...' : 'Registrando...') : (esEdicion ? 'Guardar cambios' : 'Registrar Gasto')}
          </Button>
        </div>
      </form>
    </div>
  );
}

export default FormularioGasto;

