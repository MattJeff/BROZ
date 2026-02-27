use axum::body::Body;
use axum::extract::{OriginalUri, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use broz_shared::ApiErrorResponse;
use std::sync::Arc;

use crate::AppState;
use super::auth::extract_auth_user;
use super::rate_limit::check_rate_limit;

/// Paths that do not require JWT authentication.
const PUBLIC_PATHS: &[&str] = &[
    "/api/auth/signup",
    "/api/auth/login",
    "/api/auth/refresh",
    "/api/auth/oauth",
    "/api/auth/forgot-password",
    "/api/auth/reset-password",
];

/// Headers that must not be forwarded (hop-by-hop).
const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "host",
];

/// Determines whether a given path is public (no JWT required).
fn is_public(path: &str) -> bool {
    PUBLIC_PATHS.iter().any(|p| path.starts_with(p))
}

/// Strip the gateway prefix from the path to produce the upstream path.
///
/// Mapping:
/// - /api/auth/...          -> /...           (strip "/api/auth")
/// - /api/users/...         -> /...           (strip "/api/users")
/// - /api/follows/...       -> /follows/...   (strip "/api", keep /follows)
/// - /api/messages/...      -> /...           (strip "/api/messages")
/// - /api/notifications/... -> /notifications/... (strip "/api", keep /notifications)
/// - /api/interactions/...  -> /...           (strip "/api/interactions")
/// - /api/livecam/...       -> /livecam/...   (strip "/api", keep /livecam)
/// - /api/admin/...         -> /admin/...     (strip "/api", keep /admin)
/// - /api/analytics/...     -> /...           (strip "/api/analytics")
fn strip_prefix(path: &str) -> &str {
    // Routes that keep their second segment as part of the upstream path
    if path.starts_with("/api/follows") {
        return &path[4..]; // "/api/follows/123" -> "/follows/123"
    }
    if path.starts_with("/api/admin") {
        return &path[4..]; // "/api/admin/reports" -> "/admin/reports"
    }
    if path.starts_with("/api/livecam") {
        return &path[4..]; // "/api/livecam/request" -> "/livecam/request"
    }
    if path.starts_with("/api/notifications") {
        return &path[4..]; // "/api/notifications/unread-count" -> "/notifications/unread-count"
    }

    // All other routes: strip both /api and the service segment
    let prefixes = &[
        "/api/auth",
        "/api/users",
        "/api/messages",
        "/api/interactions",
        "/api/analytics",
    ];

    for prefix in prefixes {
        if path.starts_with(prefix) {
            let rest = &path[prefix.len()..];
            if rest.is_empty() {
                return "/";
            }
            return rest; // e.g. "/api/auth/login" -> "/login"
        }
    }

    // Fallback: forward as-is
    path
}

/// The catch-all proxy handler.
///
/// 1. Extract path from OriginalUri
/// 2. Resolve upstream service (404 if unknown prefix)
/// 3. For non-public paths: validate JWT and check rate limits
/// 4. Strip the gateway prefix to build the upstream path
/// 5. Forward the request (method, headers, body, query string)
/// 6. Return the upstream response
pub async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    OriginalUri(original_uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let path = original_uri.path();
    let query = original_uri.query();

    // 1. Resolve upstream
    let upstream_base = match state.config.resolve_upstream(path) {
        Some(url) => url,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiErrorResponse::new("E0003", "no upstream service for this path")),
            )
                .into_response();
        }
    };

    // 2. Auth + rate limit for non-public paths
    if !is_public(path) {
        let auth_info = match extract_auth_user(&headers, &state.config.jwt_secret) {
            Ok(info) => info,
            Err(resp) => return resp,
        };

        if let Err(status) = check_rate_limit(
            &state.redis,
            auth_info.user_id,
            auth_info.role,
            &state.config,
        )
        .await
        {
            return (
                status,
                Json(ApiErrorResponse::new("E0006", "rate limit exceeded")),
            )
                .into_response();
        }
    }

    // 3. Build upstream URL
    let upstream_path = strip_prefix(path);
    let upstream_url = match query {
        Some(q) => format!("{upstream_base}{upstream_path}?{q}"),
        None => format!("{upstream_base}{upstream_path}"),
    };

    // 4. Read body (max 10 MB)
    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ApiErrorResponse::new("E0009", "request body too large (max 10MB)")),
            )
                .into_response();
        }
    };

    // 5. Build upstream request
    let mut upstream_req = state
        .http_client
        .request(
            reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
            &upstream_url,
        )
        .body(body_bytes.to_vec());

    // Forward headers, skipping hop-by-hop
    for (name, value) in headers.iter() {
        let name_lower = name.as_str().to_lowercase();
        if HOP_BY_HOP_HEADERS.contains(&name_lower.as_str()) {
            continue;
        }
        if let Ok(val) = value.to_str() {
            upstream_req = upstream_req.header(name.as_str(), val);
        }
    }

    // 6. Send and return upstream response
    let upstream_resp = match upstream_req.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, upstream = %upstream_url, "upstream request failed");
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiErrorResponse::new("E0007", format!("upstream unavailable: {e}"))),
            )
                .into_response();
        }
    };

    let status = StatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut response_headers = HeaderMap::new();
    for (name, value) in upstream_resp.headers().iter() {
        let name_lower = name.as_str().to_lowercase();
        if HOP_BY_HOP_HEADERS.contains(&name_lower.as_str()) {
            continue;
        }
        if let (Ok(hn), Ok(hv)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            response_headers.insert(hn, hv);
        }
    }

    let resp_body = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "failed to read upstream response body");
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiErrorResponse::new("E0007", "failed to read upstream response")),
            )
                .into_response();
        }
    };

    (status, response_headers, resp_body).into_response()
}
