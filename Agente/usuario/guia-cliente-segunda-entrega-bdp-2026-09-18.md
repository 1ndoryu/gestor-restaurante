# Guía del cliente — Segunda entrega: lo nuevo desde julio

> **Fecha:** 18 de septiembre de 2026
> **Continúa a:** Guía del cliente — Revisión de la integración con BDP (19/07/2026)
> **Objetivo:** resumir en lenguaje sencillo lo añadido y comprobado desde la
> primera guía. Lo marcado como verificado se comprobó con operaciones reales
> de prueba en septiembre (importes mínimos, fuera de horas de servicio,
> anuladas después).

## 1. Lo que funciona sin el BDP (casi todo)

El día a día del restaurante no necesita la conexión:

- **Ventas:** vender y cobrar (en uno o varios pagos), propinas, anular ventas
  con motivo, factura local e historial de ventas.
- **Reservas:** calendario, canales de reserva y control de no-shows.
- **Clientes:** altas, edición y puntos de fidelización.
- **Catálogo local:** artículos, departamentos, familias, precios e impuestos.
- **Menús y packs** locales.
- **Compras y gastos:** albaranes, borradores y conciliación.
- **Stock local e inventario:** ajustes de stock, conteos y aplicación al stock.
- **Plano de sala** local (zonas y mesas).
- **Marketing:** campañas, plantillas y recordatorios de WhatsApp.
- **Gestión:** trabajadores y permisos, reseñas, control de inactividad,
  configuración, copias de seguridad y reporte de errores.
- **Acceso** con usuario y contraseña.

Todo lo de los puntos 2 y 3 requiere conexión; sin ella, esos botones aparecen
ocultos o desactivados.

## 2. Lo que la aplicación puede recibir del BDP (lectura)

- **Catálogo:** 559 artículos con precio, impuesto y familia (verificado 18/09).
- **Clientes,** con revisión previa: lo nuevo se copia, lo existente se vincula,
  nada se reemplaza sin avisar (verificado 18/09).
- **Plano de sala:** 7 salones y 87 mesas traídos al plano local (verificado 18/09).
- **Datos de referencia** (departamentos, formas de pago, puestos, empleados),
  que la aplicación usa para cuadrar ventas y pagos.
- **Albaranes de compra** (pendiente: indicar el código del perfil de
  exportación del terminal).
- **Detalle de una comanda** (totales y pagos; pendiente: suscripción de pago).
- **Existencias de stock** por artículo (pendiente: crear el almacén en el TPV).
- Cada importación muestra primero una vista previa y pide confirmación
  explícita antes de aplicar. Al aplicar solo se crea lo que falta; nunca se
  borra nada.

## 3. Lo que la aplicación puede enviar al BDP (escritura)

Verificado con operaciones reales:

- Crear un cliente con código nuevo, sin tocar los existentes.
- Crear una comanda, también ya cobrada al crearla.
- Añadir propina a una venta.
- Sumar o restar puntos de fidelización, siempre con motivo.
- Anular una comanda.
- Dar de alta y modificar artículos y departamentos (en pausa por decisión del
  restaurante: lo creado no se puede borrar).

Pendiente de un paso en el terminal o en la suscripción, sin cambios en la
aplicación:

- Cobrar una comanda ya existente (pendiente: suscripción de pago).
- Facturar una comanda (pendiente: probarla en real; una factura no se puede deshacer).
- Mover stock o inventario (pendiente: crear el almacén en el TPV).
- Mostrar el aviso de camarero en el terminal (pendiente: poner la IP del
  servidor de mensajes en la configuración del terminal).

## 4. Lo que no puede escribir (limitación del BDP, sin arreglo posible)

- **Borrar** artículos o departamentos: solo se pueden desactivar.
- **Modificar** una comanda ya enviada: hay que anularla y crear otra.

## 5. Lo que no puede leer (limitación del BDP, sin arreglo posible)

- **Avisos automáticos:** el BDP no avisa cuando algo cambia; la aplicación
  tiene que preguntar.

## 6. Cómo se trabaja

- Estado normal en solo lectura; cada envío requiere autorización puntual y el
  sistema vuelve solo a lectura después de operar.
- No existe sincronización automática en las dos direcciones, para evitar
  duplicados.
- Lo editado a mano no lo borra la conexión.
- Cada trabajador ve su menú según su función; lo técnico queda reservado al
  dueño (delegable por sección si se desea).
- Los respaldos protegen los datos de la aplicación; no pueden deshacer nada
  dentro del BDP.

## 7. Pendiente del restaurante (nada urgente, nada bloqueante)

1. Código del perfil de exportación (albaranes).
2. Suscripción de pago (cobrar comandas existentes, detalle y factura).
3. Crear el almacén en el TPV (stock e inventario).
4. IP del servidor de mensajes en el terminal (aviso de camarero).
5. Anular en el terminal la comanda de prueba 6258 y el departamento de
   prueba 901 cuando se indique.
