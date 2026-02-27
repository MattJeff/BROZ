use axum::{routing::{get, post, put, delete}, Router, extract::DefaultBodyLimit};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::pg::PgConnection;
use std::sync::Arc;
use socketioxide::SocketIo;
use dashmap::DashMap;
use uuid::Uuid;
use crate::socket::handlers::CallSession;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod config;
mod events;
mod models;
mod routes;
mod schema;
mod socket;

use config::AppConfig;
use broz_shared::clients::rabbitmq::RabbitMQClient;
use broz_shared::clients::redis::RedisClient;
use broz_shared::clients::minio::MinioClient;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub struct AppState {
    pub db: DbPool,
    pub config: AppConfig,
    pub rabbitmq: RabbitMQClient,
    pub redis: RedisClient,
    pub minio: MinioClient,
    pub io: SocketIo,
    pub active_calls: DashMap<Uuid, CallSession>,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    broz_shared::middleware::init_tracing("broz-messaging");

    let config = AppConfig::load()?;
    let port = config.port;

    let manager = ConnectionManager::<PgConnection>::new(&config.database_url);
    let db = Pool::builder().max_size(10).build(manager)?;

    let rabbitmq = RabbitMQClient::connect(&config.rabbitmq_url).await?;
    let redis = RedisClient::connect(&config.redis_url).await?;

    let minio = MinioClient::new(
        &config.minio_endpoint,
        &config.minio_access_key,
        &config.minio_secret_key,
        &config.minio_bucket,
        &config.minio_public_url,
    )
    .await;

    // Build Socket.IO layer - we need io in AppState for emitting from REST routes
    let (sio_layer, io) = SocketIo::builder().build_layer();

    let http_client = reqwest::Client::new();
    let state = Arc::new(AppState { db, config, rabbitmq, redis, minio, io: io.clone(), active_calls: DashMap::new(), http_client });

    // Configure the Socket.IO namespace with state via closure
    io.ns("/", {
        let state = state.clone();
        move |socket: socketioxide::extract::SocketRef| {
            let state = state.clone();
            async move {
                socket::handlers::on_connect_with_state(socket, state).await;
            }
        }
    });

    // Spawn RabbitMQ subscriber for follow.accepted events
    let sub_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = events::subscriber::listen_follow_accepted(sub_state).await {
            tracing::error!(error = %e, "follow.accepted subscriber failed");
        }
    });

    let app = Router::new()
        // Health
        .route("/health", get(routes::health::health_check))
        // Conversations
        .route("/conversations", get(routes::conversations::list_conversations))
        .route("/conversations/group", post(routes::conversations::create_group))
        .route("/conversations/:id", get(routes::conversations::get_conversation))
        .route("/conversations/:id/members", post(routes::conversations::add_member))
        .route("/conversations/group/:id/photo", post(routes::conversations::update_group_photo)
            .layer(DefaultBodyLimit::max(50 * 1024 * 1024)))
        .route("/conversations/group/:id/name", put(routes::conversations::rename_group))
        // Messages
        .route("/conversations/:id/messages", get(routes::messages::list_messages).post(routes::messages::send_message))
        .route("/messages/:id", delete(routes::messages::delete_message))
        .route("/conversations/:id/read", post(routes::messages::mark_as_read))
        // Simple send (frontend-friendly)
        .route("/send", post(routes::messages::send_message_simple))
        // Media upload
        .route("/send-media", post(routes::messages::send_media)
            .layer(DefaultBodyLimit::max(50 * 1024 * 1024)))
        // Unread count
        .route("/unread-count", get(routes::messages::get_unread_count))
        .layer(sio_layer)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "broz-messaging starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
