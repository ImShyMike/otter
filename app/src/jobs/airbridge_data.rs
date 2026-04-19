use std::{pin::Pin, time::Duration};

use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use time::OffsetDateTime;
use tracing::{Instrument, error, info, instrument};

use crate::utils::{
    http,
    serde::{
        deserialize_null_float, deserialize_null_screenshot, deserialize_null_string,
        deserialize_timestamp,
    },
};

const AIRBRIDGE_API_URL: &str =
    "https://api2.hackclub.com/v0.1/Unified%20YSWS%20Projects%20DB/Approved%20Projects";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);
const BATCH_SIZE: usize = 1000;

#[derive(Deserialize)]
struct AirbridgeFields {
    #[serde(
        default,
        rename = "Hours Spent",
        deserialize_with = "deserialize_null_float"
    )]
    hours_spent: Option<f64>,
    #[serde(
        default,
        rename = "Code URL",
        deserialize_with = "deserialize_null_string"
    )]
    code_url: Option<String>,
    #[serde(
        default,
        rename = "Playable URL",
        deserialize_with = "deserialize_null_string"
    )]
    playable_url: Option<String>,
    #[serde(
        default,
        rename = "Approved At",
        deserialize_with = "deserialize_timestamp"
    )]
    approved_at: Option<OffsetDateTime>,
    #[serde(
        default,
        rename = "GitHub Username",
        deserialize_with = "deserialize_null_string"
    )]
    github_username: Option<String>,
    #[serde(
        default,
        rename = "YSWS–Name",
        deserialize_with = "deserialize_null_string"
    )]
    ysws_name: Option<String>,
    #[serde(
        default,
        rename = "Screenshot",
        deserialize_with = "deserialize_null_screenshot"
    )]
    screenshot_url: Option<String>,
}

#[derive(Deserialize)]
struct AirbridgeEntry {
    id: String,
    fields: AirbridgeFields,
}

pub fn run<'a>(pg: &'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(
        async move {
            info!("starting");

            let http_client = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()?;

            let body = fetch_airbridge_data(&http_client).await?;

            let entries: Vec<AirbridgeEntry> = tracing::info_span!("deserialize_entries")
                .in_scope(|| {
                    serde_json::from_str(&body).map_err(|e| {
                        error!("deserialization failed at byte {}: {e}", e.column());
                        e
                    })
                })?;

            let entries_count = entries.len();
            let entries: Vec<AirbridgeEntry> = entries
                .into_iter()
                .filter(|e| e.fields.ysws_name.is_some())
                .collect();

            info!(
                "fetched {} entries from airbridge ({} skipped with null ysws)",
                entries.len(),
                entries_count - entries.len()
            );

            upsert_entries(&entries, pg).await?;
            update_screenshot_urls(&entries, pg).await?;
            soft_delete_missing(&entries, pg).await?;

            info!("done");

            Ok(())
        }
        .instrument(tracing::info_span!("airbridge_data")),
    )
}

#[instrument(skip_all)]
async fn fetch_airbridge_data(http_client: &reqwest::Client) -> anyhow::Result<String> {
    Ok(http::fetch_with_retries(http_client, AIRBRIDGE_API_URL, 3)
        .await?
        .text()
        .await?)
}

#[instrument(skip_all)]
async fn upsert_entries(entries: &[AirbridgeEntry], pg: &PgPool) -> anyhow::Result<()> {
    let mut tx = pg.begin().await?;
    let mut modified: u64 = 0;

    for chunk in entries.chunks(BATCH_SIZE) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO projects (airtable_id, ysws, approved_at, code_url, demo_url, github_username, true_hours) ",
        );

        qb.push_values(chunk, |mut b, entry| {
            b.push_bind(&entry.id)
                .push_bind(entry.fields.ysws_name.as_ref().unwrap())
                .push_bind(entry.fields.approved_at)
                .push_bind(&entry.fields.code_url)
                .push_bind(&entry.fields.playable_url)
                .push_bind(&entry.fields.github_username)
                .push_bind(entry.fields.hours_spent);
        });

        qb.push(
            " ON CONFLICT (airtable_id) DO UPDATE SET \
                ysws = EXCLUDED.ysws, \
                approved_at = EXCLUDED.approved_at, \
                code_url = EXCLUDED.code_url, \
                demo_url = EXCLUDED.demo_url, \
                github_username = EXCLUDED.github_username, \
                true_hours = EXCLUDED.true_hours, \
                deleted_at = NULL \
                WHERE projects.ysws IS DISTINCT FROM EXCLUDED.ysws \
                OR projects.approved_at IS DISTINCT FROM EXCLUDED.approved_at \
                OR projects.code_url IS DISTINCT FROM EXCLUDED.code_url \
                OR projects.demo_url IS DISTINCT FROM EXCLUDED.demo_url \
                OR projects.github_username IS DISTINCT FROM EXCLUDED.github_username \
                OR projects.true_hours IS DISTINCT FROM EXCLUDED.true_hours \
                OR projects.deleted_at IS NOT NULL",
        );

        let result = qb.build().execute(&mut *tx).await?;
        modified += result.rows_affected();
    }

    tx.commit().await?;
    info!("upserted {} entries ({} modified)", entries.len(), modified);

    Ok(())
}

#[instrument(skip_all)]
async fn update_screenshot_urls(entries: &[AirbridgeEntry], pg: &PgPool) -> anyhow::Result<()> {
    let mut tx = pg.begin().await?;
    let mut urls_updated = 0;

    for chunk in entries.chunks(BATCH_SIZE) {
        let ids: Vec<&str> = chunk.iter().map(|e| e.id.as_str()).collect();
        let urls: Vec<Option<&str>> = chunk
            .iter()
            .map(|e| e.fields.screenshot_url.as_deref())
            .collect();

        let result = sqlx::query(
            "UPDATE projects SET screenshot_url = data.screenshot_url \
                FROM UNNEST($1::text[], $2::text[]) AS data(airtable_id, screenshot_url) \
                WHERE projects.airtable_id = data.airtable_id \
                AND projects.screenshot_url IS DISTINCT FROM data.screenshot_url",
        )
        .bind(&ids)
        .bind(&urls)
        .execute(&mut *tx)
        .await?;
        urls_updated += result.rows_affected();
    }

    tx.commit().await?;
    info!("updated screenshot URLs for {} entries", urls_updated);

    Ok(())
}

#[instrument(skip_all)]
async fn soft_delete_missing(entries: &[AirbridgeEntry], pg: &PgPool) -> anyhow::Result<()> {
    let airtable_ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    let deleted = sqlx::query_scalar!(
        "UPDATE projects SET deleted_at = NOW() WHERE airtable_id != ALL($1) AND deleted_at IS NULL RETURNING 1 as count",
        &airtable_ids as &[&str]
    )
    .fetch_all(pg)
    .await?;

    if !deleted.is_empty() {
        info!("soft-deleted {} missing projects", deleted.len());
    }

    Ok(())
}
