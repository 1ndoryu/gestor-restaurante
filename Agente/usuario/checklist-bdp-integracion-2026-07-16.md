# Checklist: Integración completa BDP — Pruebas manuales

> **Fecha:** 2026-07-16
> **Alcance:** Sync BDP + Backup/Restauración + Configuración
> **URL local:** http://localhost:5174/configuracion
>
> **Orden de pruebas:** Sin BDP → Solo lectura BDP → Escritura BDP

---

## 1️⃣ SIN BDP — UI + Backup local (no requiere conexión al TPV)

> Estas pruebas se pueden hacer sin tener el servidor BDP activo.
> Verifican que la interfaz, los snapshots y la auditoría funcionan localmente.

### Pestañas de Configuración
- [ ] **Pestañas visibles:** En `/configuracion` aparecen 5 pestañas: General, Integraciones, Chatbot, BDP Conexión, BDP Backup
- [ ] **Pestaña "BDP Conexión":** Al hacer click, muestra el formulario de conexión (URL, login, password, etc.)
- [ ] **Pestaña "BDP Backup":** Al hacer click, muestra el panel de snapshots sin crashes
- [ ] **Sin errores en consola:** Abrir DevTools → Console, no hay errores rojos al navegar entre pestañas

### Configuración BDP Conexión
- [ ] **Campos visibles:** URL pública BDP, Login, Password, Código integrador, Terminal POS, Empleado, Perfil artículos
- [ ] **Toggle sync:** El switch "Sincronización BDP activa" funciona (on/off)
- [ ] **Mapeos colapsados:** Por defecto se ve "Configuración avanzada (mapeos)" con chevron derecho y nota informativa
- [ ] **Expandir mapeos:** Click en "Configuración avanzada" → se despliegan los campos JSON de mapeos
- [ ] **Guardar conexión:** Click "Guardar conexión BDP" → muestra "Guardando..." → éxito (persiste al recargar)

### Snapshots (Backup local)
- [ ] **Tab "Snapshots" visible:** Muestra la tabla (vacía o con datos existentes)
- [ ] **Tab "Auditoría" visible:** Muestra la tabla (vacía o con datos existentes)
- [ ] **Snapshot completo:** Click "Crear completo" → aparece en la tabla con tipo `completo`, estado `disponible`, fecha actual
- [ ] **Snapshot parcial:** Seleccionar tipos (menú, productos, etc.) → click "Crear parcial" → aparece con tipo `parcial`
- [ ] **Snapshot Glory:** Seleccionar tipos → click "Crear Glory" → aparece con tipo `glory`
- [ ] **Notas opcionales:** Crear snapshot con y sin nota, verificar que se guarda
- [ ] **Loading state:** Al crear, el botón muestra spinner/deshabilitado mientras procesa

### Eliminar snapshots
- [ ] **Eliminar:** Click en botón eliminar → confirmar → snapshot desaparece de la tabla
- [ ] **Confirmación:** Pide confirmación antes de eliminar (no se borra accidentalmente)

### Auditoría
- [ ] **Registros aparecen:** Después de crear/eliminar snapshots, la tab "Auditoría" muestra los registros
- [ ] **Detalle correcto:** Cada entrada muestra operación, resultado (éxito/error), timestamp, usuario

### Modo de sincronización
- [ ] **Leer modo actual:** El panel de Backup muestra el modo actual (read_only / unidirectional / bidirectional)
- [ ] **Cambiar modo:** Seleccionar otro modo → se actualiza en el panel

### Manejo de errores (sin BDP)
- [ ] **Botón "Probar conexión":** Click → muestra error claro (no crash) porque no hay BDP
- [ ] **Botón "Probar sincronización segura":** Click → muestra error o estado pendiente (no crash)
- [ ] **Snapshot con BDP caído:** Crear snapshot funciona igual (es backup local, no depende de BDP)

---

## 2️⃣ SOLO LECTURA BDP — Diagnóstico y validación

> Estas pruebas requieren que el servidor BDP esté activo pero NO escriben datos.
> Verifican la conexión, autenticación y estado del TPV.

### Diagnóstico BDP
- [ ] **Probar conexión:** Click → muestra estado (health_ok, login_ok, versión BDP)
- [ ] **Info de versión:** Muestra versión, sub_version y aplicación del TPV
- [ ] **Credenciales incorrectas:** Cambiar password a algo malo → probar conexión → error de autenticación

### Sync dry-run (lectura)
- [ ] **Probar sincronización segura:** Click → ejecuta checks de lectura sin escribir nada
- [ ] **Checks individuales:** Cada check muestra nombre, endpoint, ok/error, cantidad de registros
- [ ] **Estado "listo para sincronizar":** Muestra si todo está configurado correctamente

### Restaurar snapshot (lectura local, escritura Glory)
- [ ] **Restaurar Glory desde snapshot:** Click restaurar → confirmar → resultado con detalle de tablas
- [ ] **Restaurar no destructiva:** Los datos del restaurante (reservas, ventas) NO se pierden
- [ ] **Error: snapshot inexistente:** Intentar restaurar un snapshot eliminado → error claro

---

## 3️⃣ ESCRITURA BDP — Sync real y auditoría pre-write

> ⚠️ **Estas pruebas modifican datos en el TPV/BDP.** Dejarlas para el final.
> Requieren BDP activo + sync_mode != read_only.

### Pre-write audit
- [ ] **Sync unidirectional:** Al escribir datos hacia BDP, se registra entrada de auditoría ANTES de la escritura
- [ ] **Sync bidirectional:** Igual que unidirectional pero en ambas direcciones
- [ ] **Sync read_only:** NO se permite escritura hacia BDP (endpoint devuelve error 403 o similar)

### Escritura real a BDP
- [ ] **Sync de productos:** Los productos de Glory aparecen en BDP después de sincronizar
- [ ] **Sync de ventas:** Las ventas de Glory se envían a BDP correctamente
- [ ] **Mapeo de formas de pago:** Los métodos de pago Glory se mapean a tenders BDP
- [ ] **Mapeo de canales:** Los canales Glory (sala, barra, domicilio) se mapean a order types BDP
- [ ] **Auditoría de escritura:** Cada operación de sync genera entrada en el log de auditoría

### Edge cases de escritura
- [ ] **Rate limiting:** Demasiadas peticiones a BDP → muestra error de rate limit
- [ ] **Timeout:** Operación que tarda mucho → timeout claro (no spinner infinito)
- [ ] **Token expirado mid-sync:** Si el token BDP expira durante sync → error de autenticación, no crash
- [ ] **Rollback parcial:** Si una escritura falla a mitad → las anteriores no se revierten (audit log lo registra)

---

## 📋 Comandos útiles

```bash
# Frontend compila sin errores
cd frontend && npx tsc --noEmit

# Backend compila
cargo check

# Tests backend (25 tests BDP backup)
cargo test --test bdp_backup

# Verificar que el dev server arranca
npm run dev
```

---

## 🐛 Bugs conocidos ya corregidos

- ✅ `snapshots.map is not a function` — Fix: customInstance wrapper extrae `.data` (BKP-008)
- ✅ FK `usuarios(id)` → `users(id)` en migración (BKP-007)
- ✅ `NaiveDateTime` → `DateTime<Utc>` para TIMESTAMPTZ (BKP-007)
- ✅ 25 tests backend pasando (BKP-007)
- ✅ Config BDP separada en pestaña propia, mapeos colapsados (BKP-008)
