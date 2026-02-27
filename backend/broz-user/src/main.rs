use axum::{routing::{get, post, put, patch, delete}, Router, extract::DefaultBodyLimit};
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
mod services;

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    broz_shared::middleware::init_tracing("broz-user");

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

    let state = Arc::new(AppState { db, config, rabbitmq, redis, minio });

    // Spawn RabbitMQ subscriber for user.registered events
    let sub_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = events::subscriber::listen_user_registered(sub_state).await {
            tracing::error!(error = %e, "user.registered subscriber failed");
        }
    });

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/me", get(routes::profile::get_profile).patch(routes::profile::update_profile))
        .route("/onboarding", post(routes::profile::complete_onboarding))
        .route("/check-pseudo", get(routes::profile::check_display_name))
        .route("/search", get(routes::search::search_users))
        .route("/follows/:id", post(routes::follows::send_follow_request).delete(routes::follows::remove_follow))
        .route("/follows/:id/respond", put(routes::follows::respond_follow))
        .route("/followers", get(routes::follows::list_followers))
        .route("/following", get(routes::follows::list_following))
        .route("/likes", post(routes::likes::send_like))
        .route("/likes/check/:target_id", get(routes::likes::check_like))
        .route("/photo", post(routes::photo::upload_photo)
            .layer(DefaultBodyLimit::max(10 * 1024 * 1024)))
        // Internal service-to-service endpoints (no auth)
        .route("/internal/presence", post(routes::internal::update_presence))
        .route("/internal/follower-ids/:id", get(routes::internal::get_follower_ids))
        .route("/internal/profiles/batch", post(routes::internal::batch_profiles))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "broz-user starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
