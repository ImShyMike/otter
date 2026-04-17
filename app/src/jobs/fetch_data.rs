use std::{pin::Pin, time::Duration};

use pgvector::Vector;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use time::OffsetDateTime;
use tracing::{error, info};

use crate::utils::serde::{
    deserialize_null_float, deserialize_null_int, deserialize_null_string, deserialize_timestamp,
};
use crate::utils::{embeddings, http};

const SHIPS_API_URL: &str = "https://ships.hackclub.com/api/v1/ysws_entries?all=true";
const AIRBRIDGE_API_URL: &str = "https://api2.hackclub.com/v0.1/Unified%20YSWS%20Projects%20DB/Approved%20Projects?select=%7B%22fields%22%3A%5B%22Hours%20Spent%22%5D%7D";
const BATCH_SIZE: usize = 1000;
const EMBED_BATCH_SIZE: usize = 128;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const MIN_DESCRIPTION_SIZE: i32 = 50;

#[derive(Deserialize)]
struct YswsEntry {
    id: String,
    ysws: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    approved_at: Option<OffsetDateTime>,
    #[serde(deserialize_with = "deserialize_null_string")]
    code_url: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    country: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    demo_url: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    description: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    github_username: Option<String>,
    #[serde(deserialize_with = "deserialize_null_int")]
    hours: Option<i32>,
    #[serde(deserialize_with = "deserialize_null_string")]
    screenshot_url: Option<String>,
    #[serde(default)]
    github_stars: i32,
    #[serde(deserialize_with = "deserialize_null_string")]
    display_name: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    archived_demo: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    archived_repo: Option<String>,
}

#[derive(Deserialize)]
struct AirbridgeEntry {
    id: String,
    fields: AirbridgeFields,
}

#[derive(Deserialize)]
struct AirbridgeFields {
    #[serde(
        default,
        rename = "Hours Spent",
        deserialize_with = "deserialize_null_float"
    )]
    hours_spent: Option<f64>,
}

#[derive(sqlx::FromRow)]
struct EmbedRow {
    id: i32,
    display_name: Option<String>,
    description: Option<String>,
}

pub fn run<'a>(pg: &'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        info!("starting");

        let http_client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        update_data(&http_client, pg).await?;

        update_true_hours(&http_client, pg).await?;

        embed_new_projects(pg).await?;

        info!("done");

        Ok(())
    })
}

async fn update_data(http_client: &reqwest::Client, pg: &PgPool) -> anyhow::Result<()> {
    let body = http::fetch_with_retries(http_client, SHIPS_API_URL, 3)
        .await?
        .text()
        .await?;

    info!("fetched data from API, deserializing");

    let entries: Vec<YswsEntry> = serde_json::from_str(&body).map_err(|e| {
        error!("deserialization failed at byte {}: {e}", e.column());
        e
    })?;

    info!("fetched {} entries", entries.len());

    let mut tx = pg.begin().await?;
    let mut modified: u64 = 0;

    for chunk in entries.chunks(BATCH_SIZE) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO projects (airtable_id, ysws, approved_at, code_url, country, demo_url, description, github_username, hours, screenshot_url, github_stars, display_name, archived_demo, archived_repo) ",
        );

        qb.push_values(chunk, |mut b, entry| {
            b.push_bind(&entry.id)
                .push_bind(&entry.ysws)
                .push_bind(entry.approved_at)
                .push_bind(&entry.code_url)
                .push_bind(&entry.country)
                .push_bind(&entry.demo_url)
                .push_bind(&entry.description)
                .push_bind(&entry.github_username)
                .push_bind(entry.hours)
                .push_bind(&entry.screenshot_url)
                .push_bind(entry.github_stars)
                .push_bind(&entry.display_name)
                .push_bind(&entry.archived_demo)
                .push_bind(&entry.archived_repo);
        });

        qb.push(
            " ON CONFLICT (airtable_id) DO UPDATE SET \
                ysws = EXCLUDED.ysws, \
                approved_at = EXCLUDED.approved_at, \
                code_url = EXCLUDED.code_url, \
                country = EXCLUDED.country, \
                demo_url = EXCLUDED.demo_url, \
                description = EXCLUDED.description, \
                github_username = EXCLUDED.github_username, \
                hours = EXCLUDED.hours, \
                github_stars = EXCLUDED.github_stars, \
                display_name = EXCLUDED.display_name, \
                archived_demo = EXCLUDED.archived_demo, \
                archived_repo = EXCLUDED.archived_repo, \
                deleted_at = NULL \
                WHERE projects.ysws IS DISTINCT FROM EXCLUDED.ysws \
                OR projects.approved_at IS DISTINCT FROM EXCLUDED.approved_at \
                OR projects.code_url IS DISTINCT FROM EXCLUDED.code_url \
                OR projects.country IS DISTINCT FROM EXCLUDED.country \
                OR projects.demo_url IS DISTINCT FROM EXCLUDED.demo_url \
                OR projects.description IS DISTINCT FROM EXCLUDED.description \
                OR projects.github_username IS DISTINCT FROM EXCLUDED.github_username \
                OR projects.hours IS DISTINCT FROM EXCLUDED.hours \
                OR projects.github_stars IS DISTINCT FROM EXCLUDED.github_stars \
                OR projects.display_name IS DISTINCT FROM EXCLUDED.display_name \
                OR projects.archived_demo IS DISTINCT FROM EXCLUDED.archived_demo \
                OR projects.archived_repo IS DISTINCT FROM EXCLUDED.archived_repo \
                OR projects.deleted_at IS NOT NULL",
        );

        let result = qb.build().execute(&mut *tx).await?;
        modified += result.rows_affected();
    }

    tx.commit().await?;
    info!("upserted {} entries ({} modified)", entries.len(), modified);

    // update screenshot urls separately
    let mut tx = pg.begin().await?;
    let mut urls_updated = 0;
    for chunk in entries.chunks(BATCH_SIZE) {
        let ids: Vec<&str> = chunk.iter().map(|e| e.id.as_str()).collect();
        let urls: Vec<Option<&str>> = chunk.iter().map(|e| e.screenshot_url.as_deref()).collect();

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

    info!("updated screenshot URLs for {} entries", urls_updated);

    let airtable_ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    let deleted = sqlx::query_scalar!(
        "UPDATE projects SET deleted_at = NOW() WHERE airtable_id != ALL($1) AND deleted_at IS NULL RETURNING 1 as count",
        &airtable_ids as &[&str]
    )
        .fetch_all(&mut *tx)
        .await?;

    if !deleted.is_empty() {
        info!("soft-deleted {} missing projects", deleted.len());
    }

    tx.commit().await?;

    Ok(())
}

async fn update_true_hours(http_client: &reqwest::Client, pg: &PgPool) -> anyhow::Result<()> {
    info!("updating true hours using airbridge");

    let body = http::fetch_with_retries(http_client, AIRBRIDGE_API_URL, 3)
        .await?
        .text()
        .await?;

    let entries: Vec<AirbridgeEntry> = serde_json::from_str(&body).map_err(|e| {
        error!("deserialization failed at byte {}: {e}", e.column());
        e
    })?;

    info!("fetched {} entries from airbridge", entries.len());

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

async fn embed_new_projects(pg: &PgPool) -> anyhow::Result<()> {
    let rows: Vec<EmbedRow> = sqlx::query_as(
        "SELECT p.id, p.display_name, p.description FROM projects p \
         LEFT JOIN project_embeddings pe ON p.id = pe.project_id \
         WHERE pe.project_id IS NULL AND p.deleted_at IS NULL \
         AND p.description IS NOT NULL AND LENGTH(p.description) >= $1",
    )
    .bind(MIN_DESCRIPTION_SIZE)
    .fetch_all(pg)
    .await?;

    if rows.is_empty() {
        info!("no new projects to embed");
        return Ok(());
    }

    info!("embedding {} new projects", rows.len());

    for (batch_idx, chunk) in rows.chunks(EMBED_BATCH_SIZE).enumerate() {
        let texts: Vec<String> = chunk
            .iter()
            .map(|row| {
                format!(
                    "{} {}",
                    row.display_name.as_deref().unwrap_or(""),
                    row.description.as_deref().unwrap_or("")
                )
                .trim()
                .to_string()
            })
            .collect();

        let (model_name, vectors) = embeddings::get_embeddings(&texts, false).await?;

        for (row, vec) in chunk.iter().zip(vectors) {
            sqlx::query(
                "INSERT INTO project_embeddings (project_id, embedding, model) \
                 VALUES ($1, $2, $3) \
                 ON CONFLICT (project_id) DO UPDATE SET embedding = $2, model = $3, updated_at = NOW()",
            )
            .bind(row.id)
            .bind(Vector::from(vec))
            .bind(&model_name)
            .execute(pg)
            .await?;
        }

        let done = batch_idx * EMBED_BATCH_SIZE + chunk.len();
        info!("embedded {done}/{}", rows.len());
    }

    info!("embedding complete");
    Ok(())
}
