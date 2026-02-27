use axum::routing::{get, post};
use axum::Router;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod config;
mod events;
mod models;
mod routes;
mod schema;
mod services;

use config::AppConfig;
use broz_shared::clients::rabbitmq::RabbitMQClient;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub struct AppState {
    pub db: DbPool,
    pub config: AppConfig,
    pub rabbitmq: RabbitMQClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    broz_shared::middleware::init_tracing("broz-notification");

    let config = AppConfig::load()?;
    let port = config.port;

    // Set JWT_SECRET env var for the auth extractor middleware
    std::env::set_var("JWT_SECRET", &config.jwt_secret);

    let manager = ConnectionManager::<PgConnection>::new(&config.database_url);
    let db = Pool::builder().max_size(10).build(manager)?;

    let rabbitmq = RabbitMQClient::connect(&config.rabbitmq_url).await?;

    let state = Arc::new(AppState { db, config, rabbitmq });

    // Spawn follow event subscriber
    let follow_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = events::subscriber::listen_follow_events(follow_state).await {
            tracing::error!(error = %e, "follow event subscriber failed");
        }
    });

    // Spawn like event subscriber
    let like_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = events::subscriber::listen_like_events(like_state).await {
            tracing::error!(error = %e, "like event subscriber failed");
        }
    });

    // Spawn message event subscriber
    let message_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = events::subscriber::listen_message_events(message_state).await {
            tracing::error!(error = %e, "message event subscriber failed");
        }
    });

    // Spawn sanction event subscriber
    let sanction_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = events::subscriber::listen_sanction_events(sanction_state).await {
            tracing::error!(error = %e, "sanction event subscriber failed");
        }
    });

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/notifications", get(routes::notifications::list_notifications))
        .route("/notifications/unread-count", get(routes::notifications::unread_count))
        .route("/notifications/mark-all-read", post(routes::notifications::mark_all_read))
        .route("/notifications/:id/read", post(routes::notifications::mark_read))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "broz-notification starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
