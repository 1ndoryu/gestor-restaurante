# Plataforma de Gestión de Restaurantes — Roadmap

> **URL producción:** http://restaurante.wandori.us (Swagger UI: /swagger-ui/)
> **URL legacy (no funcional):** ~~http://app-b8s0cks444o0sogo8kg8wcgw.66.94.100.241.sslip.io~~
> **Servidor:** 66.94.100.241 (Coolify, servicio UUID: b8s0cks444o0sogo8kg8wcgw)
> **Deploy:** usar `coolify-manager-rs` directamente, sin wrapper local: `cd "c:\Users\Owner\OneDrive\Documentos\WP\app\public\wp-content\themes\glorytemplate\.agent\coolify-manager-rs" ; .\target\release\coolify-manager.exe deploy --name glory-rest --update --skip-backup`
> **Repositorio:** 1ndoryu/glory-rs, rama glory-rs-rest

## Stack

| Capa            | Herramienta                          |
| --------------- | ------------------------------------ |
| Framework web   | Axum 0.7                             |
| OpenAPI         | utoipa 4 + utoipa-swagger-ui 7       |
| Base de datos   | SQLx 0.8 (PostgreSQL 18)             |
| Validación      | validator 0.18                       |
| Auth            | jsonwebtoken + argon2 + SHA-256 (API keys) |
| Frontend        | React 18 + TypeScript + Vite + Tailwind v4 + shadcn/ui |
| State           | React Query + Zustand                |
| Codegen         | Orval 8 (tags-split)                 |
| Linter          | clippy (deny all + warn pedantic)    |

## Notas del cliente

- Diseño minimalista, simple, intuitivo.
- Una instancia por propietario/restaurante (no multi-tenant con registro público).
- Si conseguimos más negocios, clonar la plataforma para cada nuevo cliente usando glory-rs como template.
- Parte económica simplificada: solo Gastos, Ventas y Margen (Ventas - Gastos).
- Secciones omitidas/pausadas: Dashboard analítico avanzado, Administración de documentos, Conciliación, Incidencias, Tesorería, Bancos.

## Estado actual

WhatsApp ya quedó validado hasta `accepted` en Meta, pero la entrega real sigue bloqueada por la verificación de empresa/número. BDP ya tiene conectividad real por Tailscale y WebLink validado con `Health`, `Login` y `GetVersion`; además existe un dry-run seguro de sincronización que lee catálogo/permisos reales y usa `CreateOrder` con `OrderOperationType = 1` para validar sin crear comandas.

## Tareas pendientes

- 065A-1 — Probar marketing WhatsApp real y recordatorios con número propio: el sistema ya está cableado a proveedores reales, pero falta validarlo con Meta/WhatsApp en entorno real. Estado 2026-05-06: la Cloud API acepto envios directos y `hello_world`, pero la cuenta sigue usando el `Test Number` de Meta (`+1 555-646-0107`, `NOT_VERIFIED`) y la empresa quedo en verificacion pendiente con ETA de 2 dias; hasta que Meta habilite la cuenta o el numero real quede verificado, el circuito no puede cerrarse con entrega confirmada. Cuando se habilite: usar el número del usuario como contacto destinatario, configurar credenciales reales solo en UI/secret manager, no copiar tokens al repo, enviar campaña real, validar historial/contadores/errores, probar recordatorio automático por WhatsApp con reserva/control temporal y documentar resultado. No enviar a clientes reales hasta confirmar el circuito completo.
- 065A-4 — Validar BDP/WebLink contra PC real del restaurante: conectividad Tailscale y diagnostico Health/Login/GetVersion ya probados. El dry-run seguro en produccion confirma lecturas reales (`POS`, empleado, empleados del POS, formas de pago, departamentos y articulos) y `escritura_real=false`; `CreateOrder OnlyCheck` sigue bloqueado por BDP con `[300035]-NO SE HA DEFINIDO UNA SERIE DE FACTURACION VALIDA` incluso sin pagos. Pendiente operativo: configurar serie/destino WebLink en BDP, repetir `Probar sincronización segura` hasta `listo_para_sincronizar = true`, documentar captura/resultado para el cliente y, solo después, activar escrituras reales.
