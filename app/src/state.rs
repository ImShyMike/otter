use deadpool_redis::Pool;
use sqlx::PgPool;
use tokio_cron_scheduler::JobScheduler;

#[derive(Clone)]
pub struct AppState {
    pub pg: PgPool,
    pub redis: Pool,
    pub _scheduler: JobScheduler,
}
