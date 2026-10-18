/* Barra de consulta del explorador: en demo filtra las definiciones; en real pide un tipo
 * y un código numérico para consultar BDP. Sin estado propio: lo recibe de BdpExplorador. */
import { Loader2, Search } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { TIPOS, type ExploreType } from './constantes';

interface BarraConsultaProps {
  demoMode: boolean;
  busqueda: string;
  setBusqueda: (valor: string) => void;
  tipo: ExploreType;
  setTipo: (valor: ExploreType) => void;
  identificador: string;
  setIdentificador: (valor: string) => void;
  setBuscado: (valor: boolean) => void;
  isLoading: boolean;
  buscar: () => void;
}

export function BarraConsulta({ demoMode, busqueda, setBusqueda, tipo, setTipo, identificador, setIdentificador, setBuscado, isLoading, buscar }: BarraConsultaProps) {
  return (
    <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
      {demoMode ? (
        <>
          <div className="flex flex-wrap gap-3 items-center">
            <Input
              type="search"
              placeholder="Buscar nombre, código o descripción..."
              value={busqueda}
              onChange={(e) => setBusqueda(e.target.value)}
              className="max-w-xs"
            />
            <Select
              value={tipo}
              onValueChange={(v) => setTipo(v as ExploreType)}
            >
              <SelectTrigger className="w-full sm:w-44">
                <SelectValue placeholder="Tipo" />
              </SelectTrigger>
              <SelectContent>
                {TIPOS.map((t) => (
                  <SelectItem key={t.value} value={t.value}>
                    {t.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </>
      ) : (
        <div className="flex flex-wrap gap-3 items-end">
          <Select
            value={tipo}
            onValueChange={(v) => {
              setTipo(v as ExploreType);
              setBuscado(false);
            }}
          >
            <SelectTrigger className="w-full sm:w-44">
              <SelectValue placeholder="Tipo" />
            </SelectTrigger>
            <SelectContent>
              {TIPOS.filter((t) => t.value !== 'all').map((t) => (
                <SelectItem key={t.value} value={t.value}>
                  {t.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            type="number"
            min={1}
            className="w-36"
            placeholder="Código numérico..."
            value={identificador}
            onChange={(e) => {
              setIdentificador(e.target.value);
              setBuscado(false);
            }}
          />
          <Button onClick={buscar} disabled={isLoading}>
            {isLoading ? (
              <Loader2 className="size-3.5 animate-spin mr-1" />
            ) : (
              <Search className="size-3.5 mr-1" />
            )}
            Consultar
          </Button>
        </div>
      )}
    </div>
  );
}
