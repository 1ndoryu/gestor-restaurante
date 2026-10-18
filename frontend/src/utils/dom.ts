/* [267A-8] Acceso al DOM centralizado: los componentes que necesitan montar
 * un nodo fuera de su árbol (p. ej. diálogos modales) piden el contenedor aquí,
 * así el acceso a `document` queda en un único límite portable. */

/* Crea un <div> al final de <body> y lo devuelve. Quien lo llama es responsable
 * de retirarlo (`contenedor.remove()`) cuando ya no lo necesite. */
export function crearContenedorEnCuerpo(): HTMLDivElement {
    const contenedor = document.createElement('div');
    document.body.appendChild(contenedor);
    return contenedor;
}
