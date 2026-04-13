use std::pin::Pin;

use sqlx::PgPool;

pub fn run<'a>(_pg: &'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        // TODO: actually add a job here
        Ok(())
    })
}
