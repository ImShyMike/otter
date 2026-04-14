mod fetch_data;

use std::pin::Pin;

use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};

type JobFn =
    for<'a> fn(&'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

/// Job list
pub enum JobKind {
    FetchData,
}

impl JobKind {
    const ALL: &[JobKind] = &[JobKind::FetchData];

    fn lock_id(&self) -> i64 {
        match self {
            JobKind::FetchData => 1,
        }
    }

    fn cron(&self) -> &'static str {
        match self {
            JobKind::FetchData => "0 0 */3 * * *",
        }
    }

    fn run_fn(&self) -> JobFn {
        match self {
            JobKind::FetchData => fetch_data::run,
        }
    }
}

/// Run a specific job
pub async fn run_job(pg: &PgPool, job: JobKind) -> anyhow::Result<()> {
    with_lock(pg, job.lock_id(), job.run_fn()).await
}

/// Registers all scheduled jobs and starts the cron scheduler
pub async fn schedule_all(pg: &PgPool) -> anyhow::Result<()> {
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
    Ok(())
}

async fn with_lock(pg: &PgPool, lock_id: i64, f: JobFn) -> anyhow::Result<()> {
    let mut tx = pg.begin().await?;

    let acquired = sqlx::query_scalar!("SELECT pg_try_advisory_xact_lock($1)", lock_id)
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);

    if !acquired {
        return Ok(());
    }

    f(pg).await?;

    tx.commit().await?;
    Ok(())
}
