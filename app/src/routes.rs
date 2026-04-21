use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable as ScalarServable};

use crate::handlers;
use crate::state::AppState;

pub fn build() -> Router<AppState> {
    #[derive(OpenApi)]
    struct ApiDoc;

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .route("/", get(|| async { "API is up!" }))
        .route("/health", get(handlers::health))
        .nest("/api", handlers::api::router())
        .split_for_parts();

    router
        .merge(Scalar::with_url("/docs", api))
        .layer(CorsLayer::very_permissive())
        .layer(
            TraceLayer::new_for_http().make_span_with(
                tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO),
            ),
        )
}
