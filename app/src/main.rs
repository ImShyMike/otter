mod error;
mod handlers;
mod jobs;
mod routes;
mod state;
mod telemetry;
mod utils;

use std::env;

use deadpool_redis::{Config, Runtime};
use sqlx::postgres::PgPoolOptions;

use jobs::JobKind;
use state::AppState;

const DEFAULT_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/otter";
const DEFAULT_REDIS_URL: &str = "redis://localhost:6379";
const DEFAULT_HOST: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    telemetry::init();

    let database_url = env::var("DATABASE_URL").unwrap_or(DEFAULT_DATABASE_URL.to_string());
    let redis_url = env::var("REDIS_URL").unwrap_or(DEFAULT_REDIS_URL.to_string());
    let host = env::var("HOST").unwrap_or(DEFAULT_HOST.to_string());

    let pg = PgPoolOptions::new()
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET jit = off").execute(&mut *conn).await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await?;

    let cfg = Config::from_url(&redis_url);
    let redis = cfg.create_pool(Some(Runtime::Tokio1))?;

    sqlx::migrate!("./migrations").run(&pg).await?;

    jobs::schedule_all(&pg).await?;

    let pg_startup = pg.clone();
    tokio::spawn(async move {
        if let Err(e) = jobs::run_job(&pg_startup, JobKind::ShipsData).await {
            tracing::error!("startup fetch_data failed: {e}");
        }
    });
    let pg_startup = pg.clone();
    tokio::spawn(async move {
        if let Err(e) = jobs::run_job(&pg_startup, JobKind::AirbridgeData).await {
            tracing::error!("startup airbridge_data failed: {e}");
        }
    });

    let state = AppState { pg, redis };
    let app = routes::build().with_state(state);

    let listener = tokio::net::TcpListener::bind(&host).await?;
    tracing::info!("listening on http://{host}");
    axum::serve(listener, app).await?;

    Ok(())
}
