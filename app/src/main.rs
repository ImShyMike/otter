mod error;
mod handlers;
mod jobs;
mod routes;
mod state;
mod telemetry;
mod utils;

use std::env;
use std::str::FromStr;
use std::time::Duration;

use deadpool_redis::{Config, Runtime};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use jobs::JobKind;
use state::AppState;

const DEFAULT_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/otter";
const DEFAULT_REDIS_URL: &str = "redis://localhost:6379";
const DEFAULT_HOST: &str = "0.0.0.0:3000";

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

utoipa_ts::export!("../frontend/src/lib/types.ts");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    telemetry::init();

    let database_url = env::var("DATABASE_URL").unwrap_or(DEFAULT_DATABASE_URL.to_string());
    let redis_url = env::var("REDIS_URL").unwrap_or(DEFAULT_REDIS_URL.to_string());
    let host = env::var("HOST").unwrap_or(DEFAULT_HOST.to_string());

    let pg_options = PgConnectOptions::from_str(&database_url)?.statement_cache_capacity(32);
    let pg = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET jit = off").execute(&mut *conn).await?;
                Ok(())
            })
        })
        .connect_with(pg_options)
        .await?;

    let cfg = Config::from_url(&redis_url);
    let redis = cfg.create_pool(Some(Runtime::Tokio1))?;

    sqlx::migrate!("./migrations").run(&pg).await?;

    let scheduler = jobs::schedule_all(&pg).await?;

    let run_jobs_on_startup = env::var("RUN_JOBS_ON_STARTUP")
        .map(|v| v == "true")
        .unwrap_or(true);

    if run_jobs_on_startup {
        let pg_startup = pg.clone();
        tokio::spawn(async move {
            let startup_jobs = [
                JobKind::ShipsData,
                JobKind::AirbridgeData,
                JobKind::FinesData,
                JobKind::SlackData,
            ];
            for job in startup_jobs {
                let job_name = format!("{job:?}").to_lowercase();
                if let Err(e) = jobs::run_job(&pg_startup, job).await {
                    tracing::error!("startup {} failed: {e}", job_name);
                }
            }
        });
    }

    let state = AppState {
        pg,
        redis,
        _scheduler: scheduler,
    };
    let app = routes::build().with_state(state);

    let listener = tokio::net::TcpListener::bind(&host).await?;
    tracing::info!("listening on http://{host}");
    axum::serve(listener, app).await?;

    Ok(())
}
