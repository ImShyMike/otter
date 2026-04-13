use deadpool_redis::Pool;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pg: PgPool,
    pub redis: Pool,
}
