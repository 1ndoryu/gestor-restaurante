# Contrato: ModifyArticleAndUpdateProfile (Q2.2)

> **Referencia:** `049A-1/S2` — Implementación read-modify-write para
> `ModifyArticleAndUpdateProfile`. Este documento describe el contrato
> completo del endpoint y su uso desde el servicio BDP push.

## Endpoint

```
POST /API/Articles/ModifyAndUpdateProfiles
Content-Type: application/json
Authorization: Bearer <token>
```

## Estructura de la request

```json
{
  "ArticleData": { /* ArticleListDataType (89 campos) */ },
  "ProfilesList": [ /* opcional, lista de perfiles */ ],
  "AllProfiles": true  /* opcional, true = todos los perfiles */
}
```

### ArticleData — 89 campos requeridos por el BDP real

Según el contrato validado por el simulador (`server.py:_modify_article`),
el endpoint **rechaza** cualquier `ArticleData` que falte de los siguientes:

| # | Campo | Tipo | Descripción |
|---|-------|------|-------------|
| 1 | `ArtCode` | int | Código del artículo (obligatorio, >0) |
| 2 | `ArtDescription` | string | Nombre corto del artículo |
| 3 | `DeptCode` | int | Código del departamento |
| 4 | `DeptDescription` | string | Nombre del departamento |
| 5 | `MenuDish` | bool | ¿Es plato de menú? |
| 6 | `WebArticle` | bool | ¿Se muestra en web? |
| 7 | `POS_SupplementsProfileID` | int | Perfil de suplementos TPV |
| 8 | `SelfOrdering_CommentsProfileID` | int | Perfil comentarios autopédido |
| 9 | `SelfOrdering_SupplementsProfileID` | int | Perfil suplementos autopédido |
| 10 | `POS_MenuID` | int | ID de menú TPV |
| 11 | `POS_FastfoodID` | int | ID de fastfood TPV |
| 12 | `POS_PackID` | int | ID de pack TPV |
| 13 | `Is_Inventoriable` | bool | ¿Es inventariable? |
| 14 | `BuyTAVCode` | int | Código IVA compra |
| 15 | `BuyTAVPer` | decimal | % IVA compra |
| 16 | `TAVCode` | int | Código IVA venta |
| 17 | `TAVPer` | decimal | % IVA venta |
| 18 | `AuxPrinters` | string | Impresoras auxiliares |
| 19 | `Commissionable` | bool | ¿Comisionable? |
| 20 | `ModifiablePrice` | bool | ¿Precio modificable en TPV? |
| 21 | `DontPrintTicketValue0` | bool | No imprimir ticket si valor 0 |
| 22 | `Weight` | decimal | Peso |
| 23 | `DontNotifyUnitsPrice0` | bool | No notificar precio 0 |
| 24 | `NotifyModifyPriceUnits` | bool | Notificar cambio precio unidades |
| 25 | `TwoForOne` | bool | 2×1 |
| 26 | `POS_CommentsProfileID` | int | Perfil comentarios TPV |
| 27 | `ErrorMessage` | string | Siempre `""` en escritura |
| 28 | `PriceConfirmation` | bool | Confirmar precio |
| 29 | `FreeDescription` | string | Descripción libre |
| 30 | `IsCombinable` | bool | ¿Es combinable? |
| 31 | `CombinedDescription` | string | Descripción del combinado |
| 32 | `CombBasePrice1` | decimal | Precio base combinado 1 |
| 33 | `CombBasePrice2` | decimal | Precio base combinado 2 |
| 34 | `CombBasePrice3` | decimal | Precio base combinado 3 |
| 35 | `CombBasePrice4` | decimal | Precio base combinado 4 |
| 36 | `CombBasePrice5` | decimal | Precio base combinado 5 |
| 37 | `CombAuxPrice1` | decimal | Precio auxiliar combinado 1 |
| 38 | `CombAuxPrice2` | decimal | Precio auxiliar combinado 2 |
| 39 | `CombAuxPrice3` | decimal | Precio auxiliar combinado 3 |
| 40 | `CombAuxPrice4` | decimal | Precio auxiliar combinado 4 |
| 41 | `CombAuxPrice5` | decimal | Precio auxiliar combinado 5 |
| 42 | `ActivateAlwaysCombined` | bool | Activar siempre combinado |
| 43 | `MandatoryCombined` | bool | Combinado obligatorio |
| 44 | `CombinedAssocType` | int | Tipo asociación combinado |
| 45 | `CombinedDepartmentAssoc` | int | Depto. asociado combinado |
| 46 | `CombinedDepartmentAssocDescription` | string | Descripción depto. asociado |
| 47 | `CombinedMaxiscreenAssoc` | int | Maxipantalla asociada combinado |
| 48 | `CombinedMaxiscreenAssocDescription` | string | Descripción maxipantalla asociada |
| 49 | `ApplyDiscountsInComb` | bool | Aplicar descuentos en combinado |
| 50 | `Price1` | decimal | Precio 1 |
| 51 | `Price2` | decimal | Precio 2 |
| 52 | `Price3` | decimal | Precio 3 |
| 53 | `Price4` | decimal | Precio 4 |
| 54 | `Price5` | decimal | Precio 5 |
| 55 | `Dct1` | decimal | Descuento 1 |
| 56 | `Dct2` | decimal | Descuento 2 |
| 57 | `Dct3` | decimal | Descuento 3 |
| 58 | `Dct4` | decimal | Descuento 4 |
| 59 | `Dct5` | decimal | Descuento 5 |
| 60 | `GraphDescrip1` | string | Descripción gráfica 1 |
| 61 | `GraphDescrip2` | string | Descripción gráfica 2 |
| 62 | `GraphDescrip3` | string | Descripción gráfica 3 |
| 63 | `ExtendedArtDescription` | string | Descripción extendida |
| 64 | `Proportion1Description` | string | Descripción proporción 1 |
| 65 | `Proportion2Active` | bool | Proporción 2 activa |
| 66 | `Proportion3Active` | bool | Proporción 3 activa |
| 67 | `Proportion4Active` | bool | Proporción 4 activa |
| 68 | `Proportion5Active` | bool | Proporción 5 activa |
| 69 | `Proportion6Active` | bool | Proporción 6 activa |
| 70 | `Proportion7Active` | bool | Proporción 7 activa |
| 71 | `Proportion8Active` | bool | Proporción 8 activa |
| 72 | `Proportion9Active` | bool | Proporción 9 activa |
| 73 | `Proportion2Amount` | decimal | Cantidad proporción 2 |
| 74 | `Proportion3Amount` | decimal | Cantidad proporción 3 |
| 75 | `Proportion4Amount` | decimal | Cantidad proporción 4 |
| 76 | `Proportion5Amount` | decimal | Cantidad proporción 5 |
| 77 | `Proportion6Amount` | decimal | Cantidad proporción 6 |
| 78 | `Proportion7Amount` | decimal | Cantidad proporción 7 |
| 79 | `Proportion8Amount` | decimal | Cantidad proporción 8 |
| 80 | `Proportion9Amount` | decimal | Cantidad proporción 9 |
| 81 | `Proportion2Description` | string | Descripción proporción 2 |
| 82 | `Proportion3Description` | string | Descripción proporción 3 |
| 83 | `Proportion4Description` | string | Descripción proporción 4 |
| 84 | `Proportion5Description` | string | Descripción proporción 5 |
| 85 | `Proportion6Description` | string | Descripción proporción 6 |
| 86 | `Proportion7Description` | string | Descripción proporción 7 |
| 87 | `Proportion8Description` | string | Descripción proporción 8 |
| 88 | `Proportion9Description` | string | Descripción proporción 9 |
| 89 | `ApplyDiscountsInProp` | bool | Aplicar descuentos en proporción |

**Campos de proporción con precio y descuento** (Proportion2..9, 7 grupos × 7 campos):

| Rango | Campos |
|-------|--------|
| 90–94 | `Proportion{N}Price1` a `Proportion{N}Price5` (5 precios × 8 proporciones = 40 campos) |
| 95–96 | `Proportion{N}PluDiscount`, `Proportion{N}PluDiscountDescription` (2 campos × 8 proporciones = 16 campos) |

**Totales**: 89 campos básicos + 40 precios de proporción + 16 descuentos de proporción
= **145 campos** en el ArticleListDataType completo.

> **Nota:** El código Rust modela una parte de estos campos en
> `BdpArticleData` y el resto se conserva en `extra` (`serde(flatten)`).
> La validación del simulador exige los 89 campos básicos.

## Estructura de la respuesta

```json
{
  "ErrorMessage": "",
  "ListaErroresArticulo": []   /* opcional, lista de campos inválidos */
}
```

- `"ErrorMessage"` vacío = éxito.
- `"ListaErroresArticulo"` con campos ausentes = rechazo.
- `"ErrorMessage"` no vacío = error general (artículo inexistente, token,
  etc.).

## Flujo read-modify-write (Q2.2)

### Arquitectura

```
                    ┌──────────────────┐
                    │   BDP Push cola  │
                    │ payload parcial  │
                    └────────┬─────────┘
                             │
                             ▼
              ┌──────────────────────────────┐
              │  enriquecer_payload_         │
              │  modificar_articulo()        │
              │                              │
              │  1. Deserializar payload     │
              │     → BdpModifyArticleRequest│
              │  2. Extraer ArtCode          │
              │  3. GET /API/Articles/Get    │
              │     → ArticleData completo   │
              │  4. fusionar_article_data()  │
              │     → merge parcial→completo │
              │  5. Preservar profiles/      │
              │     all_profiles originales  │
              └────────────┬────────────────┘
                           │
                           ▼
              ┌──────────────────────────────┐
              │  dispatcher                  │
              │  POST /API/Articles/         │
              │  ModifyAndUpdateProfiles     │
              └──────────────────────────────┘
```

### `enriquecer_payload_modificar_articulo`

**Archivo:** `src/services/bdp_push.rs:805-837`

1. Deserializa `payload` → `BdpModifyArticleRequest`
2. Extrae `ArtCode` del `article_data` parcial
3. Llama a `GET /API/Articles/Get { ArtCode }` vía `client.get_article()`
4. Obtiene el `ArticleData` completo del BDP
5. Fusiona: `fusionar_article_data(completo, parcial)` — sobrescribe en el
   objeto completo solo los campos que el payload parcial aporta
6. Preserva `profiles_list` y `all_profiles` del payload original
   (no se tocan durante el enriquecimiento)

**Si GetArticle falla** (artículo inexistente, timeout, error BDP):
- Se cancela el armado BDP (`BdpWriteGuard::cancelar_armado_push`)
- La fila queda disponible para reintento

### `fusionar_article_data`

**Archivo:** `src/services/bdp_push.rs:840-851`

```rust
fn fusionar_article_data(mut completo: Value, parcial: &Value) -> Result<Value, String>
```

- Itera sobre las keys del objeto `parcial`
- Para cada key, inserta/sobrescribe el valor en `completo`
- Devuelve el objeto completo modificado
- Error si `completo` o `parcial` no son objetos JSON

### Dispatch

**Archivo:** `src/services/bdp_push.rs:739-790`

```rust
match (dominio, operacion) {
    (DOMINIO_ARTICULO, OPERACION_MODIFICAR) => {
        let req: BdpModifyArticleRequest =
            serde_json::from_value(payload.clone()).map_err(mal)?;
        client.modify_article_and_update_profile(&req).await
    }
    // ...
}
```

## Campos: quién los aporta

| Origen | Campos |
|--------|--------|
| **GetArticle (remoto)** | Todo el `ArticleData` completo: 89+ campos del BDP (precios, proporciones, descuentos, flags, perfiles de impresora, IVA, combinados, etc.) |
| **Payload local (patch)** | Solo los campos que la edición local modificó. Normalmente: `ArtCode`, `ArtDescription`, `Price1..5`, `DeptCode`, `DeptDescription`, `WebArticle`, flags de menú/stock, etc. |
| **profiles_list** | Viene del payload original (no se enriquece). Opcional. |
| **all_profiles** | Viene del payload original (no se enriquece). Opcional. |

**Regla de fusión:** El patch local gana sobre el remoto para las keys que
aporte; el resto se conserva del `ArticleData` devuelto por GetArticle.

## Validación del contrato (simulador)

En `server.py:_modify_article` (líneas 455–523):

1. Verifica que `ArticleData` contenga **todos** los 89 campos requeridos
   (lista taxativa).
2. Si falta alguno → `"ListaErroresArticulo": [campos_ausentes]`.
3. Requiere `ProfilesList` o `AllProfiles=true`.
4. Si el artículo no existe en `state.articles` → error.
5. En éxito, hace `update(first)` — merge superficial sobre el artículo
   almacenado.

## Tests de integración

### `flush_modificar_articulo_enriquece_payload_con_get_article`

**Archivo:** `tests/bdp_push.rs` (línea 328+)

- Mockea `GetArticle` → devuelve `{ArtCode, ArtDescription, DeptCode, Price5, WebArticle}`
- Encola payload parcial con solo `{ArtCode, ArtDescription}` + `AllProfiles=false` + `ProfilesList`
- Flush ejecuta:
  1. GetArticle → obtiene `{WebArticle: true, DeptCode: 7, Price5: 42.5}` además de lo enviado
  2. Fusiona → `ArticleData` final tiene `Price5: 42.5` y `DeptCode: 7` del remoto
  3. `ArtDescription: "Editado localmente"` del patch local (gana)
  4. `AllProfiles: false` y `ProfilesList` preservados
- Asserts: sincronizados=1, errores=0, campos fusionados OK

### `flush_modificar_articulo_fallo_get_cancela_armado`

- Mockea `GetArticle` → devuelve `ArticleData: null`
- Verifica que el armado se cancele y la fila quede disponible para reintento

## Referencias en el código

| Archivo | Líneas | Propósito |
|---------|--------|-----------|
| `src/services/bdp_weblink_catalog.rs` | 1330–1370 | `BdpArticleData` — subconjunto modelado |
| `src/services/bdp_weblink_catalog.rs` | 1377–1382 | `BdpModifyArticleRequest` — request struct |
| `src/services/bdp_push.rs` | 617–641 | Llamada a enriquecimiento en `procesar_uno` |
| `src/services/bdp_push.rs` | 739–790 | `dispatch` — ruteo al cliente WebLink |
| `src/services/bdp_push.rs` | 795–837 | `enriquecer_payload_modificar_articulo` |
| `src/services/bdp_push.rs` | 840–851 | `fusionar_article_data` |
| `tools/bdp-weblink-simulator/server.py` | 455–523 | `_modify_article` — contrato validado |
| `tests/bdp_push.rs` | 328–470 | Test de enriquecimiento exitoso |
| `tests/bdp_push.rs` | 472+ | Test de fallo GetArticle |