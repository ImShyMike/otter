use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::extract::State;
use serde::Serialize;
use time::Date;
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, sqlx::FromRow)]
struct FineProjectRow {
    fine_id: i32,
    transaction_id: String,
    amount_cents: i32,
    fine_ysws: Option<String>,
    fine_date: Date,
    project_id: i32,
    airtable_id: String,
    project_ysws: String,
    approved_at: Option<i64>,
    code_url: Option<String>,
    country: Option<String>,
    demo_url: Option<String>,
    description: Option<String>,
    github_username: Option<String>,
    hours: Option<i32>,
    true_hours: Option<f64>,
    display_name: Option<String>,
    archived_demo: Option<String>,
    archived_repo: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ProjectSummary {
    pub id: i32,
    pub airtable_id: String,
    pub ysws: String,
    pub approved_at: Option<i64>,
    pub code_url: Option<String>,
    pub country: Option<String>,
    pub demo_url: Option<String>,
    pub description: Option<String>,
    pub github_username: Option<String>,
    pub hours: Option<i32>,
    pub true_hours: Option<f64>,
    pub display_name: Option<String>,
    pub archived_demo: Option<String>,
    pub archived_repo: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct FineItem {
    pub id: i32,
    pub transaction_id: String,
    pub amount_cents: i32,
    pub ysws: Option<String>,
    pub date: String,
    pub projects: Vec<ProjectSummary>,
}

#[utoipa::path(
    get,
    path = "/fines",
    responses(
        (status = 200, description = "Fines matched to deleted projects", body = Vec<FineItem>),
    )
)]
#[instrument(skip(state))]
pub async fn fines(State(state): State<AppState>) -> Result<Json<Vec<FineItem>>, AppError> {
    let rows = sqlx::query_as::<_, FineProjectRow>(
        r#"
        SELECT
          f.id AS fine_id,
          f.transaction_id,
          f.amount_cents,
          f.ysws AS fine_ysws,
          f.date AS fine_date,
          p.id AS project_id,
          p.airtable_id,
          p.ysws AS project_ysws,
          p.approved_at,
          p.code_url,
          p.country,
          p.demo_url,
          p.description,
          p.github_username,
          p.hours,
          p.true_hours,
          p.display_name,
          p.archived_demo,
          p.archived_repo
        FROM fines f
        JOIN projects p
          ON f.ysws IS NOT NULL
          AND p.ysws IS NOT NULL
          AND p.ysws = f.ysws
        JOIN project_changes jc
          ON jc.project_id = p.id
          AND jc.is_delete = true
        WHERE jc.changed_at BETWEEN (f.date - INTERVAL '1 day') AND (f.date + INTERVAL '1 day')
        ORDER BY f.id DESC
        "#,
    )
    .fetch_all(&state.pg)
    .await?;

    let mut fines: Vec<FineItem> = Vec::new();
    let mut fine_indexes: HashMap<i32, usize> = HashMap::new();
    let mut seen_projects_by_fine: HashMap<i32, HashSet<i32>> = HashMap::new();

    for row in rows {
        let project = ProjectSummary {
            id: row.project_id,
            airtable_id: row.airtable_id,
            ysws: row.project_ysws,
            approved_at: row.approved_at,
            code_url: row.code_url,
            country: row.country,
            demo_url: row.demo_url,
            description: row.description,
            github_username: row.github_username,
            hours: row.hours,
            true_hours: row.true_hours,
            display_name: row.display_name,
            archived_demo: row.archived_demo,
            archived_repo: row.archived_repo,
        };

        let fine_index = if let Some(index) = fine_indexes.get(&row.fine_id) {
            *index
        } else {
            let index = fines.len();
            fines.push(FineItem {
                id: row.fine_id,
                transaction_id: row.transaction_id.clone(),
                amount_cents: row.amount_cents,
                ysws: row.fine_ysws.clone(),
                date: row.fine_date.to_string(),
                projects: Vec::new(),
            });
            fine_indexes.insert(row.fine_id, index);
            index
        };

        let seen_projects = seen_projects_by_fine.entry(row.fine_id).or_default();
        if seen_projects.insert(row.project_id) {
            fines[fine_index].projects.push(project);
        }
    }

    Ok(Json(fines))
}
