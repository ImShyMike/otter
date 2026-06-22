use std::{pin::Pin, time::Duration};

use pgvector::Vector;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use time::OffsetDateTime;
use tracing::{Instrument, error, info, instrument};

use crate::utils::code_url::parse_code_url;
use crate::utils::serde::{deserialize_null_int, deserialize_null_string, deserialize_timestamp};
use crate::utils::{embeddings, http};

const SHIPS_API_URL: &str = "https://ships.hackclub.com/api/v1/ysws_entries?all=true";
const BATCH_SIZE: usize = 250;
const EMBED_BATCH_SIZE: usize = 128;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const MIN_DESCRIPTION_SIZE: i32 = 35;

#[derive(Deserialize)]
struct YswsEntry {
    id: String,
    #[serde(deserialize_with = "deserialize_null_string")]
    ysws: Option<String>,
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
    slack_id: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    github_username: Option<String>,
    #[serde(deserialize_with = "deserialize_null_int")]
    hours: Option<i32>,
    #[serde(default)]
    github_stars: i32,
    #[serde(deserialize_with = "deserialize_null_string")]
    display_name: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    archived_demo: Option<String>,
    #[serde(deserialize_with = "deserialize_null_string")]
    archived_repo: Option<String>,
}

#[derive(sqlx::FromRow)]
struct EmbedRow {
    id: i32,
    display_name: Option<String>,
    description: Option<String>,
}

pub fn run<'a>(pg: &'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(
        async move {
            info!("starting");

            let http_client = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()?;

            update_data(&http_client, pg).await?;

            embed_new_projects(pg).await?;

            info!("done");

            Ok(())
        }
        .instrument(tracing::info_span!("ships_data")),
    )
}

#[instrument(skip_all)]
async fn update_data(http_client: &reqwest::Client, pg: &PgPool) -> anyhow::Result<()> {
    let body = fetch_ships_data(http_client).await?;

    let entries: Vec<YswsEntry> = tracing::info_span!("deserialize_entries").in_scope(|| {
        serde_json::from_str(&body).map_err(|e| {
            error!("deserialization failed at byte {}: {e}", e.column());
            anyhow::Error::from(e)
        })
    })?;

    let entries_count = entries.len();
    info!("fetched {} entries", entries_count);

    let entries: Vec<YswsEntry> = entries.into_iter().filter(|e| e.ysws.is_some()).collect();

    info!(
        "skipped {} entries with null ysws",
        entries_count - entries.len()
    );

    upsert_projects(&entries, pg).await?;

    Ok(())
}

#[instrument(skip_all)]
async fn fetch_ships_data(http_client: &reqwest::Client) -> anyhow::Result<String> {
    Ok(
        http::fetch_with_retries(3, || http_client.get(SHIPS_API_URL))
            .await?
            .text()
            .await?,
    )
}

#[instrument(skip_all)]
async fn upsert_projects(entries: &[YswsEntry], pg: &PgPool) -> anyhow::Result<()> {
    let mut tx = pg.begin().await?;
    let mut modified: u64 = 0;

    for chunk in entries.chunks(BATCH_SIZE) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO projects (airtable_id, ysws, approved_at, code_url, country, demo_url, description, slack_id, github_username, hours, github_stars, display_name, archived_demo, archived_repo, inferred_repo, inferred_username, is_github_url) ",
        );

        qb.push_values(chunk, |mut b, entry| {
            let parsed = entry
                .code_url
                .as_deref()
                .map(parse_code_url)
                .unwrap_or_default();
            b.push_bind(&entry.id)
                .push_bind(&entry.ysws)
                .push_bind(entry.approved_at.map(|t| t.unix_timestamp()))
                .push_bind(&entry.code_url)
                .push_bind(&entry.country)
                .push_bind(&entry.demo_url)
                .push_bind(&entry.description)
                .push_bind(&entry.slack_id)
                .push_bind(&entry.github_username)
                .push_bind(entry.hours)
                .push_bind(entry.github_stars)
                .push_bind(&entry.display_name)
                .push_bind(&entry.archived_demo)
                .push_bind(&entry.archived_repo)
                .push_bind(parsed.repo)
                .push_bind(parsed.owner)
                .push_bind(parsed.is_github);
        });

        qb.push(
            " ON CONFLICT (airtable_id) DO UPDATE SET \
                ysws = EXCLUDED.ysws, \
                approved_at = EXCLUDED.approved_at, \
                code_url = EXCLUDED.code_url, \
                country = EXCLUDED.country, \
                demo_url = EXCLUDED.demo_url, \
                description = EXCLUDED.description, \
                slack_id = EXCLUDED.slack_id, \
                github_username = EXCLUDED.github_username, \
                hours = EXCLUDED.hours, \
                github_stars = EXCLUDED.github_stars, \
                display_name = EXCLUDED.display_name, \
                archived_demo = EXCLUDED.archived_demo, \
                archived_repo = EXCLUDED.archived_repo, \
                inferred_repo = EXCLUDED.inferred_repo, \
                inferred_username = EXCLUDED.inferred_username, \
                is_github_url = EXCLUDED.is_github_url \
                WHERE projects.deleted_at IS NULL \
                AND (projects.ysws IS DISTINCT FROM EXCLUDED.ysws \
                OR projects.approved_at IS DISTINCT FROM EXCLUDED.approved_at \
                OR projects.code_url IS DISTINCT FROM EXCLUDED.code_url \
                OR projects.country IS DISTINCT FROM EXCLUDED.country \
                OR projects.demo_url IS DISTINCT FROM EXCLUDED.demo_url \
                OR projects.description IS DISTINCT FROM EXCLUDED.description \
                OR projects.slack_id IS DISTINCT FROM EXCLUDED.slack_id \
                OR projects.github_username IS DISTINCT FROM EXCLUDED.github_username \
                OR projects.hours IS DISTINCT FROM EXCLUDED.hours \
                OR projects.github_stars IS DISTINCT FROM EXCLUDED.github_stars \
                OR projects.display_name IS DISTINCT FROM EXCLUDED.display_name \
                OR projects.archived_demo IS DISTINCT FROM EXCLUDED.archived_demo \
                OR projects.archived_repo IS DISTINCT FROM EXCLUDED.archived_repo \
                OR projects.inferred_repo IS DISTINCT FROM EXCLUDED.inferred_repo \
                OR projects.inferred_username IS DISTINCT FROM EXCLUDED.inferred_username \
                OR projects.is_github_url IS DISTINCT FROM EXCLUDED.is_github_url)",
        );

        let result = qb.build().persistent(false).execute(&mut *tx).await?;
        modified += result.rows_affected();
    }

    tx.commit().await?;
    info!("upserted {} entries ({} modified)", entries.len(), modified);

    Ok(())
}

#[instrument(skip_all)]
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
        let vectors: Vec<Vector> = vectors.into_iter().map(Vector::from).collect();

        let mut qb: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO project_embeddings (project_id, embedding, model) ");

        qb.push_values(chunk.iter().zip(vectors.iter()), |mut b, (row, vector)| {
            b.push_bind(row.id).push_bind(vector).push_bind(&model_name);
        });

        qb.push(
            " ON CONFLICT (project_id) DO UPDATE SET embedding = EXCLUDED.embedding, model = EXCLUDED.model, updated_at = NOW()",
        );

        qb.build().persistent(false).execute(pg).await?;

        let done = batch_idx * EMBED_BATCH_SIZE + chunk.len();
        info!("embedded {done}/{}", rows.len());
    }

    info!("embedding complete");
    Ok(())
}
