/// <reference types="vite/client" />

/* [247A-1] Variables de entorno VITE para desarrollo local.
 * Se definen en frontend/.env y se exponen al navegador via Vite. */
interface ImportMetaEnv {
  readonly VITE_DEMO_EMAIL?: string;
  readonly VITE_DEMO_PASSWORD?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
