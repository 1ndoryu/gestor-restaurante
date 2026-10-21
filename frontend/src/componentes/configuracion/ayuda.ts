/* [D33] Texto de la ayuda de cada pestaña de Configuración. Una sola fuente por
 * pestaña, en el mismo orden en que aparecen las tarjetas: quien cambia un ajuste
 * actualiza su bloque aquí, sin abrir el formulario. */

import type { BloqueAyuda } from './AyudaSeccion';
import type { IdSeccion } from './secciones';

interface AyudaDeSeccion {
  intro: string;
  bloques: readonly BloqueAyuda[];
}

export const AYUDA: Record<IdSeccion, AyudaDeSeccion> = {
  restaurante: {
    intro: 'Lo que identifica al restaurante y lo que se le pide al cliente al reservar.',
    bloques: [
      {
        titulo: 'El nombre que sale en los tickets y en la web',
        queHace: 'Es el nombre del establecimiento en la aplicación, en los documentos que se generan y en el enlace público de reservas.',
        detalle: 'Si se deja vacío, algunos documentos salen sin nombre. Cámbialo y vuelve a emitir el documento que ya hayas impreso.',
      },
      {
        titulo: 'Qué datos se piden al reservar',
        queHace: 'Marca qué campos son obligatorios al crear una reserva: nombre, apellidos, email y teléfono.',
        detalle: 'Fecha y hora no se pueden desactivar: una reserva sin ellas no existe. Si exiges demasiado, el cliente abandona el formulario; con nombre y teléfono suele bastar.',
      },
      {
        titulo: 'El IVA que se aplica por defecto',
        queHace: 'Es el IVA que llevan los artículos nuevos si nadie lo cambia en el alta.',
        detalle: 'En España, hostelería suele ser el 10 %. Un valor equivocado obliga a corregir el IVA artículo por artículo.',
      },
      {
        titulo: 'Horario de los turnos',
        queHace: 'Define los rangos de desayuno, comida y cena con los que se clasifican las reservas y las ventas.',
        detalle: 'Los rangos no deben solaparse: una reserva cae en el primer turno que la contiene. Ajústalos a tu horario real, no al de la plantilla.',
      },
    ],
  },
  operacion: {
    intro: 'Cómo se cierra el ciclo de una reserva, cómo se anula una venta y con qué herramientas externas se sincroniza.',
    bloques: [
      {
        titulo: 'Una venta al completar cada reserva',
        queHace: 'Crea la venta automáticamente cuando una reserva se marca como completada.',
        detalle: 'Si lo desactivas, cada reserva completada habrá que pasarla a venta a mano o no contará en los ingresos.',
      },
      {
        titulo: 'Cómo se anula una venta',
        queHace: '«Crédito completo» exige motivo y la resta del resumen diario; «Solo estado» solo marca la venta como anulada.',
        detalle: 'Si la anulación tiene que cuadrar con el cierre de caja, usa crédito completo. El cambio afecta a las anulaciones siguientes, no a las ya hechas.',
      },
      {
        titulo: 'Enlace y sincronización con Haddock',
        queHace: 'Añade un enlace a tu cuenta de Haddock y sincroniza las ventas por su POS API.',
        detalle: 'El token se obtiene en Haddock → Configuración → Integraciones → POS API → Conectar. Sin token, el enlace funciona pero no se sincroniza nada.',
      },
      {
        titulo: 'Pedir reseñas y enlazar WhatsApp',
        queHace: 'Decide a dónde va un cliente que puntúa 4-5 estrellas, el teléfono que se enlaza en WhatsApp y el botón de reserva de esos mensajes.',
        detalle: 'El enlace de Google Reviews se copia del botón «Pedir reseñas» de la ficha de Google del restaurante. Si está vacío, no se ofrece reseña.',
      },
    ],
  },
  conexiones: {
    intro: 'Todo lo que sale de la app hacia otro sistema: el TPV, el chatbot, la IA y el marketing.',
    bloques: [
      {
        titulo: 'Conexión BDP (el TPV)',
        queHace: 'Dirección, usuario, contraseña y código de integrador del WebLink del restaurante.',
        detalle: 'Estos datos los deja el instalador de BDP: no se inventan. «Diagnóstico» comprueba que la dirección responde y que el usuario entra; no escribe nada.',
      },
      {
        titulo: 'Datos que se intercambian con BDP',
        queHace: 'Formas de pago, canales de pedido e IVA: traduce los nombres de la web a los códigos numéricos del TPV.',
        detalle: 'Los códigos se ven en el TPV, en la ficha de un artículo ya creado o en el alta de una forma de pago. Si falta el de IVA, BDP rechaza crear artículos («el código de IVA de venta 0 no existe»).',
      },
      {
        titulo: 'Parámetros de escritura',
        queHace: 'Cuándo se envía la cola a BDP, el primer código libre para artículos nuevos, el almacén por defecto y el motivo de regularización.',
        detalle: 'En modo automático cada cambio se envía al momento; en manual espera a que alguien pulse «Enviar a BDP». El almacén por defecto debe ser uno real del TPV.',
      },
      {
        titulo: 'Qué funciones avanzadas del TPV se usan',
        queHace: 'Abre capacidades concretas: auto-arming de permisos, pagos parciales, cancelar comandas y compras (leer, borrar, conciliar).',
        detalle: 'Todas vienen apagadas. Se activan cuando se han probado contra el BDP real; activar una sin probar puede provocar escrituras en el TPV del restaurante.',
      },
      {
        titulo: 'Modo de operaciones BDP',
        queHace: 'Decide si la app solo lee del TPV, pide autorización manual antes de escribir o escribe automáticamente con auto-arming.',
        detalle: 'Empieza en «Solo lectura». Cada cambio pide confirmación, destino, alcance, motivo y duración. Es el ajuste de seguridad más importante de esta pestaña.',
      },
      {
        titulo: 'Leer facturas y albaranes con IA',
        queHace: 'Con una clave de Groq, la app reconoce el texto de facturas, albaranes y tickets al subirlos.',
        detalle: 'La clave se crea gratis en console.groq.com/keys. Sin clave, los documentos se suben igual pero hay que teclearlos.',
      },
      {
        titulo: 'Chatbot y marketing',
        queHace: 'Claves API del chatbot, servidor de correo, Twilio y Meta para los envíos de marketing.',
        detalle: 'Los secretos (claves, tokens) se muestran una sola vez al crearlos y no se pueden volver a leer desde aquí. Cada proveedor los da en su panel.',
      },
    ],
  },
  chatbot: {
    intro: 'Las claves con las que un chatbot externo entra a los datos del restaurante.',
    bloques: [
      {
        titulo: 'Crear una clave API',
        queHace: 'Genera una clave para que el chatbot consulte reservas, carta y disponibilidad.',
        detalle: 'La clave se ve una sola vez: cópiala al crearla. Si se pierde, se revoca y se crea otra; no se puede recuperar.',
      },
      {
        titulo: 'Revocar una clave',
        queHace: 'Anula una clave existente al instante.',
        detalle: 'El chatbot deja de funcionar en el siguiente mensaje. Revoca las claves que ya no uses: una clave olvidada es una puerta abierta.',
      },
    ],
  },
  acceso: {
    intro: 'Quién puede hacer cada acción sensible y cómo se protege el terminal del local.',
    bloques: [
      {
        titulo: 'Quién puede tocar cada cosa',
        queHace: 'Elige, por acción (catálogo, stock, albaranes, anulaciones, pagos y facturas), si puede hacerlo solo el admin, también los trabajadores o cualquiera.',
        detalle: 'El servidor aplica el permiso en cada operación: la pantalla solo lo declara. Un cambio afecta al siguiente intento, no a lo ya hecho.',
      },
      {
        titulo: 'Contraseña del terminal',
        queHace: 'Rota la contraseña del terminal del local pidiendo antes la actual.',
        detalle: 'Estar dentro de una sesión abierta no basta para cambiarla. Cada intento de acceso queda registrado, con el resultado y sin guardar nunca la contraseña.',
      },
    ],
  },
  avanzado: {
    intro: 'El detalle de la integración con el TPV y el contenido de prueba. Aquí se entra cuando algo falla o hay que diagnosticar.',
    bloques: [
      {
        titulo: 'Seguridad, respaldos e historial BDP',
        queHace: 'Consulta los respaldos locales, el resultado de cada escritura autorizada y permite cambiar el permiso de operación.',
        detalle: 'El permiso vuelve solo a «Solo lectura» al terminar la operación. Si una escritura aparece con resultado «parcial» o «error», aquí está el mensaje exacto del TPV.',
      },
      {
        titulo: 'Avisos del TPV',
        queHace: 'Lo que el TPV manda a la web (comandas creadas, modificadas, traspasos, cobros) y cómo se procesó.',
        detalle: 'Un aviso «rechazado» lleva el motivo del propio TPV, p. ej. «no se ha establecido una IP de Servidor de Mensajes»: eso se arregla en el terminal, no aquí.',
      },
      {
        titulo: 'Libro de stock',
        queHace: 'Operaciones de stock cuyo resultado no se pudo confirmar porque la respuesta se perdió.',
        detalle: 'Se dictamina una por una: «aplicada» o «no aplicada». Sin dictamen, el artículo queda bloqueado para nuevas operaciones.',
      },
      {
        titulo: 'Datos de ejemplo y borrado',
        queHace: 'Muestra datos ficticios en las pantallas BDP y permite recargar o borrar el contenido de prueba de la cuenta.',
        detalle: 'El modo de ejemplo no escribe en el TPV. «Borrar todos los datos» elimina reservas, ventas, clientes, artículos y comandas, y conserva la cuenta: no se puede deshacer.',
      },
    ],
  },
};
