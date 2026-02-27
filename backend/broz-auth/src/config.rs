use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_db")]
    pub database_url: String,
    #[serde(default = "default_rabbitmq")]
    pub rabbitmq_url: String,
    #[serde(default = "default_redis")]
    pub redis_url: String,
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_access_ttl")]
    pub jwt_access_ttl: i64,
    #[serde(default = "default_refresh_ttl")]
    pub jwt_refresh_ttl: i64,
    #[serde(default = "default_resend_api_key")]
    pub resend_api_key: String,
    #[serde(default = "default_from_email")]
    pub from_email: String,
    #[serde(default = "default_google_client_id")]
    pub google_client_id: String,
    #[serde(default = "default_google_client_secret")]
    pub google_client_secret: String,
    #[serde(default = "default_google_redirect_uri")]
    pub google_redirect_uri: String,
}

fn default_port() -> u16 { 3001 }
fn default_db() -> String { "postgres://brozadmin:password@localhost:5432/broz_auth".into() }
fn default_rabbitmq() -> String { "amqp://guest:guest@localhost:5672/%2f".into() }
fn default_redis() -> String { "redis://localhost:6379".into() }
fn default_jwt_secret() -> String { "development-secret-change-in-production".into() }
fn default_access_ttl() -> i64 { 3600 }
fn default_refresh_ttl() -> i64 { 2592000 }
fn default_resend_api_key() -> String { "re_test_key".into() }
fn default_from_email() -> String { "noreply@brozr.com".into() }
fn default_google_client_id() -> String { String::new() }
fn default_google_client_secret() -> String { String::new() }
fn default_google_redirect_uri() -> String { "http://localhost:5173/auth/callback".into() }

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("BROZ_AUTH").separator("__"))
            .build()?;
        Ok(config.try_deserialize().unwrap_or_else(|_| Self {
            port: default_port(),
            database_url: default_db(),
            rabbitmq_url: default_rabbitmq(),
            redis_url: default_redis(),
            jwt_secret: default_jwt_secret(),
            jwt_access_ttl: default_access_ttl(),
            jwt_refresh_ttl: default_refresh_ttl(),
            resend_api_key: default_resend_api_key(),
            from_email: default_from_email(),
            google_client_id: default_google_client_id(),
            google_client_secret: default_google_client_secret(),
            google_redirect_uri: default_google_redirect_uri(),
        }))
    }
}
