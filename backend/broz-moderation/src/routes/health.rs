use axum::Json;
use broz_shared::types::api::HealthResponse;

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::healthy("broz-moderation", env!("CARGO_PKG_VERSION")))
}
