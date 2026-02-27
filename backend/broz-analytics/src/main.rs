use axum::routing::get;
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
    broz_shared::middleware::init_tracing("broz-analytics");

    let config = AppConfig::load()?;
    let port = config.port;

    // Set JWT_SECRET env var so the shared auth extractor can read it
    std::env::set_var("JWT_SECRET", &config.jwt_secret);

    let manager = ConnectionManager::<PgConnection>::new(&config.database_url);
    let db = Pool::builder().max_size(10).build(manager)?;

    let rabbitmq = RabbitMQClient::connect(&config.rabbitmq_url).await?;

    let state = Arc::new(AppState { db, config, rabbitmq });

    // Spawn RabbitMQ subscriber for all events
    let sub_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = events::subscriber::listen_all_events(sub_state).await {
            tracing::error!(error = %e, "analytics event subscriber failed");
        }
    });

    // Spawn hourly aggregation task
    services::aggregation::spawn_aggregation_task(state.clone());

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/stats/overview", get(routes::stats::get_overview))
        .route("/stats/daily", get(routes::stats::get_daily_stats))
        .route("/stats/events", get(routes::stats::get_events))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "broz-analytics starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
