/* Definiciones de ejemplo del explorador BDP (menús, fastfood y packs) para el modo demo. */

export interface BdpExplorerItem {
  id: number;
  type: 'menu' | 'fastfood' | 'pack';
  code: string;
  name: string;
  description: string;
  lines: { articleCode: string; articleName: string; quantity: number }[];
}

export const mockExplorerItems: BdpExplorerItem[] = [
  {
    id: 1,
    type: 'menu',
    code: 'MENU-01',
    name: 'Menú del día',
    description: 'Menú completo de lunes a viernes',
    lines: [
      { articleCode: 'ART-001', articleName: 'Coca-Cola Zero 33cl', quantity: 1 },
      { articleCode: 'ART-003', articleName: 'Jamón Ibérico 100g', quantity: 1 },
    ],
  },
  {
    id: 2,
    type: 'menu',
    code: 'MENU-02',
    name: 'Menú fin de semana',
    description: 'Menú especial fin de semana',
    lines: [
      { articleCode: 'ART-002', articleName: 'Estrella Galicia 33cl', quantity: 1 },
      { articleCode: 'ART-004', articleName: 'Tarta de queso', quantity: 1 },
    ],
  },
  {
    id: 3,
    type: 'pack',
    code: 'PACK-01',
    name: 'Pack familiar',
    description: 'Pack para 4 personas',
    lines: [
      { articleCode: 'ART-001', articleName: 'Coca-Cola Zero 33cl', quantity: 4 },
      { articleCode: 'ART-003', articleName: 'Jamón Ibérico 100g', quantity: 2 },
    ],
  },
  {
    id: 4,
    type: 'fastfood',
    code: 'FAST-01',
    name: 'Venta rápida barra',
    description: 'Modalidad de venta rápida en barra',
    lines: [
      { articleCode: 'ART-005', articleName: 'Café solo', quantity: 1 },
      { articleCode: 'ART-006', articleName: 'Botella agua 50cl', quantity: 1 },
    ],
  },
];
