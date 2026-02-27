use axum::routing::{get, post, put, delete};
use axum::Router;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::pg::PgConnection;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod config;
mod events;
mod models;
mod routes;
mod schema;

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
    broz_shared::middleware::init_tracing("broz-moderation");

    let config = AppConfig::load()?;
    let port = config.port;

    let manager = ConnectionManager::<PgConnection>::new(&config.database_url);
    let db = Pool::builder().max_size(10).build(manager)?;

    let rabbitmq = RabbitMQClient::connect(&config.rabbitmq_url).await?;

    let state = Arc::new(AppState { db, config, rabbitmq });

    let admin_routes = Router::new()
        .route("/reports", get(routes::admin_routes::list_reports))
        .route("/reports/:id", get(routes::admin_routes::get_report))
        .route("/reports/:id/review", put(routes::admin_routes::review_report))
        .route("/users/:id", get(routes::admin_routes::get_user_sanctions))
        .route("/users/:id/sanction", post(routes::admin_routes::issue_sanction))
        .route("/users/:id/sanction/:sid", delete(routes::admin_routes::lift_sanction))
        .route("/sanctions", get(routes::admin_routes::list_active_sanctions))
        .route("/stats", get(routes::admin_routes::get_stats))
        .route("/audit-log", get(routes::admin_routes::get_audit_log));

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/report", post(routes::user_routes::create_report))
        .nest("/admin", admin_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "broz-moderation starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
