use axum::Json;
use axum::extract::{Query, State};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::handlers::api::local_only;
use crate::state::AppState;
use crate::utils::embeddings;

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchQuery {
    q: String,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    fts_weight: Option<f32>,
    #[serde(default)]
    semantic_weight: Option<f32>,
    #[serde(default)]
    trigram_weight: Option<f32>,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct SearchResult {
    id: i32,
    airtable_id: String,
    ysws: String,
    approved_at: Option<i64>,
    code_url: Option<String>,
    country: Option<String>,
    demo_url: Option<String>,
    description: Option<String>,
    github_username: Option<String>,
    hours: Option<i32>,
    true_hours: Option<f64>,
    has_media: bool,
    github_stars: i32,
    display_name: Option<String>,
    archived_demo: Option<String>,
    archived_repo: Option<String>,
    score: f64,
}

#[derive(Debug, Clone)]
struct ParsedFilters {
    user: Option<String>,
    cleaned_query: String,
    embedding_query: String,
}

fn parse_filters(query: &str) -> ParsedFilters {
    let mut user = None;
    let mut cleaned_query = query.to_string();

    // extract user:username pattern
    if let Ok(re) = Regex::new(r"\buser:(\S+)")
        && let Some(caps) = re.captures(&cleaned_query)
    {
        user = Some(caps[1].to_string());
        cleaned_query = re.replace_all(&cleaned_query, "").trim().to_string();
    }

    let embedding_query = cleaned_query
        .replace('"', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let embedding_query = if embedding_query.is_empty() {
        cleaned_query.clone()
    } else {
        embedding_query
    };

    ParsedFilters {
        user,
        cleaned_query,
        embedding_query,
    }
}

#[utoipa::path(
    get,
    path = "/search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results", body = Vec<SearchResult>),
        (status = 400, description = "Bad request"),
    )
)]
#[instrument(skip(state), fields(q = %params.q, limit = params.limit))]
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    let filters = parse_filters(&params.q);
    let limit = params.limit.unwrap_or(10).min(100);
    let fts_weight = params.fts_weight.unwrap_or(0.7).max(0.0);
    let semantic_weight = params.semantic_weight.unwrap_or(0.2).max(0.0);
    let trigram_weight = params.trigram_weight.unwrap_or(0.1).max(0.0);
    let fts_candidate_limit = (limit * 10).clamp(100, 2000);
    let semantic_candidate_limit = (limit * 25).clamp(200, 5000);
    let trigram_candidate_limit = (limit * 15).clamp(150, 3000);

    // user-only search
    if filters.cleaned_query.is_empty()
        && let Some(username) = filters.user.as_deref()
    {
        return user_search(username, limit, &state).await;
    }

    // normalize weights
    let total_weight = fts_weight + semantic_weight + trigram_weight;
    let (fts_weight, semantic_weight, trigram_weight) = if total_weight > 0.0 {
        (
            fts_weight / total_weight,
            semantic_weight / total_weight,
            trigram_weight / total_weight,
        )
    } else {
        (0.7, 0.2, 0.1)
    };

    // get embeddings
    let (_, embeddings) = embeddings::get_embeddings_with_cache(
        std::slice::from_ref(&filters.embedding_query),
        &state.redis,
        local_only(),
    )
    .await?;
    let query_embedding = &embeddings[0];

    // build query filter for user if specified
    let user_filter = if let Some(ref username) = filters.user {
        format!(
            "AND (p.github_username ILIKE '%{}%' OR p.display_name ILIKE '%{}%' OR p.code_url ILIKE '%{}%')",
            username.replace("'", "''"),
            username.replace("'", "''"),
            username.replace("'", "''")
        )
    } else {
        String::new()
    };

    // run query
    let results: Vec<SearchResult> = sqlx::query_as(
        &format!(
            r#"
        WITH fts_results AS (
            SELECT
                p.id,
                COALESCE(ts_rank_cd(p.tsv, query), 0) as fts_score
            FROM projects p, websearch_to_tsquery('english', $1) query
            WHERE p.tsv @@ query AND p.deleted_at IS NULL {}
            ORDER BY fts_score DESC
            LIMIT $7
        ),
        quoted_phrases AS (
            SELECT lower(m[1]) AS phrase
            FROM regexp_matches($1, '"([^"]+)"', 'g') AS m
        ),
        phrase_results AS (
            SELECT
                p.id,
                SUM(
                    CASE
                        WHEN STRPOS(lower(COALESCE(p.display_name, '')), qp.phrase) > 0 THEN 1.0
                        ELSE 0.0
                    END +
                    CASE
                        WHEN STRPOS(lower(COALESCE(p.description, '')), qp.phrase) > 0 THEN 0.75
                        ELSE 0.0
                    END
                )::double precision as phrase_score
            FROM projects p
            INNER JOIN quoted_phrases qp ON TRUE
            WHERE p.deleted_at IS NULL
            GROUP BY p.id
        ),
        embedding_candidates AS (
            SELECT
                pe.project_id AS id,
                pe.embedding <=> $2::vector AS distance
            FROM project_embeddings pe
            ORDER BY pe.embedding <=> $2::vector
            LIMIT $8
        ),
        embedding_results AS (
            SELECT
                ec.id,
                GREATEST(0.0, LEAST(1.0, 1.0 - (ec.distance / 2.0))) as similarity_score
            FROM embedding_candidates ec
            INNER JOIN projects p ON p.id = ec.id
            WHERE p.deleted_at IS NULL AND p.description IS NOT NULL AND LENGTH(p.description) > 50 {}
        ),
        trigram_results AS (
            SELECT
                p.id,
                GREATEST(
                    similarity(COALESCE(p.display_name, ''), $1),
                    similarity(COALESCE(p.ysws, ''), $1),
                    similarity(COALESCE(p.github_username, ''), $1),
                    similarity(COALESCE(p.country, ''), $1)
                ) as trigram_score
            FROM projects p
            WHERE p.deleted_at IS NULL
              AND (
                    COALESCE(p.display_name, '') % $1 OR
                    COALESCE(p.ysws, '') % $1 OR
                    COALESCE(p.github_username, '') % $1 OR
                    COALESCE(p.country, '') % $1
              ) {}
            ORDER BY trigram_score DESC
            LIMIT $9
        ),
        candidates AS (
            SELECT id FROM fts_results
            UNION
            SELECT id FROM phrase_results
            UNION
            SELECT id FROM embedding_results
            UNION
            SELECT id FROM trigram_results
        ),
        scored AS (
            SELECT
                p.id,
                p.airtable_id,
                p.ysws,
                EXTRACT(EPOCH FROM p.approved_at)::bigint AS approved_at,
                p.code_url,
                p.country,
                p.demo_url,
                p.description,
                p.github_username,
                p.hours,
                p.true_hours,
                (p.media_url IS NOT NULL) AS has_media,
                p.github_stars,
                p.display_name,
                p.archived_demo,
                p.archived_repo,
                (
                    COALESCE(f.fts_score, 0) * $3 +
                    COALESCE(ph.phrase_score, 0) * GREATEST($3, $4) +
                    COALESCE(e.similarity_score, 0) * $4 +
                    COALESCE(t.trigram_score, 0) * $5
                )::double precision as raw_score
            FROM projects p
            INNER JOIN candidates c ON p.id = c.id
            LEFT JOIN fts_results f ON p.id = f.id
            LEFT JOIN phrase_results ph ON p.id = ph.id
            LEFT JOIN embedding_results e ON p.id = e.id
            LEFT JOIN trigram_results t ON p.id = t.id
        )
        SELECT
            s.id,
            s.airtable_id,
            s.ysws,
            s.approved_at,
            s.code_url,
            s.country,
            s.demo_url,
            s.description,
            s.github_username,
            s.hours,
            s.true_hours,
            s.has_media,
            s.github_stars,
            s.display_name,
            s.archived_demo,
            s.archived_repo,
            CASE
                WHEN MAX(s.raw_score) OVER () > 0
                    THEN (s.raw_score / MAX(s.raw_score) OVER ())::double precision
                ELSE 0.0::double precision
            END AS score
        FROM scored s
        ORDER BY s.raw_score DESC
        LIMIT $6
        "#,
            user_filter, user_filter, user_filter
        )
    )
    .bind(&filters.cleaned_query)
    .bind(query_embedding)
    .bind(fts_weight)
    .bind(semantic_weight)
    .bind(trigram_weight)
    .bind(limit)
    .bind(fts_candidate_limit)
    .bind(semantic_candidate_limit)
    .bind(trigram_candidate_limit)
    .fetch_all(&state.pg)
    .await?;

    Ok(Json(results))
}

pub async fn user_search(
    username: &str,
    limit: i64,
    state: &AppState,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    let results: Vec<SearchResult> = sqlx::query_as(
        r#"
        SELECT
            p.id,
            p.airtable_id,
            p.ysws,
            EXTRACT(EPOCH FROM p.approved_at)::bigint AS approved_at,
            p.code_url,
            p.country,
            p.demo_url,
            p.description,
            p.github_username,
            p.hours,
            p.true_hours,
            (p.media_url IS NOT NULL) AS has_media,
            p.github_stars,
            p.display_name,
            p.archived_demo,
            p.archived_repo,
            1.0::double precision as score
        FROM projects p
        WHERE p.deleted_at IS NULL
            AND (
            p.github_username ILIKE $1
            OR p.display_name ILIKE $1
            OR p.code_url ILIKE $1
            )
        ORDER BY p.approved_at DESC
        LIMIT $2
        "#,
    )
    .bind(format!("%{}%", username))
    .bind(limit)
    .fetch_all(&state.pg)
    .await?;

    Ok(Json(results))
}
