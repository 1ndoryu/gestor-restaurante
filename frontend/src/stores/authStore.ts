/* 253A-7: Store de autenticación JWT — Zustand para estado de cliente.
 * [085A-1] estaAutenticado() decodifica el JWT y verifica expiración client-side.
 * Un token expirado se trata como sesión cerrada y se limpia inmediatamente.
 * Esto evita el estado roto donde el store tiene token pero el backend rechaza con 401. */

import { create } from 'zustand';

interface AuthState {
  token: string | null;
  iniciarSesion: (token: string) => void;
  cerrarSesion: () => void;
  estaAutenticado: () => boolean;
  /* [149A-3/F3] Rol efectivo de la sesión (del JWT: effective_role admin/trabajador).
   * Solo para UI honesta (menú deshabilitado con aviso); la autorización real
   * siempre la aplica el backend (403). */
  rolEfectivo: () => 'admin' | 'trabajador' | null;
  esTrabajador: () => boolean;
}

/* Decodifica el rol efectivo del JWT sin verificar firma (solo UI honesta).
 * El backend serializa UserRole en minúsculas: "admin" | "trabajador". */
function rolDelToken(token: string): 'admin' | 'trabajador' | null {
  try {
    const [, payload] = token.split('.');
    const decoded = JSON.parse(atob(payload)) as { effective_role?: string };
    if (decoded.effective_role === 'admin') return 'admin';
    if (decoded.effective_role === 'trabajador') return 'trabajador';
    return null;
  } catch {
    return null;
  }
}
/* Decodifica el payload del JWT sin verificar firma (solo para checar exp client-side). */
function tokenEsValido(token: string): boolean {
  try {
    const [, payload] = token.split('.');
    const decoded = JSON.parse(atob(payload)) as { exp?: number };
    if (!decoded.exp) return false;
    return decoded.exp * 1000 > Date.now();
  } catch {
    return false;
  }
}

export const useAuthStore = create<AuthState>((set, get) => {
  /* Limpiar token expirado al inicializar para no arrancar con sesión rota */
  const tokenGuardado = localStorage.getItem('token');
  if (tokenGuardado && !tokenEsValido(tokenGuardado)) {
    localStorage.removeItem('token');
  }

  return {
    token: tokenEsValido(tokenGuardado ?? '') ? tokenGuardado : null,

    iniciarSesion: (token: string) => {
      localStorage.setItem('token', token);
      set({ token });
    },

    cerrarSesion: () => {
      localStorage.removeItem('token');
      set({ token: null });
    },

    rolEfectivo: () => {
      const token = get().token;
      return token ? rolDelToken(token) : null;
    },

    esTrabajador: () => {
      const token = get().token;
      return token ? rolDelToken(token) === 'trabajador' : false;
    },

    estaAutenticado: () => {
      const token = get().token;
      if (!token) return false;
      if (!tokenEsValido(token)) {
        /* Token expirado — limpiar de forma sincrónica */
        localStorage.removeItem('token');
        set({ token: null });
        return false;
      }
      return true;
    },
  };
});
