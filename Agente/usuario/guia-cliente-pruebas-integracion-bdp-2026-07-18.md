# Guía del cliente — Revisión de la integración con BDP

> **Fecha:** 18 de julio de 2026
> **Objetivo:** comprobar lo que puede revisarse sin crear, cobrar, facturar ni modificar datos dentro de BDP.
> **Responsable de estas comprobaciones:** personal autorizado del restaurante.
> **Límite:** nuestro equipo no realizará pruebas contra el BDP del restaurante.

## Antes de comenzar

La aplicación fue revisada y probada localmente con un simulador, sin conectarse al BDP del restaurante. Estas pruebas dan una confianza técnica alta, pero no pueden garantizar que la versión y configuración concretas de BDP respondan exactamente igual.

Durante toda esta revisión debe aparecer el modo **Solo lectura**. No active **Escritura temporal**.

Las pruebas de esta guía pueden:

- consultar información de BDP y dejar registros técnicos de conexión;
- copiar o actualizar información dentro de Glory cuando se indique expresamente;
- generar una carga breve al leer catálogos o clientes numerosos.

No deben crear ni modificar comandas, clientes, pagos, facturas, precios, mesas o configuración dentro de BDP.

## Acciones que NO deben utilizarse

No pulse ni autorice ninguna de estas acciones durante esta revisión:

- **Validar con simulador local**, cuando la dirección configurada sea la del restaurante;
- **Escritura temporal** o cualquier cambio desde **Solo lectura**;
- **Reintentar BDP** en una venta;
- **Crear o vincular cliente en BDP**;
- **Registrar pago en BDP**;
- **Facturar en BDP**.

Estas acciones utilizan rutas que pueden escribir en el sistema real. Los pagos y facturas también pueden afectar caja, informes y numeración fiscal.

## Qué se agregó a la aplicación

### Configuración y seguridad

- Datos de conexión de BDP separados de la configuración general.
- Botón para comprobar conexión, inicio de sesión y versión.
- Modo **Solo lectura** como estado seguro.
- Escritura temporal limitada a una operación y un registro concreto. No se usa en esta guía.
- Historial de operaciones y resultados.
- Snapshots para conservar evidencia antes de operaciones importantes. Un snapshot no puede deshacer cambios hechos en BDP.
- Polling automático opcional y apagado por defecto.

### Catálogo y precios

- Lectura del catálogo de artículos de BDP.
- Mapeo entre artículos de Glory y artículos BDP.
- Nombre, descripción, precio, IVA, departamento, familia, subfamilia y código de barras.
- Actualización de precios desde BDP hacia Glory.
- Artículo genérico de respaldo cuando una línea no tiene mapeo.

### Ventas y comandas

- Ventas con varias líneas, cantidades, precios, IVA y descuentos.
- Total calculado desde las líneas.
- Cliente, canal y forma de pago relacionados con los códigos configurados.
- Estado BDP visible en la lista de ventas.
- Filtros para pendientes, sincronizadas, canceladas, facturadas o con error.
- Una venta ya enviada a BDP no se puede editar desde Glory, para evitar duplicados o diferencias.
- Pago y factura aparecen como acciones separadas. No se prueban en esta guía.

### Clientes

- Estado y código BDP visibles en Glory.
- Previsualización de clientes disponibles en BDP.
- Importación hacia Glory con resumen de nuevos, vínculos, conflictos y registros inválidos.
- Los conflictos se omiten; no se sobrescriben vínculos existentes.
- La creación de un cliente nuevo dentro de BDP requiere una autorización diferente y no se prueba aquí.

### Salones y mesas

- Lectura de salones y mesas existentes en BDP.
- Previsualización antes de aplicar.
- Al aplicar, Glory agrega solamente las zonas y mesas locales que faltan.
- No elimina ni cambia mesas dentro de BDP.
- Las mesas nuevas se colocan inicialmente en una cuadrícula para poder ordenarlas después en Glory.

### Estados de pedidos

- Consulta manual del estado de comandas que ya tengan un identificador BDP.
- Consulta automática opcional, con intervalo limitado y apagada por defecto.
- Estados visibles: pendiente, aceptada, cancelada, facturada o error.
- Las consultas actualizan solamente el estado mostrado en Glory.

### Menús y funciones no incluidas

La aplicación puede leer información técnica de menús, packs y fast-food, pero todavía no ofrece una pantalla completa para administrarlos. Stock, compras, transferencias, tallas, colores y fidelización no forman parte de esta integración.

## Pruebas que debe realizar el cliente

Realice una prueba por vez. Si aparece algo inesperado, deténgase, tome una captura y no repita la acción.

### 1. Revisar el modo seguro

- [ ] Entrar en **Configuración > BDP**.
- [ ] Confirmar que el modo indica **Solo lectura**.
- [ ] Confirmar que **Polling automático** está apagado antes de empezar.
- [ ] Confirmar que no hay una escritura temporal activa.

**Resultado esperado:** la aplicación permite revisar la configuración, pero no enviar clientes, comandas, pagos ni facturas.

### 2. Probar solamente la conexión

- [ ] Pulsar **Probar conexión** una sola vez.
- [ ] Comprobar que muestra conexión, inicio de sesión y versión.
- [ ] Anotar cualquier mensaje completo si falla.

**Qué hace:** abre una sesión y lee información técnica de BDP.
**Qué no hace:** no crea ni modifica datos de negocio.

No pulse **Validar con simulador local**: esa acción debe permanecer deshabilitada cuando la dirección es la del restaurante.

### 3. Revisar los mapeos

- [ ] Abrir **Configuración avanzada**.
- [ ] Revisar que las formas de pago conocidas tengan un código BDP.
- [ ] Revisar que los canales de venta tengan un tipo de pedido.
- [ ] Revisar el artículo de respaldo.
- [ ] Revisar la política para ventas sin cliente BDP.
- [ ] Guardar únicamente si los valores son correctos y fueron confirmados por el restaurante.

**Resultado esperado:** los valores se conservan al recargar la pantalla. Guardarlos cambia Glory, no BDP.

### 4. Revisar el catálogo

- [ ] Abrir los mapeos de artículos.
- [ ] Comprobar que se ven código, nombre, precio, IVA y familia cuando están disponibles.
- [ ] Ejecutar la sincronización del catálogo una sola vez, si el restaurante autoriza la lectura completa.
- [ ] Comprobar que no aparecen artículos duplicados.
- [ ] Ejecutar la actualización de precios una sola vez.
- [ ] Comparar manualmente tres artículos conocidos con BDP.

**Resultado esperado:** BDP no cambia; Glory crea o actualiza su copia local y sus mapeos.
**Detenerse si:** desaparecen mapeos, se duplican artículos o un precio cambia al artículo equivocado.

### 5. Revisar clientes sin crear clientes en BDP

- [ ] Abrir **Clientes > Importar desde BDP**.
- [ ] Pulsar primero **Previsualizar sin cambios**.
- [ ] Revisar los totales de nuevos, vínculos, sin cambios, conflictos e inválidos.
- [ ] No continuar si el resultado parece desproporcionado o faltan muchos clientes esperados.
- [ ] Si el restaurante acepta copiar esa información a Glory, escribir la confirmación solicitada y pulsar **Aplicar en Glory** una sola vez.
- [ ] Repetir la previsualización y comprobar que no vuelve a proponer los mismos clientes como nuevos.

**Resultado esperado:** no cambia ningún cliente en BDP. La aplicación agrega o vincula solamente registros locales seguros.
**No usar:** el botón individual para crear/vincular un cliente en BDP.

### 6. Revisar salones y mesas

- [ ] Abrir **Plano de sala**.
- [ ] Pulsar **Previsualizar BDP**.
- [ ] Comparar nombres de salones y números de mesa con el TPV.
- [ ] Si el resultado es correcto, aplicar la importación en Glory.
- [ ] Comprobar que solo se agregaron los elementos faltantes.
- [ ] Ejecutar otra previsualización y confirmar que no propone duplicados.

**Resultado esperado:** BDP no cambia. Glory agrega zonas y mesas locales sin borrar las existentes.

### 7. Revisar ventas locales sin enviarlas

- [ ] Mantener BDP en **Solo lectura**.
- [ ] Crear una venta de prueba identificada claramente como local.
- [ ] Agregar varias líneas con distintas cantidades, precios, IVA y descuento.
- [ ] Confirmar que subtotales y total son correctos.
- [ ] Guardar y volver a abrir la venta.
- [ ] Editar sus líneas y comprobar que no se pierden datos.
- [ ] Revisar el indicador de artículo mapeado o artículo de respaldo.
- [ ] Confirmar que la venta no recibe un número de pedido BDP.
- [ ] Eliminar la venta local de prueba siguiendo el procedimiento normal de Glory.

**Resultado esperado:** la prueba cambia únicamente Glory. No aparece ninguna comanda nueva en BDP, cocina o TPV.

### 8. Revisar estados y filtros

- [ ] Revisar la columna BDP de la lista de ventas.
- [ ] Probar cada filtro disponible.
- [ ] Abrir una venta que ya estuviera vinculada históricamente con BDP.
- [ ] Si se autoriza una consulta de estado, ejecutarla una sola vez.
- [ ] Confirmar que una venta facturada o cancelada no queda como pendiente.
- [ ] Confirmar que un error se muestra de forma visible y no como éxito.

**Resultado esperado:** consultar estado no cambia BDP; solamente actualiza la información local mostrada en Glory.

No active el polling automático durante la primera revisión. Puede evaluarse después, en una ventana corta, observando la carga del equipo BDP.

### 9. Revisar snapshots y auditoría

- [ ] Crear un snapshot de **Glory** con ventas, clientes o mapeos de prueba.
- [ ] Confirmar que aparece en el historial.
- [ ] Abrir el historial de auditoría y comprobar que muestra operación, resultado y error cuando corresponda.
- [ ] Confirmar que no se muestran contraseñas ni tokens.

Un snapshot completo de BDP hace varias lecturas grandes. No es necesario para esta primera revisión y no debe confundirse con una copia capaz de restaurar BDP.

### 10. Revisar presentación y uso diario

- [ ] Revisar las pantallas en un teléfono o ancho pequeño.
- [ ] Revisarlas también en un ordenador.
- [ ] Comprobar que diálogos, tablas y botones no quedan fuera de pantalla.
- [ ] Confirmar que toda espera muestra progreso.
- [ ] Confirmar que todo fallo muestra un mensaje visible.
- [ ] Confirmar que cerrar un diálogo no ejecuta la acción.

## Cómo informar un problema

No repita una acción que haya quedado pendiente, ambigua o sin respuesta clara. Envíe:

1. fecha y hora;
2. pantalla utilizada;
3. botón pulsado;
4. resultado esperado;
5. resultado obtenido;
6. mensaje completo mostrado;
7. captura de pantalla;
8. si BDP, cocina o TPV mostraron algo nuevo.

No incluya contraseñas, tokens ni datos personales completos en capturas o mensajes.

## Criterio de aceptación de esta etapa

La etapa se considera aprobada cuando:

- todas las comprobaciones seleccionadas terminan sin modificar BDP;
- no aparecen duplicados ni pérdidas de información en Glory;
- totales, artículos, clientes, mesas y estados coinciden con lo esperado;
- los errores son visibles y no provocan reintentos automáticos;
- BDP continúa en **Solo lectura**;
- no se creó ninguna comanda, cliente, pago o factura en BDP.

Las funciones de escritura quedarán como **no verificadas en el BDP real**, no como fallidas. Solo podrían validarse posteriormente por decisión y responsabilidad del restaurante, mediante un procedimiento independiente.
