/* [287A-7] Estado accionable para Compras BDP desactivado.
 * Evita peticiones 422 previsibles y lleva al usuario al ajuste correcto. */

import { Settings } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function BdpPurchaseFeatureNotice() {
  const navigate = useNavigate();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Compras BDP está desactivado</CardTitle>
        <CardDescription>
          Activa la lectura de albaranes para consultar e importar Compras desde BDP. Esta opción no modifica BDP.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Button
          variant="outline"
          onClick={() => navigate('/configuracion', { state: { bdpSection: 'bdp' } })}
        >
          <Settings className="size-4" />
          Abrir Configuración BDP
        </Button>
      </CardContent>
    </Card>
  );
}
