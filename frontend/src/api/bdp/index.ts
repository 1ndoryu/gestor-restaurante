/* Punto de entrada del módulo BDP. Re-exporta todo el reparto interno para que
 * los importadores sigan usando `api/bdp` sin cambiar sus rutas. */

export * from './tipos';
export * from './helpers';
export * from './requests-ventas';
export * from './requests-articulos';
export * from './requests-inventario';
export * from './requests-albaranes';
export * from './requests-menus';
export * from './requests-sincronizacion';
export * from './hooks';
