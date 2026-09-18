# Guía del cliente — Segunda entrega: lo nuevo desde julio

> **Fecha:** 18 de septiembre de 2026
> **Continúa a:** Guía del cliente — Revisión de la integración con BDP (19/07/2026)
> **Objetivo:** resumir en lenguaje sencillo todo lo que se ha añadido y comprobado
> desde la primera guía. Sin pruebas pendientes: lo que aquí aparece ya está
> verificado, incluso contra el BDP real del restaurante cuando así se indica.

## 1. La aplicación funciona con o sin conexión al BDP

Decisión principal del periodo: el restaurante no depende de la conexión.

- Se puede vender, cobrar, crear artículos, ajustar el stock local, anular ventas
  (pidiendo motivo), crear albaranes, menús y ver el historial, todo sin conexión.
- Un indicador visible muestra en qué modo se está trabajando.
- Lo que se edita a mano no lo borra la conexión al sincronizar.
- Si no hay conexión, los botones que la requieren aparecen ocultos o
  desactivados: nunca se finge una acción que no se hizo.

## 2. Lo que ya llega del BDP a la aplicación (verificado el 18/09)

Importaciones comprobadas una por una en la web del restaurante:

- **Catálogo:** 559 artículos con precio, impuesto y familia.
- **Clientes:** importación con revisión previa (lo nuevo se copia, lo existente
  se vincula, nada se reemplaza sin avisar).
- **Plano de sala:** 7 salones y 87 mesas traídos del BDP al plano local.
- **Stock:** consulta de existencias cuando el BDP las devuelve. El stock se
  consulta, no se edita desde la aplicación: se edita en el BDP.
- Cada importación muestra primero una vista previa y pide confirmación
  explícita antes de aplicar. Al aplicar solo se crea lo que falta; nunca se
  borra nada.

## 3. Lo que ya se puede enviar al BDP (verificado contra el BDP real)

Comprobado con operaciones reales de prueba (`PRUEBA-*`, importes mínimos,
fuera de horas de servicio, anuladas después):

- **Crear comanda ya cobrada:** funciona (comanda 6338, 0,11 €).
- **Anular una comanda en el BDP:** funciona (la 6338 se anuló por este medio).
- **Propina sobre una venta:** funciona.
- **Crear cliente con código nuevo:** funciona (sin tocar los existentes).
- **Llamar al camarero desde el plano:** llega al BDP.
- **Puntos de fidelización** en la ficha del cliente, siempre con motivo.

Las protecciones de la primera guía siguen vigentes: estado normal en solo
lectura, cada envío requiere autorización puntual y el sistema vuelve solo a
lectura después de operar. No existe sincronización automática en las dos
direcciones, para evitar duplicados.

## 4. Compras: albaranes, borradores y conciliación

Nuevo desde agosto, funciona sin conexión y listo para cuando el BDP aporte datos:

- Alta de albaranes locales (serie reservada, sin choques con los del BDP).
- Borradores y conciliación con gastos.
- Filtro por proveedor y por fechas.
- La sincronización de albaranes desde el BDP queda preparada: solo falta
  indicar el código del perfil de exportación tal como aparece en el terminal
  (ver §6).

## 5. Permisos por trabajador

- Cada trabajador ve su menú según su función: la operativa diaria (ventas,
  reservas, clientes, plano) abierta; lo técnico (configuración, trabajadores,
  sincronización) reservado al dueño, delegable por sección si se desea.
- Sin pantallas que se quedan "cargando" ni listas vacías falsas: si algo no
  está permitido, se dice claramente.

## 6. Rendimiento y servidor

- Medido con pruebas de carga: sobrado para el uso del restaurante, aguanta
  miles de consultas simultáneas. Vender y cobrar siguen yendo directas,
  sin atajos.
- Web con cifrado, acceso con contraseña segura y copias de seguridad.
- Decisiones acordadas: el servidor se mantiene donde está hasta elegir destino
  en España/UE; el segundo factor de acceso será con cuenta de Google; el modo
  multi-local queda en espera hasta confirmar si todos los locales usarán BDP.

## 7. Límites vigentes (sin cambios)

- No se administran desde la aplicación: stock del BDP, transferencias,
  fidelización avanzada del BDP ni menús/packs completos (solo consulta).
- Los respaldos protegen los datos de la aplicación; no pueden deshacer nada
  dentro del BDP (una comanda, un pago o una factura solo se corrigen con el
  procedimiento manual del restaurante en el BDP).

## 8. Próximos pasos (nada urgente, nada bloqueante)

1. **Perfil de albaranes:** indicar el código del perfil de exportación del
   terminal para activar la sincronización de compras.
2. **Suscripción de pago de WebLink:** solo si se quiere pagar o facturar
   comandas ya existentes desde la aplicación. Crearlas ya cobradas y
   anularlas funciona con la versión actual.
3. **Limpieza en el terminal:** anular la comanda de prueba 6258 y el
   departamento de prueba 901 cuando se indique.
