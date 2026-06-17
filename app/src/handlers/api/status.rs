use axum::{Json, extract::State};
use serde::Serialize;
use time::OffsetDateTime;
use tracing::instrument;
use utoipa::ToSchema;

use std::sync::OnceLock;
use tokio::sync::RwLock;

use crate::state::AppState;

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct ServerStatus {
    pub last_refreshed_at: Option<i64>,
    pub total_projects: Option<i64>,
}

static LAST_REFRESHED_AT: OnceLock<RwLock<Option<i64>>> = OnceLock::new();

fn last_refreshed_at() -> &'static RwLock<Option<i64>> {
    LAST_REFRESHED_AT.get_or_init(|| RwLock::new(None))
}

#[utoipa_ts::path(
    get,
    path = "/status",
    responses(
        (status = 200, description = "Server status data", body = ServerStatus),
    )
)]
#[instrument(skip_all)]
pub async fn data_refresh_status(State(state): State<AppState>) -> Json<ServerStatus> {
    let refreshed_at = *last_refreshed_at().read().await;

    let total_projects =
        sqlx::query_scalar!("SELECT COUNT(*) FROM projects WHERE deleted_at IS NULL")
            .fetch_one(&state.pg)
            .await;

    let total_projects = match total_projects {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to fetch total projects count: {:?}", e);
            None
        }
    };

    Json(ServerStatus {
        last_refreshed_at: refreshed_at,
        total_projects,
    })
}

pub async fn set_last_refreshed_at(ts: OffsetDateTime) {
    *last_refreshed_at().write().await = Some(ts.unix_timestamp());
}
