use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;
use crate::utils::embeddings;

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    fts_weight: Option<f32>,
    #[serde(default)]
    semantic_weight: Option<f32>,
}

#[derive(Serialize)]
pub struct SearchResult {
    id: i32,
    display_name: Option<String>,
    description: Option<String>,
    ysws: String,
    country: Option<String>,
    code_url: Option<String>,
    demo_url: Option<String>,
    score: f32,
}

#[derive(sqlx::FromRow)]
struct RawResult {
    id: i32,
    display_name: Option<String>,
    description: Option<String>,
    ysws: String,
    country: Option<String>,
    code_url: Option<String>,
    demo_url: Option<String>,
    score: f64,
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    let limit = params.limit.unwrap_or(10).min(100);
    let fts_weight = params.fts_weight.unwrap_or(0.4);
    let semantic_weight = params.semantic_weight.unwrap_or(0.6);

    // Normalize weights
    let total_weight = fts_weight + semantic_weight;
    let fts_weight = fts_weight / total_weight;
    let semantic_weight = semantic_weight / total_weight;

    // Get embedding for semantic search (with caching)
    let (_, embeddings) =
        embeddings::get_embeddings_with_cache(std::slice::from_ref(&params.q), &state.redis)
            .await?;
    let query_embedding = &embeddings[0];

    // Run hybrid search query
    let results: Vec<RawResult> = sqlx::query_as(
        r#"
        WITH fts_results AS (
            SELECT 
                id,
                COALESCE(ts_rank(tsv, query), 0) as fts_score
            FROM projects, plainto_tsquery('english', $1) query
            WHERE tsv @@ query AND deleted_at IS NULL
        ),
        embedding_results AS (
            SELECT 
                id,
                1.0 - (embedding <=> $2::vector) as similarity_score
            FROM projects
            WHERE embedding IS NOT NULL AND deleted_at IS NULL AND description IS NOT NULL AND LENGTH(description) > 50
        )
        SELECT 
            p.id,
            p.display_name,
            p.description,
            p.ysws,
            p.country,
            p.code_url,
            p.demo_url,
            (
                COALESCE(f.fts_score, 0) * $3 + 
                COALESCE(e.similarity_score, 0) * $4
            )::double precision as score
        FROM projects p
        LEFT JOIN fts_results f ON p.id = f.id
        LEFT JOIN embedding_results e ON p.id = e.id
        WHERE f.fts_score > 0 OR e.similarity_score > 0
        ORDER BY score DESC
        LIMIT $5
        "#,
    )
    .bind(params.q)
    .bind(query_embedding)
    .bind(fts_weight)
    .bind(semantic_weight)
    .bind(limit)
    .fetch_all(&state.pg)
    .await?;

    let results = results
        .into_iter()
        .map(|r| SearchResult {
            id: r.id,
            display_name: r.display_name,
            description: r.description,
            ysws: r.ysws,
            country: r.country,
            code_url: r.code_url,
            demo_url: r.demo_url,
            score: r.score as f32,
        })
        .collect();

    Ok(Json(results))
}
