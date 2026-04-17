use axum::{Router, routing::get};
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
        .route("/explorer", get(handlers::explorer))
        .route("/health", get(handlers::health))
        .nest("/api", handlers::api::router())
        .split_for_parts();

    router.merge(Scalar::with_url("/scalar", api))
}
