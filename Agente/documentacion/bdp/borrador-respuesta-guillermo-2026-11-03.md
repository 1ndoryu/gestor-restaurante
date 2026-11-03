# Borrador de respuesta a Guillermo — 03/11/2026

Para enviar por la usuaria (no se ha enviado nada).

Hola Guillermo, perdona la tardanza. Resumen de tu mensaje del 2 de noviembre:

- **Modificar una comanda (1):** ya funciona. Si se cambia una comanda desde la web, llega al TPV con las líneas añadidas o quitadas y el total correcto.
- **Cambios hechos en el TPV (2):** la web los detecta sola; si la comanda ya no coincide, avisa con «Conflicto» y no deja cobrar hasta revisarla.
- **Mesa ocupada (3):** ya se comprueba antes de enviar y el mensaje dice qué mesa está ocupada y por qué comanda.
- **Factura con nombre, NIF y dirección (4):** probada de principio a fin y, al repetir la petición, no se genera otra factura. Falta comparar el documento dentro del BDP; lo haremos en la prueba contigo.
- **Reinicio de BDP.NET (5):** probado el 27/10; la comanda queda pendiente en la web y se envía una sola vez al volver el TPV (el reenvío es manual, con un botón).
- **Formas de pago y empleados (6):** se actualizan solos con la cadencia que se configure.
- **Versión del BDP (7) y datos de producción (10):** necesitamos probar en el BDP del restaurante.
- **Stock, cocina, llamada al camarero e inventarios (8–9):** quedan apagados hasta que confirmes que los usáis.
- **Piloto por etapas (11):** de acuerdo.

**Sí podrás ver el BDP además de la web**, mientras el PC de pruebas y el túnel estén encendidos. ¿Qué día y hora te va bien para una prueba conjunta?

Sobre tus audios del 27 y 28/10: las licencias las llevas tú, de acuerdo; la prueba como propietario en un día normal la podemos hacer contra el BDP del portátil (`rest-dev.wandori.us`, te paso el enlace y las credenciales). De la URL y el JSON del chatbot, dime en qué página del restaurante va; para integrarlo necesito el acceso al hosting (punto 1).

Para avanzar necesitamos de tu parte:

1. Acceso al hosting/panel de Entre Tenedores.
2. Las formas de pago del restaurante.
3. Los tipos de pedido que usan.
4. El perfil de documentos que usa su BDP.
5. Los responsables que aprobarán los controles (D32).
