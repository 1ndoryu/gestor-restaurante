import { z } from "zod"

// Esquema de cada fila de la tabla; lo reexporta data-table.tsx.
export const schema = z.object({
  id: z.number(),
  header: z.string(),
  type: z.string(),
  status: z.string(),
  target: z.string(),
  limit: z.string(),
  reviewer: z.string(),
})

export type FilaTabla = z.infer<typeof schema>
