mod airbridge_data;
mod fines_data;
mod ships_data;
mod slack_data;

use std::pin::Pin;

use sqlx::postgres::PgConnection;
use sqlx::{Connection, PgPool};
use time::OffsetDateTime;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::handlers::api::status::set_last_refreshed_at;

type JobFn =
    for<'a> fn(&'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

/// Job list
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum JobKind {
    ShipsData,
    AirbridgeData,
    FinesData,
    SlackData,
}

impl JobKind {
    const ALL: &[JobKind] = &[
        JobKind::ShipsData,
        JobKind::AirbridgeData,
        JobKind::FinesData,
        JobKind::SlackData,
    ];

    fn lock_id(&self) -> i64 {
        match self {
            JobKind::ShipsData => 1,
            JobKind::AirbridgeData => 2,
            JobKind::FinesData => 3,
            JobKind::SlackData => 4,
        }
    }

    fn cron(&self) -> &'static str {
        match self {
            JobKind::ShipsData => "0 0 */3 * * *",
            JobKind::AirbridgeData => "0 10 */1 * * *",
            JobKind::FinesData => "0 20 */1 * * *",
            JobKind::SlackData => "0 30 */6 * * *",
        }
    }

    fn run_fn(&self) -> JobFn {
        match self {
            JobKind::ShipsData => ships_data::run,
            JobKind::AirbridgeData => airbridge_data::run,
            JobKind::FinesData => fines_data::run,
            JobKind::SlackData => slack_data::run,
        }
    }
}

/// Run a specific job
pub async fn run_job(pg: &PgPool, job: JobKind) -> anyhow::Result<()> {
    with_lock(pg, job.lock_id(), job.run_fn()).await
}

/// Registers all scheduled jobs and starts the cron scheduler
pub async fn schedule_all(pg: &PgPool) -> anyhow::Result<JobScheduler> {
    let sched = JobScheduler::new().await?;

    for kind in JobKind::ALL {
        let pg = pg.clone();
        let lock_id = kind.lock_id();
        let f = kind.run_fn();
        sched
            .add(Job::new_async(kind.cron(), move |_uuid, _lock| {
                let pg = pg.clone();
                Box::pin(async move {
                    if let Err(e) = with_lock(&pg, lock_id, f).await {
                        tracing::error!("job failed: {e}");
                    }
                })
            })?)
            .await?;
    }

    sched.start().await?;
    Ok(sched)
}

async fn with_lock(pg: &PgPool, lock_id: i64, f: JobFn) -> anyhow::Result<()> {
    let connect_options = pg.connect_options();
    let mut lock_conn = PgConnection::connect_with(connect_options.as_ref()).await?;
    sqlx::query("SET jit = off").execute(&mut lock_conn).await?;

    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(lock_id)
        .fetch_one(&mut lock_conn)
        .await?;

    if !acquired {
        return Ok(());
    }

    let result = f(pg).await;

    if result.is_ok() && matches!(lock_id, 1 | 2) {
        set_last_refreshed_at(OffsetDateTime::now_utc()).await;
    }

    let unlock_result: anyhow::Result<bool> = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
        .bind(lock_id)
        .fetch_one(&mut lock_conn)
        .await
        .map_err(Into::into);

    result?;
    if !unlock_result? {
        anyhow::bail!("failed to release advisory lock {lock_id}");
    }

    Ok(())
}
