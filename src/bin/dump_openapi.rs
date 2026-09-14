/* Utilidad para generar el schema OpenAPI sin necesitar base de datos.
 * Uso: cargo run --bin dump_openapi > frontend/openapi-debug.json */

use glory_backend::handlers::ApiDoc;
use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    /* La utilidad propaga el fallo de serializacion con `?` en lugar de entrar en
     * panico, para que el error salga por stderr y el codigo de salida lo delate. */
    print!("{}", ApiDoc::openapi().to_json()?);
    Ok(())
}
