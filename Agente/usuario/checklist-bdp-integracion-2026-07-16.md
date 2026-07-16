# Checklist: Integración completa BDP — Pruebas manuales

> **Fecha:** 2026-07-16
> **Alcance:** Sync BDP + Backup/Restauración + Configuración
> **URL local:** http://localhost:5174/configuracion

---

## 🔴 PRIORIDAD ALTA — Funcionalidad core

### 1. Panel de Backup & Seguridad BDP
- [ ] **Carga sin errores:** La pestaña "Backup BDP" aparece en Configuración sin crashes (verificar consola del navegador)
- [ ] **Tab "Snapshots" visible:** Al hacer click, muestra la tabla (vacía o con datos)
- [ ] **Tab "Auditoría" visible:** Al hacer click, muestra la tabla (vacía o con datos)

### 2. Crear snapshots
- [ ] **Snapshot completo:** Click "Crear completo" → aparece en la tabla con tipo `completo`, estado `disponible`, fecha actual
- [ ] **Snapshot parcial:** Seleccionar tipos (menú, productos, etc.) → click "Crear parcial" → aparece con tipo `parcial` y los tipos seleccionados
- [ ] **Snapshot Glory:** Seleccionar tipos → click "Crear Glory" → aparece con tipo `glory`
- [ ] **Notas opcionales:** Crear snapshot con y sin nota, verificar que se guarda correctamente
- [ ] **Loading state:** Al crear, el botón muestra spinner/deshabilitado mientras procesa

### 3. Restaurar desde snapshot
- [ ] **Restaurar Glory:** Click en botón restaurar de un snapshot → confirmar diálogo → resultado exitoso con detalle de tablas restauradas
- [ ] **Restaurar no destructiva:** Los datos del restaurante NO se pierden (solo Glory se sobreescribe)
- [ ] **Error handling:** Intentar restaurar un snapshot expirado o eliminado → muestra error claro

### 4. Eliminar snapshots
- [ ] **Eliminar:** Click en botón eliminar → confirmar → snapshot desaparece de la tabla
- [ ] **Confirmación:** Pide confirmación antes de eliminar (no se borra accidentalmente)

### 5. Auditoría
- [ ] **Registros aparecen:** Después de crear/restaurar snapshots, la tab "Auditoría" muestra los registros
- [ ] **Detalle correcto:** Cada entrada muestra operación, resultado (éxito/error), timestamp, usuario
- [ ] **Filtro por operación:** Si hay filtros, verificar que funcionan

---

## 🟡 PRIORIDAD MEDIA — Sync BDP

### 6. Modo de sincronización
- [ ] **Leer modo actual:** El panel muestra el modo actual (read_only / unidirectional / bidirectional)
- [ ] **Cambiar modo:** Seleccionar otro modo → guardar → verificar que persiste al recargar la página
- [ ] **Validación backend:** El backend rechaza modos inválidos

### 7. Pre-write audit (si aplica)
- [ ] **Sync unidirectional:** Al escribir datos hacia BDP, se registra entrada de auditoría ANTES de la escritura
- [ ] **Sync bidirectional:** Igual que unidirectional pero en ambas direcciones
- [ ] **Sync read_only:** NO se permite escritura hacia BDP (si hay UI de escritura, debe estar deshabilitada)

---

## 🟢 PRIORIDAD BAJA — Edge cases

### 8. Limpieza de snapshots expirados
- [ ] **Expiración:** Crear snapshot manualmente con `expires_at` en el pasado → ejecutar limpieza → se elimina
- [ ] **Configuración retención:** Verificar que `bdp_backup_retention_days` se respeta al crear nuevos snapshots

### 9. Configuración BDP
- [ ] **Variables de entorno:** Las credenciales de BDP (URL, token, usuario) se cargan desde `.env` / config del servidor
- [ ] **Mapeo de entidades:** El mapeo BDP ↔ Glory funciona correctamente (si hay UI, verificar que muestra los valores actuales)
- [ ] **Guardar config:** Los cambios de configuración persisten al recargar

### 10. Manejo de errores
- [ ] **Sin conexión BDP:** Si el servidor BDP no responde, muestra error claro (no crash)
- [ ] **Token expirado:** Si el token BDP expira, muestra error de autenticación
- [ ] **Rate limiting:** Si hay demasiadas peticiones, muestra error de rate limit
- [ ] **Timeout:** Si una operación tarda mucho, muestra timeout (no spinner infinito)

---

## 📋 Comandos útiles para pruebas

```bash
# Verificar que el frontend compila sin errores
cd frontend && npx tsc --noEmit

# Verificar que el backend compila
cargo check

# Verificar tests del backend
cargo test --test bdp_backup

# Ver logs del backend en producción
# (vía coolify-manager-rs)
```

---

## 🐛 Bugs conocidos ya corregidos

- ✅ `snapshots.map is not a function` — Fix: customInstance wrapper extrae `.data` correctamente
- ✅ FK `usuarios(id)` → `users(id)` en migración
- ✅ `NaiveDateTime` → `DateTime<Utc>` para TIMESTAMPTZ
- ✅ 25 tests backend pasando (BKP-007)
