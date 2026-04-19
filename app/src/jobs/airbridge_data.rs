use std::{pin::Pin, time::Duration};

use serde::Deserialize;
use sqlx::PgPool;
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
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const BATCH_SIZE: usize = 1000;

#[derive(Deserialize)]
struct AirbridgeFields {
    #[serde(rename = "Record ID")]
    record_id: String,
    #[serde(
        default,
        rename = "Hours Spent",
        deserialize_with = "deserialize_null_float"
    )]
    hours_spent: Option<f64>,
    #[serde(rename = "Code URL", deserialize_with = "deserialize_null_string")]
    code_url: Option<String>,
    #[serde(rename = "Playable URL", deserialize_with = "deserialize_null_string")]
    playable_url: Option<String>,
    #[serde(rename = "Approved At", deserialize_with = "deserialize_timestamp")]
    approved_at: Option<OffsetDateTime>,
    #[serde(
        rename = "GitHub Username",
        deserialize_with = "deserialize_null_string"
    )]
    github_username: Option<String>,
    #[serde(rename = "YSWS–Name", deserialize_with = "deserialize_null_string")]
    ysws_name: Option<String>,
    #[serde(
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

            update_true_hours(&http_client, pg).await?;

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
async fn update_true_hours_entries(
    entries: Vec<AirbridgeEntry>,
    pg: &PgPool,
) -> anyhow::Result<()> {
    let mut tx = pg.begin().await?;
    let mut hours_updated = 0;

    for chunk in entries.chunks(BATCH_SIZE) {
        let ids: Vec<&str> = chunk.iter().map(|e| e.id.as_str()).collect();
        let hours: Vec<Option<f64>> = chunk.iter().map(|e| e.fields.hours_spent).collect();

        let result = sqlx::query(
            "UPDATE projects SET true_hours = data.hours_spent \
             FROM UNNEST($1::text[], $2::float8[]) AS data(airtable_id, hours_spent) \
             WHERE projects.airtable_id = data.airtable_id \
             AND projects.true_hours IS DISTINCT FROM data.hours_spent",
        )
        .bind(&ids)
        .bind(&hours)
        .execute(&mut *tx)
        .await?;

        hours_updated += result.rows_affected();
    }

    tx.commit().await?;

    info!("updated true hours for {} entries", hours_updated);

    Ok(())
}

#[instrument(skip_all)]
async fn update_true_hours(http_client: &reqwest::Client, pg: &PgPool) -> anyhow::Result<()> {
    info!("updating true hours using airbridge");

    let body = fetch_airbridge_data(http_client).await?;

    let entries: Vec<AirbridgeEntry> =
        tracing::info_span!("deserialize_entries").in_scope(|| {
            serde_json::from_str(&body).map_err(|e| {
                error!("deserialization failed at byte {}: {e}", e.column());
                e
            })
        })?;

    info!("fetched {} entries from airbridge", entries.len());

    update_true_hours_entries(entries, pg).await?;

    Ok(())
}
