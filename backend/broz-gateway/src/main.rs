use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use axum::http::{header, Method};
use tower_http::cors::{AllowHeaders, AllowMethods, CorsLayer};
use tower_http::trace::TraceLayer;

use broz_gateway::config::AppConfig;
use broz_gateway::routes::{health, proxy};
use broz_gateway::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    broz_shared::middleware::init_tracing("broz-gateway");

    // Load configuration
    let config = AppConfig::load()?;
    let port = config.port;

    // Connect to Redis
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_conn = redis_client.get_connection_manager().await?;
    tracing::info!(url = %config.redis_url, "connected to Redis");

    // Initialize Prometheus metrics
    let metrics_handle = broz_shared::middleware::init_metrics();

    // Build HTTP client for upstream proxying
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Build shared state
    let state = Arc::new(AppState {
        config,
        http_client,
        redis: tokio::sync::Mutex::new(redis_conn),
        metrics_handle,
    });

    // Build router
    let app = Router::new()
        .route("/health", get(health::health_check))
        .route("/metrics", get(health::metrics))
        .fallback(proxy::proxy_handler)
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:3000".parse().unwrap(),
                    "http://127.0.0.1:3000".parse().unwrap(),
                ])
                .allow_methods(AllowMethods::list([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ]))
                .allow_headers(AllowHeaders::list([
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    header::ACCEPT,
                ]))
                .allow_credentials(true),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "broz-gateway starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
