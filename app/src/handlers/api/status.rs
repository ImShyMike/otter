use axum::Json;
use serde::Serialize;
use time::OffsetDateTime;
use tracing::instrument;
use utoipa::ToSchema;

use std::sync::OnceLock;
use tokio::sync::RwLock;

#[derive(Serialize, ToSchema)]
pub struct ServerStatus {
    pub last_refreshed_at: Option<i64>,
}

static LAST_REFRESHED_AT: OnceLock<RwLock<Option<i64>>> = OnceLock::new();

fn last_refreshed_at() -> &'static RwLock<Option<i64>> {
    LAST_REFRESHED_AT.get_or_init(|| RwLock::new(None))
}

#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = 200, description = "Server status data", body = ServerStatus),
    )
)]
#[instrument]
pub async fn data_refresh_status() -> Json<ServerStatus> {
    let refreshed_at = *last_refreshed_at().read().await;

    Json(ServerStatus {
        last_refreshed_at: refreshed_at,
    })
}

pub async fn set_last_refreshed_at(ts: OffsetDateTime) {
    *last_refreshed_at().write().await = Some(ts.unix_timestamp());
}
