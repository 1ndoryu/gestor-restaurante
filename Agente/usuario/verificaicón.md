Puedes darle esta lista completa al otro agente. Está pensada para revisión local y estática: sin desplegar, sin conectar al BDP real y sin ejecutar escrituras.

# Checklist final antes de entregar al cliente

## 1. Reglas obligatorias para el agente

- [ ] No desplegar a producción.
- [ ] No conectar con el BDP del restaurante.
- [ ] No ejecutar `CreateOrder`, `CreateCustomer`, pagos, facturación ni endpoints equivalentes.
- [ ] No utilizar `OnlyCheck` contra el restaurante.
- [ ] No activar sincronización, polling ni permisos temporales.
- [ ] No modificar variables de producción.
- [ ] No cambiar datos existentes de bases locales no desechables.
- [ ] Usar exclusivamente tests, mocks, simulador local, análisis estático y bases temporales.
- [ ] Separar claramente “verificado localmente” de “pendiente de confirmar por el cliente”.
- [ ] Entregar evidencia concreta para cada conclusión: archivo, línea, test o comando.
- [ ] No afirmar que algo es seguro solo porque compila.
- [ ] No afirmar compatibilidad real con BDP si solo fue probado con simulador.

## 2. Plan de integración completa

- [ ] Leer íntegramente el plan original de integración BDP.
- [ ] Leer todos los checklists derivados del plan.
- [ ] Comparar cada requisito del plan con código implementado.
- [ ] Detectar requisitos omitidos, implementados parcialmente o interpretados de forma distinta.
- [ ] Confirmar que cada funcionalidad del backend tiene una manifestación accesible en el frontend cuando corresponda.
- [ ] Identificar endpoints implementados pero sin uso desde la aplicación.
- [ ] Identificar botones o pantallas que llaman endpoints inexistentes.
- [ ] Detectar funcionalidades presentadas en la interfaz que realmente no estén implementadas.
- [ ] Comprobar que la guía del cliente coincide con el comportamiento real.
- [ ] Crear una matriz: requisito → backend → frontend → test → estado.
- [ ] Revisar que el roadmap no presente como pendiente algo ya terminado ni como terminado algo incompleto.

## 3. Alcance funcional BDP

Verificar la implementación completa de:

- [ ] Conexión y autenticación con WebLink.
- [ ] Health check.
- [ ] Consulta de versión.
- [ ] Catálogo de artículos.
- [ ] Consulta individual de artículos.
- [ ] Precios.
- [ ] Impuestos.
- [ ] Familias y departamentos.
- [ ] Códigos de barras.
- [ ] Clientes.
- [ ] Vista previa de importación de clientes.
- [ ] Vinculación Glory–BDP.
- [ ] Salones.
- [ ] Mesas.
- [ ] Vista previa antes de modificar el plano local.
- [ ] Estados de comandas.
- [ ] Polling manual.
- [ ] Polling automático.
- [ ] Menús.
- [ ] Packs.
- [ ] Fast food/modalidades de venta.
- [ ] Creación de clientes.
- [ ] Creación de comandas.
- [ ] Comandas con varias líneas.
- [ ] Cantidades.
- [ ] Precios.
- [ ] Impuestos.
- [ ] Descuentos.
- [ ] Cliente asociado.
- [ ] Mesa y canal.
- [ ] Forma de pago.
- [ ] Registro de pago completo.
- [ ] Facturación.
- [ ] Prevención de duplicados.
- [ ] Conciliación después de respuestas ambiguas.

También confirmar explícitamente que no estén anunciadas como disponibles:

- [ ] Sincronización bidireccional automática.
- [ ] Pagos parciales.
- [ ] Administración de inventario.
- [ ] Compras.
- [ ] Transferencias.
- [ ] Tallas y colores.
- [ ] Fidelización.
- [ ] Restauración automática de datos dentro de BDP.
- [ ] Anulación automática de pagos o facturas.

## 4. Dirección de sincronización

- [ ] Confirmar que `BDP → Glory` solamente utiliza operaciones de lectura.
- [ ] Confirmar qué importaciones modifican únicamente la base local de Glory.
- [ ] Confirmar que `Glory → BDP` no es un modo permanente.
- [ ] Confirmar que solo permite una operación exacta.
- [ ] Confirmar que exige una entidad exacta.
- [ ] Confirmar que exige un motivo.
- [ ] Confirmar que exige destino exacto.
- [ ] Confirmar que exige snapshot previo válido.
- [ ] Confirmar que la autorización tiene vencimiento.
- [ ] Confirmar que el cupo es exactamente uno.
- [ ] Confirmar que vuelve a `read_only` antes de la llamada remota.
- [ ] Confirmar que un error no vuelve a habilitar el permiso.
- [ ] Confirmar que `bidirectional` sea rechazado por backend.
- [ ] Confirmar que la interfaz no dé a entender que existe sincronización automática en dos direcciones.
- [ ] Confirmar que “Integración activa” no equivalga a “Escrituras autorizadas”.
- [ ] Confirmar que activar polling no habilite escrituras.

## 5. Configuración automática en producción

- [ ] Confirmar que las variables `BDP_*` realmente son consumidas por el runtime.
- [ ] Confirmar que el bootstrap solo se activa con `BDP_BOOTSTRAP_USER_EMAIL`.
- [ ] Confirmar que el correo debe corresponder exactamente a una cuenta existente.
- [ ] Confirmar que no selecciona el primer usuario ni aplica configuración globalmente.
- [ ] Confirmar que falla de forma segura si la cuenta no existe.
- [ ] Confirmar que no hace llamadas HTTP durante el bootstrap.
- [ ] Confirmar que es idempotente.
- [ ] Confirmar que solo se aplica una vez.
- [ ] Confirmar que no sobrescribe valores ya confirmados.
- [ ] Confirmar que elimina el placeholder inválido `GLORY`.
- [ ] Confirmar que códigos de artículo y cliente sean numéricos positivos.
- [ ] Confirmar que POS, empleado y perfil sean enteros positivos.
- [ ] Confirmar que los mapeos JSON sean objetos válidos.
- [ ] Confirmar que el intervalo de polling esté entre 10 y 600 segundos.
- [ ] Confirmar que la URL no contenga ruta, query, fragmento ni credenciales.
- [ ] Confirmar que integración y polling quedan apagados inicialmente.
- [ ] Confirmar que la sincronización queda en solo lectura.
- [ ] Confirmar que se eliminan autorizaciones temporales anteriores.
- [ ] Confirmar que la allowlist de escrituras permanece independiente.
- [ ] Confirmar que desplegar no autoriza automáticamente el destino real.
- [ ] Confirmar que nunca se guardan contraseñas en auditoría o logs.
- [ ] Confirmar que las migraciones se ejecutan antes del bootstrap.
- [ ] Confirmar que la configuración existente sobrevive a despliegues posteriores.
- [ ] Enumerar qué variables deberán configurarse antes del despliegue real.
- [ ] Verificar que `.env.example` documente todas las variables necesarias.

## 6. Allowlist y destino remoto

- [ ] Confirmar denegación por defecto si `BDP_WRITE_ALLOWED_ORIGINS` está vacío.
- [ ] Confirmar que loopback solo se admite para simuladores locales.
- [ ] Confirmar que una URL externa requiere allowlist explícita.
- [ ] Confirmar comparación canónica de protocolo, host y puerto.
- [ ] Confirmar rechazo de URLs con paths incrustados.
- [ ] Confirmar rechazo de credenciales dentro de la URL.
- [ ] Confirmar rechazo de destinos distintos al autorizado.
- [ ] Confirmar que cambiar URL, login, integrador, POS, empleado o perfil invalida permisos preparados.
- [ ] Confirmar que `OnlyCheck` tiene una allowlist independiente.
- [ ] Confirmar que una redirección HTTP no pueda saltarse la allowlist.
- [ ] Confirmar timeouts de red acotados.
- [ ] Confirmar que los errores no exponen credenciales ni payloads sensibles.

## 7. Seguridad de escrituras

Para cliente, comanda, pago y factura:

- [ ] Validación estricta de entrada.
- [ ] Entidad local exacta.
- [ ] Estado local compatible con la operación.
- [ ] Prevención de doble clic o requests simultáneos.
- [ ] Idempotencia local.
- [ ] Restricción única en base de datos cuando corresponda.
- [ ] Transacción local apropiada.
- [ ] Permiso temporal consumido atómicamente.
- [ ] Snapshot previo vinculado al destino exacto.
- [ ] Huella de conexión válida.
- [ ] Modo cerrado antes de enviar HTTP.
- [ ] Auditoría creada antes de la llamada remota.
- [ ] Auditoría finalizada después de la respuesta.
- [ ] Resultado ambiguo ante timeout o pérdida de respuesta.
- [ ] Bloqueo de reintentos cuando el resultado sea ambiguo.
- [ ] Nunca convertir una respuesta dudosa en éxito.
- [ ] Mensaje visible para el usuario.
- [ ] No permitir repetir automáticamente.
- [ ] No usar `.unwrap()` con respuestas externas.
- [ ] No registrar contraseñas, tokens ni información personal completa.

## 8. Creación de comandas

- [ ] Revisar el payload exacto esperado por BDP.
- [ ] Comprobar nombres y mayúsculas de campos.
- [ ] Comprobar `POS`, empleado, perfil y tipo de pedido.
- [ ] Comprobar cliente opcional y cliente por defecto.
- [ ] Comprobar forma de pago mapeada.
- [ ] Comprobar artículos mapeados.
- [ ] Comprobar fallback únicamente con artículo BDP numérico confirmado.
- [ ] Bloquear si falta un mapeo crítico.
- [ ] Comparar suma de líneas con total.
- [ ] Comprobar redondeos.
- [ ] Comprobar IVA incluido/no incluido.
- [ ] Comprobar descuentos por línea y totales.
- [ ] Comprobar cantidades decimales y enteras.
- [ ] Comprobar ventas sin líneas.
- [ ] Comprobar artículos inexistentes.
- [ ] Comprobar respuesta sin identificador.
- [ ] Comprobar códigos de error conocidos de BDP.
- [ ] Comprobar que una venta enviada quede protegida contra ediciones incompatibles.
- [ ] Comprobar que una comanda no pueda enviarse dos veces por concurrencia.

## 9. Clientes BDP

- [ ] El código debe elegirse expresamente.
- [ ] El código debe ser válido según el contrato BDP.
- [ ] No reemplazar un cliente existente.
- [ ] No generar código automáticamente sin confirmación.
- [ ] No crear cliente si ya existe una vinculación.
- [ ] Separar vista previa de importación y confirmación.
- [ ] Normalizar correctamente nombres vacíos o de una palabra.
- [ ] Manejar teléfonos, emails y documentos opcionales.
- [ ] No exponer datos personales completos en logs.
- [ ] Bloquear ventas si está activa la opción de exigir cliente confirmado.
- [ ] Comprobar concurrencia al vincular o crear.

## 10. Pagos y facturación

- [ ] Solo permitir pago de comanda conocida.
- [ ] Solo saldo completo pendiente.
- [ ] Bloquear pagos parciales.
- [ ] Validar importe exacto.
- [ ] Validar forma de pago.
- [ ] Bloquear pago duplicado.
- [ ] Bloquear pago de comanda ya pagada.
- [ ] Tratar timeout como resultado ambiguo.
- [ ] Bloquear facturación si la comanda no está pagada.
- [ ] Bloquear facturación duplicada.
- [ ] Conservar número y serie devueltos.
- [ ] No inventar número de factura si BDP no lo devuelve.
- [ ] Mostrar claramente las consecuencias fiscales.
- [ ] Confirmar que no existe rollback automático en BDP.
- [ ] Confirmar que la guía exige procedimiento manual de anulación.

## 11. Polling y sincronización de estados

- [ ] Polling apagado por defecto.
- [ ] Polling requiere integración activa.
- [ ] Polling solo consulta.
- [ ] Intervalo mínimo y máximo validados.
- [ ] Evitar consultas duplicadas por múltiples workers.
- [ ] Evitar N+1 innecesario.
- [ ] Programación persistente y recuperable tras reinicio.
- [ ] Manejar órdenes inexistentes.
- [ ] Manejar respuestas parciales.
- [ ] Manejar errores sin marcar estados falsos.
- [ ] No convertir estados desconocidos en estados terminales.
- [ ] No volver a consultar indefinidamente ventas ya terminadas.
- [ ] Registrar errores útiles sin llenar auditoría de escrituras.
- [ ] Comprobar cancelación limpia de tareas al apagar el servidor.

## 12. Catálogo, precios, clientes y mesas

- [ ] Importaciones BDP → Glory no llaman endpoints de escritura.
- [ ] Vista previa antes de modificaciones locales importantes.
- [ ] Upsert atómico.
- [ ] Constraints únicas.
- [ ] No borrar datos locales ausentes en una respuesta parcial.
- [ ] No sobrescribir mapeos manuales confirmados.
- [ ] Detectar artículos duplicados.
- [ ] Detectar códigos cambiados.
- [ ] Conservar fecha de última sincronización.
- [ ] Manejar precios enteros y decimales.
- [ ] Manejar IVA ausente.
- [ ] Manejar catálogo vacío sin borrar todo.
- [ ] Manejar timeout a mitad de importación.
- [ ] Crear salones/mesas solo después de confirmación cuando corresponda.
- [ ] Evitar duplicación de zonas y mesas.
- [ ] Confirmar que menús y packs son informativos si no existe administración completa.

## 13. Auditoría

- [ ] Cada escritura autorizada crea un registro.
- [ ] Fecha y hora.
- [ ] Usuario responsable.
- [ ] Operación traducible y entendible.
- [ ] Dirección.
- [ ] Tipo e ID de entidad.
- [ ] Destino canónico.
- [ ] Motivo de autorización.
- [ ] Snapshot o evidencia vinculada.
- [ ] Estado inicial `pendiente`.
- [ ] Resultado final `exito`, `error` o `ambiguo`.
- [ ] Error sanitizado.
- [ ] Respuesta sanitizada.
- [ ] Nunca guardar contraseña o token.
- [ ] Nunca guardar datos personales innecesarios.
- [ ] El bootstrap queda auditado.
- [ ] El bootstrap no expone secretos.
- [ ] La interfaz interpreta los valores reales del backend.
- [ ] No buscar `ok` si backend devuelve `exito`.
- [ ] Operaciones y direcciones aparecen en español.
- [ ] El motivo se muestra en la tabla.
- [ ] El historial tiene paginación o límite razonable.
- [ ] Explicar que las lecturas automáticas no generan una fila por consulta.
- [ ] Explicar que snapshots e historial son mecanismos diferentes.

## 14. Snapshots y respaldos

- [ ] Diferenciar respaldo local de Glory y snapshot leído de BDP.
- [ ] Confirmar que un snapshot BDP no se presenta como rollback.
- [ ] Confirmar creación manual de snapshots.
- [ ] Confirmar snapshot automático antes de escrituras.
- [ ] Confirmar snapshot del destino exacto.
- [ ] Confirmar huella de conexión exacta.
- [ ] Confirmar vigencia máxima.
- [ ] Confirmar que un snapshot parcial no habilita escritura.
- [ ] Confirmar que una lectura fallida invalida el snapshot completo.
- [ ] Confirmar que clientes, artículos, departamentos, salones y empleados estén presentes.
- [ ] Confirmar restauración solamente sobre datos locales permitidos.
- [ ] Confirmar transacción durante restore.
- [ ] Confirmar validación del propietario del snapshot.
- [ ] Confirmar que otro usuario no pueda leer o restaurar snapshots ajenos.
- [ ] Confirmar retención y eliminación.
- [ ] Confirmar mensajes de error visibles.
- [ ] Confirmar pruebas de backup/restore con base desechable.
- [ ] Confirmar procedimiento productivo de backup de PostgreSQL.
- [ ] Confirmar que el backup productivo esté almacenado fuera del contenedor.
- [ ] Confirmar que exista prueba documentada de restauración, no solo de creación.

## 15. Base de datos y migraciones

- [ ] Aplicar todas las migraciones en PostgreSQL vacío temporal.
- [ ] Aplicarlas sobre una copia con esquema anterior.
- [ ] Confirmar orden correcto.
- [ ] Confirmar que las migraciones sean repetibles donde usan `IF NOT EXISTS`.
- [ ] Confirmar defaults seguros.
- [ ] Confirmar constraints e índices.
- [ ] Confirmar compatibilidad con datos que todavía tengan `GLORY`.
- [ ] Confirmar que añadir `authorization_reason` no rompe consultas antiguas.
- [ ] Confirmar que el modelo Rust coincide con columnas y nulabilidad.
- [ ] Confirmar que los tests SQLx ejecutan todas las migraciones.
- [ ] Revisar migraciones `down`, aunque no se planee usarlas en producción.
- [ ] Confirmar que ninguna migración borra datos.
- [ ] Estimar bloqueo y duración de migraciones.
- [ ] Confirmar que el despliegue ejecuta migraciones antes del servidor.
- [ ] Confirmar estrategia de recuperación si una migración falla.

## 16. Backend

- [ ] `cargo fmt --check`.
- [ ] `cargo check` con `SQLX_OFFLINE=true`.
- [ ] `cargo clippy -- -D warnings`.
- [ ] Todas las pruebas unitarias.
- [ ] Pruebas SQLx BDP.
- [ ] Tests de concurrencia.
- [ ] Tests de timeouts y respuestas ambiguas.
- [ ] Tests de autorización por usuario.
- [ ] Tests de allowlist.
- [ ] Tests de URL canónica.
- [ ] Tests de bootstrap idempotente.
- [ ] Tests de preservación de configuración.
- [ ] Tests de ausencia de secretos en auditoría.
- [ ] Revisar manejo global de errores.
- [ ] Revisar que todos los endpoints críticos devuelvan resultado.
- [ ] Revisar que no existan fallos silenciosos.
- [ ] Revisar logs y niveles.
- [ ] Revisar límites de payload.
- [ ] Revisar rate limiting.
- [ ] Revisar CORS.
- [ ] Revisar autenticación y autorización.
- [ ] Confirmar aislamiento estricto por `user_id`.

## 17. Frontend

- [ ] Build de producción.
- [ ] Type-check.
- [ ] Abrir la pantalla autenticada en entorno local.
- [ ] Revisar en 320 px.
- [ ] Revisar en 768 px.
- [ ] Revisar en 1024 px o más.
- [ ] Confirmar una sola pestaña BDP.
- [ ] Confirmar textos entendibles.
- [ ] Confirmar que la configuración técnica esté colapsada.
- [ ] Confirmar que no se prometen defaults automáticos falsos.
- [ ] Confirmar explicación de las dos direcciones.
- [ ] Confirmar aviso de que no existe doble vía automática.
- [ ] Confirmar que el permiso de escritura está visible en la sección BDP.
- [ ] Confirmar que el modo seleccionado muestra descripción.
- [ ] Confirmar que el historial traduce operaciones y estados.
- [ ] Confirmar motivo y entidad en auditoría.
- [ ] Confirmar estados vacíos.
- [ ] Confirmar estados de carga.
- [ ] Confirmar errores visibles mediante toast.
- [ ] Confirmar que una mutación fallida no se presenta como exitosa.
- [ ] Confirmar rollback de actualizaciones optimistas si existen.
- [ ] Confirmar que guardar configuración invalida autorizaciones previas.
- [ ] Confirmar que credenciales nunca regresan completas desde la API.
- [ ] Confirmar navegación mediante teclado y etiquetas accesibles.
- [ ] Confirmar que tablas extensas no rompan mobile.

## 18. Guía del cliente

- [ ] Debe estar escrita para alguien no técnico.
- [ ] Debe resumir toda la integración.
- [ ] Debe explicar qué información entra desde BDP.
- [ ] Debe explicar qué información puede enviarse.
- [ ] Debe explicar qué no forma parte de la integración.
- [ ] Debe explicar “Integración activa”.
- [ ] Debe explicar BDP → Glory.
- [ ] Debe explicar Glory → BDP.
- [ ] Debe aclarar que no existe doble vía automática.
- [ ] Debe explicar polling.
- [ ] Debe explicar mapeos sin pedir al cliente que edite JSON.
- [ ] Debe explicar qué se registra en auditoría.
- [ ] Debe aclarar qué lecturas no se registran individualmente.
- [ ] Debe explicar snapshots.
- [ ] Debe aclarar que snapshots no restauran BDP.
- [ ] Debe incluir únicamente pruebas que nosotros no podamos confirmar.
- [ ] Debe listar exactamente los efectos en BDP.
- [ ] Debe exigir aprobación para pagos y facturas.
- [ ] Debe indicar “no repetir” ante respuestas dudosas.
- [ ] Debe pedir capturas sin secretos ni datos personales completos.
- [ ] Debe incluir criterios de aceptación.
- [ ] Debe marcar pruebas no realizadas como “no verificadas”, no “fallidas”.
- [ ] Debe evitar términos como UUID, JSON, endpoint o payload salvo explicación indispensable.

## 19. Producción e infraestructura, sin desplegar todavía

- [ ] Identificar servicio y rama correctos.
- [ ] Confirmar que no se desplegará otra rama por error.
- [ ] Confirmar persistencia de PostgreSQL.
- [ ] Confirmar que las credenciales de PostgreSQL no cambiarán.
- [ ] Confirmar volumen y política de respaldo.
- [ ] Confirmar espacio disponible.
- [ ] Confirmar health check real de la aplicación.
- [ ] Confirmar rollback de código.
- [ ] Confirmar procedimiento si las migraciones arrancan pero el servicio falla.
- [ ] Confirmar variables BDP necesarias.
- [ ] Confirmar cuenta exacta para `BDP_BOOTSTRAP_USER_EMAIL`.
- [ ] Confirmar códigos reales de POS, empleado y perfil.
- [ ] Confirmar mapeos reales de pagos y canales.
- [ ] Confirmar artículo fallback real.
- [ ] Confirmar cliente genérico real.
- [ ] Mantener vacía la allowlist de escritura durante el primer despliegue.
- [ ] Confirmar que el primer arranque no inicia polling.
- [ ] Confirmar que no existe autorización temporal persistente.
- [ ] Preparar una verificación postdeploy exclusivamente de aplicación y base de datos.
- [ ] No ejecutar diagnóstico que contacte BDP sin autorización.
- [ ] Preparar rollback mediante `coolify-manager-rs`.
- [ ] No usar SSH directo.

## 20. Secretos

- [ ] Revisar que ningún secreto esté versionado.
- [ ] Revisar historial Git reciente.
- [ ] Revisar logs de tests.
- [ ] Revisar auditoría BDP.
- [ ] Revisar mensajes de error.
- [ ] Revisar `.env.example`.
- [ ] Confirmar que contraseñas nunca aparezcan en respuestas API.
- [ ] Confirmar que frontend no persista secretos.
- [ ] Confirmar que los logs del bootstrap solo indiquen usuario y resultado.
- [ ] Rotar la `SUPERMEMORY_API_KEY` expuesta anteriormente.
- [ ] Corregir el perfil PowerShell personal que contiene esa clave.
- [ ] Confirmar que el self-check usa `-NoProfile`.
- [ ] Confirmar que CI enmascare secretos.

## 21. Informe que debe entregar el agente

Pídele entregar:

- [ ] Resumen ejecutivo.
- [ ] Riesgos críticos, altos, medios y bajos.
- [ ] Matriz completa requisito–implementación–prueba.
- [ ] Lista de funciones realmente verificadas.
- [ ] Lista de funciones no verificables sin BDP real.
- [ ] Lista de diferencias entre plan, código, frontend y guía.
- [ ] Evidencia de cada hallazgo.
- [ ] Correcciones recomendadas, sin aplicarlas si no fueron autorizadas.
- [ ] Resultado exacto de builds y tests.
- [ ] Confirmación de que no contactó producción.
- [ ] Confirmación de que no hizo escrituras.
- [ ] Condiciones mínimas para autorizar el despliegue.
- [ ] Condiciones mínimas para que el cliente realice las cuatro pruebas.
- [ ] Veredicto separado:
  - listo para desplegar en modo cerrado;
  - listo para lecturas reales;
  - listo para pedir pruebas de escritura al cliente;
  - no listo, con bloqueos concretos.

La pregunta final que debe responder el agente es:

> “¿Existe algún camino, incluyendo errores, concurrencia, reinicios o configuración incompleta, por el cual el despliegue, una lectura o una acción accidental puedan crear, duplicar, modificar, pagar o facturar algo en el BDP sin una autorización temporal explícita para esa entidad exacta?”

PD. Yo he cambiado la palabra 'Glory' por 'La Aplicación Web' porque asi el cliente la entiende mejor