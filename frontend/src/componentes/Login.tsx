/* [134A-29] Login — soporta propietario, registro y trabajador.
 * Toggle "Soy trabajador" cambia el endpoint a login-trabajador. */

import { Link } from 'react-router-dom';
import useLoginForm from '../hooks/useLoginForm';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { BannerEntorno } from '@/components/banner-entorno';

function Login() {
  const {
    credenciales, cambiarCredencial,
    modoRegistro, alternarModo,
    esTrabajador, alternarTrabajador,
    error, manejarEnvio, cargando,
  } = useLoginForm();

  const titulo = modoRegistro
    ? 'Crea tu cuenta'
    : esTrabajador
      ? 'Acceso de trabajador'
      : 'Inicia sesión para continuar';

  return (
    <div className="flex min-h-screen flex-col bg-background">
      <BannerEntorno />
      <div className="flex flex-1 items-center justify-center p-4">
      <Card className="w-full max-w-sm">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">Gestión Restaurante</CardTitle>
          <CardDescription>{titulo}</CardDescription>
        </CardHeader>
        <CardContent>
          {error && (
            <div className="mb-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          <form className="flex flex-col gap-4" onSubmit={manejarEnvio}>
            <div className="flex flex-col gap-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                value={credenciales.email}
                onChange={(e) => cambiarCredencial('email', e.target.value)}
                placeholder="correo@ejemplo.com"
                autoComplete="email"
              />
            </div>

            <div className="flex flex-col gap-2">
              <Label htmlFor="password">Contraseña</Label>
              <Input
                id="password"
                type="password"
                value={credenciales.password}
                onChange={(e) => cambiarCredencial('password', e.target.value)}
                placeholder="••••••••"
                autoComplete={modoRegistro ? 'new-password' : 'current-password'}
              />
            </div>

            <Button type="submit" className="w-full" disabled={cargando}>
              {cargando ? 'Cargando...' : modoRegistro ? 'Crear cuenta' : 'Entrar'}
            </Button>

            {!modoRegistro && !esTrabajador && (
              <p className="text-center text-sm text-muted-foreground">
                <Link to="/forgot-password" className="underline underline-offset-4 hover:text-primary">
                  ¿Olvidaste tu contraseña?
                </Link>
              </p>
            )}
          </form>
        </CardContent>
        <CardFooter className="flex flex-col gap-2">
          {!esTrabajador && (
            <p className="text-sm text-muted-foreground">
              {modoRegistro ? '¿Ya tienes cuenta? ' : '¿No tienes cuenta? '}
              <span onClick={alternarModo} className="cursor-pointer underline underline-offset-4 hover:text-primary">
                {modoRegistro ? 'Inicia sesión' : 'Regístrate'}
              </span>
            </p>
          )}
          {!modoRegistro && (
            <p className="text-sm text-muted-foreground">
              <span onClick={alternarTrabajador} className="cursor-pointer underline underline-offset-4 hover:text-primary">
                {esTrabajador ? '← Volver al login de propietario' : 'Soy trabajador'}
              </span>
            </p>
          )}
        </CardFooter>
      </Card>
      </div>
    </div>
  );
}

export default Login;
