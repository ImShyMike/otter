use std::env;

use axum::response::Redirect;
use tracing::instrument;

use crate::error::AppError;

#[instrument()]
pub async fn homepage() -> Result<Redirect, AppError> {
    let fontend_url =
        env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    Ok(Redirect::temporary(&fontend_url))
}
