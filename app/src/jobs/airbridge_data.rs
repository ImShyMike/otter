use std::{collections::HashMap, fmt::Write as _, pin::Pin, time::Duration};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tracing::{Instrument, error, info, instrument};

use crate::utils::{
    http,
    serde::{deserialize_null_float, deserialize_null_string, deserialize_timestamp},
};

const AIRBRIDGE_API_URL: &str =
    "https://api2.hackclub.com/v0.1/Unified%20YSWS%20Projects%20DB/Approved%20Projects";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);
const PREVIEW_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const BATCH_SIZE: usize = 1000;
const PREVIEW_BATCH_SIZE: usize = 32;
const PREVIEW_MAX_BYTES: usize = 5 * 1024 * 1024;
const PREVIEW_SIZE_PX: u32 = 32;
const BLURHASH_X_COMPONENTS: u32 = 4;
const BLURHASH_Y_COMPONENTS: u32 = 3;

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

#[derive(Deserialize, Clone)]
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

#[derive(Deserialize, Clone)]
pub struct AirtableThumbnails {
    #[serde(default)]
    pub small: Option<AirtableThumbnail>,
    #[serde(default)]
    pub large: Option<AirtableThumbnail>,
    #[serde(default)]
    pub full: Option<AirtableThumbnail>,
}

#[derive(Deserialize, Clone)]
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

struct PreviewState {
    id: i32,
    preview_blurhash_source_key: Option<String>,
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
    let rows = sqlx::query(
        "SELECT id, airtable_id, preview_blurhash_source_key FROM projects WHERE airtable_id = ANY($1)",
    )
    .bind(&airtable_ids)
    .fetch_all(pg)
    .await?;

    let project_state_by_airtable: HashMap<String, PreviewState> = rows
        .into_iter()
        .map(|r| {
            let airtable_id: String = r.get("airtable_id");
            (
                airtable_id,
                PreviewState {
                    id: r.get("id"),
                    preview_blurhash_source_key: r.get("preview_blurhash_source_key"),
                },
            )
        })
        .collect();

    let preview_http_client = reqwest::Client::builder()
        .timeout(PREVIEW_REQUEST_TIMEOUT)
        .build()?;

    let mut project_previews: Vec<(i32, String, Option<String>)> =
        Vec::with_capacity(entries.len());
    let preview_batch_count = entries.len().div_ceil(PREVIEW_BATCH_SIZE);
    info!(
        projects = entries.len(),
        batches = preview_batch_count,
        "syncing media previews"
    );

    for (batch_idx, chunk) in entries.chunks(PREVIEW_BATCH_SIZE).enumerate() {
        // calculate which projects changed their media
        let mut to_process: Vec<(i32, String, Vec<AirtableAttachment>)> = Vec::new();
        for entry in chunk {
            if let Some(state) = project_state_by_airtable.get(&entry.id) {
                let source_key = preview_source_key(&entry.fields.screenshots);
                if state.preview_blurhash_source_key.as_deref() == Some(source_key.as_str()) {
                    // no change, skip
                    continue;
                }

                // record for processing
                to_process.push((state.id, source_key, entry.fields.screenshots.clone()));
            }
        }

        if to_process.is_empty() {
            continue;
        }

        let mut tasks = JoinSet::new();

        info!(
            batch = batch_idx + 1,
            total_batches = preview_batch_count,
            batch_projects = to_process.len(),
            "processing media preview batch"
        );

        // compue blurhashes in parallel
        for (project_id, source_key, screenshots) in to_process {
            if screenshots.is_empty() {
                project_previews.push((project_id, source_key, None));
                continue;
            }

            let http_client = preview_http_client.clone();
            tasks.spawn(async move {
                let mut preview_blurhash = None;
                for attachment in &screenshots {
                    preview_blurhash = compute_attachment_blurhash(&http_client, attachment).await;
                    if preview_blurhash.is_some() {
                        break;
                    }
                }

                (project_id, source_key, preview_blurhash)
            });
        }

        while let Some(result) = tasks.join_next().await {
            project_previews.push(result?);
        }
    }

    let mut tx = pg.begin().await?;
    let mut upserted: u64 = 0;
    let mut deleted: u64 = 0;

    for chunk in entries.chunks(BATCH_SIZE) {
        let media_rows: Vec<(i32, &AirtableAttachment)> = chunk
            .iter()
            .filter_map(|entry| {
                project_state_by_airtable
                    .get(&entry.id)
                    .map(|state| (state.id, entry))
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
        let Some(state) = project_state_by_airtable.get(&entry.id) else {
            continue;
        };
        let pid = state.id;
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

        // update project preview_blurhash for projects that had media changes
        for preview_chunk in project_previews.chunks(BATCH_SIZE) {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                "UPDATE projects AS p SET preview_blurhash = v.preview_blurhash, preview_blurhash_source_key = v.preview_blurhash_source_key FROM (",
            );
            qb.push_values(
                preview_chunk,
                |mut b, (project_id, preview_blurhash_source_key, preview_blurhash)| {
                    b.push_bind(*project_id)
                        .push_bind(preview_blurhash_source_key)
                        .push_bind(preview_blurhash);
                },
            );
            qb.push(
                ") AS v(id, preview_blurhash_source_key, preview_blurhash) WHERE p.id = v.id AND p.deleted_at IS NULL",
            );
            qb.build().execute(&mut *tx).await?;
        }
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

/// small -> large -> full -> original
fn preferred_preview_url(att: &AirtableAttachment) -> &str {
    att.thumbnails
        .as_ref()
        .and_then(|thumbs| thumbs.small.as_ref())
        .map(|thumb| thumb.url.as_str())
        .or_else(|| {
            att.thumbnails
                .as_ref()
                .and_then(|thumbs| thumbs.large.as_ref())
                .map(|thumb| thumb.url.as_str())
        })
        .or_else(|| {
            att.thumbnails
                .as_ref()
                .and_then(|thumbs| thumbs.full.as_ref())
                .map(|thumb| thumb.url.as_str())
        })
        .unwrap_or(att.url.as_str())
}

async fn compute_attachment_blurhash(
    http_client: &reqwest::Client,
    att: &AirtableAttachment,
) -> Option<String> {
    if let Some(mime) = att.mime_type.as_deref()
        && !is_preview_compatible_mime(mime)
    {
        return None;
    }

    let url = preferred_preview_url(att);
    compute_blurhash_from_url(http_client, url).await
}

fn is_preview_compatible_mime(mime: &str) -> bool {
    mime.starts_with("image/") || mime.starts_with("video/")
}

async fn compute_blurhash_from_url(http_client: &reqwest::Client, url: &str) -> Option<String> {
    let response = http::fetch_with_retries(http_client, url, 2).await.ok()?;
    let bytes = response.bytes().await.ok()?;
    if bytes.len() > PREVIEW_MAX_BYTES {
        return None;
    }

    let thumbnail = image::load_from_memory(&bytes)
        .ok()?
        .thumbnail(PREVIEW_SIZE_PX, PREVIEW_SIZE_PX)
        .to_rgba8();
    let (width, height) = thumbnail.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    blurhash::encode(
        BLURHASH_X_COMPONENTS,
        BLURHASH_Y_COMPONENTS,
        width,
        height,
        &thumbnail,
    )
    .ok()
}

fn preview_source_key(screenshots: &[AirtableAttachment]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((screenshots.len() as u64).to_le_bytes());

    for screenshot in screenshots {
        hash_string(&mut hasher, &screenshot.id);
        hash_option_i64(&mut hasher, screenshot.size);
    }

    to_hex(hasher.finalize())
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hasher.update([0x01]);
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn hash_option_i64(hasher: &mut Sha256, value: Option<i64>) {
    match value {
        Some(value) => {
            hasher.update([0x01]);
            hasher.update(value.to_le_bytes());
        }
        None => hasher.update([0x00]),
    }
}

fn to_hex(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }

    output
}
