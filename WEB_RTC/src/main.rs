mod analytics;
mod auth;
mod config;
mod events;
mod recording;
mod room;
mod api;
mod sfu;
mod sse;
mod error;
mod turn_server;
mod webhook;

use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue, Method},
    middleware::{self, Next},
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

// ─── AppState ───────────────────────────────────────────────────────────────

pub struct AppState {
    pub rooms: std::sync::RwLock<HashMap<String, Arc<room::Room>>>,
    pub api_keys: std::sync::RwLock<HashMap<String, auth::ApiKey>>,
    pub jwt_secret: String,
    pub config: config::Config,
    pub event_bus: events::EventBus,
    pub webhooks: webhook::WebhookStore,
    pub analytics: analytics::AnalyticsStore,
    pub recording: Option<Arc<recording::RecordingManager>>,
}

// ─── Page handlers ─────────────────────────────────────────────────────────

async fn landing_handler() -> Html<&'static str> {
    Html(include_str!("../static/landing.html"))
}

async fn playground_handler() -> Html<&'static str> {
    Html(include_str!("../index.html"))
}

async fn dashboard_handler() -> Html<&'static str> {
    Html(include_str!("../static/dashboard.html"))
}

async fn docs_handler() -> Html<&'static str> {
    Html(include_str!("../static/docs.html"))
}

// ─── Health endpoint ────────────────────────────────────────────────────────

async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rooms = state.rooms.read().unwrap();
    let rooms_active = rooms.len();
    let subscribers_active: u64 = rooms
        .values()
        .map(|r| r.subscriber_count())
        .sum();

    Json(serde_json::json!({
        "status": "ok",
        "version": "0.3.0",
        "rooms_active": rooms_active,
        "subscribers_active": subscribers_active,
        "tls_enabled": state.config.tls_enabled,
        "turn_embedded": state.config.turn_embedded,
    }))
}

// ─── Version header middleware ──────────────────────────────────────────────

async fn version_header_middleware(
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        HeaderName::from_static("x-liverelay-version"),
        HeaderValue::from_static("0.3.0"),
    );
    response
}

// ─── CORS configuration ────────────────────────────────────────────────────

fn build_cors_layer(allowed_origins: &str) -> CorsLayer {
    if allowed_origins == "*" {
        warn!("CORS: permissive mode (allow all origins) — not suitable for production");
        CorsLayer::permissive()
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<HeaderValue>().expect("invalid origin header value"))
            .collect();

        info!("CORS: restricted to {} origin(s)", origins.len());

        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("authorization"),
            ])
    }
}

// ─── TLS configuration ─────────────────────────────────────────────────────

/// Load TLS certificate and key from PEM files and build an
/// `axum_server::tls_rustls::RustlsConfig`.
async fn load_tls_config(
    cert_path: &str,
    key_path: &str,
) -> Result<axum_server::tls_rustls::RustlsConfig, Box<dyn std::error::Error>> {
    info!("Loading TLS certificate from: {}", cert_path);
    info!("Loading TLS private key from:  {}", key_path);

    let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
        cert_path,
        key_path,
    )
    .await?;

    info!("TLS configuration loaded successfully");
    Ok(config)
}

// ─── Entry point ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // ── Install rustls CryptoProvider (required by rustls 0.23+) ────────
    // Must happen before any TLS/DTLS operation (including WebRTC).
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // ── Load configuration ──────────────────────────────────────────────
    // Load .env before anything else so LIVERELAY_LOG_LEVEL is available.
    let _ = dotenvy::dotenv();

    let log_level = std::env::var("LIVERELAY_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    // Initialize tracing with configurable log level.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&log_level)),
        )
        .init();

    let cfg = config::Config::from_env();

    // ── Start embedded TURN server (if configured) ──────────────────────

    let _turn_handle = if cfg.turn_embedded {
        match turn_server::start_embedded_turn(&cfg).await {
            Ok(server) => {
                info!("Embedded TURN server is running");
                Some(server)
            }
            Err(e) => {
                error!("Failed to start embedded TURN server: {e}");
                error!("Continuing without TURN — NAT traversal may fail");
                None
            }
        }
    } else {
        if cfg.turn_urls.is_empty() {
            warn!("No TURN server configured — clients behind symmetric NAT will fail to connect");
        }
        None
    };

    // ── Bootstrap API key ───────────────────────────────────────────────

    let bootstrap_key = std::env::var("LIVERELAY_API_KEY")
        .unwrap_or_else(|_| auth::generate_api_key());
    let mut initial_keys = HashMap::new();
    initial_keys.insert(
        bootstrap_key.clone(),
        auth::ApiKey {
            key: bootstrap_key.clone(),
            name: "bootstrap".to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    );
    info!("Bootstrap API key: {bootstrap_key}");

    let bind_addr = cfg.bind_addr.clone();
    let tls_enabled = cfg.tls_enabled;
    let tls_cert_path = cfg.tls_cert_path.clone();
    let tls_key_path = cfg.tls_key_path.clone();
    let allowed_origins = cfg.allowed_origins.clone();

    let event_bus = events::EventBus::new();
    let webhook_store = webhook::WebhookStore::new();
    let analytics_store = analytics::AnalyticsStore::new();

    // ── Recording subsystem ────────────────────────────────────────────
    let recording_dir = std::env::var("LIVERELAY_RECORDING_DIR")
        .unwrap_or_else(|_| "./recordings".to_string());
    let recording_max_secs = std::env::var("LIVERELAY_RECORDING_MAX_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let recording_mgr = Arc::new(recording::RecordingManager::new(
        recording::RecordingConfig {
            base_dir: std::path::PathBuf::from(&recording_dir),
            max_duration_secs: recording_max_secs,
        },
    ));
    info!("Recording directory: {recording_dir}");

    let state = Arc::new(AppState {
        rooms: std::sync::RwLock::new(HashMap::new()),
        api_keys: std::sync::RwLock::new(initial_keys),
        jwt_secret: cfg.jwt_secret.clone(),
        config: cfg,
        event_bus: event_bus.clone(),
        webhooks: webhook_store.clone(),
        analytics: analytics_store.clone(),
        recording: Some(recording_mgr),
    });

    // ── Start background event consumers ────────────────────────────────

    // Webhook dispatcher: delivers events to registered webhook endpoints.
    let _webhook_handle = webhook::spawn_webhook_dispatcher(
        event_bus.clone(),
        webhook_store,
        webhook::RetryPolicy::default(),
    );

    // Analytics stats collector: gathers WebRTC quality metrics every 5s.
    let _stats_handle = analytics::spawn_stats_collector(
        state.clone(),
        std::time::Duration::from_secs(5),
        analytics::QualityThresholds::default(),
    );

    // ── Build CORS layer ────────────────────────────────────────────────

    let cors = build_cors_layer(&allowed_origins);

    // ── Build router ────────────────────────────────────────────────────

    let app = Router::new()
        // Frontend pages
        .route("/", get(landing_handler))
        .route("/playground", get(playground_handler))
        .route("/dashboard", get(dashboard_handler))
        .route("/docs", get(docs_handler))
        .nest_service("/static", ServeDir::new("static"))
        // Health (no auth required)
        .route("/health", get(health_handler))
        // ICE server config (requires JWT)
        .route("/v1/ice-servers", get(turn_server::get_ice_servers))
        // REST API (v1)
        .route("/v1/rooms", post(api::create_room))
        .route("/v1/rooms", get(api::list_rooms))
        .route("/v1/rooms/:room_id", get(api::get_room))
        .route("/v1/rooms/:room_id", delete(api::delete_room))
        .route("/v1/rooms/:room_id/token", post(api::create_room_token))
        .route("/v1/keys", post(api::create_api_key))
        // Webhooks API
        .route("/v1/webhooks", post(webhook::create_webhook))
        .route("/v1/webhooks", get(webhook::list_webhooks))
        .route("/v1/webhooks/:webhook_id", delete(webhook::delete_webhook))
        // Server-Sent Events (real-time event stream)
        .route("/v1/events", get(sse::sse_events))
        // Analytics API
        .route("/v1/analytics", get(analytics::get_analytics))
        // Recording API
        .route("/v1/rooms/:room_id/recording/start", post(recording::start_recording))
        .route("/v1/rooms/:room_id/recording/stop", post(recording::stop_recording))
        .route("/v1/rooms/:room_id/recordings", get(recording::list_room_recordings))
        // SFU WebRTC signaling
        .route("/sfu/publish", post(sfu::sfu_publish))
        .route("/sfu/subscribe", post(sfu::sfu_subscribe))
        .route("/sfu/call", post(sfu::sfu_call))
        .route("/sfu/conference", post(sfu::sfu_conference))
        .route("/sfu/conference/subscribe", post(sfu::sfu_conference_subscribe))
        // Middleware
        .layer(middleware::from_fn(version_header_middleware))
        .layer(cors)
        .with_state(state);

    // ── Start server (plain HTTP or HTTPS) ──────────────────────────────

    if tls_enabled {
        let cert_path = tls_cert_path.as_deref().expect(
            "LIVERELAY_TLS_CERT_PATH must be set when TLS is enabled",
        );
        let key_path = tls_key_path.as_deref().expect(
            "LIVERELAY_TLS_KEY_PATH must be set when TLS is enabled",
        );

        let tls_config = load_tls_config(cert_path, key_path)
            .await
            .expect("Failed to load TLS configuration");

        info!("LiveRelay SFU listening on https://{bind_addr}");
        let addr: std::net::SocketAddr = bind_addr.parse().expect("invalid bind address");

        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        let protocol = "http";
        info!("LiveRelay SFU listening on {protocol}://{bind_addr}");

        let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}
