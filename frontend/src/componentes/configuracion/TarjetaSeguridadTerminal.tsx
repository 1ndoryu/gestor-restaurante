/* [D02] Contraseña del terminal y registro de intentos de acceso.
 * Rotar aquí deja el rastro que pide la ficha: la vieja deja de entrar, la nueva
 * entra y cada intento queda anotado sin guardar nunca la contraseña. */

import { useCallback, useEffect, useState } from 'react';
import { KeyRound, RefreshCw, ShieldCheck, ShieldX } from 'lucide-react';
import axios from '@/api/axios-instance';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { toast } from 'sonner';

interface IntentoAcceso {
  created_at: string;
  email: string;
  exito: boolean;
  motivo: string;
}

const MOTIVOS: Record<string, string> = {
  ok: 'Acceso correcto',
  credenciales_invalidas: 'Credenciales incorrectas',
  hash_invalido: 'Registro de contraseña dañado',
  rotacion: 'Contraseña rotada',
  error: 'Error al verificar',
};

function fecha(iso: string): string {
  const valor = new Date(iso);
  return Number.isNaN(valor.getTime()) ? iso : valor.toLocaleString();
}

/** El formulario y el listado comparten un único objeto de estado: menos piezas
 * sueltas que sincronizar y una sola fuente de verdad para «qué está pasando». */
interface EstadoSeguridad {
  actual: string;
  nueva: string;
  repetida: string;
  guardando: boolean;
  intentos: IntentoAcceso[];
  cargandoIntentos: boolean;
}

const ESTADO_INICIAL: EstadoSeguridad = {
  actual: '',
  nueva: '',
  repetida: '',
  guardando: false,
  intentos: [],
  cargandoIntentos: false,
};

export function TarjetaSeguridadTerminal() {
  const [estado, setEstado] = useState<EstadoSeguridad>(ESTADO_INICIAL);
  const { actual, nueva, repetida, guardando, intentos, cargandoIntentos } = estado;

  const cargarIntentos = useCallback(async () => {
    setEstado((previo) => ({ ...previo, cargandoIntentos: true }));
    try {
      const resp = await axios.get<IntentoAcceso[]>('/api/auth/intentos', { params: { limite: 20 } });
      setEstado((previo) => ({ ...previo, intentos: resp.data }));
    } catch {
      /* Sin permiso o sin conexión: la lista queda vacía y se avisa al recargar. */
      setEstado((previo) => ({ ...previo, intentos: [] }));
    } finally {
      setEstado((previo) => ({ ...previo, cargandoIntentos: false }));
    }
  }, []);

  useEffect(() => {
    void cargarIntentos();
  }, [cargarIntentos]);

  async function cambiar() {
    if (nueva !== repetida) {
      toast.error('No coinciden', { description: 'La contraseña nueva y su repetición deben ser iguales.' });
      return;
    }
    setEstado((previo) => ({ ...previo, guardando: true }));
    try {
      await axios.put('/api/auth/password', { actual, nueva });
      toast.success('Contraseña cambiada', { description: 'A partir de ahora entra solo la nueva.' });
      setEstado((previo) => ({ ...previo, actual: '', nueva: '', repetida: '' }));
      await cargarIntentos();
    } catch (err: unknown) {
      const mensaje =
        (err as { response?: { data?: { message?: string } } })?.response?.data?.message ??
        'No se pudo cambiar la contraseña';
      toast.error('Error', { description: mensaje });
    } finally {
      setEstado((previo) => ({ ...previo, guardando: false }));
    }
  }

  /** Cambia un campo del formulario sin tocar el resto del estado. */
  function cambiarCampo(campo: 'actual' | 'nueva' | 'repetida', valor: string) {
    setEstado((previo) => ({ ...previo, [campo]: valor }));
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <KeyRound className="size-4" />
          Contraseña del terminal
        </CardTitle>
        <CardDescription>
          Cambiar la contraseña pide la actual: una sesión abierta sin conocerla no basta. Los
          intentos de acceso, con y sin éxito, quedan anotados abajo.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        <div className="grid max-w-md gap-3">
          <div className="flex flex-col gap-2">
            <Label htmlFor="password-actual">Contraseña actual</Label>
            <Input
              id="password-actual"
              type="password"
              autoComplete="current-password"
              value={actual}
              onChange={(evento) => cambiarCampo('actual', evento.target.value)}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="password-nueva">Contraseña nueva (mínimo 8 caracteres)</Label>
            <Input
              id="password-nueva"
              type="password"
              autoComplete="new-password"
              value={nueva}
              onChange={(evento) => cambiarCampo('nueva', evento.target.value)}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="password-repetida">Repite la contraseña nueva</Label>
            <Input
              id="password-repetida"
              type="password"
              autoComplete="new-password"
              value={repetida}
              onChange={(evento) => cambiarCampo('repetida', evento.target.value)}
            />
          </div>
          <Button
            type="button"
            onClick={cambiar}
            disabled={guardando || !actual || nueva.length < 8}
            className="w-fit"
          >
            {guardando ? 'Cambiando...' : 'Cambiar contraseña'}
          </Button>
        </div>

        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <p className="text-sm font-medium">Últimos intentos de acceso</p>
            <Button type="button" variant="ghost" size="sm" onClick={cargarIntentos} disabled={cargandoIntentos}>
              <RefreshCw className={`size-4 ${cargandoIntentos ? 'animate-spin' : ''}`} />
              Actualizar
            </Button>
          </div>
          {intentos.length === 0 ? (
            <p className="text-xs text-muted-foreground">Sin intentos registrados.</p>
          ) : (
            <ul className="flex flex-col divide-y rounded-md border text-sm">
              {intentos.map((intento, indice) => (
                <li key={`${intento.created_at}-${indice}`} className="flex items-center gap-3 px-3 py-2">
                  {intento.exito ? (
                    <ShieldCheck className="size-4 text-green-600" />
                  ) : (
                    <ShieldX className="size-4 text-destructive" />
                  )}
                  <span className="tabular-nums text-muted-foreground">{fecha(intento.created_at)}</span>
                  <span className="truncate">{intento.email}</span>
                  <Badge variant={intento.exito ? 'secondary' : 'destructive'} className="ml-auto shrink-0">
                    {MOTIVOS[intento.motivo] ?? intento.motivo}
                  </Badge>
                </li>
              ))}
            </ul>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
