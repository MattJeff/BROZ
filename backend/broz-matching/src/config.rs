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
    #[serde(default = "default_user_service_url")]
    pub user_service_url: String,
}

fn default_port() -> u16 { 3003 }
fn default_db() -> String { "postgres://brozadmin:password@localhost:5432/broz_matching".into() }
fn default_rabbitmq() -> String { "amqp://guest:guest@localhost:5672/%2f".into() }
fn default_redis() -> String { "redis://localhost:6379".into() }
fn default_jwt_secret() -> String { "development-secret-change-in-production".into() }
fn default_user_service_url() -> String { "http://localhost:3002".into() }

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("BROZ_MATCHING").separator("__"))
            .build()?;
        Ok(config.try_deserialize().unwrap_or_else(|_| Self {
            port: default_port(),
            database_url: default_db(),
            rabbitmq_url: default_rabbitmq(),
            redis_url: default_redis(),
            jwt_secret: default_jwt_secret(),
            user_service_url: default_user_service_url(),
        }))
    }
}
