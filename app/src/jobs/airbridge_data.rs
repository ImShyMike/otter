use std::{collections::HashMap, pin::Pin, time::Duration};

use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use time::OffsetDateTime;
use tracing::{Instrument, error, info, instrument};

use crate::utils::{
    http,
    serde::{deserialize_null_float, deserialize_null_string, deserialize_timestamp},
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
    #[serde(default, rename = "Screenshot")]
    screenshots: Vec<AirtableAttachment>,
}

#[derive(Deserialize)]
pub struct AirtableAttachment {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default, rename = "type")]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub size: Option<i64>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub thumbnails: Option<AirtableThumbnails>,
}

#[derive(Deserialize)]
pub struct AirtableThumbnails {
    #[serde(default)]
    pub small: Option<AirtableThumbnail>,
    #[serde(default)]
    pub large: Option<AirtableThumbnail>,
    #[serde(default)]
    pub full: Option<AirtableThumbnail>,
}

#[derive(Deserialize)]
pub struct AirtableThumbnail {
    pub url: String,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
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
            sync_media(&entries, pg).await?;
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
                .push_bind(entry.fields.approved_at.map(|t| t.unix_timestamp()))
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
async fn sync_media(entries: &[AirbridgeEntry], pg: &PgPool) -> anyhow::Result<()> {
    let airtable_ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    let rows = sqlx::query!(
        "SELECT id, airtable_id FROM projects WHERE airtable_id = ANY($1)",
        &airtable_ids as &[&str]
    )
    .fetch_all(pg)
    .await?;
    let project_id_by_airtable: HashMap<String, i32> =
        rows.into_iter().map(|r| (r.airtable_id, r.id)).collect();

    let mut tx = pg.begin().await?;
    let mut upserted: u64 = 0;
    let mut deleted: u64 = 0;

    for chunk in entries.chunks(BATCH_SIZE) {
        let media_rows: Vec<(i32, &AirtableAttachment)> = chunk
            .iter()
            .filter_map(|entry| {
                project_id_by_airtable
                    .get(&entry.id)
                    .copied()
                    .map(|pid| (pid, entry))
            })
            .flat_map(|(pid, entry)| entry.fields.screenshots.iter().map(move |a| (pid, a)))
            .collect();

        if media_rows.is_empty() {
            continue;
        }

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO media (project_id, airtable_id, filename, mime_type, size_bytes, \
             width, height, url, \
             thumb_small_url, thumb_small_width, thumb_small_height, \
             thumb_large_url, thumb_large_width, thumb_large_height, \
             thumb_full_url, thumb_full_width, thumb_full_height) ",
        );

        qb.push_values(media_rows.iter(), |mut b, (pid, att)| {
            let small = att.thumbnails.as_ref().and_then(|t| t.small.as_ref());
            let large = att.thumbnails.as_ref().and_then(|t| t.large.as_ref());
            let full = att.thumbnails.as_ref().and_then(|t| t.full.as_ref());

            b.push_bind(*pid)
                .push_bind(&att.id)
                .push_bind(&att.filename)
                .push_bind(
                    att.mime_type
                        .as_deref()
                        .unwrap_or("application/octet-stream"),
                )
                .push_bind(att.size)
                .push_bind(att.width)
                .push_bind(att.height)
                .push_bind(&att.url)
                .push_bind(small.map(|t| t.url.as_str()))
                .push_bind(small.and_then(|t| t.width))
                .push_bind(small.and_then(|t| t.height))
                .push_bind(large.map(|t| t.url.as_str()))
                .push_bind(large.and_then(|t| t.width))
                .push_bind(large.and_then(|t| t.height))
                .push_bind(full.map(|t| t.url.as_str()))
                .push_bind(full.and_then(|t| t.width))
                .push_bind(full.and_then(|t| t.height));
        });

        qb.push(
            " ON CONFLICT (project_id, airtable_id) DO UPDATE SET \
                filename = EXCLUDED.filename, \
                mime_type = EXCLUDED.mime_type, \
                size_bytes = EXCLUDED.size_bytes, \
                width = EXCLUDED.width, \
                height = EXCLUDED.height, \
                url = EXCLUDED.url, \
                thumb_small_url = EXCLUDED.thumb_small_url, \
                thumb_small_width = EXCLUDED.thumb_small_width, \
                thumb_small_height = EXCLUDED.thumb_small_height, \
                thumb_large_url = EXCLUDED.thumb_large_url, \
                thumb_large_width = EXCLUDED.thumb_large_width, \
                thumb_large_height = EXCLUDED.thumb_large_height, \
                thumb_full_url = EXCLUDED.thumb_full_url, \
                thumb_full_width = EXCLUDED.thumb_full_width, \
                thumb_full_height = EXCLUDED.thumb_full_height, \
                updated_at = NOW() \
                WHERE media.url IS DISTINCT FROM EXCLUDED.url \
                OR media.filename IS DISTINCT FROM EXCLUDED.filename \
                OR media.mime_type IS DISTINCT FROM EXCLUDED.mime_type \
                OR media.size_bytes IS DISTINCT FROM EXCLUDED.size_bytes \
                OR media.width IS DISTINCT FROM EXCLUDED.width \
                OR media.height IS DISTINCT FROM EXCLUDED.height \
                OR media.thumb_small_url IS DISTINCT FROM EXCLUDED.thumb_small_url \
                OR media.thumb_large_url IS DISTINCT FROM EXCLUDED.thumb_large_url \
                OR media.thumb_full_url IS DISTINCT FROM EXCLUDED.thumb_full_url",
        );

        upserted += qb.build().execute(&mut *tx).await?.rows_affected();
    }

    let mut keep_project_ids: Vec<i32> = Vec::new();
    let mut keep_airtable_ids: Vec<&str> = Vec::new();
    let mut synced_project_ids: Vec<i32> = Vec::with_capacity(entries.len());

    for entry in entries {
        let Some(&pid) = project_id_by_airtable.get(&entry.id) else {
            continue;
        };
        synced_project_ids.push(pid);
        for att in &entry.fields.screenshots {
            keep_project_ids.push(pid);
            keep_airtable_ids.push(att.id.as_str());
        }
    }

    if !synced_project_ids.is_empty() {
        let result = sqlx::query(
            "DELETE FROM media m \
             WHERE m.project_id = ANY($1) \
               AND NOT EXISTS ( \
                   SELECT 1 FROM UNNEST($2::int[], $3::text[]) AS keep(project_id, airtable_id) \
                   WHERE keep.project_id = m.project_id AND keep.airtable_id = m.airtable_id \
               )",
        )
        .bind(&synced_project_ids)
        .bind(&keep_project_ids)
        .bind(&keep_airtable_ids)
        .execute(&mut *tx)
        .await?;
        deleted = result.rows_affected();
    }

    tx.commit().await?;
    info!(
        "synced media: {} upserted, {} deleted (across {} projects)",
        upserted,
        deleted,
        synced_project_ids.len()
    );

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
