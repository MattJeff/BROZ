use std::sync::Arc;

use axum::routing::{get, post, put};
use axum::Router;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use socketioxide::SocketIo;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod config;
mod events;
mod matching;
mod models;
mod routes;
mod schema;
mod socket;

use config::AppConfig;
use broz_shared::clients::rabbitmq::RabbitMQClient;
use broz_shared::clients::redis::RedisClient;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub struct AppState {
    pub db: DbPool,
    pub config: AppConfig,
    pub rabbitmq: RabbitMQClient,
    pub redis: RedisClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    broz_shared::middleware::init_tracing("broz-matching");

    let config = AppConfig::load()?;
    let port = config.port;

    // Database pool
    let manager = ConnectionManager::<PgConnection>::new(&config.database_url);
    let db = Pool::builder().max_size(10).build(manager)?;

    // Infrastructure clients
    let rabbitmq = RabbitMQClient::connect(&config.rabbitmq_url).await?;
    let redis = RedisClient::connect(&config.redis_url).await?;

    let state = Arc::new(AppState {
        db,
        config,
        rabbitmq,
        redis,
    });

    // Socket.IO setup
    let (sio_layer, io) = SocketIo::builder()
        .with_state(state.clone())
        .build_layer();

    io.ns("/", socket::handlers::on_connect);

    // Axum router with REST endpoints + Socket.IO layer
    let app = Router::new()
        // Health
        .route("/health", get(routes::health::health_check))
        // LiveCam REST endpoints
        .route("/livecam/request", post(routes::livecam::create_livecam_request))
        .route(
            "/livecam/:id/respond",
            put(routes::livecam::respond_livecam_request),
        )
        .route(
            "/livecam/pending",
            get(routes::livecam::get_pending_requests),
        )
        .layer(sio_layer)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "broz-matching starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
