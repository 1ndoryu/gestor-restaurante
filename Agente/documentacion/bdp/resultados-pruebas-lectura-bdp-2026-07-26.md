# Resultados — Pruebas de lectura BDP (2026-07-26)

## 1. Auditoría de seguridad ✅

El recorrido de código de estas 4 páginas no invoca endpoints comerciales de escritura en BDP. Las consultas pueden crear una sesión y quedar registradas en los logs internos de BDP, pero no crean ni modifican clientes, comandas, pagos, facturas o stock.

| Página | Endpoints API | Efecto en BDP | Efecto en Glory |
|--------|---------------|---------------|-----------------|
| **Stock** | `GET /api/bdp/article-maps`, `POST /api/bdp/article-maps/sync-catalog` | Solo lectura (GetPosArticles, GetPricesArticles) | Escribe en DB local |
| **Explorador** | `GET /api/bdp/menus/:id`, `GET /api/bdp/fastfoods/:id`, `GET /api/bdp/packs/:id` | Solo lectura | Ninguno |
| **Historial** | `GET /api/bdp/audit`, `GET /api/bdp/backup/snapshots` | **No contacta BDP** | Lee audit log y snapshots locales |
| **Compras** | `GET /api/bdp/purchase-notes`, `POST /api/bdp/purchase-notes/sync` | Solo lectura (ExportPurchaseNotes) | Escribe albaranes en DB local |

**Nota:** Los endpoints de "Sync catálogo" y "Sync albaranes" leen de BDP y escriben solo en la DB local de Glory. Los endpoints de "Borrador" y "Conciliar" no contactan BDP en absoluto.

## 2. Pruebas de carga de páginas ✅

Las 4 páginas se cargan correctamente en modo demo con datos de ejemplo:

| Página | Estado | Datos mostrados | Errores |
|--------|--------|-----------------|---------|
| **Stock** (`/bdp/stock`) | ✅ Carga OK | 6 artículos demo, filtros y paginación funcionan | Ninguno |
| **Explorador** (`/bdp/explorador`) | ✅ Carga OK | 4 definiciones (menú, fastfood, pack), búsqueda funciona | Ninguno |
| **Historial** (`/bdp/historial`) | ✅ Carga OK | 3 entradas de auditoría, 2 snapshots, pestañas funcionan | Ninguno |
| **Compras** (`/bdp/compras`) | ✅ Carga OK | 4 albaranes, filtros de proveedor y fecha funcionan | Ninguno |

**Console warnings (no bloqueantes):**
- React Router Future Flag Warning (v7 compatibility)
- Deprecated feature warning
- Form field sin id/name attribute

## 3. Conectividad al BDP real ⚠️

| Verificación | Resultado |
|---|---|
| Credenciales configuradas | ✅ Variables requeridas presentes. Sus valores no se documentan ni se registran. |
| URL BDP | Origen privado configurado mediante variable de entorno; no se publica en documentación. |
| Health check | ❌ **Falló** — el restaurante se desconectó de TailScale durante la prueba |
| Sync habilitado | ✅ `bdp_sync_enabled=true` |
| Modo operación | `read_only` |

**Importante:** La IP `100.83.196.35` es accesible desde local **cuando el restaurante está conectado a TailScale**. No es necesario deployar a producción ni usar VPN. La prueba falló porque el restaurante se desconectó de la red en ese momento.

**Para repetir las pruebas:** Confirmar TailScale, la tarifa/perfil de artículos y el código real de la plantilla `ExportPurchaseNotes` antes de re-ejecutar.

## 3.1 Reejecución segura — 2026-07-28

Las llamadas se ejecutaron una por una desde el test ignorado `bdp_readonly`, sin iniciar la aplicación, sin base de datos y con las allowlists de escritura/OnlyCheck ausentes.

| Lectura | Resultado | Evidencia |
|---|---|---|
| `POST /Service/Health` | ✅ Correcto | BDP respondió `is_alive=true` en 1,18 s |
| `POST /Auth/Login` | ✅ Correcto | Sesión emitida con expiración válida; token y credenciales no se registraron |
| `ExportArticles` | ⚠️ Contrato correcto, sin datos | BDP devolvió el array `Articles`, pero vacío; revisar tarifa/perfil de exportación antes de considerar Stock validado |
| `GetTenderList` | ✅ Correcto | BDP real usa `TenderList`; se recibieron 18 formas de pago sin imprimir sus datos |
| `ExportPurchaseNotes` | ❌ Configuración BDP pendiente | El rango de proveedores ya fue corregido, pero BDP rechazó el perfil porque la plantilla indicada no existe o está mal configurada. No se probaron códigos alternativos |

Ninguna de estas llamadas invocó endpoints de clientes, comandas, pagos, facturas, cancelación ni `OnlyCheck`. No se produjo ninguna escritura comercial en BDP.

**Bloqueos reales descubiertos:** confirmar qué tarifa/perfil hace exportables los artículos y obtener del restaurante el código exacto de la plantilla `ExportPurchaseNotes`. Hasta entonces, Stock y Compras no están funcionalmente validados contra datos reales.

## 4. Próximos pasos para pruebas reales

| # | Acción | Requisito |
|---|---|---|
| 1 | Verificar que el restaurante está en TailScale | Ejecutar el test acotado `bdp_real_health`, que usa `POST /Service/Health` sin mostrar secretos |
| 2 | Probar Stock contra BDP real | Local + TailScale activo → `http://localhost:5173/bdp/stock`, desactivar demo mode, pulsar "Sync catálogo" |
| 3 | Probar Explorador contra BDP real | Local + TailScale → buscar código de menú/pack/fastfood conocido del restaurante |
| 4 | Probar Historial contra BDP real | Local + TailScale → verificar entradas de auditoría (lee DB local, no requiere BDP activo) |
| 5 | Probar Compras contra BDP real | Local + TailScale → pulsar "Sync albaranes" con rango de fechas reciente |

**Recomendación:** Hacer las lecturas fuera de horas punta, con TailScale verificado y sin ejecutar pruebas de escritura en la misma sesión.
