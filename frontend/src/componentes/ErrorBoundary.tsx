/* [283A-26] ErrorBoundary global — captura errores de React no manejados.
 * Muestra un fallback con opción de recargar o reportar el error al backend.
 * El reporte se envía por email al admin vía POST /api/reportar-error. */

import { Component, type ErrorInfo, type ReactNode } from 'react';
import { toast } from 'sonner';
import axios from '@/api/axios-instance';
import { logger } from '@/utils/logger';

interface Props { children: ReactNode }
interface State { hasError: boolean; error: Error | null }

class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    logger.error('ErrorBoundary:', error, errorInfo);
  }

  handleReport = async () => {
    const { error } = this.state;
    if (!error) return;
    /* [303A-2] Migrado de raw fetch a axios para usar interceptors JWT/401 */
    try {
      await axios.post('/api/reportar-error', {
        mensaje: error.message,
        stack: error.stack ?? null,
        url: window.location.href,
        navegador: navigator.userAgent,
      });
      toast.success('Error reportado correctamente');
    } catch {
      toast.error('No se pudo enviar el reporte');
    }
  };

  handleReload = () => window.location.reload();

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center min-h-screen gap-4 p-8 text-center">
          <h1 className="text-xl font-bold">Algo salió mal</h1>
          <p className="text-muted-foreground max-w-lg">{this.state.error?.message}</p>
          <div className="flex gap-2">
            <button
              type="button"
              className="px-4 py-2 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90"
              onClick={this.handleReload}
            >Recargar página</button>
            <button
              type="button"
              className="px-4 py-2 rounded-md border border-input bg-background text-sm font-medium hover:bg-accent"
              onClick={this.handleReport}
            >Reportar error</button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

export default ErrorBoundary;
