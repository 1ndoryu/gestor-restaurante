# Guía del cliente — Segunda entrega: lo nuevo desde julio

> **Fecha:** 18 de septiembre de 2026
> **Continúa a:** Guía del cliente — Revisión de la integración con BDP (19/07/2026)
> **Objetivo:** resumir en lenguaje sencillo lo añadido y comprobado desde la
> primera guía. Lo marcado como verificado se comprobó con operaciones reales
> de prueba en septiembre (importes mínimos, fuera de horas de servicio,
> anuladas después).

## 1. Lo que funciona sin el BDP (casi todo)

El día a día del restaurante no necesita la conexión:

- **Ventas:** vender y cobrar en uno o varios pagos, propinas, anular ventas
  pidiendo el motivo, factura local con su numeración, historial completo y
  eliminación de ventas que nunca se enviaron.
- **Reservas:** calendario por días, canales de reserva y control de no-shows
  con estadísticas.
- **Clientes:** altas, edición, saldo de puntos y etiquetas.
- **Catálogo local:** artículos con precio, impuesto y familia, departamentos,
  familias y códigos de barras.
- **Menús y packs** locales.
- **Compras y gastos:** albaranes con serie propia, borradores, conciliación
  con gastos y filtros por proveedor y fechas.
- **Stock local e inventario:** ajustes de stock, conteos físicos por zonas y
  aplicación de los conteos al stock.
- **Plano de sala** local, con zonas y mesas editables.
- **Marketing:** campañas, plantillas de WhatsApp y recordatorios.
- **Gestión:** trabajadores con permisos por sección, reseñas de clientes,
  control de inactividad, configuración, copias de seguridad y botón de
  reportar errores.
- **Acceso** con usuario y contraseña.

Todo lo de los puntos 2 y 3 requiere conexión; sin ella, esos botones aparecen
ocultos o desactivados y nada finge haberse hecho.

## 2. Lo que la aplicación puede recibir del BDP (lectura)

Importaciones comprobadas una por una en la web del restaurante (18/09):

- **Catálogo:** 559 artículos con precio, impuesto y familia.
- **Clientes,** con revisión previa: lo nuevo se copia, lo existente se vincula,
  nada se reemplaza sin avisar (en la prueba: 1 nuevo y 4 con datos incompletos).
- **Plano de sala:** 7 salones y 87 mesas traídos al plano local.
- **Datos de referencia** (19 departamentos, 10 formas de pago, puestos y
  empleados), que la aplicación usa para cuadrar ventas y pagos.
- **Albaranes de compra** (pendiente: indicar el código del perfil de
  exportación del terminal).
- **Detalle de una comanda** (totales y pagos; pendiente: suscripción de pago).
- **Existencias de stock** por artículo (pendiente: crear el almacén en el TPV).
- Cada importación muestra primero una vista previa con el detalle (nuevos,
  vinculados, inválidos) y pide confirmación explícita antes de aplicar. Al
  aplicar solo se crea lo que falta; nunca se borra ni se duplica nada.

## 3. Lo que la aplicación puede enviar al BDP (escritura)

Verificado con operaciones reales de prueba:

- Crear un cliente con código nuevo, sin tocar los existentes (cliente 900001).
- Crear una comanda, también ya cobrada al crearla (comandas 6258 y 6338,
  0,11 € cada una).
- Añadir propina a una venta (0,01 € sobre la 6258, confirmada).
- Sumar o restar puntos de fidelización, siempre con motivo (probado +1/−1,
  neto cero).
- Anular una comanda (la 6338 se anuló por este medio y quedó registrada).
- Dar de alta y modificar artículos y departamentos (artículo 90000003 y
  departamento 901; en pausa por decisión del restaurante: lo creado no se
  puede borrar).

Pendiente de un paso en el terminal o en la suscripción, sin cambios en la
aplicación:

- Cobrar una comanda ya existente (pendiente: suscripción de pago).
- Facturar una comanda (pendiente: probarla en real; una factura no se puede deshacer).
- Mover stock o inventario (pendiente: crear el almacén en el TPV).
- Mostrar el aviso de camarero en el terminal (pendiente: poner la IP del
  servidor de mensajes en la configuración del terminal).

## 4. Estado de la conexión de un vistazo

| Operación                                                             | Estado                                      |
| --------------------------------------------------------------------- | ------------------------------------------- |
| Recibir catálogo, clientes y plano                                    | Funciona (verificado el 18/09)              |
| Recibir albaranes                                                     | Funciona cuando se indique el perfil        |
| Recibir detalle de comandas y existencias                             | Funciona cuando se active lo pendiente (§8) |
| Crear cliente, comanda (cobrada o no), propina, puntos, anular, altas | Funciona (verificado en septiembre)         |
| Cobrar comanda existente, facturar, stock, camarero                   | Funciona cuando se active lo pendiente (§8) |
| Borrar artículos, modificar comanda enviada, avisos automáticos       | No lo permite el BDP                        |

## 5. Lo que no puede escribir (limitación del BDP, sin arreglo posible)

- **Borrar** artículos o departamentos: solo se pueden desactivar (quedan
  ocultos pero existen).
- **Modificar** una comanda ya enviada: hay que anularla y crear otra.

## 6. Lo que no puede leer (limitación del BDP, sin arreglo posible)

- **Avisos automáticos:** el BDP no avisa cuando algo cambia; la aplicación
  repasa periódicamente para ponerse al día.

## 7. Cómo se trabaja

- Estado normal en solo lectura, con indicador visible del modo en pantalla.
- Cada envío importante muestra antes su vista previa y requiere autorización
  puntual; después el sistema vuelve solo a lectura.
- No existe sincronización automática en las dos direcciones, para evitar
  duplicados: cada movimiento queda registrado con quién lo hizo y cuándo.
- Lo editado a mano no lo borra la conexión.
- Cada trabajador ve su menú según su función; lo técnico (configuración,
  trabajadores, sincronización) queda reservado al dueño, delegable por
  sección si se desea.
- Los respaldos protegen los datos de la aplicación; no pueden deshacer nada
  dentro del BDP (una comanda, un pago o una factura solo se corrigen con el
  procedimiento del restaurante en el BDP).

## 8. Pendiente del restaurante (nada urgente, nada bloqueante)

| Pendiente                                  | Lo hace                         | Activa                                        |
| ------------------------------------------ | ------------------------------- | --------------------------------------------- |
| Código del perfil de exportación           | Restaurante (en el terminal)    | Recibir albaranes                             |
| Suscripción de pago                        | Restaurante (con el proveedor)  | Cobrar comandas existentes, detalle y factura |
| Crear el almacén en el TPV                 | Restaurante (en el terminal)    | Stock e inventario                            |
| IP del servidor de mensajes en el terminal | Restaurante (en el terminal)    | Aviso de camarero                             |
| Anular comanda 6258 y departamento 901     | Restaurante (cuando se indique) | Limpieza de pruebas                           |
