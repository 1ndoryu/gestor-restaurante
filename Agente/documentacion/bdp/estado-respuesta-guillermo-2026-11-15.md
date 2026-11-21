# Respuesta a Guillermo — 2026-11-15

Estado real tras el repaso del 15/11 (ver `Agente/completados/tareas-2026-11-15.md`).
El mensaje del 04/11 (`enviado-guillermo-2026-11-04.txt`) ya está enviado: no se reescribe.

Guillermo escribió por última vez el 11/11 («espero que estés mejor»). No hay
respuesta pendiente por mi parte más que la que sigue a su pregunta abierta del
02/11 (si verá el BDP durante las pruebas) y a la petición del 10/11 (párrafo de
Google Drive en guillechatbots).

## Puntos a contestar

1. **Sí, verás el BDP.** El entorno de pruebas está conectado al BDP real de mi
   portátil (licencia 84 «WeblinkRestAPI», versión 36.2). Mientras dure la sesión
   podemos mirar los dos a la vez: la web en el navegador y el TPV en la pantalla
   compartida. Lo levantamos cuando tú digas y lo dejo arrancado.
2. **Fecha y hora.** Proponer una franja concreta, en horario de tarde, y
   confirmar antes que el portátil tiene el TPV abierto (si se cierra, la web
   pierde el WebLink).
3. **Hosting de Entre Tenedores.** Confirmado que no lo tienen; me toca buscarlo
   en los mensajes de Fiverr. Dos sitios distintos:
   - `www.entretenedores.net` — WordPress, el que lleva el chatbot.
   - `restauranteentretenedores.com` — ficha en Uniiti.
   Aún no sé cuál de los dos está en manos de la persona de Fiverr; hay que
   aclararlo antes de pedir nada.
4. **Párrafo de Google Drive en guillechatbots**: comprobado el 15/11 en
   `guillechatbots.es/privacidad/` — ya está puesto (la página se renderiza
   en cliente; el HTML crudo sale vacío). Punto cerrado.

## Lo que ya está hecho y él no sabe

- **Pregunta abierta para él:** el 05/11 dijo que el restaurante le confirmó que
  usaría stock, cocina, llamada al camarero e inventarios; las respuestas del
  propietario del 10/11 («no entendemos esas funciones») no lo respaldan. Hay
  que aclararlo antes del piloto: si no las usan, no se preparan.
- Formas de pago, tipos de pedido y perfil de documentos: las pantallas de
  Configuración → BDP existen y cargan; falta que el restaurante meta los datos.
  En el entorno de pruebas ya está configurado como muestra: formas de pago
  `Efectivo → 1` y tipos de pedido `comedor 1, barra 0, terraza 1, delivery 2`
  (leídos de los `TenderId` reales del BDP demo; los tipos de pedido son
  **inferidos**, no leídos de su servidor: hay que confirmarlos con él).
- Artículo por defecto: estaba en `1001 / COCA-COLA`. Cualquier artículo de la
  web sin equivalencia en el catálogo BDP —y faltan la mayoría de los platos
  reales: pulpo, calamares, croquetas, jamón, paella, chuletillas, merluza,
  bacalao, postre— se facturaba como una Coca-Cola. Cambiado a `4031 / VARIOS`
  (existe en el BDP del demo y en el catálogo local, precio 0, IVA 10 %).
  **[Refutado el 20/11]** `4031` no existe en el TPV demo: la exportación real
  (`/API/Articles/Export`, todos los departamentos) devuelve solo 1001 COCA-COLA
  y 1002 SCHWEPPES LIMÓN. El valor por defecto se dejó en `1001` (ver `20AB`).
- Revisión de comandas (el punto 1 de su lista del 02/11): hecho el 05/11.
- Reintento automático de la cola (su corrección del 05/11): hecho, `Agente/completados/tareas-2026-11-11.md`.
- Chatbot: funcionando desde el 05/11.
