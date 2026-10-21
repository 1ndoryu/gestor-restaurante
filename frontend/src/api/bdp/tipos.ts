/* Contratos de la API BDP (tipos manuales). Sin lógica: cualquier módulo
 * puede importarlos sin arrastrar red ni React. */

/** Extensión manual de VentaConCliente con campos BDP.
 *  Cuando Orval codegen se regenere, estos campos vendrán del schema generado. */
export interface VentaConClienteBdp {
  /* [128A-1/F4] Anulación local de ventas */
  anulada?: boolean;
  /** @nullable */
  anulada_at?: string | null;
  /** @nullable */
  anulacion_motivo?: string | null;
  /** @nullable */
  anulacion_usuario?: string | null;
  bdp_synced?: boolean;
  bdp_order_id?: string | null;
  bdp_sync_error?: string | null;
  bdp_order_status?: string | null;
  bdp_invoiced?: boolean;
  /* [128A-1/F6] Factura local mínima (A7/D9) */
  facturada_local?: boolean;
  /** @nullable */
  factura_numero?: string | null;
  /** @nullable */
  factura_fecha?: string | null;
}

/** Response de anulación local de venta (POST /api/ventas/:id/anular). */
export interface AnularVentaResponse {
  venta: Record<string, unknown>;
  anulada: boolean;
}

export interface BdpOrderStatusResponse {
  venta_id: string;
  bdp_order_id: string | null;
  bdp_order_status: string | null;
  bdp_synced: boolean;
  bdp_sync_error: string | null;
}

export interface BdpPollResponse {
  updated: number;
}

/** Estado BDP mapeado para UI — hereda de VentaConCliente (generated). */
export type BdpStatus = 'pending' | 'accepted' | 'cancelled' | 'invoiced' | 'error' | 'none';

/** Estructura de un mapeo artículo Glory → BDP (GET /api/bdp/article-maps). */
export interface BdpArticleMapItem {
  id: string;
  user_id: string;
  articulo_glory_codigo: string;
  articulo_bdp_codigo: string;
  articulo_bdp_nombre: string;
}

/* [128A-1/F3] Stock local por almacén (fuente de verdad: bdp_article_stock).
 * `stock_actual` de BdpArticleMap es el snapshot BDP de solo lectura; el
 * ajuste manual escribe aquí y nunca pisa `stock_actual`. */
export interface BdpArticleStockItem {
  id: string;
  user_id: string;
  articulo_glory_codigo: string;
  warehouse_id: string;
  warehouse_name: string;
  stock: string;
  /* [128A-1/F3] El sync BDP no sobrescribe filas ajustadas localmente. */
  ajustado_local: boolean;
  ultima_sync_at?: string | null;
  created_at: string;
  updated_at: string;
}

/** Request para ajustar stock local (entrada/salida). */
export interface AjustarBdpArticleStockRequest {
  articulo_glory_codigo: string;
  delta: string;
  motivo: string;
  warehouse_id?: string;
  idempotency_key?: string;
}

/** Request para traspasar stock entre almacenes de BDP (POST /api/bdp/article-stock/traspasar).
 * `cod_traspaso` lo fija el TPV: identifica el motivo del movimiento en BDP. */
export interface TraspasarBdpArticleStockRequest {
  articulo_glory_codigo: string;
  unidades: string;
  cod_traspaso: number;
  almacen_origen: number;
  almacen_destino: number;
}

/** Albarán de compra BDP importado. */
export interface BdpPurchaseNote {
  id: string;
  user_id: string;
  serie: string;
  numero: string;
  fecha: string | null;
  codigo_proveedor: string | null;
  nombre_proveedor: string | null;
  total: string | null;
  /* [128A-1/F5] Procedencia: 'bdp' (importado) | 'local' (creado en la app). */
  origen: 'local' | 'bdp';
  estado: 'pendiente' | 'borrador' | 'conciliado';
  gasto_id: string | null;
  datos_bdp: Record<string, unknown>;
  ultima_sync_at: string | null;
  created_at: string;
  updated_at: string;
}

/** Línea de un albarán local (IVA por línea — A10). */
export interface BdpPurchaseNoteLineaLocal {
  descripcion: string;
  cantidad: string;
  precio_unitario: string;
  iva_pct: string;
}

/** Request para crear un albarán de compra local (F5, M18). */
export interface CrearBdpPurchaseNoteRequest {
  serie?: string;
  numero?: string;
  fecha?: string;
  codigo_proveedor?: string;
  nombre_proveedor?: string;
  total?: string;
  lineas?: BdpPurchaseNoteLineaLocal[];
}

/** Request para actualizar un albarán de compra local (F5). */
export interface ActualizarBdpPurchaseNoteRequest {
  numero?: string;
  fecha?: string;
  codigo_proveedor?: string;
  nombre_proveedor?: string;
  total?: string;
  lineas?: BdpPurchaseNoteLineaLocal[];
}

/** Parámetros para listar albaranes. */
export interface BdpPurchaseNoteFilters {
  proveedor?: string;
  fecha_desde?: string;
  fecha_hasta?: string;
}

/** Request para sincronizar albaranes desde BDP. */
export interface BdpPurchaseNoteSyncRequest {
  export_profile_code?: number;
  fecha_desde?: string;
  fecha_hasta?: string;
  proveedor_desde?: number;
  proveedor_hasta?: number;
}

/** Resumen de sincronización. */
export interface BdpPurchaseNoteSyncResult {
  procesados: number;
  total_bdp: number;
}

/** Request para conciliar un albarán. */
export interface BdpPurchaseNoteReconcileRequest {
  gasto_existente_id?: string;
  categoria_id?: string;
}

/** Resultado de la conciliación. */
export interface BdpPurchaseNoteReconcileResult {
  albaran_id: string;
  gasto_id: string;
  accion: string;
}

/** Tipo de agrupación local: 'menu' | 'pack'. */
export type BdpMenuLocalTipo = 'menu' | 'pack';

/** Línea de un menú/pack local (artículo del catálogo). */
export interface BdpMenuLocalLinea {
  id: string;
  menu_id: string;
  articulo_codigo: string | null;
  descripcion: string;
  cantidad: string;
  precio_unitario: string;
  created_at: string;
}

/** Menú/pack local con sus líneas cargadas. */
export interface BdpMenuLocalConLineas {
  id: string;
  user_id: string;
  tipo: BdpMenuLocalTipo;
  nombre: string;
  descripcion: string | null;
  precio: string;
  activo: boolean;
  created_at: string;
  updated_at: string;
  lineas: BdpMenuLocalLinea[];
}

/** Línea de menú/pack en las peticiones de creación/edición. */
export interface BdpMenuLocalLineaRequest {
  articulo_codigo?: string;
  descripcion: string;
  cantidad?: string;
  precio_unitario?: string;
}

/** Request para crear un menú/pack local (F7). */
export interface CrearBdpMenuLocalRequest {
  tipo: BdpMenuLocalTipo;
  nombre: string;
  descripcion?: string;
  precio?: string;
  activo?: boolean;
  lineas: BdpMenuLocalLineaRequest[];
}

/** Request para actualizar un menú/pack local (F7). */
export interface ActualizarBdpMenuLocalRequest {
  tipo?: BdpMenuLocalTipo;
  nombre?: string;
  descripcion?: string;
  precio?: string;
  activo?: boolean;
  lineas?: BdpMenuLocalLineaRequest[];
}

/** Parámetros para listar menús/packs locales. */
export interface BdpMenuLocalFilters {
  tipo?: BdpMenuLocalTipo;
  activo?: boolean;
  busqueda?: string;
}

export interface AgregarPropinaRequest {
  amount: string;
  /** true suma a la propina existente en BDP, false la sustituye (D8). */
  add_tip?: boolean;
}

export interface PuntoCliente {
  id: string;
  user_id: string;
  cliente_id: string;
  bdp_customer_code: number;
  points_added: string;
  reason: string;
  created_at: string;
}

export interface PuntosClienteResponse {
  saldo: string;
  historial: PuntoCliente[];
}

export interface SumarPuntosRequest {
  /** Positivo suma, negativo resta. */
  points_added: string;
  reason: string;
}

export type BdpCatalogoTipo = 'departamento' | 'familia';

export interface BdpCatalogoClasificacion {
  id: string;
  user_id: string;
  tipo: BdpCatalogoTipo;
  code: number;
  nombre: string;
  created_at: string;
  updated_at: string;
}

export interface CrearBdpClasificacionRequest {
  tipo: BdpCatalogoTipo;
  nombre: string;
}

export interface BdpImportDepartamentosResultado {
  creados: number;
  vinculados: number;
  conflictos: string[];
  omitidos_fuera_rango: number[];
  total_bdp: number;
}

export interface InventarioLineaRequest {
  articulo_glory_codigo: string;
  unidades_contadas: string;
}

export interface RegistrarInventarioRequest {
  articulos: InventarioLineaRequest[];
}

export interface RegistrarInventarioResponse {
  enviados: number;
  omitidos_sin_bdp: number;
}

export interface BdpConteoInventario {
  id: string;
  fecha: string;
  observaciones: string;
  estado: string;
  /** [L7] Almacén al que se aplicó: «0» es el agregado local «General». */
  warehouse_id: string;
  creado_el: string;
  total_lineas: number;
}

export interface BdpConteoInventarioLinea {
  id: string;
  articulo_glory_codigo: string;
  esperado: number;
  contado: number;
  diferencia: number;
  aplicado_al_stock: boolean;
}

export interface ConteoInventarioCreado {
  conteo: BdpConteoInventario;
  lineas: BdpConteoInventarioLinea[];
  reutilizado: boolean;
  aplicadas: number;
  encolados: number;
  omitidos_sin_bdp: number;
}

export interface BdpPushFlushResumen {
  procesados: number;
  sincronizados: number;
  pendientes_suscripcion: number;
  rechazados: number;
  errores: number;
  omitidos_standalone: number;
  omitidos_manual: number;
}

export interface BdpPushFila {
  id: string;
  dominio: string;
  entidad_id: string;
  operacion: string;
  estado: string;
  reintentos: number;
  ultimo_error: string | null;
  updated_at: string;
}

