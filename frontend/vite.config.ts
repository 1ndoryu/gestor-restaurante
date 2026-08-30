import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

/* [198A-2] Target del backend configurable para levantar stacks de prueba aislados
 * sin tocar el puerto fijo. Default http://localhost:3000 (comportamiento histórico). */
const apiTarget = process.env.VITE_API_TARGET || 'http://localhost:3000';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@glory': path.resolve(__dirname, '../glory-rs/frontend'),
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 5173,
    /* Permitir servir archivos del submodulo glory-rs */
    fs: {
      allow: ['..'],
    },
    /* Proxy API requests al backend Rust en desarrollo */
    proxy: {
      '/api': {
        target: apiTarget,
        changeOrigin: true,
      },
      '/swagger-ui': {
        target: apiTarget,
        changeOrigin: true,
      },
      '/api-docs': {
        target: apiTarget,
        changeOrigin: true,
      },
    },
  },
});
