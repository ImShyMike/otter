mod error;
mod handlers;
mod jobs;
mod routes;
mod state;

use std::env;

use deadpool_redis::{Config, Runtime};
use sqlx::PgPool;

use jobs::JobKind;
use state::AppState;

const DEFAULT_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/otter";
const DEFAULT_REDIS_URL: &str = "redis://localhost:6379";
const DEFAULT_HOST: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url = env::var("DATABASE_URL").unwrap_or(DEFAULT_DATABASE_URL.to_string());
    let redis_url = env::var("REDIS_URL").unwrap_or(DEFAULT_REDIS_URL.to_string());

    let pg = PgPool::connect(&database_url).await?;

    let cfg = Config::from_url(&redis_url);
    let redis = cfg.create_pool(Some(Runtime::Tokio1))?;

    sqlx::migrate!("./migrations").run(&pg).await?;

    jobs::schedule_all(&pg).await?;

    let pg_startup = pg.clone();
    tokio::spawn(async move {
        if let Err(e) = jobs::run_job(&pg_startup, JobKind::FetchData).await {
            tracing::error!("startup fetch_data failed: {e}");
        }
    });

    let state = AppState { pg, redis };
    let app = routes::build().with_state(state);

    let listener = tokio::net::TcpListener::bind(DEFAULT_HOST).await?;
    tracing::info!("listening on http://{DEFAULT_HOST}");
    axum::serve(listener, app).await?;

    Ok(())
}
