mod cleanup;

use std::pin::Pin;

use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Job to lock ID mapping
#[repr(i64)]
enum LockId {
    Cleanup = 1,
}

type JobFn =
    for<'a> fn(&'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

/// Registers all jobs and starts the scheduler
pub async fn schedule_all(pg: &PgPool) -> anyhow::Result<()> {
    let sched = JobScheduler::new().await?;

    register(&sched, pg, "0 0 */3 * * *", LockId::Cleanup, cleanup::run).await?;

    sched.start().await?;
    Ok(())
}

async fn register(
    sched: &JobScheduler,
    pg: &PgPool,
    cron: &str,
    lock_id: LockId,
    f: JobFn,
) -> anyhow::Result<()> {
    let pg = pg.clone();
    let lock = lock_id as i64;
    sched
        .add(Job::new_async(cron, move |_uuid, _lock| {
            let pg = pg.clone();
            Box::pin(async move {
                if let Err(e) = with_lock(&pg, lock, f).await {
                    eprintln!("job failed: {e}");
                }
            })
        })?)
        .await?;
    Ok(())
}

async fn with_lock(pg: &PgPool, lock_id: i64, f: JobFn) -> anyhow::Result<()> {
    let mut tx = pg.begin().await?;

    let (acquired,): (bool,) = sqlx::query_as("SELECT pg_try_advisory_xact_lock($1)")
        .bind(lock_id)
        .fetch_one(&mut *tx)
        .await?;

    if !acquired {
        return Ok(());
    }

    f(pg).await?;

    tx.commit().await?;
    Ok(())
}
