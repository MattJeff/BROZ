use axum::{routing::{get, post}, Router};
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
use broz_shared::clients::email::EmailClient;
use broz_shared::clients::rabbitmq::RabbitMQClient;
use broz_shared::clients::redis::RedisClient;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub struct AppState {
    pub db: DbPool,
    pub config: AppConfig,
    pub rabbitmq: RabbitMQClient,
    pub redis: RedisClient,
    pub email: EmailClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    broz_shared::middleware::init_tracing("broz-auth");

    let config = AppConfig::load()?;
    let port = config.port;

    let manager = ConnectionManager::<PgConnection>::new(&config.database_url);
    let db = Pool::builder().max_size(10).build(manager)?;

    let rabbitmq = RabbitMQClient::connect(&config.rabbitmq_url).await?;
    let redis = RedisClient::connect(&config.redis_url).await?;
    let email = EmailClient::new(&config.resend_api_key, &config.from_email, "BROZ");

    let state = Arc::new(AppState { db, config, rabbitmq, redis, email });

    // Spawn RabbitMQ subscriber for sanction events
    let sub_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = events::subscriber::listen_sanction_issued(sub_state).await {
            tracing::error!(error = %e, "sanction subscriber failed");
        }
    });

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/signup", post(routes::register::register))
        .route("/login", post(routes::login::login))
        .route("/verify-email", post(routes::verify_email::verify_email))
        .route("/resend-code", post(routes::resend_code::resend_code))
        .route("/refresh", post(routes::refresh::refresh_token))
        .route("/logout", post(routes::logout::logout))
        .route("/forgot-password", post(routes::forgot_password::forgot_password))
        .route("/reset-password", post(routes::reset_password::reset_password))
        .route("/me", get(routes::me::me))
        .route("/oauth/google", post(routes::oauth::google_oauth))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(addr = %addr, "broz-auth starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
