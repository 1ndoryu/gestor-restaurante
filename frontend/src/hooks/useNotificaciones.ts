/* [283A-20] Hook de notificaciones en tiempo real.
 * Conecta via SSE al backend para recibir notificaciones al instante.
 * Al montar, carga las notificaciones existentes y abre un EventSource. */

import { useEffect, useRef } from 'react';
import axios from '@/api/axios-instance';
import { toast } from 'sonner';
import { useNotificacionStore, type Notificacion } from '@/stores/notificacionStore';
import { logger } from '@/utils/logger';

const API_BASE = import.meta.env.VITE_API_URL || '';

export function useNotificaciones() {
    const { setItems, setNoLeidas, agregarNotificacion } = useNotificacionStore();
    const eventSourceRef = useRef<EventSource | null>(null);

    useEffect(() => {
        const controller = new AbortController();

        /* Carga inicial: notificaciones recientes + conteo no leídas */
        async function cargarInicial() {
            try {
                const [listRes, countRes] = await Promise.all([
                    axios.get<Notificacion[]>('/api/notificaciones?limite=50', { signal: controller.signal }),
                    axios.get<{ count: number }>('/api/notificaciones/count', { signal: controller.signal }),
                ]);
                setItems(listRes.data);
                setNoLeidas(countRes.data.count);
            } catch {
                /* Silenciar abort errors al desmontar */
            }
        }
        cargarInicial();

        /* SSE: El token JWT se pasa como query param porque EventSource no soporta headers.
         * El backend acepta ?token= como alternativa al header Authorization. */
        const token = localStorage.getItem('token');
        if (token) {
            const url = `${API_BASE}/api/notificaciones/stream?token=${encodeURIComponent(token)}`;
            const es = new EventSource(url);
            eventSourceRef.current = es;

            es.addEventListener('notificacion', (e) => {
                try {
                    const notif: Notificacion = JSON.parse(e.data);
                    agregarNotificacion(notif);
                    toast.info(notif.titulo, { description: notif.mensaje });
                } catch (parseErr) {
                    /* [303A-2] Logear en vez de silenciar para diagnóstico */
                    logger.warn('Notificación SSE: JSON inválido', parseErr);
                }
            });

            es.onerror = () => {
                /* SSE reconecta automáticamente. Cerrar si el componente se desmontó. */
            };
        }

        return () => {
            controller.abort();
            eventSourceRef.current?.close();
            eventSourceRef.current = null;
        };
    }, [setItems, setNoLeidas, agregarNotificacion]);
}
