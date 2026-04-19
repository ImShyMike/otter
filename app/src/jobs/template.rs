use std::pin::Pin;

use sqlx::PgPool;
use tracing::{Instrument, instrument};

pub fn run<'a>(pg: &'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(
        async move {
            // code here

            Ok(())
        }
        .instrument(tracing::info_span!("airbridge_data")),
    )
}
