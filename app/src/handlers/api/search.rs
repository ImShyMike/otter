use std::time::Instant;

use axum::Json;
use axum::extract::{Query, State};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::handlers::api::{ProjectItem, local_only};
use crate::state::AppState;
use crate::utils::embeddings;

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchQuery {
    q: String,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    fts_weight: Option<f32>,
    #[serde(default)]
    semantic_weight: Option<f32>,
    #[serde(default)]
    trigram_weight: Option<f32>,
}

#[derive(Serialize, ToSchema)]
pub struct SearchTimings {
    embeddings_ms: f64,
    query_ms: f64,
}

#[derive(Serialize, ToSchema)]
pub struct SearchResults {
    data: Vec<SearchResult>,
    total: i64,
    page: i64,
    per_page: i64,
    timings: SearchTimings,
}

#[derive(Serialize, ToSchema)]
pub struct SearchResult {
    #[serde(flatten)]
    pub item: ProjectItem,
    pub score: f64,
}

#[derive(sqlx::FromRow)]
struct SearchRow {
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
    inferred_repo: Option<String>,
    inferred_username: Option<String>,
    score: f64,
    _total: i64,
}

impl From<SearchRow> for SearchResult {
    fn from(row: SearchRow) -> Self {
        Self {
            item: ProjectItem {
                id: row.id,
                airtable_id: row.airtable_id,
                ysws: row.ysws,
                approved_at: row.approved_at,
                code_url: row.code_url,
                country: row.country,
                demo_url: row.demo_url,
                description: row.description,
                github_username: row.github_username,
                hours: row.hours,
                true_hours: row.true_hours,
                has_media: row.has_media,
                github_stars: row.github_stars,
                display_name: row.display_name,
                archived_demo: row.archived_demo,
                archived_repo: row.archived_repo,
                inferred_repo: row.inferred_repo,
                inferred_username: row.inferred_username,
            },
            score: row.score,
        }
    }
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
        (status = 200, description = "Search results", body = SearchResults),
        (status = 400, description = "Bad request"),
    )
)]
#[instrument(skip(state), fields(q = %params.q, limit = params.limit))]
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResults>, AppError> {
    let filters = parse_filters(&params.q);
    let limit = params.limit.unwrap_or(10).min(100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;
    let fts_weight = params.fts_weight.unwrap_or(0.7).max(0.0);
    let semantic_weight = params.semantic_weight.unwrap_or(0.2).max(0.0);
    let trigram_weight = params.trigram_weight.unwrap_or(0.1).max(0.0);
    let user_like = filters.user.as_ref().map(|u| format!("%{}%", u));
    let fts_candidate_limit = (limit * 10).clamp(100, 2000);
    let phrase_candidate_limit = (limit * 8).clamp(80, 1500);
    let semantic_candidate_limit = (limit * 8).clamp(80, 1250);
    let trigram_candidate_limit = (limit * 15).clamp(150, 3000);
    let literal_candidate_limit = (limit * 12).clamp(120, 2500);

    // user-only search
    if filters.cleaned_query.is_empty()
        && let Some(username) = filters.user.as_deref()
    {
        return user_search(username, limit, page, &state).await;
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
    let embed_start = Instant::now();
    let (_, embeddings) = embeddings::get_embeddings_with_cache(
        std::slice::from_ref(&filters.embedding_query),
        &state.redis,
        local_only(),
    )
    .await?;
    let query_embedding = &embeddings[0];
    let embeddings_ms = embed_start.elapsed().as_secs_f64() * 1000.0;

    // run query
    let query_start = Instant::now();
    let rows: Vec<SearchRow> = sqlx::query_as(
        r#"
        WITH filtered_projects AS NOT MATERIALIZED (
            SELECT
                p.id,
                p.airtable_id,
                p.ysws,
                p.approved_at,
                p.code_url,
                p.country,
                p.demo_url,
                p.description,
                p.github_username,
                p.hours,
                p.true_hours,
                p.has_media,
                p.github_stars,
                p.display_name,
                p.archived_demo,
                p.archived_repo,
                p.tsv,
                p.inferred_repo,
                p.inferred_username,
                COALESCE(p.github_username, p.inferred_username, '') AS search_username,
                COALESCE(REPLACE(REPLACE(p.inferred_repo, '-', ' '), '_', ' '), '') AS search_repo
            FROM projects p
            WHERE p.deleted_at IS NULL
              AND (
                    $10::text IS NULL OR
                    p.github_username ILIKE $10 OR
                    p.inferred_username ILIKE $10 OR
                    p.display_name ILIKE $10 OR
                    p.code_url ILIKE $10 OR
                    p.inferred_repo ILIKE $10
              )
        ),
        fts_results AS (
            SELECT
                p.id,
                COALESCE(ts_rank_cd(p.tsv, query), 0) as fts_score
            FROM filtered_projects p, websearch_to_tsquery('english', $1) query
            WHERE p.tsv @@ query
            ORDER BY fts_score DESC
            LIMIT $7
        ),
        quoted_phrases AS (
            SELECT lower(m[1]) AS phrase
            FROM regexp_matches($1, '"([^"]+)"', 'g') AS m
        ),
        phrase_scored AS (
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
                    END +
                    CASE
                        WHEN STRPOS(lower(p.search_repo), qp.phrase) > 0 THEN 1.0
                        ELSE 0.0
                    END +
                    CASE
                        WHEN STRPOS(lower(p.search_username), qp.phrase) > 0 THEN 0.75
                        ELSE 0.0
                    END
                )::double precision as phrase_score
            FROM filtered_projects p
            INNER JOIN quoted_phrases qp ON TRUE
            GROUP BY p.id
        ),
        phrase_results AS (
            SELECT
                ps.id,
                ps.phrase_score
            FROM phrase_scored ps
            WHERE ps.phrase_score > 0
            ORDER BY ps.phrase_score DESC
            LIMIT $11
        ),
        query_terms AS (
            SELECT lower(trim(BOTH FROM REPLACE($1, '"', ''))) AS raw_q
        ),
        literal_results AS (
            SELECT
                p.id,
                GREATEST(
                    CASE
                        WHEN qt.raw_q <> '' AND lower(COALESCE(p.inferred_repo, '')) = qt.raw_q THEN 4.0
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND lower(COALESCE(p.display_name, '')) = qt.raw_q THEN 3.5
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND lower(COALESCE(p.ysws, '')) = qt.raw_q THEN 3.0
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND lower(p.search_repo) LIKE qt.raw_q || '%' THEN 2.4
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND lower(COALESCE(p.display_name, '')) LIKE qt.raw_q || '%' THEN 2.2
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND lower(p.search_username) LIKE qt.raw_q || '%' THEN 2.0
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND STRPOS(lower(p.search_repo), qt.raw_q) > 0 THEN 1.5
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND STRPOS(lower(COALESCE(p.display_name, '')), qt.raw_q) > 0 THEN 1.35
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND STRPOS(lower(p.search_username), qt.raw_q) > 0 THEN 1.2
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND STRPOS(lower(COALESCE(p.code_url, '')), qt.raw_q) > 0 THEN 1.0
                        ELSE 0.0
                    END,
                    CASE
                        WHEN qt.raw_q <> '' AND STRPOS(lower(COALESCE(p.demo_url, '')), qt.raw_q) > 0 THEN 1.0
                        ELSE 0.0
                    END
                )::double precision AS literal_score
            FROM filtered_projects p
            CROSS JOIN query_terms qt
            WHERE qt.raw_q <> ''
              AND (
                    lower(COALESCE(p.inferred_repo, '')) = qt.raw_q OR
                    lower(COALESCE(p.display_name, '')) = qt.raw_q OR
                    lower(COALESCE(p.ysws, '')) = qt.raw_q OR
                    lower(p.search_repo) LIKE qt.raw_q || '%' OR
                    lower(COALESCE(p.display_name, '')) LIKE qt.raw_q || '%' OR
                    lower(p.search_username) LIKE qt.raw_q || '%' OR
                    STRPOS(lower(p.search_repo), qt.raw_q) > 0 OR
                    STRPOS(lower(COALESCE(p.display_name, '')), qt.raw_q) > 0 OR
                    STRPOS(lower(p.search_username), qt.raw_q) > 0 OR
                    STRPOS(lower(COALESCE(p.code_url, '')), qt.raw_q) > 0 OR
                    STRPOS(lower(COALESCE(p.demo_url, '')), qt.raw_q) > 0
              )
            ORDER BY literal_score DESC
            LIMIT $12
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
            INNER JOIN filtered_projects p ON p.id = ec.id
            WHERE p.description IS NOT NULL
              AND LENGTH(p.description) > 50
        ),
        trigram_results AS (
            SELECT
                p.id,
                GREATEST(
                    similarity(p.display_name, $1),
                    similarity(p.ysws, $1),
                    similarity(p.github_username, $1),
                    similarity(p.inferred_username, $1),
                    similarity(p.inferred_repo, $1)
                ) as trigram_score
            FROM projects p
            WHERE p.deleted_at IS NULL
              AND (
                    p.display_name % $1 OR
                    p.ysws % $1 OR
                    p.github_username % $1 OR
                    p.inferred_username % $1 OR
                    p.inferred_repo % $1
              )
            ORDER BY trigram_score DESC
            LIMIT $9
        ),
        candidates AS (
            SELECT id FROM fts_results
            UNION
            SELECT id FROM phrase_results
            UNION
            SELECT id FROM literal_results
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
                p.approved_at,
                p.code_url,
                p.country,
                p.demo_url,
                p.description,
                p.github_username,
                p.hours,
                p.true_hours,
                p.has_media,
                p.github_stars,
                p.display_name,
                p.archived_demo,
                p.archived_repo,
                p.inferred_repo,
                p.inferred_username,
                (
                    COALESCE(f.fts_score, 0) * $3 +
                    COALESCE(ph.phrase_score, 0) * GREATEST($3, $4) +
                    COALESCE(l.literal_score, 0) * GREATEST($3, $5) +
                    COALESCE(e.similarity_score, 0) * $4 +
                    COALESCE(t.trigram_score, 0) * $5
                )::double precision as raw_score
            FROM filtered_projects p
            INNER JOIN candidates c ON p.id = c.id
            LEFT JOIN fts_results f ON p.id = f.id
            LEFT JOIN phrase_results ph ON p.id = ph.id
            LEFT JOIN literal_results l ON p.id = l.id
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
            s.inferred_repo,
            s.inferred_username,
            CASE
                WHEN MAX(s.raw_score) OVER () > 0
                    THEN (s.raw_score / MAX(s.raw_score) OVER ())::double precision
                ELSE 0.0::double precision
            END AS score,
            COUNT(*) OVER() AS _total
        FROM scored s
        ORDER BY s.raw_score DESC, s.approved_at DESC NULLS LAST
        LIMIT $6
        OFFSET $13
        "#,
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
    .bind(user_like)
    .bind(phrase_candidate_limit)
    .bind(literal_candidate_limit)
    .bind(offset)
    .fetch_all(&state.pg)
    .await?;
    let query_ms = query_start.elapsed().as_secs_f64() * 1000.0;

    let total = rows.first().map(|r| r._total).unwrap_or(0);
    let results: Vec<SearchResult> = rows.into_iter().map(SearchResult::from).collect();

    Ok(Json(SearchResults {
        data: results,
        total,
        page,
        per_page: limit,
        timings: SearchTimings {
            embeddings_ms,
            query_ms,
        },
    }))
}

pub async fn user_search(
    username: &str,
    limit: i64,
    page: i64,
    state: &AppState,
) -> Result<Json<SearchResults>, AppError> {
    let offset = (page - 1) * limit;
    let query_start = Instant::now();
    let rows: Vec<SearchRow> = sqlx::query_as(
        r#"
        SELECT
            p.id,
            p.airtable_id,
            p.ysws,
            p.approved_at,
            p.code_url,
            p.country,
            p.demo_url,
            p.description,
            p.github_username,
            p.hours,
            p.true_hours,
            p.has_media,
            p.github_stars,
            p.display_name,
            p.archived_demo,
            p.archived_repo,
            p.inferred_repo,
            p.inferred_username,
            1.0::double precision as score,
            COUNT(*) OVER() AS _total
        FROM projects p
        WHERE p.deleted_at IS NULL
            AND (
            p.github_username ILIKE $1
            OR p.inferred_username ILIKE $1
            OR p.display_name ILIKE $1
            OR p.inferred_repo ILIKE $1
            OR p.code_url ILIKE $1
            )
        ORDER BY p.approved_at DESC
        LIMIT $2
        OFFSET $3
        "#,
    )
    .bind(format!("%{}%", username))
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pg)
    .await?;
    let query_ms = query_start.elapsed().as_secs_f64() * 1000.0;

    let total = rows.first().map(|r| r._total).unwrap_or(0);
    let results: Vec<SearchResult> = rows.into_iter().map(SearchResult::from).collect();

    Ok(Json(SearchResults {
        data: results,
        total,
        page,
        per_page: limit,
        timings: SearchTimings {
            embeddings_ms: 0.0,
            query_ms,
        },
    }))
}
