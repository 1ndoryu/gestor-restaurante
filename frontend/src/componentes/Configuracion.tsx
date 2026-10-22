/* [263A-16] Configuración del restaurante.
 * [D33] Reorganizada en seis pestañas por la pregunta que se hace la usuaria, no
 * por módulo técnico. Cada pestaña tiene su botón de ayuda (modal con todo el
 * detalle), su estado de cambios sin guardar y su punto de guardado; antes había
 * un «Guardar» escondido a media página y cinco tarjetas que se guardaban solas
 * sin decirlo. Lo que guarda al instante lleva su marca y no comparte formulario. */

import { useEffect, useMemo, useRef, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useConfiguracion } from '../hooks/useConfiguracion';
import { useIntegraciones } from '../hooks/useIntegraciones';
import IntegracionesMarketing from './IntegracionesMarketing';
import ConfigChatbot from './ConfigChatbot';
import ConfigBdp from './ConfigBdp';
import PanelBdpBackup from './PanelBdpBackup';
import BuzonAvisosCard from './config-bdp/BuzonAvisosCard';
import { DocumentosVentaCard } from './config-bdp/DocumentosVentaCard';
import LibroStockCard from './config-bdp/LibroStockCard';
import OperacionesNoApiCard from './config-bdp/OperacionesNoApiCard';
import { useOperacionesNoApi } from './config-bdp/useOperacionesNoApi';
import { TarjetasNegocio } from './configuracion/TarjetasNegocio';
import { TarjetasOperativas } from './configuracion/TarjetasOperativas';
import { TarjetasIntegraciones } from './configuracion/TarjetasIntegraciones';
import { TarjetaTurnos } from './configuracion/TarjetaTurnos';
import { TarjetaSeguridadTerminal } from './configuracion/TarjetaSeguridadTerminal';
import { TarjetaPermisos } from './configuracion/TarjetaPermisos';
import { TarjetaDatosPrueba } from './configuracion/TarjetaDatosPrueba';
import { TarjetaClaveGroq } from './configuracion/TarjetaClaveGroq';
import { CardSeccion } from './configuracion/CardSeccion';
import { BarraGuardado } from './configuracion/BarraGuardado';
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

/* [D33b] Cabecera de cada pestaña: una línea que dice qué contiene. La ayuda ya
 * no va aquí sino en la cabecera de cada tarjeta: una pestaña mezcla siete
 * ajustes y un modal único obligaba a leerlos todos para encontrar uno. */
function CabeceraSeccion({ id }: { id: IdSeccion }) {
  const def = SECCIONES.find((s) => s.id === id);
  if (!def) return null;
  return <p className="text-sm text-muted-foreground">{def.descripcion}</p>;
}

function Configuracion() {
  const {
    config, referencia, cambiarCampo, guardar, deshacer, mensajes, cargando, guardandoEn,
  } = useConfiguracion();
  /* [D33] Marketing viaja por otro endpoint pero comparte botón con Conexiones,
   * así que su estado entra en el mismo cálculo de cambios y en el mismo
   * «Deshacer». */
  const marketing = useIntegraciones();
  const operacionesNoApi = useOperacionesNoApi();
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

  /* [D33] Los campos de Conexiones incluyen los de marketing, que viajan por su
   * propio endpoint: sin sumarlos, esa pestaña se podía abandonar con claves a
   * medio escribir y el botón no se encendía al tocarlas. */
  const sucias = useMemo(() => {
    const base = Object.fromEntries(
      SECCIONES.map((s) => [s.id, camposSucios(s.id, config, referencia)]),
    ) as Record<IdSeccion, boolean>;
    return {
      ...base,
      conexiones: base.conexiones || marketing.sucio || operacionesNoApi.sucio,
    };
  }, [config, referencia, marketing.sucio, operacionesNoApi.sucio]);

  /* [D33] Un único punto de guardado por pestaña, y siempre en la esquina. Las
   * pestañas que solo tienen tarjetas de guardado al instante no lo muestran:
   * un «Guardar» ahí sería decorativo. */
  const seccionActual = SECCIONES.find((s) => s.id === activeTab);
  const puedeGuardar = (seccionActual?.campos.length ?? 0) > 0;

  /* Conexiones guarda tres cosas con un botón: la configuración, las claves de
   * marketing y las operaciones no API. Cada una va por su endpoint y solo se
   * envía lo que tiene cambios. */
  const guardarSeccion = async (seccion: IdSeccion) => {
    if (seccion !== 'conexiones') {
      await guardar(seccion);
      return;
    }
    await Promise.all([
      guardar(seccion),
      marketing.sucio ? marketing.guardar() : Promise.resolve(),
      operacionesNoApi.sucio ? operacionesNoApi.guardarTodo() : Promise.resolve(),
    ]);
  };

  const deshacerSeccion = (seccion: IdSeccion) => {
    deshacer(seccion);
    if (seccion !== 'conexiones') return;
    if (marketing.sucio) marketing.deshacer();
    if (operacionesNoApi.sucio) operacionesNoApi.deshacerTodo();
  };

  /* [D33] No se cambia de pestaña con cambios sin guardar. Se avisa y se deja
   * ahí, en vez de perderlos o de saltar a otra sección: el aviso «sin
   * guardar» del tab y el botón «Deshacer» son la salida. */
  const cambiarPestana = (valor: string) => {
    const destino = valor as IdSeccion;
    if (destino === activeTab) return;
    if (sucias[activeTab]) {
      toast.error('Hay cambios sin guardar', {
        description: 'Guárdalos o pulsa «Deshacer» para cambiar de pestaña sin perderlos.',
      });
      return;
    }
    setActiveTab(destino);
  };

  /* Salir con cambios pendientes se avisa: antes se perdían sin decir nada. El
   * acceso a `window` está en `platform/` (límite portable de Sentinel). */
  useEffect(() => avisarAlSalir(() => Object.values(sucias).some(Boolean)), [sucias]);

  if (cargando) return <p className="text-sm text-muted-foreground">Cargando configuración...</p>;

  return (
    <Tabs
      value={activeTab}
      onValueChange={cambiarPestana}
      className="relative flex max-w-5xl flex-col pb-20"
    >
      {/* Seis pestañas no caben en el móvil. No se parte en filas: el trigger
          compartido mide `h-[calc(100%-1px)]`, así que con varias filas la lista
          no crece a su contenido y la última fila se monta encima de la sección.
          En una sola fila con scroll horizontal cada pestaña conserva su altura
          y el panel queda siempre debajo. */}
      <TabsList className="flex h-auto! max-w-full justify-start overflow-x-auto">
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

      <TabsContent value="restaurante" className="mt-4 flex flex-col gap-8">
        <CabeceraSeccion id="restaurante" />
        <CardSeccion
          titulo="Marca con nombre al restaurante"
          descripcion="Lo que identifica al negocio y lo que se pide al cliente al reservar"
        >
          <TarjetasNegocio config={config} cambiarCampo={cambiarCampo} />
          <TarjetaTurnos config={config} cambiarCampo={cambiarCampo} />
        </CardSeccion>
      </TabsContent>

      <TabsContent value="operacion" className="mt-4 flex flex-col gap-8">
        <CabeceraSeccion id="operacion" />
        <CardSeccion
          titulo="Cómo se cierra una reserva y cómo se anula una venta"
          descripcion="Ventas automáticas, anulaciones, turnos de sincronización con Haddock y reseñas"
        >
          <TarjetasOperativas config={config} cambiarCampo={cambiarCampo} />
          <TarjetasIntegraciones config={config} cambiarCampo={cambiarCampo} />
        </CardSeccion>
      </TabsContent>

      <TabsContent value="conexiones" className="mt-4 flex flex-col gap-8">
        <CabeceraSeccion id="conexiones" />
        <CardSeccion
          titulo="Datos de conexión con otros sistemas"
          descripcion="TPV BDP, IA y marketing. Los diagnósticos y las pruebas no escriben nada."
        >
          {/* La clave de Groq pertenece a esta pestaña: sin su campo el mapa de
              campos de `secciones.ts` marcaba cambios sucios que no se veían. */}
          <TarjetaClaveGroq config={config} cambiarCampo={cambiarCampo} />
          <ConfigBdp config={config} cambiarCampo={cambiarCampo} />
          {/* Los documentos se guardan con el mismo botón: su cadencia y su
              perfil son campos de esta pestaña, no de una tarjeta aparte. */}
          <DocumentosVentaCard config={config} cambiarCampo={cambiarCampo} />
          {/* [D33] El ancla del enlace profundo (bdpArming) baja a esta tarjeta:
              el panel de respaldos ya vive en «Avanzado». Va dentro de la misma
              sección para que su ritmo y su ancho sean los de las demás. */}
          <div ref={bdpPanelRef} className="scroll-mt-4">
            <OperacionesNoApiCard operacionesNoApi={operacionesNoApi} />
          </div>
        </CardSeccion>
        {/* Las claves de marketing viajan por otro endpoint, pero se guardan con
            el botón de la pestaña: dos «Guardar» en la misma pantalla es justo lo
            que se estaba corrigiendo. */}
        <CardSeccion
          titulo="Claves de servicios externos"
          descripcion="Las cuentas con las que salen los correos y los mensajes"
        >
          <IntegracionesMarketing integraciones={marketing} />
        </CardSeccion>
      </TabsContent>

      <TabsContent value="chatbot" className="mt-4 flex flex-col gap-8">
        <CabeceraSeccion id="chatbot" />
        <CardSeccion
          titulo="Claves de acceso del chatbot"
          descripcion="Cómo entra un chatbot externo a las reservas y a la carta"
          alInstante
        >
          <ConfigChatbot />
        </CardSeccion>
      </TabsContent>

      <TabsContent value="acceso" className="mt-4 flex flex-col gap-8">
        <CabeceraSeccion id="acceso" />
        <CardSeccion
          titulo="Permisos de cada rol"
          descripcion="Qué puede hacer un admin, un trabajador o cualquiera"
        >
          <TarjetaPermisos config={config} cambiarCampo={cambiarCampo} />
        </CardSeccion>
        <CardSeccion
          titulo="Contraseña del terminal del local"
          descripcion="Lo que protege la pantalla que está en la barra"
          alInstante
        >
          <TarjetaSeguridadTerminal />
        </CardSeccion>
      </TabsContent>

      {/* Avanzado no tiene formulario: todo lo de aquí guarda al instante y
          guarda solo (respaldos, dictámenes, datos de prueba), así que un botón
          «Guardar» sería decorativo y mentiría. */}
      <TabsContent value="avanzado" className="mt-4 flex flex-col gap-8">
        <CabeceraSeccion id="avanzado" />
        <CardSeccion
          titulo="Respaldos, avisos y datos de prueba"
          descripcion="Lo que la app guarda del TPV y el contenido ficticio para probar"
          alInstante
        >
          <PanelBdpBackup config={config} />
          <LibroStockCard />
          <BuzonAvisosCard />
          <TarjetaDatosPrueba operandoSeed={operandoSeed} ejecutarOperacion={ejecutarOperacion} />
        </CardSeccion>
      </TabsContent>
      {/* Fuera de los `TabsContent` para que no se remonte al cambiar de
          pestaña; su estado sale de la pestaña activa. */}
      {puedeGuardar && (
        <BarraGuardado
          sucio={sucias[activeTab]}
          guardando={
            guardandoEn === activeTab
            || (activeTab === 'conexiones' && (marketing.guardando || operacionesNoApi.guardando !== null))
          }
          mensaje={mensajes[activeTab] || (activeTab === 'conexiones' ? marketing.mensaje : '')}
          onGuardar={() => void guardarSeccion(activeTab)}
          onDeshacer={() => deshacerSeccion(activeTab)}
        />
      )}
    </Tabs>
  );
}

export default Configuracion;
