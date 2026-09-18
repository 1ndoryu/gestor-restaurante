/* [267A-8] Confirmaciones con componente propio: `window.confirm`/`prompt`
 * lanzan "not supported" en el navegador empaquetado, y el `dismiss`
 * programático de sonner depende de `requestAnimationFrame` (congelado en ese
 * entorno: verificado que nunca dispara). Diálogo modal con promesa, sin
 * rAF ni temporizadores. */
import { useState, type ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

type Cerrar = () => void;

function montar(contenido: (cerrar: Cerrar) => ReactNode): void {
    const contenedor = document.createElement('div');
    document.body.appendChild(contenedor);
    const root = createRoot(contenedor);
    const cerrar: Cerrar = () => {
        root.unmount();
        contenedor.remove();
    };
    root.render(<>{contenido(cerrar)}</>);
}

function Marco({
    titulo,
    descripcion,
    alFondo,
    children,
}: {
    titulo: string;
    descripcion?: string;
    alFondo: () => void;
    children: ReactNode;
}) {
    return (
        <div
            className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 p-4"
            onMouseDown={(e) => {
                if (e.target === e.currentTarget) alFondo();
            }}
        >
            <div className="w-full max-w-md rounded-lg border bg-background p-4 shadow-lg">
                <p className="text-sm font-semibold">{titulo}</p>
                {descripcion ? <p className="mt-1 text-xs text-muted-foreground">{descripcion}</p> : null}
                <div className="mt-3">{children}</div>
            </div>
        </div>
    );
}

interface ConfirmarOpts {
    titulo: string;
    descripcion?: string;
    textoConfirmar?: string;
    textoCancelar?: string;
}

/** Diálogo sí/no. Resuelve `true` solo con el botón de confirmación. */
export function confirmarConDialogo(opts: ConfirmarOpts): Promise<boolean> {
    return new Promise((resolve) => {
        let hecho = false;
        montar((cerrar) => {
            const fin = (valor: boolean) => {
                if (hecho) return;
                hecho = true;
                cerrar();
                resolve(valor);
            };
            return (
                <Marco titulo={opts.titulo} descripcion={opts.descripcion} alFondo={() => fin(false)}>
                    <div className="flex justify-end gap-2">
                        <Button size="sm" variant="outline" onClick={() => fin(false)}>
                            {opts.textoCancelar ?? 'Cancelar'}
                        </Button>
                        <Button size="sm" variant="destructive" onClick={() => fin(true)}>
                            {opts.textoConfirmar ?? 'Confirmar'}
                        </Button>
                    </div>
                </Marco>
            );
        });
    });
}

interface PedirTextoOpts {
    titulo: string;
    descripcion?: string;
    placeholder?: string;
    textoConfirmar?: string;
    /** Devuelve mensaje de error si el valor no vale, o `null` si vale. */
    validar?: (valor: string) => string | null;
}

function FormularioTexto({
    opts,
    fin,
}: {
    opts: PedirTextoOpts;
    fin: (valor: string | null) => void;
}) {
    const [valor, setValor] = useState('');
    const [error, setError] = useState<string | null>(null);
    const enviar = () => {
        const fallo = opts.validar?.(valor) ?? null;
        if (fallo) setError(fallo);
        else fin(valor);
    };
    return (
        <div className="flex flex-col gap-2">
            <Input
                autoFocus
                value={valor}
                placeholder={opts.placeholder ?? ''}
                onChange={(e) => {
                    setValor(e.target.value);
                    setError(null);
                }}
                onKeyDown={(e) => {
                    if (e.key === 'Enter') enviar();
                    if (e.key === 'Escape') fin(null);
                }}
            />
            {error ? <p className="text-xs text-red-500">{error}</p> : null}
            <div className="flex justify-end gap-2">
                <Button size="sm" variant="outline" onClick={() => fin(null)}>
                    Cancelar
                </Button>
                <Button size="sm" onClick={enviar}>
                    {opts.textoConfirmar ?? 'Confirmar'}
                </Button>
            </div>
        </div>
    );
}

/** Diálogo con campo de texto. Resuelve el texto o `null` al cancelar. */
export function pedirTextoConDialogo(opts: PedirTextoOpts): Promise<string | null> {
    return new Promise((resolve) => {
        let hecho = false;
        montar((cerrar) => {
            const fin = (valor: string | null) => {
                if (hecho) return;
                hecho = true;
                cerrar();
                resolve(valor);
            };
            return (
                <Marco titulo={opts.titulo} descripcion={opts.descripcion} alFondo={() => fin(null)}>
                    <FormularioTexto opts={opts} fin={fin} />
                </Marco>
            );
        });
    });
}

interface ElegirOpcionOpts<T extends string> {
    titulo: string;
    descripcion?: string;
    opciones: { valor: T; etiqueta: string }[];
}

/** Diálogo con botones de opción. Resuelve el valor elegido o `null`. */
export function elegirOpcionConDialogo<T extends string>(opts: ElegirOpcionOpts<T>): Promise<T | null> {
    return new Promise((resolve) => {
        let hecho = false;
        montar((cerrar) => {
            const fin = (valor: T | null) => {
                if (hecho) return;
                hecho = true;
                cerrar();
                resolve(valor);
            };
            return (
                <Marco titulo={opts.titulo} descripcion={opts.descripcion} alFondo={() => fin(null)}>
                    <div className="flex flex-wrap gap-2">
                        {opts.opciones.map((op) => (
                            <Button key={op.valor} size="sm" variant="outline" onClick={() => fin(op.valor)}>
                                {op.etiqueta}
                            </Button>
                        ))}
                        <Button size="sm" variant="ghost" onClick={() => fin(null)}>
                            Cancelar
                        </Button>
                    </div>
                </Marco>
            );
        });
    });
}
