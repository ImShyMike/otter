use std::pin::Pin;

use pgvector::Vector;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use time::OffsetDateTime;

use crate::utils::embeddings;

const API_URL: &str = "https://ships.hackclub.com/api/v1/ysws_entries?all=true";
const BATCH_SIZE: usize = 1000;
const EMBED_BATCH_SIZE: usize = 128;

fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    match Value::deserialize(deserializer)? {
        Value::Number(n) => {
            let ts = n
                .as_i64()
                .ok_or_else(|| serde::de::Error::custom("invalid timestamp"))?;
            OffsetDateTime::from_unix_timestamp(ts)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
        Value::Null | Value::String(_) => Ok(None),
        _ => Err(serde::de::Error::custom("expected number, null, or string")),
    }
}

fn deserialize_null_int<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde_json::Value;
    match Value::deserialize(deserializer)? {
        Value::Number(n) => n
            .as_i64()
            .map(|v| Some(v as i32))
            .ok_or_else(|| serde::de::Error::custom("invalid number")),
        Value::Null | Value::String(_) => Ok(None),
        _ => Err(serde::de::Error::custom("expected number, null, or string")),
    }
}

fn deserialize_null_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.filter(|s| s != "null"))
}

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

pub fn run<'a>(pg: &'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        tracing::info!("fetch_data: starting");

        let http_client = reqwest::Client::new();

        let body = http_client.get(API_URL).send().await?.text().await?;

        let entries: Vec<YswsEntry> = serde_json::from_str(&body).map_err(|e| {
            tracing::error!(
                "fetch_data: deserialization failed at byte {}: {e}",
                e.column()
            );
            e
        })?;

        tracing::info!("fetch_data: fetched {} entries", entries.len());

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
                 screenshot_url = EXCLUDED.screenshot_url, \
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
                 OR projects.screenshot_url IS DISTINCT FROM EXCLUDED.screenshot_url \
                 OR projects.github_stars IS DISTINCT FROM EXCLUDED.github_stars \
                 OR projects.display_name IS DISTINCT FROM EXCLUDED.display_name \
                 OR projects.archived_demo IS DISTINCT FROM EXCLUDED.archived_demo \
                 OR projects.archived_repo IS DISTINCT FROM EXCLUDED.archived_repo \
                 OR projects.deleted_at IS NOT NULL",
            );

            let result = qb.build().execute(pg).await?;
            modified += result.rows_affected();
        }

        let airtable_ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        let deleted = sqlx::query_scalar!(
            "UPDATE projects SET deleted_at = NOW() WHERE airtable_id != ALL($1) AND deleted_at IS NULL RETURNING 1 as count",
            &airtable_ids as &[&str]
        )
            .fetch_all(pg)
            .await?;

        if !deleted.is_empty() {
            tracing::info!(
                "fetch_data: soft-deleted {} missing projects",
                deleted.len()
            );
        }

        tracing::info!(
            "fetch_data: synced {} entries ({modified} modified)",
            entries.len()
        );

        embed_new_projects(pg).await?;

        tracing::info!("fetch_data: done");

        Ok(())
    })
}

async fn embed_new_projects(pg: &PgPool) -> anyhow::Result<()> {
    let rows = sqlx::query!(
        "SELECT id, display_name, description FROM projects WHERE embedding IS NULL AND deleted_at IS NULL"
    )
    .fetch_all(pg)
    .await?;

    if rows.is_empty() {
        tracing::info!("fetch_data: no new projects to embed");
        return Ok(());
    }

    tracing::info!("fetch_data: embedding {} new projects", rows.len());

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

        let (model_name, vectors) = embeddings::get_embeddings(&texts).await?;

        for (row, vec) in chunk.iter().zip(vectors) {
            sqlx::query("UPDATE projects SET embedding = $1, embedding_model = $2 WHERE id = $3")
                .bind(Vector::from(vec))
                .bind(&model_name)
                .bind(row.id)
                .execute(pg)
                .await?;
        }

        let done = batch_idx * EMBED_BATCH_SIZE + chunk.len();
        tracing::info!("fetch_data: embedded {done}/{}", rows.len());
    }

    tracing::info!("fetch_data: embedding complete");
    Ok(())
}
