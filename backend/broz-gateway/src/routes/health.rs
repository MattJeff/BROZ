use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use broz_shared::{HealthCheck, HealthResponse, HealthStatus};
use std::sync::Arc;

use crate::AppState;

/// Health check that probes all downstream services.
pub async fn health_check(State(state): State<Arc<AppState>>) -> Response {
    let services = [
        ("auth", &state.config.auth_url),
        ("user", &state.config.user_url),
        ("matching", &state.config.matching_url),
        ("messaging", &state.config.messaging_url),
        ("notification", &state.config.notification_url),
        ("moderation", &state.config.moderation_url),
        ("analytics", &state.config.analytics_url),
    ];

    let mut checks = Vec::with_capacity(services.len());

    for (name, url) in &services {
        let health_url = format!("{url}/health");
        let check = match state.http_client.get(&health_url).timeout(std::time::Duration::from_secs(3)).send().await {
            Ok(resp) if resp.status().is_success() => HealthCheck {
                name: name.to_string(),
                status: HealthStatus::Healthy,
                message: None,
            },
            Ok(resp) => HealthCheck {
                name: name.to_string(),
                status: HealthStatus::Degraded,
                message: Some(format!("status {}", resp.status())),
            },
            Err(e) => HealthCheck {
                name: name.to_string(),
                status: HealthStatus::Unhealthy,
                message: Some(format!("{e}")),
            },
        };
        checks.push(check);
    }

    let response = HealthResponse::healthy("broz-gateway", env!("CARGO_PKG_VERSION"))
        .with_checks(checks);

    let status = match response.status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK,
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status, Json(response)).into_response()
}

/// Returns Prometheus metrics.
pub async fn metrics(State(state): State<Arc<AppState>>) -> String {
    state.metrics_handle.render()
}
