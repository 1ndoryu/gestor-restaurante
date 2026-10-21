/* [263A-16] Configuración del restaurante.
 * [D33] Reorganizada en seis pestañas por la pregunta que se hace la usuaria, no
 * por módulo técnico. Cada pestaña tiene su botón de ayuda (modal con todo el
 * detalle), su estado de cambios sin guardar y su punto de guardado; antes había
 * un «Guardar» escondido a media página y cinco tarjetas que se guardaban solas
 * sin decirlo. Lo que guarda al instante lleva su marca y no comparte formulario. */

import { useEffect, useMemo, useRef, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useConfiguracion } from '../hooks/useConfiguracion';
import IntegracionesMarketing from './IntegracionesMarketing';
import ConfigChatbot from './ConfigChatbot';
import ConfigBdp from './ConfigBdp';
import PanelBdpBackup from './PanelBdpBackup';
import BuzonAvisosCard from './config-bdp/BuzonAvisosCard';
import LibroStockCard from './config-bdp/LibroStockCard';
import OperacionesNoApiCard from './config-bdp/OperacionesNoApiCard';
import { TarjetasNegocio } from './configuracion/TarjetasNegocio';
import { TarjetasOperativas } from './configuracion/TarjetasOperativas';
import { TarjetasIntegraciones } from './configuracion/TarjetasIntegraciones';
import { TarjetaTurnos } from './configuracion/TarjetaTurnos';
import { TarjetaSeguridadTerminal } from './configuracion/TarjetaSeguridadTerminal';
import { TarjetaPermisos } from './configuracion/TarjetaPermisos';
import { TarjetaDatosPrueba } from './configuracion/TarjetaDatosPrueba';
import { TarjetaClaveGroq } from './configuracion/TarjetaClaveGroq';
import { CardSeccion, GuardadoAlInstante } from './configuracion/CardSeccion';
import { AyudaSeccion } from './configuracion/AyudaSeccion';
import { AYUDA } from './configuracion/ayuda';
import { SECCIONES, type IdSeccion } from './configuracion/secciones';
import { avisarAlSalir } from '../platform/avisos-navegador';
import axios from '@/api/axios-instance';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { toast } from 'sonner';

/** Texto comparable de un valor: los mapas son JSON y el servidor los devuelve
 * con otro espaciado, así que una diferencia de formato no es un cambio. */
function comparable(valor: unknown): string {
  if (typeof valor === 'object' && valor !== null) return JSON.stringify(valor);
  const texto = String(valor ?? '');
  if (!texto.trim().startsWith('{')) return texto;
  try {
    return JSON.stringify(JSON.parse(texto));
  } catch {
    return texto;
  }
}

/** Campos de una sección que hoy difieren del último valor guardado en el
 * servidor. `referencia` es el config tal y como llegó del backend. */
function camposSucios(
  seccion: IdSeccion,
  config: object,
  referencia: object | undefined,
): boolean {
  const campos = SECCIONES.find((s) => s.id === seccion)?.campos ?? [];
  if (campos.length === 0 || !referencia) return false;
  const actual = config as Record<string, unknown>;
  const guardado = referencia as Record<string, unknown>;
  return campos.some((campo) => comparable(actual[campo]) !== comparable(guardado[campo]));
}

/* Cabecera de cada pestaña: qué hay dentro y su ayuda completa en un modal.
 * Fuera del componente para que el modal no se remonte en cada render. */
function CabeceraSeccion({ id }: { id: IdSeccion }) {
  const def = SECCIONES.find((s) => s.id === id);
  const ayuda = AYUDA[id];
  if (!def) return null;
  return (
    <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
      <p className="text-sm text-muted-foreground">{def.descripcion}</p>
      <AyudaSeccion seccion={id} titulo={def.titulo} intro={ayuda.intro} bloques={ayuda.bloques} />
    </div>
  );
}

function Configuracion() {
  const {
    config, referencia, cambiarCampo, guardar, mensajes, cargando, guardandoEn,
  } = useConfiguracion();
  const [operandoSeed, setOperandoSeed] = useState(false);
  const location = useLocation();
  const navigate = useNavigate();
  const bdpPanelRef = useRef<HTMLDivElement>(null);
  const [activeTab, setActiveTab] = useState<IdSeccion>('restaurante');

  /* [C2-2] Navegación desde el navbar o desde las páginas BDP: con bdpArming o
   * bdpSection='bdp' se abre «A qué se conecta» y se baja al panel de seguridad.
   * [D33] La antigua pestaña «BDP» se llama ahora Conexiones; el aviso de la
   * cola y el panel embebido siguen aquí para que ese enlace no cambie. */
  useEffect(() => {
    const state = location.state as { bdpArming?: boolean; bdpSection?: string } | null;
    if (state?.bdpArming || state?.bdpSection === 'bdp') {
      setActiveTab('conexiones');
      window.setTimeout(() => {
        bdpPanelRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }, 100);
      navigate('/configuracion', { replace: true, state: null });
    }
  }, [location.state, navigate]);

  /* [303A-2] Axios (no fetch crudo) para heredar interceptores de JWT y 401. */
  async function ejecutarOperacion(endpoint: string, descripcion: string) {
    setOperandoSeed(true);
    try {
      const resp = await axios.post(`/api/${endpoint}`);
      toast.success(descripcion, { description: resp.data.mensaje });
    } catch (err: unknown) {
      const msg = (err as { response?: { data?: { message?: string } } })?.response?.data?.message
        ?? 'No se pudo conectar con el servidor';
      toast.error('Error', { description: msg });
    } finally {
      setOperandoSeed(false);
    }
  }

  const sucias = useMemo(
    () => Object.fromEntries(
      SECCIONES.map((s) => [s.id, camposSucios(s.id, config, referencia)]),
    ) as Record<IdSeccion, boolean>,
    [config, referencia],
  );

  /* Salir con cambios pendientes se avisa: antes se perdían sin decir nada. El
   * acceso a `window` está en `platform/` (límite portable de Sentinel). */
  useEffect(() => avisarAlSalir(() => Object.values(sucias).some(Boolean)), [sucias]);

  if (cargando) return <p className="text-sm text-muted-foreground">Cargando configuración...</p>;

  return (
    <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as IdSeccion)} className="max-w-5xl">
      {/* En móvil las seis pestañas caen en dos filas: sin `h-auto!` la lista
          conserva los 2 rem del diseño de origen y la segunda fila se monta
          encima del texto de la sección. */}
      <TabsList className="flex h-auto! flex-wrap justify-start">
        {SECCIONES.map((s) => (
          <TabsTrigger key={s.id} value={s.id} className="gap-2">
            {s.titulo}
            {s.campos.length > 0 && sucias[s.id] && (
              <Badge variant="outline" className="border-amber-400 px-1.5 py-0 text-[10px] text-amber-700 dark:text-amber-400">
                sin guardar
              </Badge>
            )}
          </TabsTrigger>
        ))}
      </TabsList>

      <TabsContent value="restaurante" className="mt-4 flex flex-col gap-6">
        <CabeceraSeccion id="restaurante" />
        <CardSeccion
          titulo="Marca con nombre al restaurante"
          descripcion="Lo que identifica al negocio y lo que se pide al cliente al reservar"
          sucio={sucias.restaurante}
          guardando={guardandoEn === 'restaurante'}
          mensaje={mensajes.restaurante}
          onGuardar={() => void guardar('restaurante')}
        >
          <TarjetasNegocio config={config} cambiarCampo={cambiarCampo} />
          <TarjetaTurnos config={config} cambiarCampo={cambiarCampo} />
        </CardSeccion>
      </TabsContent>

      <TabsContent value="operacion" className="mt-4 flex flex-col gap-6">
        <CabeceraSeccion id="operacion" />
        <CardSeccion
          titulo="Cómo se cierra una reserva y cómo se anula una venta"
          descripcion="Ventas automáticas, anulaciones, turnos de sincronización con Haddock y reseñas"
          sucio={sucias.operacion}
          guardando={guardandoEn === 'operacion'}
          mensaje={mensajes.operacion}
          onGuardar={() => void guardar('operacion')}
        >
          <TarjetasOperativas config={config} cambiarCampo={cambiarCampo} />
          <TarjetasIntegraciones config={config} cambiarCampo={cambiarCampo} />
        </CardSeccion>
      </TabsContent>

      <TabsContent value="conexiones" className="mt-4 flex flex-col gap-6">
        <CabeceraSeccion id="conexiones" />
        <CardSeccion
          titulo="Datos de conexión con otros sistemas"
          descripcion="TPV BDP, IA y marketing. Los diagnósticos y las pruebas no escriben nada."
          sucio={sucias.conexiones}
          guardando={guardandoEn === 'conexiones'}
          mensaje={mensajes.conexiones}
          onGuardar={() => void guardar('conexiones')}
        >
          {/* La clave de Groq pertenece a esta pestaña: sin su campo el mapa de
              campos de `secciones.ts` marcaba cambios sucios que no se veían. */}
          <TarjetaClaveGroq config={config} cambiarCampo={cambiarCampo} />
          <ConfigBdp config={config} cambiarCampo={cambiarCampo} />
        </CardSeccion>
        {/* [D33] El ancla del enlace profundo (bdpArming) baja a la tarjeta de
            modos de operación: el panel de respaldos ya vive en «Avanzado». */}
        <div ref={bdpPanelRef} className="mt-6">
          <OperacionesNoApiCard />
        </div>
        {/* Estos dos guardan solos (cada tarjeta suya): marca explícita para que
            no se espere el botón de arriba. */}
        <div className="mt-6 flex flex-col gap-2">
          <p className="text-sm font-medium">Claves de servicios externos</p>
          <GuardadoAlInstante />
          <IntegracionesMarketing />
        </div>
      </TabsContent>

      <TabsContent value="chatbot" className="mt-4 flex flex-col gap-6">
        <CabeceraSeccion id="chatbot" />
        <div className="flex flex-col gap-2">
          <GuardadoAlInstante />
          <ConfigChatbot />
        </div>
      </TabsContent>

      <TabsContent value="acceso" className="mt-4 flex flex-col gap-6">
        <CabeceraSeccion id="acceso" />
        <CardSeccion
          titulo="Permisos de cada rol"
          descripcion="Qué puede hacer un admin, un trabajador o cualquiera"
          sucio={sucias.acceso}
          guardando={guardandoEn === 'acceso'}
          mensaje={mensajes.acceso}
          onGuardar={() => void guardar('acceso')}
        >
          <TarjetaPermisos config={config} cambiarCampo={cambiarCampo} />
        </CardSeccion>
        <div className="mt-6 flex flex-col gap-2">
          <GuardadoAlInstante />
          <TarjetaSeguridadTerminal />
        </div>
      </TabsContent>

      {/* Avanzado no tiene formulario: todo lo de aquí guarda al instante y
          guarda solo (respaldos, dictámenes, datos de prueba), así que un botón
          «Guardar» sería decorativo y mentiría. */}
      <TabsContent value="avanzado" className="mt-4 flex flex-col gap-6">
        <CabeceraSeccion id="avanzado" />
        <GuardadoAlInstante />
        <PanelBdpBackup config={config} />
        <LibroStockCard />
        <BuzonAvisosCard />
        <TarjetaDatosPrueba operandoSeed={operandoSeed} ejecutarOperacion={ejecutarOperacion} />
      </TabsContent>
    </Tabs>
  );
}

export default Configuracion;
