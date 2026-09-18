# Guía del cliente — Segunda entrega: lo nuevo desde julio

> **Fecha:** 18 de septiembre de 2026
> **Continúa a:** Guía del cliente — Revisión de la integración con BDP (19/07/2026)
> **Objetivo:** resumir en lenguaje sencillo lo añadido y comprobado desde la
> primera guía. Todo lo que aparece como funcionando está verificado, incluso
> con operaciones reales de prueba en septiembre (importes mínimos, fuera de
> horas de servicio, anuladas después).

## 1. Lo que la aplicación puede recibir del BDP (lectura)

Importaciones comprobadas una por una en la web del restaurante (18/09):

- **Catálogo:** 559 artículos con precio, impuesto y familia.
- **Clientes,** con revisión previa: lo nuevo se copia, lo existente se vincula,
  nada se reemplaza sin avisar.
- **Plano de sala:** 7 salones y 87 mesas traídos del BDP al plano local.
- **Datos de referencia** (departamentos, formas de pago, puestos, empleados),
  que la aplicación usa para cuadrar ventas y pagos.
- Cada importación muestra primero una vista previa y pide confirmación
  explícita antes de aplicar. Al aplicar solo se crea lo que falta; nunca se
  borra nada.

## 2. Lo que la aplicación puede enviar al BDP (escritura)

- Crear un cliente con código nuevo, sin tocar los existentes.
- Crear una comanda, también ya cobrada al crearla.
- Añadir propina a una venta.
- Sumar o restar puntos de fidelización, siempre con motivo.
- Anular una comanda.
- Dar de alta y modificar artículos y departamentos (en pausa por decisión del
  restaurante: lo creado no se puede borrar).

## 3. Lo que aún no puede escribir

- Cobrar una comanda ya existente.
- Facturar una comanda en el BDP.
- Mover stock o inventario en el BDP.
- Mostrar el aviso de camarero en el terminal.

## 4. Lo que aún no puede leer

- Las existencias de stock por artículo.
- Los albaranes de compra.
- El detalle de una comanda (totales y pagos).

## 5. Cómo se trabaja

- Estado normal en solo lectura; cada envío requiere autorización puntual y el
  sistema vuelve solo a lectura después de operar.
- No existe sincronización automática en las dos direcciones, para evitar
  duplicados.
- La aplicación funciona también sin conexión: vender, cobrar, crear artículos,
  ajustar el stock local, anular ventas (pidiendo motivo), crear albaranes y
  menús, y ver el historial. Lo editado a mano no lo borra la conexión.
- Cada trabajador ve su menú según su función; lo técnico queda reservado al
  dueño (delegable por sección si se desea).
- Los respaldos protegen los datos de la aplicación; no pueden deshacer nada
  dentro del BDP.

## 6. Pendiente del restaurante (nada urgente, nada bloqueante)

1. Indicar el código del perfil de exportación del terminal para activar la
   lectura de albaranes.
2. Anular en el terminal la comanda de prueba 6258 y el departamento de
   prueba 901 cuando se indique.
