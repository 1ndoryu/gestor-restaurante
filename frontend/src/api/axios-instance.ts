import axios from 'axios';

/* [283A-49] baseURL vacía = URLs relativas al origen actual.
 * Funciona en local (vite proxy redirige /api → localhost:3000) y en producción
 * (backend sirve el frontend desde el mismo origen). No hardcodear localhost. */
const instance = axios.create({
  baseURL: import.meta.env.VITE_API_URL || '',
});

/* Interceptor: agrega el token JWT a cada request si existe */
instance.interceptors.request.use((config) => {
  const token = localStorage.getItem('token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

/* [283A-9] Interceptor de respuesta: maneja 401 (token expirado/inválido).
 * Limpia localStorage y redirige al login para que el usuario se re-autentique.
 * Ignora 401 en rutas de auth (login, register, forgot-password) porque son esperados. */
instance.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      const url = error.config?.url || '';
      const rutasAuth = ['/auth/login', '/auth/register', '/auth/forgot-password', '/auth/reset-password'];
      const esRutaAuth = rutasAuth.some((ruta) => url.includes(ruta));
      if (!esRutaAuth) {
        localStorage.removeItem('token');
        /* Redirigir solo si no estamos ya en login */
        if (!window.location.pathname.includes('/login')) {
          window.location.href = '/login';
        }
      }
    }
    return Promise.reject(error);
  },
);

/**
 * 253A-7: Mutator para Orval v8 — la firma cambió a (url, config) en v8.
 * Orval genera tipos con forma { data, status, headers }, así que el mutator
 * debe retornar ese objeto completo (no solo response.data).
 * FIX: retornar { data, status, headers } para que los componentes puedan
 * hacer data?.status === 200 ? data.data : null correctamente.
 */
export const customInstance = async <T>(
  url: string,
  config: RequestInit,
): Promise<T> => {
  const { body, headers, method, signal } = config;

  /* [BKP-008d] Si body es un string (JSON.stringify), axios lo envía como
   * text/plain por defecto. Axum con Json<T> extractor requiere application/json.
   * Detectamos string y forzamos Content-Type para que el backend lo acepte. */
  const extraHeaders: Record<string, string> = { ...(headers as Record<string, string>) };
  if (typeof body === 'string' && !extraHeaders['Content-Type']) {
    extraHeaders['Content-Type'] = 'application/json';
  }

  const response = await instance.request({
    url,
    method: method as string,
    data: body,
    headers: extraHeaders,
    signal: signal ?? undefined,
  });

  return {
    data: response.data,
    status: response.status,
    headers: response.headers,
  } as T;
};

export default instance;
