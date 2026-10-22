/* [D33] Un grupo de tarjetas con un único punto de guardado. Ese punto no vive
 * aquí sino en la barra fija de la esquina (`BarraGuardado`): antes cada pestaña
 * tenía dos botones «Guardar» —uno arriba y otro al final— y el de arriba de una
 * tarjeta parecía guardar solo lo que tenía debajo. Lo que guarda al instante NO
 * se envuelve aquí: lleva `GuardadoAlInstante` para decirlo.
 *
 * [D33c] El grupo ya no se pinta como tarjeta contenedora: las tarjetas hijas ya
 * son tarjetas, y una caja dentro de otra hacía que en Conexiones convivieran los
 * dos estilos (unas agrupadas dentro de una caja y otras sueltas). Ahora todas
 * las pestañas son lo mismo: un título de sección y una lista de tarjetas
 * hermanas, separadas por el mismo hueco. */

import { Check } from 'lucide-react';

interface CardSeccionProps {
  /** Qué grupo es, en las palabras de la usuaria («Mi restaurante»). */
  titulo: string;
  /** Una línea con el paraqué del grupo. */
  descripcion: string;
  /** El grupo entero guarda al instante: la marca va bajo el título, no suelta
   * entre dos tarjetas, para que se lea como estado del grupo. */
  alInstante?: boolean;
  children: React.ReactNode;
}

export function CardSeccion({ titulo, descripcion, alInstante, children }: CardSeccionProps) {
  return (
    <section className="flex flex-col gap-4">
      <CabeceraGrupo titulo={titulo} descripcion={descripcion} alInstante={alInstante} />
      {children}
    </section>
  );
}

/** Cabecera de un grupo de tarjetas. Se exporta porque hay pestañas que no
 * agrupan (todo se guarda al instante) y aun así necesitan el mismo título que
 * las demás: sin él, media pantalla parece de otra aplicación. */
export function CabeceraGrupo({
  titulo,
  descripcion,
  alInstante,
}: Omit<CardSeccionProps, 'children'>) {
  return (
    <div className="flex flex-col gap-1">
      <h2 className="text-lg font-semibold">{titulo}</h2>
      <p className="text-sm text-muted-foreground">{descripcion}</p>
      {alInstante && <GuardadoAlInstante />}
    </div>
  );
}

/** Marca para lo que no espera al botón. Sin ella la pantalla mezcla dos modos de
 * guardado y no se sabe cuál es cuál; con ella, la pestaña dice de un vistazo si
 * hay que pulsar algo. */
export function GuardadoAlInstante() {
  return (
    <span className="inline-flex w-fit items-center gap-1.5 rounded-md border px-2 py-1 text-xs text-muted-foreground">
      <Check className="size-3.5" />
      Se guarda al instante, sin pulsar «Guardar»
    </span>
  );
}
