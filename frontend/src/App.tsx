/* [263A-16] App principal — reescrito con ThemeProvider + nuevo Layout shadcn.
 * El viejo Layout con BarraLateral se reemplaza por components/layout.tsx (SidebarProvider).
 * Se eliminó el import de global.css — Tailwind maneja todos los estilos ahora.
 * [283A-26] ErrorBoundary global para capturar errores y reportarlos por email. */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Routes, Route, Navigate } from 'react-router-dom';
import { useAuthStore } from './stores/authStore';
import { ThemeProvider } from '@/components/theme-provider';
import { TooltipProvider } from '@/components/ui/tooltip';
import { Toaster } from '@/components/ui/sonner';
import Layout from '@/components/layout';
import ErrorBoundary from './componentes/ErrorBoundary';
import Login from './componentes/Login';
import ListaVentas from './componentes/ListaVentas';
import ListaGastos from './componentes/ListaGastos';
import ListaReservas from './componentes/ListaReservas';
import CalendarioReservas from './componentes/CalendarioReservas';
import ListaClientes from './componentes/ListaClientes';
import ListaCanales from './componentes/ListaCanales';
import EstadisticasNoShows from './componentes/EstadisticasNoShows';
import DashboardReservas from './componentes/DashboardReservas';
import PlanoSala from './componentes/PlanoSala';
import Configuracion from './componentes/Configuracion';
import ForgotPassword from './componentes/ForgotPassword';
import ResetPassword from './componentes/ResetPassword';
import ListaCampanas from './componentes/ListaCampanas';
import FormularioCampana from './componentes/FormularioCampana';
import ListaPlantillas from './componentes/ListaPlantillas';
import FormularioPlantilla from './componentes/FormularioPlantilla';
import Recordatorios from './componentes/Recordatorios';
import ListaTrabajadores from './componentes/ListaTrabajadores';
import ListaResenas from './componentes/ListaResenas';
import Inactividad from './componentes/Inactividad';
import BdpStock from './componentes/bdp/BdpStock';
import BdpExplorador from './componentes/bdp/BdpExplorador';
import BdpHistorial from './componentes/bdp/BdpHistorial';
import BdpCompras from './componentes/bdp/BdpCompras';
import BdpCatalogo from './componentes/bdp/BdpCatalogo';
import BdpInventario from './componentes/bdp/BdpInventario';
import BdpSincronizacion from './componentes/bdp/BdpSincronizacion';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,
      retry: 1,
    },
  },
});

function App() {
  const autenticado = useAuthStore((s) => s.estaAutenticado)();

  return (
    <ErrorBoundary>
    <ThemeProvider>
      <TooltipProvider>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/login" element={autenticado ? <Navigate to="/" replace /> : <Login />} />
            <Route path="/forgot-password" element={autenticado ? <Navigate to="/" replace /> : <ForgotPassword />} />
            <Route path="/reset-password" element={<ResetPassword />} />

            <Route element={<Layout />}>
              <Route path="/" element={<DashboardReservas />} />
              <Route path="/ventas" element={<ListaVentas />} />
              <Route path="/gastos" element={<ListaGastos />} />
              <Route path="/reservas" element={<ListaReservas />} />
              <Route path="/reservas/calendario" element={<CalendarioReservas />} />
              <Route path="/clientes" element={<ListaClientes />} />
              <Route path="/canales" element={<ListaCanales />} />
              <Route path="/reservas/no-shows" element={<EstadisticasNoShows />} />
              <Route path="/plano-sala" element={<PlanoSala />} />
              <Route path="/configuracion" element={<Configuracion />} />
              <Route path="/marketing/campanas" element={<ListaCampanas />} />
              <Route path="/marketing/campanas/nueva" element={<FormularioCampana />} />
              <Route path="/marketing/plantillas" element={<ListaPlantillas />} />
              <Route path="/marketing/plantillas/nueva" element={<FormularioPlantilla />} />
              <Route path="/marketing/recordatorios" element={<Recordatorios />} />
              <Route path="/trabajadores" element={<ListaTrabajadores />} />
              <Route path="/resenas" element={<ListaResenas />} />
              <Route path="/inactividad" element={<Inactividad />} />
              <Route path="/bdp/stock" element={<BdpStock />} />
              <Route path="/bdp/explorador" element={<BdpExplorador />} />
              <Route path="/bdp/historial" element={<BdpHistorial />} />
              <Route path="/bdp/compras" element={<BdpCompras />} />
              <Route path="/bdp/catalogo" element={<BdpCatalogo />} />
              <Route path="/bdp/inventario" element={<BdpInventario />} />
              <Route path="/bdp/sincronizacion" element={<BdpSincronizacion />} />
            </Route>

            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
          <Toaster />
        </QueryClientProvider>
      </TooltipProvider>
    </ThemeProvider>
    </ErrorBoundary>
  );
}

export default App;
