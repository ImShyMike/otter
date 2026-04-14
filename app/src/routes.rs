use axum::{Router, routing::get};

use crate::handlers;
use crate::state::AppState;

pub fn build() -> Router<AppState> {
    Router::new()
        .route("/", get(|| async { "API is up!" }))
        .route("/health", get(handlers::health))
        .nest(
            "/api",
            Router::new()
                .route("/search", get(handlers::search))
                .route("/ysws_programs", get(handlers::ysws_programs))
                .route("/query", get(handlers::query)),
        )
}
