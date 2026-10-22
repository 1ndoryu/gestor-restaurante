/* [D33] Ayuda de las tarjetas de Conexiones: el TPV (conexión, modo, escritura,
 * funciones, formas de pago, documentos) y las claves de servicios externos. */

import type { BloqueAyuda } from './AyudaTarjeta';

/** Tarjetas de este bloque; `ayuda.ts` une los tres para formar `IdAyuda`. */
export type IdAyudaConexiones =
  | 'groq'
  | 'bdp-conexion'
  | 'bdp-modo'
  | 'bdp-escritura'
  | 'bdp-funciones'
  | 'bdp-formas-pago'
  | 'bdp-documentos'
  | 'bdp-operaciones-no-api'
  | 'marketing-smtp'
  | 'marketing-twilio'
  | 'marketing-meta'
  | 'bdp-respaldos'
  | 'bdp-avisos'
  | 'bdp-libro-stock';

interface AyudaDeTarjeta {
  intro: string;
  bloques: readonly BloqueAyuda[];
}

export const AYUDA: Record<IdAyudaConexiones, AyudaDeTarjeta> = {
  groq: {
    intro: 'La clave con la que la app lee documentos por sí sola.',
    bloques: [
      {
        titulo: 'Leer facturas y albaranes con IA',
        queHace: 'Con una clave de Groq, la app reconoce el texto de facturas, albaranes y tickets al subirlos.',
        detalle: 'La clave se crea gratis en console.groq.com/keys. Sin clave, los documentos se suben igual pero hay que teclearlos.',
      },
    ],
  },
  'bdp-conexion': {
    intro: 'Los datos con los que la app entra al TPV del restaurante.',
    bloques: [
      {
        titulo: 'Conexión BDP (el TPV)',
        queHace: 'Dirección, usuario, contraseña y código de integrador del WebLink del restaurante.',
        detalle: 'Estos datos los deja el instalador de BDP: no se inventan. «Comprobar contra BDP» solo lee: no escribe nada en el terminal.',
      },
      {
        titulo: 'Actualizar estados automáticamente',
        queHace: 'Vuelve a preguntar al TPV por el estado de cada comanda enviada.',
        detalle: 'Se enciende solo al elegir el envío automático: sin ella, el estado se queda congelado en la web. Si se apaga aquí, el envío automático sigue funcionando pero los estados no se refrescan.',
      },
      {
        titulo: 'Lectura automática de maestros',
        queHace: 'Cada cuánto se refrescan por su cuenta los maestros del TPV (familias, artículos, almacenes, formas de pago).',
        detalle: '«Solo al pulsar» los deja manuales. Cada 15 minutos es el equilibrio razonable; cada minuto carga el TPV sin ganar frescura real.',
      },
    ],
  },
  'bdp-modo': {
    intro: 'El ajuste de seguridad más importante de la integración.',
    bloques: [
      {
        titulo: 'Modo de operaciones BDP',
        queHace: 'Decide si la app solo lee del TPV, pide autorización manual antes de escribir o escribe automáticamente con auto-arming.',
        detalle: 'Empieza en «Solo lectura». Cada cambio pide confirmación, destino, alcance, motivo y duración. El permiso vuelve solo a «Solo lectura» al terminar la operación.',
      },
    ],
  },
  'bdp-escritura': {
    intro: 'Los códigos y parámetros que BDP exige para aceptar una escritura.',
    bloques: [
      {
        titulo: 'Datos que se intercambian con BDP',
        queHace: 'Formas de pago, canales de pedido e IVA: traduce los nombres de la web a los códigos numéricos del TPV.',
        detalle: 'Los códigos se ven en el TPV, en la ficha de un artículo ya creado o en el alta de una forma de pago. Si falta el de IVA, BDP rechaza crear artículos («el código de IVA de venta 0 no existe»).',
      },
      {
        titulo: 'Cuándo se envía la cola y qué códigos se usan por defecto',
        queHace: 'Envío automático o manual, el primer código libre para artículos nuevos, el almacén por defecto y el motivo de regularización.',
        detalle: 'En modo automático cada cambio se envía al momento; en manual espera a que alguien pulse «Enviar a BDP». El almacén por defecto debe ser uno real del TPV.',
      },
    ],
  },
  'bdp-funciones': {
    intro: 'Capacidades del TPV que la app todavía no usa por defecto.',
    bloques: [
      {
        titulo: 'Qué funciones avanzadas del TPV se usan',
        queHace: 'Abre capacidades concretas: auto-arming de permisos, pagos parciales, cancelar comandas y compras (leer, borrar, conciliar).',
        detalle: 'Todas vienen apagadas. Se activan cuando se han probado contra el BDP real; activar una sin probar puede provocar escrituras en el TPV del restaurante.',
      },
    ],
  },
  'bdp-formas-pago': {
    intro: 'Cómo se traduce cada forma de cobro de la web al TPV.',
    bloques: [
      {
        titulo: 'Formas de pago del BDP',
        queHace: 'Compara las formas de pago dadas de alta en BDP con el mapeo configurado.',
        detalle: 'Un código que no exista en BDP hace fallar el cobro. Si una forma aparece sin pareja, asígnale la suya antes de cobrar con ella.',
      },
    ],
  },
  'bdp-documentos': {
    intro: 'Las facturas y tickets que emite el TPV.',
    bloques: [
      {
        titulo: 'Documentos de venta del BDP',
        queHace: 'Elige cada cuánto y con qué perfil de exportación se leen las facturas y tickets del TPV.',
        detalle: 'Esta lectura no escribe nada en el BDP. El perfil lo deja configurado el instalador del TPV.',
      },
    ],
  },
  'bdp-operaciones-no-api': {
    intro: 'Lo que la web no puede hacer y quién lo hace en el terminal.',
    bloques: [
      {
        titulo: 'Operaciones que BDP no expone por API',
        queHace: 'Declara, por operación, quién la hace y con qué procedimiento; la web solo muestra ese texto al personal.',
        detalle: 'El responsable por defecto se usa cuando la operación no tiene el suyo. Si no hay ninguno de los dos, la web avisa de que está pendiente de asignar.',
      },
    ],
  },
  'marketing-smtp': {
    intro: 'El servidor de correo con el que salen campañas y recordatorios.',
    bloques: [
      {
        titulo: 'Email (SMTP)',
        queHace: 'Host, puerto, usuario, contraseña y remitente de los envíos por correo.',
        detalle: 'El puerto habitual es 587 con usuario y contraseña. Los proveedores piden una «contraseña de aplicación» en vez de la de la cuenta. La contraseña no se puede volver a leer desde aquí.',
      },
    ],
  },
  'marketing-twilio': {
    intro: 'La pasarela de SMS.',
    bloques: [
      {
        titulo: 'SMS (Twilio)',
        queHace: 'Account SID, Auth Token y número remitente de Twilio.',
        detalle: 'Los tres datos están en la consola de Twilio. El número debe ser uno verificado o comprado allí; si no, los SMS salen con error.',
      },
    ],
  },
  'marketing-meta': {
    intro: 'WhatsApp por la API oficial de Meta.',
    bloques: [
      {
        titulo: 'WhatsApp (Meta)',
        queHace: 'WABA ID, app de Meta, token de acceso y el identificador del número de teléfono.',
        detalle: 'El Phone Number ID es obligatorio para enviar: sin él, los mensajes no salen aunque el resto esté bien. Está en Meta Business Suite → WhatsApp → Configuración del número.',
      },
    ],
  },
  'bdp-respaldos': {
    intro: 'Lo que hace la app antes de escribir en el TPV.',
    bloques: [
      {
        titulo: 'Seguridad, respaldos e historial BDP',
        queHace: 'Consulta los respaldos locales, el resultado de cada escritura autorizada y permite cambiar el permiso de operación.',
        detalle: 'Si una escritura aparece con resultado «parcial» o «error», aquí está el mensaje exacto del TPV.',
      },
    ],
  },
  'bdp-avisos': {
    intro: 'Lo que el TPV manda a la web.',
    bloques: [
      {
        titulo: 'Avisos del TPV',
        queHace: 'Lo que el TPV manda a la web (comandas creadas, modificadas, traspasos, cobros) y cómo se procesó.',
        detalle: 'Un aviso «rechazado» lleva el motivo del propio TPV, p. ej. «no se ha establecido una IP de Servidor de Mensajes»: eso se arregla en el terminal, no aquí.',
      },
    ],
  },
  'bdp-libro-stock': {
    intro: 'Los movimientos de stock que quedaron a medias.',
    bloques: [
      {
        titulo: 'Libro de stock',
        queHace: 'Operaciones de stock cuyo resultado no se pudo confirmar porque la respuesta se perdió.',
        detalle: 'Se dictamina una por una: «aplicada» o «no aplicada». Sin dictamen, el artículo queda bloqueado para nuevas operaciones.',
      },
    ],
  },
};
