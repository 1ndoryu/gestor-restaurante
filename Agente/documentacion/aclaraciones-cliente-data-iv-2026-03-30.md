# Aclaraciones para el Cliente — Data IV (2026-03-30)

## 1. Plano de sala — min/max personas no actualizan al cambiar mesa
**Duda:** Al seleccionar una mesa distinta, los datos de min/max personas no cambiaban a menos que se hiciera clic fuera primero.
**Solución:** Corregido en tarea 303A-9. Ahora el panel de configuración se remonta completamente al seleccionar otra mesa (`key={mesaSeleccionada.id}`), garantizando que los datos se refrescan.

## 2. Plantillas WhatsApp — cabecera, pie y media
**Duda:** ¿Qué son la cabecera, el pie y el campo "media" de las plantillas?
**Aclaración:**
- **Cabecera (header):** Texto que aparece en negrita/grande al inicio del mensaje de WhatsApp. Meta lo muestra destacado arriba del cuerpo.
- **Pie (footer):** Texto pequeño que aparece al final del mensaje, debajo del cuerpo. Típicamente se usa para disclaimers o información legal.
- **Media (URL):** Meta requiere que la media (imagen, video, documento) se suba como URL pública. No soporta subir archivos directamente desde el ordenador. El flujo es: subir la imagen a un servicio de hosting (Google Drive público, Imgur, servidor propio, etc.) y pegar la URL aquí. Esto es una limitación de la API de Meta, no del sistema.
- **Flujo de envío:** Se crea la plantilla como borrador → se envía a Meta para aprobación → Meta la revisa (puede tardar minutos u horas) → cuando está aprobada, se puede usar en campañas.

## 3. Campos obligatorios — fecha y hora
**Duda:** Falta fecha y hora en la sección "campos obligatorios al reservar" de Configuración.
**Solución:** Corregido en tarea 303A-16. Ahora fecha y hora aparecen como campos siempre activos (no se pueden desactivar porque una reserva sin fecha/hora no es válida). Son visibles en la configuración para que se vea la lista completa.

## 4. IVA por defecto vs. IVA por venta
**Duda:** ¿El IVA de Configuración y el de "nueva venta" son lo mismo? ¿Puedo poner un IVA diferente en una venta específica?
**Aclaración:**
- **IVA de Configuración** es el valor por defecto que se aplica automáticamente cuando se crea una venta nueva. Si se configura al 10%, todas las ventas nuevas empezarán con 10% de IVA.
- **IVA por venta** (en "opciones avanzadas" del formulario de venta) permite modificar el IVA de esa venta específica. Esto es para excepciones: si normalmente es 10% pero una venta puntual tiene 15%, se cambia solo en esa venta.
- **No hay conflicto:** Cambiar el IVA en Configuración no afecta las ventas ya creadas. Solo cambia el default para ventas futuras. Cada venta guarda su propio IVA.

## 5. API del chatbot — Groq vs otros modelos
**Duda:** El campo "Groq API key" — ¿solo funciona con Groq o se puede usar cualquier modelo?
**Aclaración:**
- El campo dice "Groq" pero internamente el asistente usa la API con formato compatible con OpenAI. Groq es un proveedor que ofrece modelos de inteligencia artificial rápidos y con capa gratuita generosa.
- **Sí se puede usar cualquier modelo compatible con la API de OpenAI** (por ejemplo, OpenAI directamente, Together AI, Fireworks, etc.), siempre que el endpoint sea compatible. Sin embargo, actualmente el sistema está configurado para usar Groq por defecto porque ofrece una capa gratuita.
- Si se desea usar otro proveedor, se necesitaría cambiar la URL de la API en la configuración del backend (variable de entorno). Esto no es algo que el usuario final configure, sino el administrador técnico.

## 6. Datos de prueba
**Duda:** ¿Cuál es la diferencia entre "Eliminar datos" y "Recargar datos de prueba"?
**Aclaración:**
- **Eliminar datos de prueba:** Borra TODOS los datos de la cuenta (ventas, gastos, reservas, clientes, etc.), dejando la cuenta completamente vacía. Es un reset total.
- **Recargar datos de prueba:** Primero borra todo (como el anterior) y luego carga un conjunto de datos de demostración prediseñados. La cuenta queda con datos ficticios para explorar y probar la plataforma.
- En resumen: "Eliminar" = cuenta en blanco; "Recargar" = cuenta con datos de ejemplo.

## 7. Reportar error — ¿a dónde llegan los reportes?
**Duda:** Cuando el usuario envía un reporte de error, ¿a dónde llega?
**Aclaración:** Los reportes de error se envían a **CORREO_AUTOR** (configurado en el backend como email de destino para reportes). El usuario escribe lo que ocurrió y al enviar, llega un email con los detalles al correo del desarrollador/administrador.
